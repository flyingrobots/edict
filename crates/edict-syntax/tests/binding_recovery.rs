//! Failed bindings retain their cause without hiding independent errors.
use edict_syntax::{
    compile_to_core, decode_lawpack_adapter, decode_lawpack_bundle, encode_core_module,
    parse_module, prepare_lawpack_compilation, CompilerError, CompilerErrorKind, CoreModule,
};

const SOURCE: &str = include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.edict");

fn compile(source: &str) -> Result<CoreModule, Vec<CompilerError>> {
    compile_with_surface(source, true)
}

fn compile_with_surface(source: &str, surface: bool) -> Result<CoreModule, Vec<CompilerError>> {
    let module = parse_module(source).expect("recovery fixture parses");
    let bundle = decode_lawpack_bundle(
        include_bytes!("../../../fixtures/lawpack/hello-echo/manifest.cbor"),
        include_bytes!("../../../fixtures/lawpack/hello-echo/exports.cbor"),
    )
    .expect("exact lawpack");
    let adapter = decode_lawpack_adapter(
        &bundle,
        "echo.dpo@1",
        include_bytes!("../../../fixtures/lawpack/hello-echo/adapter.cbor"),
    )
    .expect("exact adapter");
    let prepared = prepare_lawpack_compilation(&module, &bundle, &adapter)
        .expect("authenticated recovery context");
    if surface {
        compile_to_core(&module, prepared.compiler_context())
    } else {
        let resolved = edict_syntax::resolve_module(&module, prepared.compiler_context())?;
        let typed = edict_syntax::type_check(&resolved)?;
        edict_syntax::lower_core(&typed)
    }
}

fn insert_bindings(bindings: &str) -> String {
    assert_eq!(SOURCE.matches("  let receipt:").count(), 1);
    SOURCE.replace("  let receipt:", &format!("{bindings}\n  let receipt:"))
}

fn failed_call() -> String {
    SOURCE.replace("hello.createGreeting(input)", "hello.createGreeting(true)")
}

fn check_kinds(source: &str, kinds: &[CompilerErrorKind]) -> Vec<CompilerError> {
    let errors = compile(source).expect_err("invalid program never produces Core");
    assert_eq!(
        errors.iter().map(|error| error.kind).collect::<Vec<_>>(),
        kinds
    );
    errors
}

#[test]
fn valid_recovery_control_retains_exact_core_bytes() {
    let core = compile(SOURCE).expect("valid effect binding compiles");
    assert_eq!(
        encode_core_module(&core).expect("canonical Core"),
        include_bytes!("../../../fixtures/lawpack/hello-echo/create-greeting.core.cbor")
    );
}

#[test]
fn failed_effect_and_dependent_receipt_report_only_the_cause() {
    let source = failed_call();
    let errors = check_kinds(&source, &[CompilerErrorKind::TypeMismatch]);
    assert!(source[errors[0].span.start..errors[0].span.end].contains("hello.createGreeting(true)"));
}

#[test]
fn poisoned_locals_propagate_through_chained_and_nested_bindings() {
    let source = insert_bindings("  let failed = missingCause;\n  let chained = failed;\n  let nested = { item: chained, };\n  let dependent = nested.item;");
    let errors = check_kinds(&source, &[CompilerErrorKind::UnresolvedType]);
    assert_eq!(
        &source[errors[0].span.start..errors[0].span.end],
        "missingCause"
    );
}

#[test]
fn poisoned_record_sibling_does_not_hide_independent_expression_error() {
    let source = failed_call().replace("message: input.message", "message: input.absent");
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::TypeMismatch,
            CompilerErrorKind::UnknownField,
        ],
    );
    assert!(source[errors[1].span.start..errors[1].span.end].contains("input.absent"));
}

#[test]
fn poisoned_record_sibling_does_not_hide_an_independent_type_mismatch() {
    let source = failed_call().replace("message: input.message", "message: true");
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::TypeMismatch,
            CompilerErrorKind::TypeMismatch,
        ],
    );
    assert_eq!(&source[errors[1].span.start..errors[1].span.end], "true");
}

#[test]
fn independent_later_binding_errors_survive_recovery() {
    let source = failed_call().replace(
        "  return {",
        "  let separate: Bool = input.message;\n  return {",
    );
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::TypeMismatch,
            CompilerErrorKind::TypeMismatch,
        ],
    );
    assert!(source[errors[1].span.start..errors[1].span.end].contains("let separate:"));
}

