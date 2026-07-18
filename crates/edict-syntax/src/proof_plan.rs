//! Echo Proof Plan generation.
//!
//! This module binds a `TargetIrArtifact` to the strict physical bounds
//! required by the Echo execution runtime and the eventual ZK prover.
//! It enforces the NASA-grade static calculation constraints:
//! No execution happens unless the physical footprint is known in advance.

use std::collections::BTreeMap;
use crate::target_ir::{TargetIrArtifact, TargetIrIntent};

/// The explicit structural bounds for an Echo execution trace.
/// This prevents out-of-memory or unbounded loop attacks against the prover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceLayout {
    pub max_ticks: usize,
    pub max_rows: usize,
    pub max_public_inputs: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EchoIntentPlan {
    pub step_bounds: usize,
    pub tick_bound: usize,
    pub public_input_count: usize,
}

/// The verifiable manifest that guarantees an intent can be safely evaluated
/// by the Echo runtime within polynomial constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EchoProofPlan {
    pub target_ir_digest: String,
    pub trace_layout: TraceLayout,
    pub intents: BTreeMap<String, EchoIntentPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofPlanFailure {
    pub reason: String,
}

/// Builds an EchoProofPlan from a certified TargetIrArtifact.
/// 
/// The `digest` must be a domain-separated SHA-256 hash of the `TargetIrArtifact`.
pub fn build_echo_proof_plan(
    artifact: &TargetIrArtifact,
    digest: &str,
) -> Result<EchoProofPlan, ProofPlanFailure> {
    let mut intents = BTreeMap::new();
    let mut max_ticks = 0;
    let mut max_rows = 0;
    let mut max_public_inputs = 0;

    for (name, intent) in &artifact.intents {
        let step_bounds = intent.steps.len();
        
        // Milestone 1 Physics Calculation: 
        // 1 tick per execution step + 1 tick per requirement validation
        let tick_bound = step_bounds + intent.requirements.len();
        
        // Public inputs: Input constraints + 1 for the target IR root digest
        let public_input_count = intent.input_constraints.len() + 1;

        if tick_bound > max_ticks {
            max_ticks = tick_bound;
        }
        if step_bounds > max_rows {
            max_rows = step_bounds;
        }
        if public_input_count > max_public_inputs {
            max_public_inputs = public_input_count;
        }

        intents.insert(
            name.clone(),
            EchoIntentPlan {
                step_bounds,
                tick_bound,
                public_input_count,
            },
        );
    }

    Ok(EchoProofPlan {
        target_ir_digest: digest.to_owned(),
        trace_layout: TraceLayout {
            max_ticks,
            max_rows,
            max_public_inputs,
        },
        intents,
    })
}
