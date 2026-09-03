//! First target-owned IR generation surface.
//!
//! This module contains the narrow v0.9 target slices. It lowers supported Core
//! effect nodes into in-memory Echo or git-warp review artifacts. It does not
//! execute a runtime, run a verifier, assemble bundles, or perform admission.

use std::collections::{BTreeMap, BTreeSet};

use crate::core_ir::{
    core_type_fits, is_lowercase_sha256_review_digest, resolved_core_type, CoreBudget, CoreExpr,
    CoreExternalActionBudget, CoreImportKind, CoreIntent, CoreModule, CoreNode, CoreObstructionArm,
    CoreObstructionReason, CorePredicate, CoreRequireFailureArm, CoreType, CoreValue,
    InputConstraint, LocalRef, ResourceRef, CORE_API_VERSION, CORE_APPLICATION_INPUT_LOCAL_ID,
};
use crate::digest_core_module;
use crate::lowerability::{LowerabilityEffectStatus, LowerabilityReport, LowerabilityStatus};
use crate::result_projection::emit_result_projection_with_closure;
use crate::{ResultProjectionArtifact, ResultProjectionFailure};

pub const ECHO_DPO_TARGET_PROFILE: &str = "echo.dpo@1";
pub const ECHO_SPAN_IR_DOMAIN: &str = "echo.span-ir/v1";
pub const GITWARP_REF_CRDT_TARGET_PROFILE: &str = "gitwarp.ref_crdt@1";
pub const GITWARP_COMMIT_REDUCER_IR_DOMAIN: &str = "gitwarp.commit-reducer-ir/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TargetSelection {
    target_ir_domain: &'static str,
    target_intrinsic_prefix: &'static str,
    supports_requirements: bool,
}

impl TargetSelection {
    fn supports_intrinsic(self, target_intrinsic: &str) -> bool {
        target_intrinsic
            .strip_prefix(self.target_intrinsic_prefix)
            .is_some_and(|suffix| suffix.starts_with('.'))
    }
}

fn target_selection_for_profile(target_profile: &str) -> Option<TargetSelection> {
    match target_profile {
        ECHO_DPO_TARGET_PROFILE => Some(TargetSelection {
            target_ir_domain: ECHO_SPAN_IR_DOMAIN,
            target_intrinsic_prefix: ECHO_DPO_TARGET_PROFILE,
            supports_requirements: true,
        }),
        GITWARP_REF_CRDT_TARGET_PROFILE => Some(TargetSelection {
            target_ir_domain: GITWARP_COMMIT_REDUCER_IR_DOMAIN,
            target_intrinsic_prefix: GITWARP_REF_CRDT_TARGET_PROFILE,
            supports_requirements: false,
        }),
        _ => None,
    }
}

/// Canonical non-generic helper signature owned by one exact imported lawpack.
///
/// Validated lawpack preparation projects this fact alongside the compiler
/// fact. The lawpack digest binds the exported helper implementation without
/// embedding application vocabulary in a target runtime.
///
/// Callers cannot fabricate helper authority from an arbitrary coordinate and
/// signature:
///
/// ```compile_fail
/// use edict_syntax::{ResourceRef, TargetPureFunctionFact};
///
/// let _forged = TargetPureFunctionFact {
///     lawpack: ResourceRef {
///         coordinate: "hello.echo@1".to_owned(),
///         digest: Some(format!("sha256:{}", "1".repeat(64))),
///     },
///     coordinate: "hello.echo@1.notExported".to_owned(),
///     type_parameters: Vec::new(),
///     parameter_types: vec!["U64".to_owned()],
///     return_type: "U64".to_owned(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetPureFunctionFact {
    lawpack: ResourceRef,
    coordinate: String,
    type_parameters: Vec<String>,
    parameter_types: Vec<String>,
    return_type: String,
}

impl TargetPureFunctionFact {
    pub(crate) fn from_validated_lawpack_export(
        lawpack: ResourceRef,
        coordinate: String,
        type_parameters: Vec<String>,
        parameter_types: Vec<String>,
        return_type: String,
    ) -> Self {
        Self {
            lawpack,
            coordinate,
            type_parameters,
            parameter_types,
            return_type,
        }
    }

    /// Exact digest-locked lawpack that exported this helper.
    #[must_use]
    pub fn lawpack(&self) -> &ResourceRef {
        &self.lawpack
    }

    /// Canonical exported helper coordinate.
    #[must_use]
    pub fn coordinate(&self) -> &str {
        &self.coordinate
    }

    /// Declared generic parameters; v1 lowering requires this to be empty.
    #[must_use]
    pub fn type_parameters(&self) -> &[String] {
        &self.type_parameters
    }

    /// Complete ordered parameter type coordinates.
    #[must_use]
    pub fn parameter_types(&self) -> &[String] {
        &self.parameter_types
    }

