//! Source-owned function contracts, exercised through actual source compilation.
//! Runtime argument evaluation belongs to the dependent Echo witness.
use edict_syntax::{
    compile_to_core, decode_canonical_cbor, digest_core_module, encode_core_module,
    lower_to_target_ir, parse_module, CanonicalValue, CompilerContext, CompilerErrorKind,
    CoreBudget, CoreExpr, CoreModule, CoreNode, PureFunctionFact, PureHelperCostFact,
    ResourceRef, TargetIrLoweringFacts, TargetLoweringStatus,
};

const JIM_BASELINE: &str = include_str!("../../../fixtures/lang/functions/range-assembly-baseline.edict");
const JIM_FUNCTIONS: &str = include_str!("../../../fixtures/lang/functions/range-assembly.edict");
const IMPORT: &str = "use lawpack example.bounds@1 digest \"sha256:1111111111111111111111111111111111111111111111111111111111111111\" as helpers;";

fn context(steps: u64) -> CompilerContext {
    let budget = CoreBudget {
        max_steps: steps,
        max_allocated_bytes: u64::MAX,
        max_output_bytes: u64::MAX,
    };
    CompilerContext::new()
        .with_operation_profile("p.read", "continuum.profile.read-only/v1")
        .with_operation_profile("text.replaceRange", "continuum.profile.read-only/v1")
        .with_budget("p.small", budget.clone())
        .with_budget("text.replaceRangeBudget", budget)
}

fn source(functions: &str, expression: &str) -> String {
    format!("package functions.example@1;\n{functions}\ntype Input = {{ value: U64, other: U64, }};\ntype Output = {{ value: U64, }};\nintent evaluate(input: Input) returns Output profile p.read basis none budget <= p.small {{\nreturn {{ value: {expression} }};\n}}")
}

fn compile_with(source: &str, context: &CompilerContext) -> CoreModule {
    let parsed = parse_module(source).expect("source-function syntax must parse");
    compile_to_core(&parsed, context).expect("closed pure source functions must compile")
}

fn compile(source: &str) -> CoreModule {
    compile_with(source, &context(4096))
}

fn field<'a>(value: &'a CanonicalValue, name: &str) -> &'a CanonicalValue {
    let CanonicalValue::Map(fields) = value else { panic!("expected canonical map") };
    fields.iter().find_map(|(key, value)| {
        (key == &CanonicalValue::Text(name.into())).then_some(value)
    }).unwrap_or_else(|| panic!("missing canonical field {name}"))
}

fn canonical(core: &CoreModule) -> CanonicalValue {
    decode_canonical_cbor(&encode_core_module(core).unwrap()).unwrap()
}

fn definition<'a>(value: &'a CanonicalValue, name: &str) -> &'a CanonicalValue {
    field(field(value, "functions"), name)
}

fn array(value: &CanonicalValue) -> &[CanonicalValue] {
    let CanonicalValue::Array(values) = value else { panic!("expected canonical array") };
    values
}

fn facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: "echo.dpo@1".into(),
            digest: Some(format!("sha256:{}", "1".repeat(64))),
        },
        target_ir_domain: "echo.span-ir/v1".into(),
        operation_profiles: vec!["continuum.profile.read-only/v1".into()],
        obstruction_coordinates: Vec::new(),
        effect_lowerings: Vec::new(),
        effect_signatures: Vec::new(),
        pure_functions: Vec::new(),
    }
}

fn reject(functions: &str, expression: &str, kind: CompilerErrorKind) {
    let authored = source(functions, expression);
    let parsed = parse_module(&authored).expect("negative type/authority case must reach compiler");
    let errors = compile_to_core(&parsed, &context(4096)).unwrap_err();
    assert!(errors.iter().any(|error| error.kind == kind), "{errors:?}");
    assert!(errors.iter().all(|error| error.span.end <= authored.len()));
}