#[test]
fn failed_shadow_does_not_use_outer_value_or_escape_its_branch() {
    let source = insert_bindings("  let outer = 1u64;\n  if true {\n    let outer = missingShadow;\n    let dependent: Bool = outer;\n  }\n  let independent: Bool = outer;");
    // Source shadowing stays rejected by the front end. The public type-check
    // phase still must recover safely when explicitly given this resolved AST.
    check_kinds(&source, &[CompilerErrorKind::SurfaceValidation]);
    let errors =
        compile_with_surface(&source, false).expect_err("invalid typed shadow produces no Core");
    assert_eq!(
        errors.iter().map(|error| error.kind).collect::<Vec<_>>(),
        [
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::TypeMismatch
        ]
    );
    assert_eq!(
        &source[errors[0].span.start..errors[0].span.end],
        "missingShadow"
    );
    assert!(source[errors[1].span.start..errors[1].span.end].contains("let independent:"));
}

#[test]
fn failed_branch_local_does_not_poison_an_unknown_outer_name() {
    let source = insert_bindings("  if true {\n    let branchOnly = missingBranch;\n    let dependent = branchOnly;\n  }\n  let independent = branchOnly;");
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::UnresolvedType,
        ],
    );
    assert_eq!(
        &source[errors[0].span.start..errors[0].span.end],
        "missingBranch"
    );
    assert_eq!(
        &source[errors[1].span.start..errors[1].span.end],
        "branchOnly"
    );
}

#[test]
fn failed_return_is_distinct_from_a_genuinely_absent_return() {
    let wrong = SOURCE.replace("message: input.message", "message: true");
    check_kinds(&wrong, &[CompilerErrorKind::TypeMismatch]);
    let start = SOURCE.find("  return {").expect("return begins");
    let absent = format!("{} }}\n", &SOURCE[..start]);
    let errors = check_kinds(&absent, &[CompilerErrorKind::TypeMismatch]);
    assert_eq!(
        errors[0].span,
        parse_module(&absent)
            .expect("absent-return source")
            .decls
            .iter()
            .find_map(|decl| match decl {
                edict_syntax::ast::Decl::Intent(intent) => Some(intent.span),
                _ => None,
            })
            .expect("intent")
    );
}

#[test]
fn poisoned_condition_does_not_hide_independent_branch_errors() {
    let source = insert_bindings("  let failed = missingCondition;\n  if failed { let left = missingLeft; } else { let right = missingRight; }");
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::UnresolvedType,
        ],
    );
    let origins: Vec<_> = errors
        .iter()
        .map(|error| &source[error.span.start..error.span.end])
        .collect();
    assert_eq!(origins, ["missingCondition", "missingLeft", "missingRight"]);
}

#[test]
fn poisoned_concatenation_does_not_hide_an_independent_operand_error() {
    let source = insert_bindings(
        "  let failed = missingConcatCause;\n  let combined = failed + missingConcatSibling;",
    );
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::UnresolvedType,
        ],
    );
    assert_eq!(
        &source[errors[1].span.start..errors[1].span.end],
        "missingConcatSibling"
    );
}

#[test]
fn poisoned_concatenation_does_not_hide_an_independent_operand_type_error() {
    let source =
        insert_bindings("  let failed = missingConcatTypeCause;\n  let combined = failed + true;");
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::TypeMismatch,
        ],
    );
    assert_eq!(&source[errors[1].span.start..errors[1].span.end], "true");
}

#[test]
fn failed_yield_branch_does_not_hide_an_independent_other_branch() {
    let source = insert_bindings("  let chosen: Bool = if true {\n    let failed = missingYield;\n    yield failed;\n  } else {\n    let independent = missingElse;\n    yield true;\n  };");
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::UnresolvedType,
        ],
    );
    assert_eq!(
        &source[errors[0].span.start..errors[0].span.end],
        "missingYield"
    );
    assert_eq!(
        &source[errors[1].span.start..errors[1].span.end],
        "missingElse"
    );
}

#[test]
fn poisoned_obstruction_shorthand_keeps_only_its_cause() {
    let valid =
        insert_bindings("  let failed = true;\n  require true else example.Failure({ failed });");
    compile(&valid).expect("valid obstruction shorthand control");
    let source = valid.replace("let failed = true;", "let failed = missingPayloadCause;");
    let errors = check_kinds(&source, &[CompilerErrorKind::UnresolvedType]);
    assert_eq!(
        &source[errors[0].span.start..errors[0].span.end],
        "missingPayloadCause"
    );
    let unknown = insert_bindings("  require true else example.Failure({ unknownPayload });");
    let errors = check_kinds(&unknown, &[CompilerErrorKind::UnresolvedType]);
    assert_eq!(
        &unknown[errors[0].span.start..errors[0].span.end],
        "unknownPayload"
    );
}