    /// Declared return type coordinate.
    #[must_use]
    pub fn return_type(&self) -> &str {
        &self.return_type
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrLoweringFacts {
    pub target_profile: ResourceRef,
    pub target_ir_domain: String,
    pub operation_profiles: Vec<String>,
    pub obstruction_coordinates: Vec<String>,
    pub effect_lowerings: Vec<TargetEffectLowering>,
    pub pure_functions: Vec<TargetPureFunctionFact>,
}

impl TargetIrLoweringFacts {
    /// Build Target IR lowering facts from an accepted native lowerability report.
    ///
    /// # Errors
    ///
    /// Returns `UnsupportedLowerabilityReport` when the lowerability report did
    /// not select native support. The v0.9 Target IR bridge does not
    /// derive target facts from unsupported or adapter-backed reports. It also
    /// cannot derive lawpack helper authority; helper-bearing Core must use
    /// validated lawpack preparation.
    pub fn from_lowerability_report(
        target_profile: ResourceRef,
        target_ir_domain: impl Into<String>,
        report: &LowerabilityReport,
    ) -> Result<Self, TargetLoweringFailure> {
        if report.status != LowerabilityStatus::Native {
            return Err(TargetLoweringFailure {
                kind: TargetLoweringFailureKind::UnsupportedLowerabilityReport,
                intent: None,
                node_index: None,
                detail: format!("{:?}", report.status),
            });
        }
        if target_profile.coordinate != report.target_profile {
            return Err(TargetLoweringFailure {
                kind: TargetLoweringFailureKind::UnsupportedTargetProfile,
                intent: None,
                node_index: None,
                detail: target_profile.coordinate,
            });
        }
        if !target_profile.is_digest_locked() {
            return Err(TargetLoweringFailure {
                kind: TargetLoweringFailureKind::UndigestedTargetProfile,
                intent: None,
                node_index: None,
                detail: target_profile
                    .digest
                    .unwrap_or_else(|| "<missing>".to_owned()),
            });
        }

        Ok(Self {
            target_profile,
            target_ir_domain: target_ir_domain.into(),
            operation_profiles: vec![report.operation_profile.clone()],
            obstruction_coordinates: report.obstruction_coordinates.clone(),
            effect_lowerings: selected_native_effect_lowerings(report),
            pure_functions: Vec::new(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetEffectLowering {
    pub effect: String,
    pub target_intrinsic: String,
    pub failure_mappings: BTreeMap<String, String>,
}

fn selected_native_effect_lowerings(report: &LowerabilityReport) -> Vec<TargetEffectLowering> {
    let mut seen = BTreeSet::new();
    let mut lowerings = Vec::new();
    for effect in &report.effect_results {
        let LowerabilityEffectStatus::Native { target_intrinsic } = &effect.status else {
            continue;
        };
        if seen.insert((effect.semantic_effect.as_str(), target_intrinsic.as_str())) {
            lowerings.push(TargetEffectLowering {
                effect: effect.semantic_effect.clone(),
                target_intrinsic: target_intrinsic.clone(),
                failure_mappings: BTreeMap::new(),
            });
        }
    }
    lowerings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLoweringStatus {
    Lowered,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLoweringFailureKind {
    UnsupportedTargetProfile,
    UnsupportedTargetIrDomain,
    UndigestedTargetProfile,
    UnsupportedTargetFeature,
    UnsupportedCoreNode,
    MissingOperationProfile,
    MissingObstruction,
    AmbiguousObstructionMapping,
    MissingEffectLowering,
    AmbiguousEffectLowering,
    UnsupportedLowerabilityReport,
    UnsupportedTargetIntrinsic,
    UnsupportedCoreAbi,
    UnsupportedCoreCapability,
    UndigestedCoreImport,
    InvalidCoreIdentity,
    NoTargetSteps,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetLoweringFailure {
    pub kind: TargetLoweringFailureKind,
    pub intent: Option<String>,
    pub node_index: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetLoweringReport {
    /// Overall Target IR lowering status.
    pub status: TargetLoweringStatus,
    /// Target IR artifact when general target lowering succeeds.
    pub artifact: Option<TargetIrArtifact>,
    /// Compiler-owned result projection per supported intent. This map may be
    /// empty even when `status` is `Lowered`; callers own admission cardinality.
    pub result_projections: BTreeMap<String, ResultProjectionArtifact>,
    /// Per-intent result-projection refusals. This map may be non-empty while
    /// `status` is `Lowered` and general `failures` remains empty.
    pub result_projection_failures: BTreeMap<String, ResultProjectionFailure>,
    /// General target-lowering failures.
    pub failures: Vec<TargetLoweringFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrArtifact {
    pub domain: String,
    pub target_profile: ResourceRef,
    pub source_core_coordinate: String,
    /// Exact semantic inputs for operation artifacts. Legacy artifacts without
    /// an explicit basis or lawpack remain byte-identical by omitting it.
    pub semantic_closure: Option<TargetIrSemanticClosure>,
    pub intents: BTreeMap<String, TargetIrIntent>,
}

/// Digest-locked Edict inputs whose meaning the Target IR artifact closes over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrSemanticClosure {
    pub source_core: ResourceRef,
    pub lawpacks: Vec<ResourceRef>,
    pub capabilities: Vec<ResourceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrIntent {
    pub operation_profile: String,
    pub basis: Option<CoreExpr>,
    pub input_constraints: Vec<InputConstraint>,
    pub core_evaluation_budget: CoreBudget,
    pub pure_bindings: Vec<TargetIrPureBinding>,
    pub requirements: Vec<TargetIrRequirement>,
    pub steps: Vec<TargetIrStep>,
    pub external_action_requests: Vec<TargetIrExternalActionRequest>,
    pub result: CoreExpr,
}

/// One source-ordered pure Core binding retained for target evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrPureBinding {
    pub id: String,
    pub binding: LocalRef,
    pub value: CoreExpr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrRequirement {
    pub id: String,
    pub predicate: CorePredicate,
    pub on_failure: TargetIrRequireFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetIrRequireFailure {
    Terminal { reason: CoreObstructionReason },
    ContinueObstructed { reason: CoreObstructionReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrStep {
    pub id: String,
    pub binding: LocalRef,
    pub effect: String,
    pub target_intrinsic: String,
    pub input: CoreExpr,
    pub obstruction_failures: Vec<String>,
    pub obstruction_arms: BTreeMap<String, CoreObstructionArm>,
}

/// A deterministic external-action request preserved as data for Echo admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrExternalActionRequest {
    pub id: String,
    pub binding: LocalRef,
    pub operation: ResourceRef,
    pub input_type: String,
    pub settlement_type: String,
    pub input_schema: ResourceRef,
    pub settlement_schema: ResourceRef,
    pub input: CoreExpr,
    pub authority_scope: CoreExpr,
    pub basis: CoreExpr,
    pub budget: CoreExternalActionBudget,
    pub reconciliation_law: ResourceRef,
}

#[must_use]
pub fn lower_to_target_ir(
    core: &CoreModule,
    facts: &TargetIrLoweringFacts,
) -> TargetLoweringReport {
    let target_selection = match validate_target_selection(facts) {
        Ok(target_selection) => target_selection,
        Err(failures) => return unsupported(failures),
    };
    let core_failures = validate_core_module(core, &facts.pure_functions);
    if !core_failures.is_empty() {
        return unsupported(core_failures);
    }
    let semantic_closure = match semantic_closure_for_core(core) {
        Ok(semantic_closure) => semantic_closure,
        Err(failure) => return unsupported(vec![failure]),
    };

    let effect_lowerings = effect_lowerings_by_coordinate(facts);
    let operation_profiles = facts
        .operation_profiles
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let obstruction_coordinates = facts
        .obstruction_coordinates
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let context = TargetLoweringContext {
        target_selection,
        obstruction_coordinates: &obstruction_coordinates,
        effect_lowerings: &effect_lowerings,
    };
    let mut failures = Vec::new();
    let mut intents = BTreeMap::new();

    for (intent_name, intent) in &core.intents {
        let lowered = lower_intent(
            intent_name,
            intent,
            &operation_profiles,
            &context,
            &mut failures,
        );
        intents.insert(intent_name.clone(), lowered);
    }

    if failures.is_empty() {
        let artifact = TargetIrArtifact {
            domain: facts.target_ir_domain.clone(),
            target_profile: facts.target_profile.clone(),
            source_core_coordinate: core.coordinate.clone(),
            semantic_closure,
            intents,
        };
        let (result_projections, result_projection_failures) =
            if let Some(expected_semantic_closure) = artifact.semantic_closure.as_ref() {
                lower_result_projections(core, &artifact, expected_semantic_closure)
            } else {
                (BTreeMap::new(), BTreeMap::new())
            };
        TargetLoweringReport {
            status: TargetLoweringStatus::Lowered,
            artifact: Some(artifact),
            result_projections,
            result_projection_failures,
            failures,
        }
    } else {
        unsupported(failures)
    }
}

fn lower_result_projections(
    core: &CoreModule,
    target_ir: &TargetIrArtifact,
    expected_semantic_closure: &TargetIrSemanticClosure,
) -> (
    BTreeMap<String, ResultProjectionArtifact>,
    BTreeMap<String, ResultProjectionFailure>,
) {
    let mut projections = BTreeMap::new();
    let mut failures = BTreeMap::new();
    for intent_name in target_ir.intents.keys() {
        match emit_result_projection_with_closure(
            core,
            target_ir,
            intent_name,
            expected_semantic_closure,
        ) {
            Ok(projection) => {
                projections.insert(intent_name.clone(), projection);
            }
            Err(failure) => {
                failures.insert(intent_name.clone(), failure);
            }
        }
    }
    (projections, failures)
}

pub(crate) fn semantic_closure_for_core(
    core: &CoreModule,
) -> Result<Option<TargetIrSemanticClosure>, TargetLoweringFailure> {
    let lawpacks = digest_locked_import_set(core, CoreImportKind::Lawpack, "lawpack")?;
    let capabilities = digest_locked_import_set(core, CoreImportKind::Capability, "capability")?;
    let has_explicit_basis = core.intents.values().any(|intent| intent.basis.is_some());
    let has_pure_bindings = core.intents.values().any(|intent| {
        intent
            .body
            .nodes
            .iter()
            .any(|node| matches!(node, CoreNode::Let { .. }))
    });
    if !has_explicit_basis && !has_pure_bindings && lawpacks.is_empty() && capabilities.is_empty() {
        return Ok(None);
    }

    let digest = digest_core_module(core).map_err(|error| TargetLoweringFailure {
        kind: TargetLoweringFailureKind::InvalidCoreIdentity,
        intent: None,
        node_index: None,
        detail: error.to_string(),
    })?;
    Ok(Some(TargetIrSemanticClosure {
        source_core: ResourceRef {
            coordinate: core.coordinate.clone(),
            digest: Some(digest.to_review_string()),
        },
        lawpacks: lawpacks.into_values().collect(),
        capabilities: capabilities.into_values().collect(),
    }))
}

fn digest_locked_import_set(
    core: &CoreModule,
    kind: CoreImportKind,
    field_name: &str,
) -> Result<BTreeMap<String, ResourceRef>, TargetLoweringFailure> {
    let mut resources = BTreeMap::new();
    for resource in core
        .imports
        .iter()
        .filter(|import| import.kind == kind)
        .map(|import| &import.resource)
    {
        if !resource
            .digest
            .as_deref()
            .is_some_and(is_lowercase_sha256_review_digest)
        {
            return Err(TargetLoweringFailure {
                kind: TargetLoweringFailureKind::UndigestedCoreImport,
                intent: None,
                node_index: None,
                detail: resource.coordinate.clone(),
            });
        }
        if let Some(prior) = resources.get(&resource.coordinate) {
            if prior != resource {
                return Err(TargetLoweringFailure {
                    kind: TargetLoweringFailureKind::InvalidCoreIdentity,
                    intent: None,
                    node_index: None,
                    detail: format!(
                        "{field_name} coordinate `{}` is bound to conflicting resources",
                        resource.coordinate
                    ),
                });
            }
        } else {
            resources.insert(resource.coordinate.clone(), resource.clone());
        }
    }
    Ok(resources)
}

fn validate_target_selection(
    facts: &TargetIrLoweringFacts,
) -> Result<TargetSelection, Vec<TargetLoweringFailure>> {
    let Some(target_selection) = target_selection_for_profile(&facts.target_profile.coordinate)
    else {
        return Err(vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetProfile,
            intent: None,
            node_index: None,
            detail: facts.target_profile.coordinate.clone(),
        }]);
    };
    if !facts.target_profile.is_digest_locked() {
        return Err(vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UndigestedTargetProfile,
            intent: None,
            node_index: None,
            detail: facts
                .target_profile
                .digest
                .clone()
                .unwrap_or_else(|| "<missing>".to_owned()),
        }]);
    }
    if facts.target_ir_domain != target_selection.target_ir_domain {
        return Err(vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetIrDomain,
            intent: None,
            node_index: None,
            detail: facts.target_ir_domain.clone(),
        }]);
    }
    Ok(target_selection)
}

fn validate_core_module(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
) -> Vec<TargetLoweringFailure> {
    if core.api_version != CORE_API_VERSION {
        return vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedCoreAbi,
            intent: None,
            node_index: None,
            detail: core.api_version.clone(),
        }];
    }
    if core.coordinate.is_empty() {
        return vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::InvalidCoreIdentity,
            intent: None,
            node_index: None,
            detail: "source Core coordinate is empty".to_owned(),
        }];
    }
    if core.intents.is_empty() {
        return vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::NoTargetSteps,
            intent: None,
            node_index: None,
            detail: "core module has no target-owned intents".to_owned(),
        }];
    }
    let floating_imports = core
        .imports
        .iter()
        .filter(|import| !import.resource.is_digest_locked())
        .map(|import| TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UndigestedCoreImport,
            intent: None,
            node_index: None,
            detail: import.resource.coordinate.clone(),
        })
        .collect::<Vec<_>>();
    if !floating_imports.is_empty() {
        return floating_imports;
    }
    let capability_failures = core
        .required_core_capabilities
        .iter()
        .map(|capability| TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedCoreCapability,
            intent: None,
            node_index: None,
            detail: capability.clone(),
        })
        .collect::<Vec<_>>();
    if !capability_failures.is_empty() {
        return capability_failures;
    }
    validate_pure_binding_graphs(core, pure_functions)
}