#[test]
fn source_function_identity_is_not_an_imported_fact() {
    let core = compile(&source("fn retain(value: U64) -> U64 { return value; }", "retain(input.value)"));
    assert!(core.imports.is_empty());
    let value = canonical(&core);
    let function = definition(&value, "retain");
    assert_eq!(field(function, "returnType"), &CanonicalValue::Text("U64".into()));
    assert_eq!(array(field(function, "params")).len(), 1);
    let result = field(field(function, "body"), "result");
    assert_eq!(field(result, "kind"), &CanonicalValue::Text("local".into()));
    assert_eq!(field(result, "ref"), &array(field(function, "params"))[0]);
    let CoreExpr::Record { fields } = &core.intents["evaluate"].body.result else { panic!("record result") };
    let CoreExpr::Call { callee, args, type_args } = &fields["value"] else { panic!("source call survives") };
    assert_eq!(callee, "functions.example@1.retain");
    assert_eq!(args.len(), 1);
    assert!(type_args.is_empty());
    let target = lower_to_target_ir(&core, &facts());
    assert_eq!(target.status, TargetLoweringStatus::Lowered, "{target:?}");
    assert_eq!(target.artifact.unwrap().intents["evaluate"].result, core.intents["evaluate"].body.result);
}

#[test]
fn jim_range_assembly_calls_an_authored_function_through_core_and_target() {
    let baseline = compile(JIM_BASELINE);
    let core = compile(JIM_FUNCTIONS);
    assert_ne!(digest_core_module(&baseline).unwrap(), digest_core_module(&core).unwrap());
    let value = canonical(&core);
    let function = definition(&value, "assembleFragments");
    assert_eq!(array(field(function, "params")).len(), 2);
    let bindings = array(field(field(function, "body"), "bindings"));
    assert_eq!(bindings.len(), 1);
    assert_eq!(field(field(&bindings[0], "value"), "callee"), &CanonicalValue::Text("core.bytes.concat".into()));
    let CoreNode::Let { value, .. } = &core.intents["assembleRange"].body.nodes[0] else { panic!("authored let") };
    let CoreExpr::Call { callee, args, .. } = value else { panic!("source-owned call") };
    assert_eq!(callee, "jedit.text.replace_range@1.assembleFragments");
    for (argument, name) in args.iter().zip(["firstFragment", "secondFragment"]) {
        assert!(matches!(argument, CoreExpr::Field { field, .. } if field == name));
    }
    let report = lower_to_target_ir(&core, &facts());
    assert_eq!(report.status, TargetLoweringStatus::Lowered, "{report:?}");
    assert_eq!(report.artifact.unwrap().intents["assembleRange"].pure_bindings[0].value, *value);
}

#[test]
fn forward_calls_and_ordered_function_locals_keep_lexical_frames() {
    let core = compile(&source(
        "fn outer(value: U64) -> U64 { let first = inner(value); let second = inner(first); return second; }\nfn inner(value: U64) -> U64 { return value; }",
        "outer(input.value)",
    ));
    let value = canonical(&core);
    let function = definition(&value, "outer");
    let bindings = array(field(field(function, "body"), "bindings"));
    assert_eq!(bindings.len(), 2);
    assert_eq!(field(field(&bindings[0], "value"), "callee"), &CanonicalValue::Text("functions.example@1.inner".into()));
    let second_args = array(field(field(&bindings[1], "value"), "args"));
    assert_eq!(field(&second_args[0], "ref"), field(&bindings[0], "binding"));
    assert_eq!(field(field(field(function, "body"), "result"), "ref"), field(&bindings[1], "binding"));
    assert_eq!(lower_to_target_ir(&core, &facts()).status, TargetLoweringStatus::Lowered);
}

#[test]
fn source_function_local_alpha_renaming_preserves_semantic_identity() {
    let original = source("fn retain(value: U64) -> U64 { let copied = value; return copied; }", "retain(input.value)");
    let renamed = original.replace("retain(value: U64)", "retain(item: U64)").replace("let copied = value; return copied;", "let saved = item; return saved;");
    assert_eq!(encode_core_module(&compile(&original)).unwrap(), encode_core_module(&compile(&renamed)).unwrap());
}

