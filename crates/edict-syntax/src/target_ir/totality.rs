//! Evaluation-scope traversal for partial Core operations at the Target boundary.
use super::{
    byte_slice, unsigned_subtraction, CoreExpr, CoreIntent, CoreNode, CorePredicate,
    CoreRequireFailureArm, InputConstraint,
};
use crate::core_ir::CoreBlock;

pub(super) fn intent_is_total(intent: &CoreIntent) -> bool {
    intent
        .basis
        .as_ref()
        .is_none_or(|basis| expression_is_total(basis, &[]))
        && intent
            .input_constraints
            .iter()
            .all(|constraint| predicate_is_total(&constraint.predicate, &[]))
        && block_is_total(&intent.body, &intent.input_constraints)
}

fn block_is_total(block: &CoreBlock, constraints: &[InputConstraint]) -> bool {
    block.nodes.iter().all(|node| match node {
        CoreNode::Let { value, .. } => expression_is_total(value, constraints),
        CoreNode::Require { predicate, arm } => {
            let (CoreRequireFailureArm::Terminal { reason }
            | CoreRequireFailureArm::ContinueObstructed { reason }) = arm;
            predicate_is_total(predicate, constraints)
                && reason
                    .payload
                    .values()
                    .all(|value| expression_is_total(value, constraints))
        }
        CoreNode::Effect {
            input,
            obstruction_map,
            ..
        } => {
            expression_is_total(input, constraints)
                && obstruction_map
                    .values()
                    .all(|arm| expression_is_total(&arm.value, constraints))
        }
        CoreNode::ExternalActionRequest {
            input,
            authority_scope,
            basis,
            budget,
            ..
        } => [
            input,
            authority_scope,
            basis,
            &budget.max_settlement_bytes,
            &budget.max_attempts,
        ]
        .iter()
        .all(|value| expression_is_total(value, constraints)),
        CoreNode::For { iter, body, .. } => {
            expression_is_total(iter, constraints) && block_is_total(body, constraints)
        }
        CoreNode::Branch {
            predicate,
            then_block,
            else_block,
            ..
        } => {
            predicate_is_total(predicate, constraints)
                && block_is_total(then_block, constraints)
                && block_is_total(else_block, constraints)
        }
    }) && expression_is_total(&block.result, constraints)
}

fn predicate_is_total(predicate: &CorePredicate, constraints: &[InputConstraint]) -> bool {
    match predicate {
        CorePredicate::True | CorePredicate::False => true,
        CorePredicate::Not(inner) => predicate_is_total(inner, constraints),
        CorePredicate::All(items) | CorePredicate::Any(items) => items
            .iter()
            .all(|item| predicate_is_total(item, constraints)),
        CorePredicate::Compare { left, right, .. } => {
            expression_is_total(left, constraints) && expression_is_total(right, constraints)
        }
    }
}

fn expression_is_total(expression: &CoreExpr, constraints: &[InputConstraint]) -> bool {
    match expression {
        CoreExpr::Local { .. } | CoreExpr::Const(_) => true,
        CoreExpr::Field { base, .. } => expression_is_total(base, constraints),
        CoreExpr::Record { fields } => fields
            .values()
            .all(|value| expression_is_total(value, constraints)),
        CoreExpr::If {
            predicate,
            then_value,
            else_value,
        } => {
            predicate_is_total(predicate, constraints)
                && expression_is_total(then_value, constraints)
                && expression_is_total(else_value, constraints)
        }
        CoreExpr::Call {
            callee,
            type_args,
            args,
        } => {
            if !args.iter().all(|arg| expression_is_total(arg, constraints)) {
                return false;
            }
            match callee.as_str() {
                unsigned_subtraction::OPERATION => {
                    let [left, right] = args.as_slice() else {
                        return false;
                    };
                    unsigned_subtraction::proven_order(left, right, constraints)
                }
                byte_slice::OPERATION => byte_slice::is_total(type_args, args, constraints),
                _ => true,
            }
        }
    }
}