fn validate_pure_binding_graphs(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
) -> Vec<TargetLoweringFailure> {
    core.intents
        .iter()
        .filter(|(_, intent)| {
            !intent
                .body
                .nodes
                .iter()
                .any(|node| matches!(node, CoreNode::For { .. }))
        })
        .filter_map(|(intent_name, intent)| {
            validate_pure_binding_graph(core, pure_functions, intent_name, intent)
        })
        .collect()
}

fn validate_pure_binding_graph(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
    intent_name: &str,
    intent: &CoreIntent,
) -> Option<TargetLoweringFailure> {
    let locals = intent
        .body
        .locals
        .iter()
        .map(|local| (local.id.as_str(), local))
        .collect::<BTreeMap<_, _>>();
    if locals.len() != intent.body.locals.len() {
        return Some(invalid_pure_binding(
            intent_name,
            None,
            "Core local table contains a duplicate identity",
        ));
    }
    let Some(input) = locals.get(CORE_APPLICATION_INPUT_LOCAL_ID).copied() else {
        return Some(invalid_pure_binding(
            intent_name,
            None,
            "missing application input local",
        ));
    };
    if input.ty != intent.input {
        return Some(invalid_pure_binding(
            intent_name,
            None,
            "application input local type does not match the intent input",
        ));
    }

    let mut available = BTreeMap::from([(input.id.as_str(), input)]);
    for (node_index, node) in intent.body.nodes.iter().enumerate() {
        let binding = match node {
            CoreNode::Let { binding, value } => {
                if !expression_references_are_available(value, &available) {
                    return Some(invalid_pure_binding(
                        intent_name,
                        Some(node_index),
                        "pure binding references an undeclared, conflicting, or later local",
                    ));
                }
                if !expression_fits_declared_type(
                    core,
                    pure_functions,
                    value,
                    &binding.ty,
                    &available,
                ) {
                    return Some(invalid_pure_binding(
                        intent_name,
                        Some(node_index),
                        "pure binding value does not match its declared type",
                    ));
                }
                Some(binding)
            }
            CoreNode::Effect { binding, .. } | CoreNode::ExternalActionRequest { binding, .. } => {
                Some(binding)
            }
            CoreNode::Branch { binding, .. } => binding.as_ref(),
            CoreNode::Require { .. } | CoreNode::For { .. } => None,
        };
        let Some(binding) = binding else {
            continue;
        };
        if locals.get(binding.id.as_str()).copied() != Some(binding)
            || available.insert(binding.id.as_str(), binding).is_some()
        {
            return Some(invalid_pure_binding(
                intent_name,
                Some(node_index),
                "binding identity is missing, duplicated, or conflicts with the Core local table",
            ));
        }
    }
    if !expression_references_are_available(&intent.body.result, &available) {
        return Some(invalid_pure_binding(
            intent_name,
            None,
            "result references an undeclared or conflicting local",
        ));
    }
    (!expression_fits_declared_type(
        core,
        pure_functions,
        &intent.body.result,
        &intent.output,
        &available,
    ))
    .then(|| {
        invalid_pure_binding(
            intent_name,
            None,
            &format!(
                "result expression does not match declared output type `{}`",
                intent.output
            ),
        )
    })
}