#[test]
fn source_function_body_and_unused_definition_change_identity() {
    let original = source("fn choose(left: U64, right: U64) -> U64 { return left; }", "choose(input.value, input.other)");
    let changed = original.replace("return left;", "return right;");
    assert_ne!(digest_core_module(&compile(&original)).unwrap(), digest_core_module(&compile(&changed)).unwrap());
    let unused = original.replace("type Input", "fn unused(value: U64) -> U64 { return value; }\ntype Input");
    assert_ne!(digest_core_module(&compile(&original)).unwrap(), digest_core_module(&compile(&unused)).unwrap());
}

#[test]
fn source_calls_remain_in_conditional_and_prebody_positions() {
    let authored = source("fn retain(value: U64) -> U64 { return value; }", "if input.value == 0u64 then retain(input.other) else input.value")
        .replace("basis none", "basis retain(input.value) where retain(input.value) <= input.other");
    let core = compile(&authored);
    let intent = &core.intents["evaluate"];
    assert!(matches!(intent.basis, Some(CoreExpr::Call { .. })));
    assert_eq!(intent.input_constraints.len(), 1);
    assert!(intent.body.nodes.is_empty(), "function calls must not be hoisted into body bindings");
    let CoreExpr::Record { fields } = &intent.body.result else { panic!("record result") };
    let CoreExpr::If { then_value, else_value, .. } = &fields["value"] else { panic!("conditional survives") };
    assert!(matches!(**then_value, CoreExpr::Call { .. }));
    assert!(matches!(**else_value, CoreExpr::Field { .. }));
    assert_eq!(lower_to_target_ir(&core, &facts()).status, TargetLoweringStatus::Lowered);
}

#[test]
fn caller_expressions_remain_single_ordered_arguments_even_when_unused() {
    let core = compile(&source(
        "fn retain(value: U64) -> U64 { return value; }\nfn first(left: U64, right: U64) -> U64 { return left; }",
        "first(retain(input.value), retain(input.other))",
    ));
    let CoreExpr::Record { fields } = &core.intents["evaluate"].body.result else { panic!("record result") };
    let CoreExpr::Call { args, .. } = &fields["value"] else { panic!("first call") };
    assert_eq!(args.len(), 2);
    for (argument, expected) in args.iter().zip(["value", "other"]) {
        let CoreExpr::Call { args, .. } = argument else { panic!("argument call evaluated at its occurrence") };
        assert!(matches!(&args[0], CoreExpr::Field { field, .. } if field == expected));
    }
    assert!(core.intents["evaluate"].body.nodes.is_empty());
}

#[test]
fn source_function_arity_and_return_types_are_checked() {
    reject("fn retain(value: U64) -> U64 { return value; }", "retain(input.value, input.other)", CompilerErrorKind::TypeMismatch);
    reject("fn retain(value: U32) -> U32 { return value; }", "retain(input.value)", CompilerErrorKind::TypeMismatch);
    reject("fn retain(value: U64) -> U32 { return value; }", "input.value", CompilerErrorKind::TypeMismatch);
}

#[test]
fn unused_functions_cannot_capture_callers_or_forward_locals() {
    reject("fn hidden(value: U64) -> U64 { return input.value; }", "input.value", CompilerErrorKind::UnresolvedType);
    reject("fn hidden(value: U64) -> U64 { let first = second; let second = value; return first; }", "input.value", CompilerErrorKind::UnresolvedType);
}

#[test]
fn source_function_duplicate_binders_and_names_are_rejected() {
    for functions in [
        "fn hidden(value: U64, value: U64) -> U64 { return value; }",
        "fn hidden(value: U64) -> U64 { let value = value; return value; }",
        "fn hidden(value: U64) -> U64 { return value; } fn hidden(value: U64) -> U64 { return value; }",
    ] {
        reject(functions, "input.value", CompilerErrorKind::SurfaceValidation);
    }
}

#[test]
fn source_function_returns_are_terminal_and_required() {
    reject("fn hidden(value: U64) -> U64 { let copy = value; }", "input.value", CompilerErrorKind::TypeMismatch);
    reject("fn hidden(value: U64) -> U64 { return value; let copy = value; }", "input.value", CompilerErrorKind::UnsupportedSourceShape);
}

