//! The target boundary must validate totality without trusting source compilation.
use edict_syntax::{
    compile_to_core, lower_to_target_ir, parse_module, CompilerContext, CoreBudget, CoreExpr,
    CoreModule, CoreNode, CorePredicate, ResourceRef, TargetIrLoweringFacts,
    TargetLoweringFailureKind, TargetLoweringStatus,
};

fn core() -> CoreModule {
    let source = "package arithmetic.target@1;
        type Input = { lower: U64, upper: U64, };
        type Output = { distance: U64, };
        intent measure(input: Input) returns Output
          profile p.read basis none budget <= p.small
          where input.lower <= input.upper {
          let distance: U64 = input.upper - input.lower;
          return { distance };
        }";
    let context = CompilerContext::new()
        .with_operation_profile("p.read", "continuum.profile.read-only/v1")
        .with_budget(
            "p.small",
            CoreBudget {
                max_steps: 64,
                max_allocated_bytes: 4096,
                max_output_bytes: 1024,
            },
        );
    compile_to_core(&parse_module(source).unwrap(), &context).unwrap()
}

fn facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: "echo.dpo@1".to_owned(),
            digest: Some(format!("sha256:{}", "1".repeat(64))),
        },
        target_ir_domain: "echo.span-ir/v1".to_owned(),
        operation_profiles: vec!["continuum.profile.read-only/v1".to_owned()],
        obstruction_coordinates: Vec::new(),
        effect_lowerings: Vec::new(),
        effect_signatures: Vec::new(),
        pure_functions: Vec::new(),
    }
}

#[test]
fn target_lowers_proven_unsigned_difference_without_changing_operands() {
    let core = core();
    let report = lower_to_target_ir(&core, &facts());
    assert_eq!(
        report.status,
        TargetLoweringStatus::Lowered,
        "{:?}",
        report.failures
    );
    let artifact = report.artifact.unwrap();
    let binding = &artifact.intents["measure"].pure_bindings[0];
    let CoreNode::Let { value, .. } = &core.intents["measure"].body.nodes[0] else {
        panic!("difference binding");
    };
    assert_eq!(&binding.value, value);
    assert!(report.result_projections.contains_key("measure"));
}

#[test]
fn target_rejects_unsigned_difference_when_core_guard_is_stripped() {
    let mut core = core();
    core.intents
        .get_mut("measure")
        .unwrap()
        .input_constraints
        .clear();
    rejects(&core);
}

#[test]
fn target_rejects_unsigned_difference_when_core_operands_are_reversed() {
    let mut core = core();
    let CoreNode::Let {
        value: CoreExpr::Call { args, .. },
        ..
    } = &mut core.intents.get_mut("measure").unwrap().body.nodes[0]
    else {
        panic!("difference call");
    };
    args.swap(0, 1);
    rejects(&core);
}

#[test]
fn target_rejects_unsigned_difference_with_disjunctive_evidence() {
    let mut core = core();
    let predicate = &mut core.intents.get_mut("measure").unwrap().input_constraints[0].predicate;
    *predicate = CorePredicate::Any(vec![predicate.clone(), CorePredicate::True]);
    rejects(&core);
}

fn rejects(core: &CoreModule) {
    let report = lower_to_target_ir(core, &facts());
    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert!(report
        .failures
        .iter()
        .any(|failure| failure.kind == TargetLoweringFailureKind::InvalidCoreIdentity));
}
