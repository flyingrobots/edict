//! First target-owned IR generation surface.
//!
//! This module starts with the narrow v0.9 Echo slice. It lowers supported Core
//! effect nodes into an in-memory `echo.span-ir/v1` review artifact. It does not
//! execute Echo, run a verifier, assemble bundles, or perform admission.

use std::collections::BTreeMap;

use crate::core_ir::{CoreExpr, CoreModule, CoreNode, ResourceRef};
use crate::lowerability::TargetProfileFacts;

pub const ECHO_DPO_TARGET_PROFILE: &str = "echo.dpo@1";
pub const ECHO_SPAN_IR_DOMAIN: &str = "echo.span-ir/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrLoweringFacts {
    pub target_profile: ResourceRef,
    pub target_ir_domain: String,
    pub effect_lowerings: Vec<TargetEffectLowering>,
}

impl TargetIrLoweringFacts {
    #[must_use]
    pub fn from_target_profile_facts(
        target_profile: ResourceRef,
        target_ir_domain: impl Into<String>,
        facts: &TargetProfileFacts,
    ) -> Self {
        Self {
            target_profile,
            target_ir_domain: target_ir_domain.into(),
            effect_lowerings: facts
                .native_effects
                .iter()
                .map(|effect| TargetEffectLowering {
                    effect: effect.coordinate.clone(),
                    target_intrinsic: effect.target_intrinsic.clone(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetEffectLowering {
    pub effect: String,
    pub target_intrinsic: String,
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
    UnsupportedCoreNode,
    MissingEffectLowering,
    AmbiguousEffectLowering,
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetIrStep {
    pub id: String,
    pub effect: String,
    pub target_intrinsic: String,
    pub input: CoreExpr,
    pub obstruction_failures: Vec<String>,
}

#[must_use]
pub fn lower_to_target_ir(
    core: &CoreModule,
    facts: &TargetIrLoweringFacts,
) -> TargetLoweringReport {
    if facts.target_profile.coordinate != ECHO_DPO_TARGET_PROFILE {
        return unsupported(vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetProfile,
            intent: None,
            node_index: None,
            detail: facts.target_profile.coordinate.clone(),
        }]);
    }
    if facts.target_ir_domain != ECHO_SPAN_IR_DOMAIN {
        return unsupported(vec![TargetLoweringFailure {
            kind: TargetLoweringFailureKind::UnsupportedTargetIrDomain,
            intent: None,
            node_index: None,
            detail: facts.target_ir_domain.clone(),
        }]);
    }

    let effect_lowerings = effect_lowerings_by_coordinate(facts);
    let mut failures = duplicate_effect_failures(&effect_lowerings);
    let mut intents = BTreeMap::new();

    for (intent_name, intent) in &core.intents {
        let mut steps = Vec::new();
        for (node_index, node) in intent.body.nodes.iter().enumerate() {
            match node {
                CoreNode::Effect {
                    effect,
                    input,
                    obstruction_map,
                    ..
                } => {
                    let lowerings = effect_lowerings
                        .get(effect.as_str())
                        .map_or([].as_slice(), Vec::as_slice);
                    match lowerings {
                        [lowering] => steps.push(TargetIrStep {
                            id: format!("{}.step.{}", intent_name, steps.len()),
                            effect: effect.clone(),
                            target_intrinsic: lowering.target_intrinsic.clone(),
                            input: input.clone(),
                            obstruction_failures: obstruction_map.keys().cloned().collect(),
                        }),
                        [] => failures.push(TargetLoweringFailure {
                            kind: TargetLoweringFailureKind::MissingEffectLowering,
                            intent: Some(intent_name.clone()),
                            node_index: Some(node_index),
                            detail: effect.clone(),
                        }),
                        _ => {}
                    }
                }
                CoreNode::Let { .. } => failures.push(TargetLoweringFailure {
                    kind: TargetLoweringFailureKind::UnsupportedCoreNode,
                    intent: Some(intent_name.clone()),
                    node_index: Some(node_index),
                    detail: "let".to_owned(),
                }),
            }
        }
        intents.insert(
            intent_name.clone(),
            TargetIrIntent {
                operation_profile: intent.required_operation_profile.clone(),
                steps,
            },
        );
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

fn effect_lowerings_by_coordinate(
    facts: &TargetIrLoweringFacts,
) -> BTreeMap<&str, Vec<&TargetEffectLowering>> {
    let mut out: BTreeMap<&str, Vec<&TargetEffectLowering>> = BTreeMap::new();
    for lowering in &facts.effect_lowerings {
        out.entry(&lowering.effect).or_default().push(lowering);
    }
    out
}

fn duplicate_effect_failures(
    effect_lowerings: &BTreeMap<&str, Vec<&TargetEffectLowering>>,
) -> Vec<TargetLoweringFailure> {
    effect_lowerings
        .iter()
        .filter(|(_, lowerings)| lowerings.len() > 1)
        .map(|(effect, _)| TargetLoweringFailure {
            kind: TargetLoweringFailureKind::AmbiguousEffectLowering,
            intent: None,
            node_index: None,
            detail: (*effect).to_owned(),
        })
        .collect()
}

fn unsupported(failures: Vec<TargetLoweringFailure>) -> TargetLoweringReport {
    TargetLoweringReport {
        status: TargetLoweringStatus::Unsupported,
        artifact: None,
        failures,
    }
}