fn expression_fits_declared_type(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
    expression: &CoreExpr,
    expected: &str,
    available: &BTreeMap<&str, &LocalRef>,
) -> bool {
    match expression {
        CoreExpr::Local { reference } => {
            available.get(reference.id.as_str()).copied() == Some(reference)
                && core_type_fits(core, &reference.ty, expected)
        }
        CoreExpr::Const(value) => core_value_fits_declared_type(core, value, expected),
        CoreExpr::Record { fields } => {
            let Some(CoreType::Record {
                fields: expected_fields,
            }) = resolved_core_type(core, expected)
            else {
                return false;
            };
            fields.keys().eq(expected_fields.keys())
                && fields.iter().all(|(field, value)| {
                    expression_fits_declared_type(
                        core,
                        pure_functions,
                        value,
                        expected_fields.get(field).expect("equal record field keys"),
                        available,
                    )
                })
        }
        CoreExpr::Field { base, field } => {
            expression_type_coordinate(core, pure_functions, base, available)
                .and_then(|base_type| resolved_core_type(core, &base_type))
                .and_then(|base_type| match base_type {
                    CoreType::Record { fields } => fields.get(field).cloned(),
                    _ => None,
                })
                .is_some_and(|field_type| core_type_fits(core, &field_type, expected))
        }
        CoreExpr::Call {
            callee,
            type_args,
            args,
        } if callee == "core.string.concat" => {
            let Some(CoreType::String {
                max: expected_max,
                canonical: expected_canonical,
            }) = resolved_core_type(core, expected)
            else {
                return false;
            };
            let mut actual_max = 0_u64;
            type_args.is_empty()
                && args.iter().all(|argument| {
                    let Some((max, canonical)) =
                        expression_string_shape(core, pure_functions, argument, available)
                    else {
                        return false;
                    };
                    let Some(next_max) = actual_max.checked_add(max) else {
                        return false;
                    };
                    actual_max = next_max;
                    canonical == expected_canonical
                })
                && actual_max <= expected_max
        }
        CoreExpr::Call {
            callee,
            type_args,
            args,
        } => validated_pure_function_fact(core, pure_functions, callee, type_args, args, available)
            .is_some_and(|fact| core_type_fits(core, &fact.return_type, expected)),
        CoreExpr::If {
            predicate,
            then_value,
            else_value,
        } => {
            predicate_fits_core_types(core, pure_functions, predicate, available)
                && expression_fits_declared_type(
                    core,
                    pure_functions,
                    then_value,
                    expected,
                    available,
                )
                && expression_fits_declared_type(
                    core,
                    pure_functions,
                    else_value,
                    expected,
                    available,
                )
        }
    }
}

