//! First target-owned IR generation surface.
//!
//! This module contains the narrow v0.9 target slices. It lowers supported Core
//! effect nodes into in-memory Echo or git-warp review artifacts. It does not
//! execute a runtime, run a verifier, assemble bundles, or perform admission.

use std::collections::{BTreeMap, BTreeSet};

use crate::core_ir::{
    CoreBudget, CoreExpr, CoreIntent, CoreModule, CoreNode, CoreObstructionArm, InputConstraint,
    LocalRef, ResourceRef, CORE_API_VERSION,
};
use crate::lowerability::{LowerabilityEffectStatus, LowerabilityReport, LowerabilityStatus};

pub const ECHO_DPO_TARGET_PROFILE: &str = "echo.dpo@1";
pub const ECHO_SPAN_IR_DOMAIN: &str = "echo.span-ir/v1";
pub const GITWARP_REF_CRDT_TARGET_PROFILE: &str = "gitwarp.ref_crdt@1";
pub const GITWARP_COMMIT_REDUCER_IR_DOMAIN: &str = "gitwarp.commit-reducer-ir/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TargetSelection {
    target_ir_domain: &'static str,
    target_intrinsic_prefix: &'static str,
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
        }),
        GITWARP_REF_CRDT_TARGET_PROFILE => Some(TargetSelection {
            target_ir_domain: GITWARP_COMMIT_REDUCER_IR_DOMAIN,
            target_intrinsic_prefix: GITWARP_REF_CRDT_TARGET_PROFILE,
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
    UnsupportedCoreNode,
    MissingOperationProfile,
    MissingObstruction,
    MissingEffectLowering,
    AmbiguousEffectLowering,
    UnsupportedLowerabilityReport,
    UnsupportedTargetIntrinsic,
    UnsupportedCoreAbi,
    UnsupportedCoreCapability,
    UndigestedCoreImport,
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
    pub intents: BTreeMap<String, TargetIrIntent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrIntent {
    pub operation_profile: String,
    pub input_constraints: Vec<InputConstraint>,
    pub core_evaluation_budget: CoreBudget,
    pub steps: Vec<TargetIrStep>,
    pub result: CoreExpr,
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
                intents,
            }),
            failures,
        }
    } else {
        unsupported(failures)
    }
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

    let mut steps = Vec::new();
    for (node_index, node) in intent.body.nodes.iter().enumerate() {
        lower_node(intent_name, node_index, node, context, &mut steps, failures);
    }
    if steps.is_empty() && intent.body.nodes.is_empty() {
        failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::NoTargetSteps,
            intent: Some(intent_name.to_owned()),
            node_index: None,
            detail: "intent has no target-owned steps".to_owned(),
        });
    }

    TargetIrIntent {
        operation_profile: intent.required_operation_profile.clone(),
        input_constraints: intent.input_constraints.clone(),
        core_evaluation_budget: intent.core_evaluation_budget.clone(),
        steps,
        result: intent.body.result.clone(),
    }
}

struct TargetLoweringContext<'a> {
    target_selection: TargetSelection,
    obstruction_coordinates: &'a BTreeSet<&'a str>,
    effect_lowerings: &'a BTreeMap<&'a str, Vec<&'a TargetEffectLowering>>,
}

fn lower_node(
    intent_name: &str,
    node_index: usize,
    node: &CoreNode,
    context: &TargetLoweringContext<'_>,
    steps: &mut Vec<TargetIrStep>,
    failures: &mut Vec<TargetLoweringFailure>,
) {
    match node {
        CoreNode::Effect {
            binding,
            effect,
            input,
            obstruction_map,
        } => lower_effect_node(
            intent_name,
            node_index,
            EffectNodeParts {
                binding,
                effect,
                input,
                obstruction_map,
            },
            context,
            steps,
            failures,
        ),
        CoreNode::Let { .. } => failures.push(TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedCoreNode,
            intent: Some(intent_name.to_owned()),
            node_index: Some(node_index),
            detail: "let".to_owned(),
        }),
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
            let unsupported_obstructions = node
                .obstruction_map
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
                obstruction_failures: node.obstruction_map.keys().cloned().collect(),
                obstruction_arms: node.obstruction_map.clone(),
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