#[test]
fn pure_functions_reject_assertions_and_effect_statements_without_erasure() {
    for statement in ["assert value == value;", "require value == value else domain.Refused;", "target.read(value);", "for item in value bounded 1 { let copy = item; }"] {
        reject(&format!("fn hidden(value: U64) -> U64 {{ {statement} return value; }}"), "input.value", CompilerErrorKind::UnsupportedSourceShape);
    }
}

#[test]
fn source_function_recursion_is_rejected_even_when_unused() {
    reject("fn cycle(value: U64) -> U64 { return cycle(value); }", "input.value", CompilerErrorKind::UnsupportedSourceShape);
    reject("fn first(value: U64) -> U64 { return second(value); } fn second(value: U64) -> U64 { return first(value); }", "input.value", CompilerErrorKind::UnsupportedSourceShape);
}

fn chain(length: usize) -> String {
    (0..length).map(|index| {
        let result = if index + 1 == length { "value".into() } else { format!("step{}(value)", index + 1) };
        format!("fn step{index}(value: U64) -> U64 {{ return {result}; }}\n")
    }).collect()
}

#[test]
fn source_function_call_depth_accepts_128_and_rejects_129() {
    compile(&source(&chain(128), "step0(input.value)"));
    reject(&chain(129), "step0(input.value)", CompilerErrorKind::InvalidBound);
}

#[test]
fn shared_suffix_depth_is_independent_of_definition_order() {
    let functions = chain(128);
    let extra = "fn zroot(value: U64) -> U64 { return step0(value); }\n";
    for declarations in [format!("{functions}{extra}"), format!("{extra}{functions}")] {
        reject(&declarations, "zroot(input.value)", CompilerErrorKind::InvalidBound);
    }
}

fn helper_context(cost: u64, budget: u64) -> CompilerContext {
    let lawpack = ResourceRef { coordinate: "example.bounds@1".into(), digest: Some(format!("sha256:{}", "1".repeat(64))) };
    context(budget)
        .with_pure_function("helpers.bump", PureFunctionFact {
            lawpack: lawpack.clone(), coordinate: "example.bounds@1.bump".into(),
            type_parameters: Vec::new(), parameter_types: vec!["U64".into()],
            return_type: "U64".into(), cost_template: "example.bounds@1.cost".into(),
        })
        .with_pure_helper_cost("helpers.cost", PureHelperCostFact {
            lawpack, coordinate: "example.bounds@1.cost".into(),
            budget: CoreBudget { max_steps: cost, max_allocated_bytes: 0, max_output_bytes: 0 },
        })
}

#[test]
fn transitive_imported_cost_counts_repeated_calls_and_unused_arguments() {
    let functions = format!("{IMPORT}\nfn costly(value: U64) -> U64 {{ return helpers.bump(value); }}\nfn first(left: U64, right: U64) -> U64 {{ return left; }}");
    let positive = source(&functions, "costly(input.value)");
    compile_with(&positive, &helper_context(10, 32));
    let repeated = source(&functions, "first(costly(input.value), costly(input.other))");
    let parsed = parse_module(&repeated).unwrap();
    let errors = compile_to_core(&parsed, &helper_context(u64::MAX / 2 + 1, u64::MAX)).unwrap_err();
    assert!(errors.iter().any(|error| error.kind == CompilerErrorKind::InvalidBound), "{errors:?}");
}

#[test]
fn transitive_totality_cannot_be_bypassed_by_a_source_function() {
    reject("fn subtract(left: U64, right: U64) -> U64 { return left - right; }", "subtract(input.value, input.other)", CompilerErrorKind::UnsupportedSourceShape);
    reject("fn cut(bytes: Bytes<max=8>, start: U64, end: U64) -> Bytes<max=8> { return slice(bytes, start, end); }", "input.value", CompilerErrorKind::UnsupportedSourceShape);
}

#[test]
fn function_free_core_omits_the_new_table() {
    let core = compile(JIM_BASELINE);
    let CanonicalValue::Map(fields) = canonical(&core) else { panic!("module map") };
    assert!(!fields.iter().any(|(key, _)| key == &CanonicalValue::Text("functions".into())));
}