fn validated_pure_function_fact<'a>(
    core: &CoreModule,
    pure_functions: &'a [TargetPureFunctionFact],
    callee: &str,
    type_args: &[String],
    args: &[CoreExpr],
    available: &BTreeMap<&str, &LocalRef>,
) -> Option<&'a TargetPureFunctionFact> {
    let mut matches = pure_functions
        .iter()
        .filter(|fact| fact.coordinate == callee);
    let fact = matches.next()?;
    if matches.next().is_some()
        || !fact.lawpack.is_digest_locked()
        || !coordinate_is_below_lawpack(&fact.coordinate, &fact.lawpack)
        || !fact.type_parameters.is_empty()
        || !type_args.is_empty()
        || args.len() != fact.parameter_types.len()
        || !core
            .imports
            .iter()
            .any(|import| import.kind == CoreImportKind::Lawpack && import.resource == fact.lawpack)
    {
        return None;
    }
    args.iter()
        .zip(&fact.parameter_types)
        .all(|(argument, parameter_type)| {
            expression_fits_declared_type(core, pure_functions, argument, parameter_type, available)
        })
        .then_some(fact)
}

fn coordinate_is_below_lawpack(coordinate: &str, lawpack: &ResourceRef) -> bool {
    coordinate
        .strip_prefix(&lawpack.coordinate)
        .and_then(|suffix| suffix.strip_prefix('.'))
        .is_some_and(|suffix| !suffix.is_empty())
}

fn expression_string_shape(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
    expression: &CoreExpr,
    available: &BTreeMap<&str, &LocalRef>,
) -> Option<(u64, String)> {
    match expression {
        CoreExpr::Const(CoreValue::String(value)) => Some((
            u64::try_from(value.chars().count()).ok()?,
            "raw-utf8".to_owned(),
        )),
        CoreExpr::Local { .. } | CoreExpr::Field { .. } => {
            let coordinate =
                expression_type_coordinate(core, pure_functions, expression, available)?;
            let CoreType::String { max, canonical } = resolved_core_type(core, &coordinate)? else {
                return None;
            };
            Some((max, canonical))
        }
        CoreExpr::Call {
            callee,
            type_args,
            args,
        } if callee == "core.string.concat" => {
            if !type_args.is_empty() {
                return None;
            }
            let mut max = 0_u64;
            let mut canonical = None;
            for argument in args {
                let (argument_max, argument_canonical) =
                    expression_string_shape(core, pure_functions, argument, available)?;
                if canonical
                    .as_ref()
                    .is_some_and(|canonical| canonical != &argument_canonical)
                {
                    return None;
                }
                canonical = Some(argument_canonical);
                max = max.checked_add(argument_max)?;
            }
            Some((max, canonical.unwrap_or_else(|| "raw-utf8".to_owned())))
        }
        CoreExpr::Call {
            callee,
            type_args,
            args,
        } => {
            let fact = validated_pure_function_fact(
                core,
                pure_functions,
                callee,
                type_args,
                args,
                available,
            )?;
            let CoreType::String { max, canonical } = resolved_core_type(core, &fact.return_type)?
            else {
                return None;
            };
            Some((max, canonical))
        }
        CoreExpr::If {
            predicate,
            then_value,
            else_value,
        } => {
            if !predicate_fits_core_types(core, pure_functions, predicate, available) {
                return None;
            }
            let (then_max, then_canonical) =
                expression_string_shape(core, pure_functions, then_value, available)?;
            let (else_max, else_canonical) =
                expression_string_shape(core, pure_functions, else_value, available)?;
            (then_canonical == else_canonical).then_some((then_max.max(else_max), then_canonical))
        }
        CoreExpr::Const(
            CoreValue::Null | CoreValue::Bool(_) | CoreValue::Int { .. } | CoreValue::Bytes(_),
        )
        | CoreExpr::Record { .. } => None,
    }
}

fn expression_type_coordinate(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
    expression: &CoreExpr,
    available: &BTreeMap<&str, &LocalRef>,
) -> Option<String> {
    match expression {
        CoreExpr::Local { reference }
            if available.get(reference.id.as_str()).copied() == Some(reference) =>
        {
            Some(reference.ty.clone())
        }
        CoreExpr::Const(CoreValue::Bool(_)) => Some("Bool".to_owned()),
        CoreExpr::Const(CoreValue::Int { width, .. }) => Some(width.clone()),
        CoreExpr::Field { base, field } => {
            expression_type_coordinate(core, pure_functions, base, available)
                .and_then(|base_type| resolved_core_type(core, &base_type))
                .and_then(|base_type| match base_type {
                    CoreType::Record { fields } => fields.get(field).cloned(),
                    _ => None,
                })
        }
        CoreExpr::Call { callee, .. } if callee == "core.string.concat" => {
            expression_string_shape(core, pure_functions, expression, available)
                .map(|(max, canonical)| format!("String<max={max},canonical={canonical}>"))
        }
        CoreExpr::Call {
            callee,
            type_args,
            args,
        } if callee != "core.string.concat" => {
            validated_pure_function_fact(core, pure_functions, callee, type_args, args, available)
                .map(|fact| fact.return_type.clone())
        }
        CoreExpr::If {
            then_value,
            else_value,
            ..
        } => {
            let then_type =
                expression_type_coordinate(core, pure_functions, then_value, available)?;
            (expression_type_coordinate(core, pure_functions, else_value, available)? == then_type)
                .then_some(then_type)
        }
        CoreExpr::Local { .. }
        | CoreExpr::Const(CoreValue::Null | CoreValue::String(_) | CoreValue::Bytes(_))
        | CoreExpr::Record { .. }
        | CoreExpr::Call { .. } => None,
    }
}

