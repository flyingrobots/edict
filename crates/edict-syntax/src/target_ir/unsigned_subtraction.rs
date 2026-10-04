//! Target-owned totality check over untrusted Core, independent of the compiler proof.
use super::{
    expression_type_coordinate, BTreeMap, CoreExpr, CoreIntent, CoreModule, CoreNode,
    CorePredicate, CoreRequireFailureArm, CoreValue, InputConstraint, LocalRef,
    TargetPureFunctionFact,
};
use crate::core_ir::{CompareOp, CoreBlock};

pub(super) const OPERATION: &str = "core.integer.subtract";

pub(super) fn type_coordinate(
    core: &CoreModule,
    functions: &[TargetPureFunctionFact],
    type_args: &[String],
    args: &[CoreExpr],
    available: &BTreeMap<&str, &LocalRef>,
) -> Option<String> {
    #[cfg(test)]
    tests::VALIDATION_VISITS.with(|count| count.set(count.get() + 1));
    let [width] = type_args else { return None };
    let [left, right] = args else { return None };
    if !matches!(width.as_str(), "U32" | "U64") {
        return None;
    }
    [left, right]
        .iter()
        .all(|operand| {
            expression_type_coordinate(core, functions, operand, available).as_ref() == Some(width)
        })
        .then(|| width.clone())
}

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
        CoreExpr::Call { callee, args, .. } => {
            if !args.iter().all(|arg| expression_is_total(arg, constraints)) {
                return false;
            }
            if callee != OPERATION {
                return true;
            }
            let [left, right] = args.as_slice() else {
                return false;
            };
            left == right
                || constant(right) == Some(0)
                || matches!((constant(left), constant(right)), (Some(a), Some(b)) if a >= b)
                || constraints
                    .iter()
                    .any(|constraint| proves_order(&constraint.predicate, left, right))
        }
    }
}

fn constant(value: &CoreExpr) -> Option<u64> {
    let CoreExpr::Const(CoreValue::Int { width, value }) = value else {
        return None;
    };
    match width.as_str() {
        "U32" => value.parse::<u32>().ok().map(u64::from),
        "U64" => value.parse().ok(),
        _ => None,
    }
}

fn proves_order(predicate: &CorePredicate, minuend: &CoreExpr, subtrahend: &CoreExpr) -> bool {
    match predicate {
        CorePredicate::All(items) => items
            .iter()
            .any(|item| proves_order(item, minuend, subtrahend)),
        CorePredicate::Compare { op, left, right } => match op {
            CompareOp::Lt | CompareOp::Le => left == subtrahend && right == minuend,
            CompareOp::Gt | CompareOp::Ge => left == minuend && right == subtrahend,
            CompareOp::Eq => {
                (left == minuend && right == subtrahend) || (left == subtrahend && right == minuend)
            }
            CompareOp::Ne => false,
        },
        CorePredicate::True
        | CorePredicate::False
        | CorePredicate::Not(_)
        | CorePredicate::Any(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    std::thread_local! {
        pub(super) static VALIDATION_VISITS: Cell<usize> = const { Cell::new(0) };
    }

    fn integer(value: &str) -> CoreExpr {
        CoreExpr::Const(CoreValue::Int {
            width: "U64".to_owned(),
            value: value.to_owned(),
        })
    }

    fn assert_bounded_validation(wrap: fn(CoreExpr) -> CoreExpr) {
        let core = CoreModule {
            api_version: crate::core_ir::CORE_API_VERSION.to_owned(),
            coordinate: "arithmetic.work@1".to_owned(),
            imports: Vec::new(),
            types: BTreeMap::new(),
            intents: BTreeMap::new(),
            required_core_capabilities: Vec::new(),
        };
        let depth = 8;
        let expression = (0..depth).fold(integer("9"), |left, _| {
            wrap(CoreExpr::Call {
                callee: OPERATION.to_owned(),
                type_args: vec!["U64".to_owned()],
                args: vec![left, integer("0")],
            })
        });
        VALIDATION_VISITS.with(|count| count.set(0));
        assert_eq!(
            expression_type_coordinate(&core, &[], &expression, &BTreeMap::new()),
            Some("U64".to_owned())
        );
        let visits = VALIDATION_VISITS.with(Cell::get);
        println!("{depth} subtraction nodes required {visits} validation visits");
        assert!(
            visits <= 4 * depth,
            "{depth} subtraction nodes required {visits} validation visits"
        );
    }

    #[test]
    fn nested_subtraction_validation_work_is_bounded() {
        assert_bounded_validation(std::convert::identity);
    }

    #[test]
    fn conditional_subtraction_validation_work_is_bounded() {
        assert_bounded_validation(|value| CoreExpr::If {
            predicate: Box::new(CorePredicate::True),
            then_value: Box::new(value),
            else_value: Box::new(integer("9")),
        });
    }

    #[test]
    fn record_field_subtraction_validation_work_is_bounded() {
        assert_bounded_validation(|value| CoreExpr::Field {
            base: Box::new(CoreExpr::Record {
                fields: BTreeMap::from([("value".to_owned(), value)]),
            }),
            field: "value".to_owned(),
        });
    }
}
