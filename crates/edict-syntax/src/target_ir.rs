//! First target-owned IR generation surface.
//!
//! This module starts with the narrow v0.9 Echo slice. It lowers supported Core
//! effect nodes into an in-memory `echo.span-ir/v1` review artifact. It does not
//! execute Echo, run a verifier, assemble bundles, or perform admission.

use std::collections::{BTreeMap, BTreeSet};

use crate::core_ir::{
    CoreExpr, CoreIntent, CoreModule, CoreNode, CoreObstructionArm, LocalRef, ResourceRef,
    CORE_API_VERSION,
};
use crate::lowerability::{LowerabilityEffectStatus, LowerabilityReport, LowerabilityStatus};

pub const ECHO_DPO_TARGET_PROFILE: &str = "echo.dpo@1";
pub const ECHO_SPAN_IR_DOMAIN: &str = "echo.span-ir/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrLoweringFacts {
    pub target_profile: ResourceRef,
    pub target_ir_domain: String,
    pub operation_profiles: Vec<String>,
    pub effect_lowerings: Vec<TargetEffectLowering>,
}

impl TargetIrLoweringFacts {
    #[must_use]
    pub fn from_lowerability_report(
        target_profile_digest: Option<String>,
        target_ir_domain: impl Into<String>,
        operation_profile: impl Into<String>,
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

        Ok(Self {
            target_profile: ResourceRef {
                coordinate: report.target_profile.clone(),
                digest: target_profile_digest,
            },
            target_ir_domain: target_ir_domain.into(),
            operation_profiles: vec![operation_profile.into()],
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
    MissingEffectLowering,
    AmbiguousEffectLowering,
    UnsupportedLowerabilityReport,
    UnsupportedTargetIntrinsic,
    UnsupportedCoreAbi,
    UnsupportedCoreCapability,
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
    let target_failures = validate_target_selection(facts);
    if !target_failures.is_empty() {
        return unsupported(target_failures);
    }
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
    let mut failures = Vec::new();
    let mut intents = BTreeMap::new();

    for (intent_name, intent) in &core.intents {
        let lowered = lower_intent(
            intent_name,
            intent,
            &operation_profiles,
            &effect_lowerings,
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

fn validate_target_selection(facts: &TargetIrLoweringFacts) -> Vec<TargetLoweringFailure> {
    if facts.target_profile.coordinate != ECHO_DPO_TARGET_PROFILE {
        return vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetProfile,
            intent: None,
            node_index: None,
            detail: facts.target_profile.coordinate.clone(),
        }];
    }
    if !facts.target_profile.is_digest_locked() {
        return vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UndigestedTargetProfile,
            intent: None,
            node_index: None,
            detail: facts
                .target_profile
                .digest
                .clone()
                .unwrap_or_else(|| "<missing>".to_owned()),
        }];
    }
    if facts.target_ir_domain != ECHO_SPAN_IR_DOMAIN {
        return vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetIrDomain,
            intent: None,
            node_index: None,
            detail: facts.target_ir_domain.clone(),
        }];
    }
    Vec::new()
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
    effect_lowerings: &BTreeMap<&str, Vec<&TargetEffectLowering>>,
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
        lower_node(
            intent_name,
            node_index,
            node,
            effect_lowerings,
            &mut steps,
            failures,
        );
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
        steps,
        result: intent.body.result.clone(),
    }
}

fn lower_node(
    intent_name: &str,
    node_index: usize,
    node: &CoreNode,
    effect_lowerings: &BTreeMap<&str, Vec<&TargetEffectLowering>>,
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
            effect_lowerings,
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
    effect_lowerings: &BTreeMap<&str, Vec<&TargetEffectLowering>>,
    steps: &mut Vec<TargetIrStep>,
    failures: &mut Vec<TargetLoweringFailure>,
) {
    let lowerings = effect_lowerings
        .get(node.effect)
        .map_or([].as_slice(), Vec::as_slice);
    match lowerings {
        [lowering] if !is_echo_target_intrinsic(&lowering.target_intrinsic) => {
            failures.push(TargetLoweringFailure {
                kind: TargetLoweringFailureKind::UnsupportedTargetIntrinsic,
                intent: Some(intent_name.to_owned()),
                node_index: Some(node_index),
                detail: lowering.target_intrinsic.clone(),
            });
        }
        [lowering] => {
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

fn is_echo_target_intrinsic(target_intrinsic: &str) -> bool {
    target_intrinsic
        .strip_prefix(ECHO_DPO_TARGET_PROFILE)
        .is_some_and(|suffix| suffix.starts_with('.'))
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