fn core_value_fits_declared_type(core: &CoreModule, value: &CoreValue, expected: &str) -> bool {
    let Some(expected) = resolved_core_type(core, expected) else {
        return false;
    };
    match (value, expected) {
        (CoreValue::Null, CoreType::Option { .. }) | (CoreValue::Bool(_), CoreType::Bool) => true,
        (
            CoreValue::Int {
                width: actual_width,
                value,
            },
            CoreType::Int { width },
        ) => actual_width == &width && crate::core_ir::parse_core_integer(&width, value).is_some(),
        (CoreValue::String(value), CoreType::String { max, canonical }) => {
            canonical == "raw-utf8"
                && u64::try_from(value.chars().count()).is_ok_and(|length| length <= max)
        }
        (CoreValue::Bytes(value), CoreType::Bytes { min, max }) => u64::try_from(value.len())
            .is_ok_and(|length| length >= min.unwrap_or(0) && length <= max),
        (
            _,
            CoreType::Nominal { .. }
            | CoreType::Record { .. }
            | CoreType::Variant { .. }
            | CoreType::List { .. }
            | CoreType::Map { .. }
            | CoreType::CapabilityRef { .. }
            | CoreType::ExternalActionRequest { .. }
            | CoreType::Option { .. }
            | CoreType::Bool
            | CoreType::Int { .. }
            | CoreType::String { .. }
            | CoreType::Bytes { .. },
        ) => false,
    }
}

fn invalid_pure_binding(
    intent_name: &str,
    node_index: Option<usize>,
    detail: &str,
) -> TargetLoweringFailure {
    TargetLoweringFailure {
        kind: TargetLoweringFailureKind::InvalidCoreIdentity,
        intent: Some(intent_name.to_owned()),
        node_index,
        detail: detail.to_owned(),
    }
}

fn expression_references_are_available(
    expression: &CoreExpr,
    available: &BTreeMap<&str, &LocalRef>,
) -> bool {
    match expression {
        CoreExpr::Local { reference } => {
            available.get(reference.id.as_str()).copied() == Some(reference)
        }
        CoreExpr::Const(_) => true,
        CoreExpr::Record { fields } => fields
            .values()
            .all(|value| expression_references_are_available(value, available)),
        CoreExpr::Field { base, .. } => expression_references_are_available(base, available),
        CoreExpr::Call { args, .. } => args
            .iter()
            .all(|argument| expression_references_are_available(argument, available)),
        CoreExpr::If {
            predicate,
            then_value,
            else_value,
        } => {
            predicate_references_are_available(predicate, available)
                && expression_references_are_available(then_value, available)
                && expression_references_are_available(else_value, available)
        }
    }
}

fn predicate_references_are_available(
    predicate: &CorePredicate,
    available: &BTreeMap<&str, &LocalRef>,
) -> bool {
    match predicate {
        CorePredicate::True | CorePredicate::False => true,
        CorePredicate::Not(value) => predicate_references_are_available(value, available),
        CorePredicate::All(values) | CorePredicate::Any(values) => values
            .iter()
            .all(|value| predicate_references_are_available(value, available)),
        CorePredicate::Compare { left, right, .. } => {
            expression_references_are_available(left, available)
                && expression_references_are_available(right, available)
        }
    }
}

fn predicate_fits_core_types(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
    predicate: &CorePredicate,
    available: &BTreeMap<&str, &LocalRef>,
) -> bool {
    if !predicate_references_are_available(predicate, available) {
        return false;
    }
    match predicate {
        CorePredicate::True | CorePredicate::False => true,
        CorePredicate::Not(value) => {
            predicate_fits_core_types(core, pure_functions, value, available)
        }
        CorePredicate::All(values) | CorePredicate::Any(values) => values
            .iter()
            .all(|value| predicate_fits_core_types(core, pure_functions, value, available)),
        CorePredicate::Compare { left, right, .. } => {
            comparison_operands_fit(core, pure_functions, left, right, available)
        }
    }
}

fn comparison_operands_fit(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
    left: &CoreExpr,
    right: &CoreExpr,
    available: &BTreeMap<&str, &LocalRef>,
) -> bool {
    let left_type = expression_type_coordinate(core, pure_functions, left, available);
    let right_type = expression_type_coordinate(core, pure_functions, right, available);
    match (left_type.as_deref(), right_type.as_deref()) {
        (Some(left_type), Some(right_type)) => {
            expression_fits_declared_type(core, pure_functions, left, left_type, available)
                && expression_fits_declared_type(core, pure_functions, right, right_type, available)
                && (core_type_fits(core, left_type, right_type)
                    || core_type_fits(core, right_type, left_type))
        }
        (Some(left_type), None) => {
            expression_fits_declared_type(core, pure_functions, left, left_type, available)
                && expression_fits_declared_type(core, pure_functions, right, left_type, available)
        }
        (None, Some(right_type)) => {
            expression_fits_declared_type(core, pure_functions, left, right_type, available)
                && expression_fits_declared_type(core, pure_functions, right, right_type, available)
        }
        (None, None) => {
            untyped_comparison_operands_fit(core, pure_functions, left, right, available)
        }
    }
}

