//! First target-owned IR generation surface.
//!
//! This module contains the narrow v0.9 target slices. It lowers supported Core
//! effect nodes into in-memory Echo or git-warp review artifacts. It does not
//! execute a runtime, run a verifier, assemble bundles, or perform admission.

use std::collections::{BTreeMap, BTreeSet};

use crate::core_ir::{
    is_lowercase_sha256_review_digest, CoreBudget, CoreExpr, CoreImportKind, CoreIntent,
    CoreModule, CoreNode, CoreObstructionArm, CoreObstructionReason, CorePredicate,
    CoreRequireFailureArm, InputConstraint, LocalRef, ResourceRef, CORE_API_VERSION,
};
use crate::digest_core_module;
use crate::lowerability::{LowerabilityEffectStatus, LowerabilityReport, LowerabilityStatus};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrLoweringFacts {
    pub target_profile: ResourceRef,
    pub target_ir_domain: String,
    pub operation_profiles: Vec<String>,
    pub obstruction_coordinates: Vec<String>,
    pub effect_lowerings: Vec<TargetEffectLowering>,
}

impl TargetIrLoweringFacts {
    /// Build Target IR lowering facts from an accepted native lowerability report.
    ///
    /// # Errors
    ///
    /// Returns `UnsupportedLowerabilityReport` when the lowerability report did
    /// not select native support. The v0.9 Target IR bridge does not
    /// derive target facts from unsupported or adapter-backed reports.
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
    pub status: TargetLoweringStatus,
    pub artifact: Option<TargetIrArtifact>,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrIntent {
    pub operation_profile: String,
    pub basis: Option<CoreExpr>,
    pub input_constraints: Vec<InputConstraint>,
    pub core_evaluation_budget: CoreBudget,
    pub requirements: Vec<TargetIrRequirement>,
    pub steps: Vec<TargetIrStep>,
    pub result: CoreExpr,
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

#[must_use]
pub fn lower_to_target_ir(
    core: &CoreModule,
    facts: &TargetIrLoweringFacts,
) -> TargetLoweringReport {
    let target_selection = match validate_target_selection(facts) {
        Ok(target_selection) => target_selection,
        Err(failures) => return unsupported(failures),
    };
    let core_failures = validate_core_module(core);
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
        TargetLoweringReport {
            status: TargetLoweringStatus::Lowered,
            artifact: Some(TargetIrArtifact {
                domain: facts.target_ir_domain.clone(),
                target_profile: facts.target_profile.clone(),
                source_core_coordinate: core.coordinate.clone(),
                semantic_closure,
                intents,
            }),
            failures,
        }
    } else {
        unsupported(failures)
    }
}

pub(crate) fn semantic_closure_for_core(
    core: &CoreModule,
) -> Result<Option<TargetIrSemanticClosure>, TargetLoweringFailure> {
    let mut lawpacks = BTreeMap::<String, ResourceRef>::new();
    for resource in core
        .imports
        .iter()
        .filter(|import| import.kind == CoreImportKind::Lawpack)
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
        if let Some(prior) = lawpacks.get(&resource.coordinate) {
            if prior != resource {
                return Err(TargetLoweringFailure {
                    kind: TargetLoweringFailureKind::InvalidCoreIdentity,
                    intent: None,
                    node_index: None,
                    detail: format!(
                        "lawpack coordinate `{}` is bound to conflicting resources",
                        resource.coordinate
                    ),
                });
            }
        } else {
            lawpacks.insert(resource.coordinate.clone(), resource.clone());
        }
    }
    let has_explicit_basis = core.intents.values().any(|intent| intent.basis.is_some());
    if !has_explicit_basis && lawpacks.is_empty() {
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
    }))
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

fn validate_core_module(core: &CoreModule) -> Vec<TargetLoweringFailure> {
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
    core.required_core_capabilities
        .iter()
        .map(|capability| TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedCoreCapability,
            intent: None,
            node_index: None,
            detail: capability.clone(),
        })
        .collect()
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
    if state.requirements.is_empty() && state.steps.is_empty() && intent.body.nodes.is_empty() {
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
        requirements: state.requirements,
        steps: state.steps,
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
    requirements: Vec<TargetIrRequirement>,
    steps: Vec<TargetIrStep>,
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
        CoreNode::Let { .. } => failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedCoreNode,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: "let".to_owned(),
        }),
        CoreNode::Require { predicate, arm } => lower_require_node(
            intent_name,
            node_index,
            predicate,
            arm,
            context,
            state,
            failures,
        ),
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
        failures,
    }
}