#[test]
fn poisoned_yield_condition_keeps_both_branch_errors() {
    let valid = insert_bindings("  let failed = true;\n  let chosen: Bool = if failed { let left = true; yield left; } else { let right = false; yield right; };");
    compile(&valid).expect("valid annotated yield control");
    let source = valid
        .replace("let failed = true;", "let failed = missingYieldCondition;")
        .replace("let left = true;", "let left = missingYieldLeft;")
        .replace("let right = false;", "let right = missingYieldRight;");
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::UnresolvedType,
        ],
    );
    let origins: Vec<_> = errors
        .iter()
        .map(|error| &source[error.span.start..error.span.end])
        .collect();
    assert_eq!(
        origins,
        [
            "missingYieldCondition",
            "missingYieldLeft",
            "missingYieldRight"
        ]
    );
}

#[test]
fn poisoned_logical_operand_keeps_the_independent_peer() {
    for op in ["&&", "||"] {
        let valid = insert_bindings(&format!(
            "  let failed = true;\n  if failed {op} false {{ let branch = true; }}"
        ));
        compile(&valid).expect("valid logical predicate control");
        let source = valid
            .replace("let failed = true;", "let failed = missingLogicalCause;")
            .replace(
                &format!("failed {op} false"),
                &format!("failed {op} missingLogicalPeer"),
            );
        let errors = check_kinds(
            &source,
            &[
                CompilerErrorKind::UnresolvedType,
                CompilerErrorKind::UnresolvedType,
            ],
        );
        assert_eq!(
            &source[errors[1].span.start..errors[1].span.end],
            "missingLogicalPeer"
        );
    }
}

#[test]
fn source_function_failed_locals_preserve_later_causes() {
    let valid = "package recovery.functions@1; fn recover() -> Bool { let failed = true; let dependent = failed; let independent = false; return dependent; } intent evaluate(input: Bool) returns Bool profile p.read basis none budget <= p.large { let anchor = input; return anchor; }";
    compile_function_recovery(valid).expect("valid source-function recovery control");
    let source = valid
        .replace("let failed = true;", "let failed = missingFunctionCause;")
        .replace(
            "let independent = false;",
            "let independent = missingFunctionSibling;",
        );
    let errors =
        compile_function_recovery(&source).expect_err("invalid function never produces Core");
    assert_eq!(
        errors.iter().map(|error| error.kind).collect::<Vec<_>>(),
        [
            CompilerErrorKind::UnresolvedType,
            CompilerErrorKind::UnresolvedType
        ]
    );
    let origins: Vec<_> = errors
        .iter()
        .map(|error| &source[error.span.start..error.span.end])
        .collect();
    assert_eq!(origins, ["missingFunctionCause", "missingFunctionSibling"]);
}

fn compile_function_recovery(source: &str) -> Result<CoreModule, Vec<CompilerError>> {
    let module = parse_module(source).expect("function recovery source parses");
    let context = edict_syntax::CompilerContext::new()
        .with_operation_profile("p.read", "continuum.profile.read-only/v1")
        .with_budget(
            "p.large",
            edict_syntax::CoreBudget {
                max_steps: 4096,
                max_allocated_bytes: u64::MAX,
                max_output_bytes: u64::MAX,
            },
        );
    compile_to_core(&module, &context)
}

#[test]
fn poisoned_require_predicate_keeps_failure_payload_causes() {
    for arm in [
        "example.Failure({ payload: true })",
        "continue obstructed { reason: example.Failure, payload: true }",
    ] {
        let valid = insert_bindings(&format!(
            "  let failed = true;\n  require failed else {arm};"
        ));
        compile(&valid).expect("valid require failure arm control");
        let source = valid
            .replace("let failed = true;", "let failed = missingRequireCause;")
            .replace("payload: true", "payload: missingRequirePayload");
        let errors = check_kinds(
            &source,
            &[
                CompilerErrorKind::UnresolvedType,
                CompilerErrorKind::UnresolvedType,
            ],
        );
        assert_eq!(
            &source[errors[1].span.start..errors[1].span.end],
            "missingRequirePayload"
        );
    }
}

#[test]
fn invalid_require_reason_keeps_independent_payload_cause() {
    let valid = insert_bindings(
        "  require true else continue obstructed { reason: example.Failure, payload: true };",
    );
    compile(&valid).expect("valid continuing obstruction control");
    let source = valid
        .replace("reason: example.Failure", "reason: true")
        .replace("payload: true", "payload: missingIndependentPayload");
    let errors = check_kinds(
        &source,
        &[
            CompilerErrorKind::UnsupportedSourceShape,
            CompilerErrorKind::UnresolvedType,
        ],
    );
    assert_eq!(
        &source[errors[1].span.start..errors[1].span.end],
        "missingIndependentPayload"
    );
}