fn untyped_comparison_operands_fit(
    core: &CoreModule,
    pure_functions: &[TargetPureFunctionFact],
    left: &CoreExpr,
    right: &CoreExpr,
    available: &BTreeMap<&str, &LocalRef>,
) -> bool {
    match (left, right) {
        (CoreExpr::Const(CoreValue::String(_)), CoreExpr::Const(CoreValue::String(_)))
        | (CoreExpr::Const(CoreValue::Bytes(_)), CoreExpr::Const(CoreValue::Bytes(_))) => true,
        (CoreExpr::Record { fields: left }, CoreExpr::Record { fields: right }) => {
            left.keys().eq(right.keys())
                && left.iter().all(|(field, left)| {
                    comparison_operands_fit(
                        core,
                        pure_functions,
                        left,
                        right.get(field).expect("equal record field keys"),
                        available,
                    )
                })
        }
        _ => false,
    }
}

fn lower_intent(
    intent_name: &str,
    intent: &CoreIntent,
    operation_profiles: &BTreeSet<&str>,
    context: &TargetLoweringContext<'_>,
    failures: &mut Vec<TargetLoweringFailure>,
) -> TargetIrIntent {
    if !operation_profiles.contains(intent.required_operation_profile.as_str()) {
        failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::MissingOperationProfile,
            intent: Some(intent_name.to_owned()),
            node_index: None,
            detail: intent.required_operation_profile.clone(),
        });
    }

    let mut state = IntentLoweringState::default();
    for (node_index, node) in intent.body.nodes.iter().enumerate() {
        lower_node(intent_name, node_index, node, context, &mut state, failures);
    }
    if state.pure_bindings.is_empty()
        && state.requirements.is_empty()
        && state.steps.is_empty()
        && state.external_action_requests.is_empty()
        && intent.body.nodes.is_empty()
    {
        failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::NoTargetSteps,
            intent: Some(intent_name.to_owned()),
            node_index: None,
            detail: "intent has no target-owned steps".to_owned(),
        });
    }

    TargetIrIntent {
        operation_profile: intent.required_operation_profile.clone(),
        basis: intent.basis.clone(),
        input_constraints: intent.input_constraints.clone(),
        core_evaluation_budget: intent.core_evaluation_budget.clone(),
        pure_bindings: state.pure_bindings,
        requirements: state.requirements,
        steps: state.steps,
        external_action_requests: state.external_action_requests,
        result: intent.body.result.clone(),
    }
}

struct TargetLoweringContext<'a> {
    target_selection: TargetSelection,
    obstruction_coordinates: &'a BTreeSet<&'a str>,
    effect_lowerings: &'a BTreeMap<&'a str, Vec<&'a TargetEffectLowering>>,
}

#[derive(Default)]
struct IntentLoweringState {
    pure_bindings: Vec<TargetIrPureBinding>,
    requirements: Vec<TargetIrRequirement>,
    steps: Vec<TargetIrStep>,
    external_action_requests: Vec<TargetIrExternalActionRequest>,
    step_outputs: BTreeSet<String>,
}

fn lower_node(
    intent_name: &str,
    node_index: usize,
    node: &CoreNode,
    context: &TargetLoweringContext<'_>,
    state: &mut IntentLoweringState,
    failures: &mut Vec<TargetLoweringFailure>,
) {
    match node {
        CoreNode::Effect {
            binding,
            effect,
            input,
            obstruction_map,
        } => {
            let steps_before = state.steps.len();
            lower_effect_node(
                intent_name,
                node_index,
                EffectNodeParts {
                    binding,
                    effect,
                    input,
                    obstruction_map,
                },
                context,
                &mut state.steps,
                failures,
            );
            if state.steps.len() > steps_before {
                state.step_outputs.insert(binding.id.clone());
            }
        }
        CoreNode::ExternalActionRequest {
            binding,
            operation,
            input_type,
            settlement_type,
            input_schema,
            settlement_schema,
            input,
            authority_scope,
            basis,
            budget,
            reconciliation_law,
        } => state
            .external_action_requests
            .push(TargetIrExternalActionRequest {
                id: format!(
                    "{}.request.{}",
                    intent_name,
                    state.external_action_requests.len()
                ),
                binding: binding.clone(),
                operation: operation.clone(),
                input_type: input_type.clone(),
                settlement_type: settlement_type.clone(),
                input_schema: input_schema.clone(),
                settlement_schema: settlement_schema.clone(),
                input: input.clone(),
                authority_scope: authority_scope.as_ref().clone(),
                basis: basis.as_ref().clone(),
                budget: budget.as_ref().clone(),
                reconciliation_law: reconciliation_law.clone(),
            }),
        CoreNode::Let { binding, value } => {
            state.pure_bindings.push(TargetIrPureBinding {
                id: format!("{}.binding.{}", intent_name, state.pure_bindings.len()),
                binding: binding.clone(),
                value: value.clone(),
            });
        }
        CoreNode::Require { predicate, arm } => lower_require_node(
            intent_name,
            node_index,
            predicate,
            arm,
            context,
            state,
            failures,
        ),
        CoreNode::For { .. } => failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedCoreNode,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: "for".to_owned(),
        }),
        CoreNode::Branch { .. } => failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedCoreNode,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: "branch".to_owned(),
        }),
    }
}

fn lower_require_node(
    intent_name: &str,
    node_index: usize,
    predicate: &CorePredicate,
    arm: &CoreRequireFailureArm,
    context: &TargetLoweringContext<'_>,
    state: &mut IntentLoweringState,
    failures: &mut Vec<TargetLoweringFailure>,
) {
    if !context.target_selection.supports_requirements {
        failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetFeature,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: "obstruction_requirement".to_owned(),
        });
        return;
    }
    if require_references_step_output(predicate, arm, &state.step_outputs) {
        failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetFeature,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: "obstruction_requirement_step_output_dependency".to_owned(),
        });
        return;
    }
    if !state.steps.is_empty() {
        failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetFeature,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: "obstruction_requirement_after_target_step".to_owned(),
        });
        return;
    }
    state.requirements.push(TargetIrRequirement {
        id: format!("{}.require.{}", intent_name, state.requirements.len()),
        predicate: predicate.clone(),
        on_failure: target_ir_require_failure(arm),
    });
}

