//! Primary effect diagnostics use the exact authenticated lawpack closure.
use edict_syntax::{
    compile_to_core, decode_lawpack_adapter, decode_lawpack_bundle, parse_module,
    prepare_lawpack_compilation, CompilerError, CompilerErrorKind, CompilerStage, WriteClass,
};

const SOURCE: &str = include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.edict");
const MANIFEST: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/manifest.cbor");
const EXPORTS: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/exports.cbor");
const ADAPTER: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/adapter.cbor");

fn compile(source: &str) -> Result<edict_syntax::CoreModule, Vec<CompilerError>> {
    let module = parse_module(source).expect("effect source parses");
    let bundle = decode_lawpack_bundle(MANIFEST, EXPORTS).unwrap();
    let adapter = decode_lawpack_adapter(&bundle, "echo.dpo@1", ADAPTER).unwrap();
    let preparation = prepare_lawpack_compilation(&module, &bundle, &adapter).unwrap();
    let context = preparation
        .compiler_context()
        .clone()
        .with_effect_write_class("other.secret", WriteClass::Create);
    compile_to_core(&module, &context)
}

#[test]
fn bare_imported_effect_reports_missing_failure_mapping() {
    let mapping = "\n    else { alreadyExists(existing) => hello.AlreadyExists }";
    assert_eq!(SOURCE.matches(mapping).count(), 1);
    let source = SOURCE.replace(mapping, "");
    let errors = compile(&source).unwrap_err();
    let cause = &errors[0];
    assert_eq!(cause.kind, CompilerErrorKind::EffectWithoutFailureMapping);
    assert_eq!(cause.stage, CompilerStage::TypeCheck);
    assert!(source[cause.span.start..cause.span.end].contains("hello.createGreeting"));
    assert!(cause.message.contains("alreadyExists"));
}

#[test]
fn unknown_effect_lists_its_exact_owner_exports() {
    for alias in ["hello", "cells"] {
        let source = SOURCE
            .replace("as hello;", &format!("as {alias};"))
            .replace("hello.", &format!("{alias}."))
            .replace(&format!("{alias}.echo@1"), "hello.echo@1");
        let requested = format!("{alias}.update(input)");
        let source = source.replace(&format!("{alias}.createGreeting(input)"), &requested);
        let errors = compile(&source).unwrap_err();
        let cause = &errors[0];
        assert_eq!(cause.kind, CompilerErrorKind::MissingContextFact);
        assert_eq!(cause.stage, CompilerStage::TypeCheck);
        assert!(source[cause.span.start..cause.span.end].contains(&requested));
        assert!(cause.message.contains("hello.echo@1"));
        assert!(cause.message.contains(&format!("{alias}.createGreeting")));
        assert!(!cause.message.contains("other.secret"));
    }
}

#[test]
fn mapped_effect_controls_remain_valid() {
    compile(SOURCE).expect("mapped effect compiles");
    let renamed = SOURCE
        .replace("as hello;", "as cells;")
        .replace("hello.", "cells.")
        .replace("cells.echo@1", "hello.echo@1");
    compile(&renamed).expect("exact owner remains valid through renamed alias");
}

#[test]
fn bare_unknown_call_remains_unresolved_function() {
    let mapping = "\n    else { alreadyExists(existing) => hello.AlreadyExists }";
    assert_eq!(SOURCE.matches(mapping).count(), 1);
    assert_eq!(SOURCE.matches("hello.createGreeting(input)").count(), 1);
    let source = SOURCE
        .replace(mapping, "")
        .replace("hello.createGreeting(input)", "hello.update(input)");
    let errors = compile(&source).expect_err("unknown bare call has no helper fact");
    let cause = &errors[0];
    assert_eq!(cause.kind, CompilerErrorKind::UnresolvedFunction);
    assert_eq!(cause.stage, CompilerStage::TypeCheck);
    assert!(source[cause.span.start..cause.span.end].contains("hello.update(input)"));
}

#[test]
fn legacy_context_without_signatures_does_not_claim_no_exports() {
    let source = r#"package a.b@1;
use lawpack hello.echo@1 digest "sha256:1111111111111111111111111111111111111111111111111111111111111111" as hello;
type Input = { id: String<max=16>, };
type Receipt = { id: String<max=16>, };
type Output = { id: String<max=16>, };
intent t(input: Input) returns Output
  profile p.effectful
  basis none
  budget <= p.tiny {
  let receipt: Receipt = hello.known(input.id)
    else { rejected(reason) => domain.WriteRejected };
  return { id: input.id };
}"#;
    let context = edict_syntax::CompilerContext::new()
        .with_operation_profile("p.effectful", "test.effectful@1")
        .with_operation_profile_write_classes("p.effectful", [WriteClass::Create])
        .with_budget(
            "p.tiny",
            edict_syntax::CoreBudget {
                max_steps: 100,
                max_allocated_bytes: 512,
                max_output_bytes: 512,
            },
        )
        .with_effect_write_class("hello.known", WriteClass::Create);
    let known = parse_module(source).expect("legacy effect source parses");
    compile_to_core(&known, &context)
        .expect("legacy write-class context still compiles its known effect");
    assert_eq!(source.matches("hello.known(input.id)").count(), 1);
    let unknown = source.replace("hello.known(input.id)", "hello.unknown(input.id)");
    let module = parse_module(&unknown).expect("unknown legacy effect parses");
    let errors =
        compile_to_core(&module, &context).expect_err("unknown effect has no write-class fact");
    let cause = &errors[0];
    assert_eq!(cause.kind, CompilerErrorKind::MissingContextFact);
    assert_eq!(cause.stage, CompilerStage::TypeCheck);
    assert!(unknown[cause.span.start..cause.span.end].contains("hello.unknown(input.id)"));
    assert!(cause
        .message
        .contains("missing authenticated effect export information"));
    assert!(!cause.message.contains("exports: none"));
}
