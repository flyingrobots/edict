//! Unsigned difference is total only after its input-domain proof.
use edict_syntax::{
    compile_to_core, digest_core_module, parse_module, CompilerContext, CoreBudget, CoreExpr,
    CoreModule, CoreNode, CoreValue,
};

fn context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("p.read", "continuum.profile.read-only/v1")
        .with_budget(
            "p.small",
            CoreBudget {
                max_steps: 64,
                max_allocated_bytes: 4096,
                max_output_bytes: 1024,
            },
        )
}

fn source(width: &str, guard: &str, expression: &str) -> String {
    format!(
        "package arithmetic.example@1;
         type Input = {{ lower: {width}, upper: {width}, other: {width}, }};
         type Output = {{ distance: {width}, }};
         intent measure(input: Input) returns Output
           profile p.read basis none budget <= p.small {guard} {{
           let distance: {width} = {expression};
           return {{ distance }};
         }}"
    )
}

fn compile(width: &str, guard: &str, expression: &str) -> CoreModule {
    let module = parse_module(&source(width, guard, expression)).expect("valid source");
    compile_to_core(&module, &context()).expect("proven difference must compile")
}

#[test]
fn guarded_unsigned_difference_preserves_width_order_and_guard() {
    for width in ["U32", "U64"] {
        for guard in [
            "input.lower <= input.upper",
            "input.lower < input.upper",
            "input.upper >= input.lower",
            "input.upper > input.lower",
            "input.lower == input.upper",
            "input.upper == input.lower",
            "true && input.lower <= input.upper",
        ] {
            let core = compile(
                width,
                &format!("where {guard}"),
                "input.upper - input.lower",
            );
            let intent = &core.intents["measure"];
            assert_eq!(intent.input_constraints.len(), 1);
            let CoreNode::Let { binding, value } = &intent.body.nodes[0] else {
                panic!("authored let must remain a binding")
            };
            assert_eq!(binding.ty, width);
            let CoreExpr::Call { callee, type_args, args } = value else {
                panic!("difference must lower to the generic prelude operation")
            };
            assert_eq!(callee, "core.integer.subtract");
            assert_eq!(type_args, &[width]);
            assert_eq!(args.len(), 2);
            for (argument, expected) in args.iter().zip(["upper", "lower"]) {
                assert!(matches!(argument, CoreExpr::Field { field, .. } if field == expected));
            }
        }
    }
}

#[test]
fn unsigned_difference_accepts_self_zero_and_ordered_constants() {
    for width in ["U32", "U64"] {
        for expression in ["input.upper - input.upper", "input.upper - 0", "9 - 4"] {
            let core = compile(width, "", expression);
            let CoreNode::Let { binding, value: CoreExpr::Call { args, .. } } =
                &core.intents["measure"].body.nodes[0]
            else {
                panic!("total arithmetic binding")
            };
            assert_eq!(binding.ty, width);
            if expression == "9 - 4" {
                assert_eq!(args, &[
                    CoreExpr::Const(CoreValue::Int { width: width.into(), value: "9".into() }),
                    CoreExpr::Const(CoreValue::Int { width: width.into(), value: "4".into() }),
                ]);
            }
        }
    }
}

#[test]
fn unsigned_difference_identity_tracks_semantics() {
    let original = compile("U64", "where input.lower <= input.upper", "input.upper - input.lower");
    let original_digest = digest_core_module(&original).unwrap();
    for changed in [
        compile("U64", "where input.lower <= input.upper", "input.upper - input.upper"),
        compile("U64", "where input.lower < input.upper", "input.upper - input.lower"),
        compile("U32", "where input.lower <= input.upper", "input.upper - input.lower"),
    ] {
        assert_ne!(original_digest, digest_core_module(&changed).unwrap());
    }
}