fn target_ir_require_failure(arm: &CoreRequireFailureArm) -> TargetIrRequireFailure {
    match arm {
        CoreRequireFailureArm::Terminal { reason } => TargetIrRequireFailure::Terminal {
            reason: reason.clone(),
        },
        CoreRequireFailureArm::ContinueObstructed { reason } => {
            TargetIrRequireFailure::ContinueObstructed {
                reason: reason.clone(),
            }
        }
    }
}

fn require_references_step_output(
    predicate: &CorePredicate,
    arm: &CoreRequireFailureArm,
    step_outputs: &BTreeSet<String>,
) -> bool {
    predicate_references_step_output(predicate, step_outputs)
        || require_arm_references_step_output(arm, step_outputs)
}

fn require_arm_references_step_output(
    arm: &CoreRequireFailureArm,
    step_outputs: &BTreeSet<String>,
) -> bool {
    match arm {
        CoreRequireFailureArm::Terminal { reason }
        | CoreRequireFailureArm::ContinueObstructed { reason } => reason
            .payload
            .values()
            .any(|expr| expr_references_step_output(expr, step_outputs)),
    }
}

fn predicate_references_step_output(
    predicate: &CorePredicate,
    step_outputs: &BTreeSet<String>,
) -> bool {
    match predicate {
        CorePredicate::True | CorePredicate::False => false,
        CorePredicate::Not(value) => predicate_references_step_output(value, step_outputs),
        CorePredicate::All(values) | CorePredicate::Any(values) => values
            .iter()
            .any(|value| predicate_references_step_output(value, step_outputs)),
        CorePredicate::Compare { left, right, .. } => {
            expr_references_step_output(left, step_outputs)
                || expr_references_step_output(right, step_outputs)
        }
    }
}

fn expr_references_step_output(expr: &CoreExpr, step_outputs: &BTreeSet<String>) -> bool {
    match expr {
        CoreExpr::Local { reference } => step_outputs.contains(&reference.id),
        CoreExpr::Const(_) => false,
        CoreExpr::Record { fields } => fields
            .values()
            .any(|field| expr_references_step_output(field, step_outputs)),
        CoreExpr::Field { base, .. } => expr_references_step_output(base, step_outputs),
        CoreExpr::Call { args, .. } => args
            .iter()
            .any(|arg| expr_references_step_output(arg, step_outputs)),
        CoreExpr::If {
            predicate,
            then_value,
            else_value,
        } => {
            predicate_references_step_output(predicate, step_outputs)
                || expr_references_step_output(then_value, step_outputs)
                || expr_references_step_output(else_value, step_outputs)
        }
    }
}

#[derive(Clone, Copy)]
struct EffectNodeParts<'a> {
    binding: &'a LocalRef,
    effect: &'a str,
    input: &'a CoreExpr,
    obstruction_map: &'a BTreeMap<String, CoreObstructionArm>,
}

fn lower_effect_node(
    intent_name: &str,
    node_index: usize,
    node: EffectNodeParts<'_>,
    context: &TargetLoweringContext<'_>,
    steps: &mut Vec<TargetIrStep>,
    failures: &mut Vec<TargetLoweringFailure>,
) {
    let lowerings = context
        .effect_lowerings
        .get(node.effect)
        .map_or([].as_slice(), Vec::as_slice);
    match lowerings {
        [lowering]
            if !context
                .target_selection
                .supports_intrinsic(&lowering.target_intrinsic) =>
        {
            failures.push(TargetLoweringFailure {
                kind: TargetLoweringFailureKind::UnsupportedTargetIntrinsic,
                intent: Some(intent_name.to_owned()),
                node_index: Some(node_index),
                detail: lowering.target_intrinsic.clone(),
            });
        }
        [lowering] => {
            let mut mapped_obstructions = BTreeMap::new();
            for (failure, arm) in node.obstruction_map {
                let mapped_failure = lowering
                    .failure_mappings
                    .get(failure)
                    .unwrap_or(failure)
                    .clone();
                if mapped_obstructions
                    .insert(mapped_failure.clone(), arm.clone())
                    .is_some()
                {
                    failures.push(TargetLoweringFailure {
                        kind: TargetLoweringFailureKind::AmbiguousObstructionMapping,
                        intent: Some(intent_name.to_owned()),
                        node_index: Some(node_index),
                        detail: mapped_failure,
                    });
                    return;
                }
            }
            let unsupported_obstructions = mapped_obstructions
                .keys()
                .filter(|failure| !context.obstruction_coordinates.contains(failure.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            if !unsupported_obstructions.is_empty() {
                failures.extend(unsupported_obstructions.into_iter().map(|failure| {
                    TargetLoweringFailure {
                        kind: TargetLoweringFailureKind::MissingObstruction,
                        intent: Some(intent_name.to_owned()),
                        node_index: Some(node_index),
                        detail: failure,
                    }
                }));
                return;
            }
            steps.push(TargetIrStep {
                id: format!("{}.step.{}", intent_name, steps.len()),
                binding: node.binding.clone(),
                effect: node.effect.to_owned(),
                target_intrinsic: lowering.target_intrinsic.clone(),
                input: node.input.clone(),
                obstruction_failures: mapped_obstructions.keys().cloned().collect(),
                obstruction_arms: mapped_obstructions,
            });
        }
        [] => failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::MissingEffectLowering,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: node.effect.to_owned(),
        }),
        _ => failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::AmbiguousEffectLowering,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: node.effect.to_owned(),
        }),
    }
}

fn effect_lowerings_by_coordinate(
    facts: &TargetIrLoweringFacts,
) -> BTreeMap<&str, Vec<&TargetEffectLowering>> {
    let mut out: BTreeMap<&str, Vec<&TargetEffectLowering>> = BTreeMap::new();
    for lowering in &facts.effect_lowerings {
        out.entry(&lowering.effect).or_default().push(lowering);
    }
    out
}

fn unsupported(failures: Vec<TargetLoweringFailure>) -> TargetLoweringReport {
    TargetLoweringReport {
        status: TargetLoweringStatus::Unsupported,
        artifact: None,
        result_projections: BTreeMap::new(),
        result_projection_failures: BTreeMap::new(),
        failures,
    }
}
