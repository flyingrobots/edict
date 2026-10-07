//! Exact lawpack signature diagnostics preserve compatibility and call identity.
use edict_syntax::{
    compile_to_core, decode_lawpack_adapter, decode_lawpack_bundle, parse_module,
    prepare_lawpack_compilation, CompilerError, CompilerErrorKind, CompilerStage, CoreModule,
};

const SOURCE: &str = include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.edict");
const MANIFEST: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/manifest.cbor");
const EXPORTS: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/exports.cbor");
const ADAPTER: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/adapter.cbor");

fn compile(source: &str) -> Result<CoreModule, Vec<CompilerError>> {
    let module = parse_module(source).expect("signature fixture parses");
    let bundle = decode_lawpack_bundle(MANIFEST, EXPORTS).expect("load exact lawpack");
    let adapter = decode_lawpack_adapter(&bundle, "echo.dpo@1", ADAPTER)
        .expect("load exact adapter");
    let preparation = prepare_lawpack_compilation(&module, &bundle, &adapter)
        .expect("authenticate signature fixture");
    compile_to_core(&module, preparation.compiler_context())
}

fn assert_signature_cause(source: &str, path: &str, expected: &str, actual: &str) {
    let errors = compile(source).expect_err("exported signature must reject");
    let cause = &errors[0];
    assert_eq!(cause.kind, CompilerErrorKind::TypeMismatch);
    assert_eq!(cause.stage, CompilerStage::TypeCheck);
    assert!(source[cause.span.start..cause.span.end].contains("hello.createGreeting(input)"));
    assert!(cause.message.contains(path), "missing field path: {cause:?}");
    assert!(cause.message.contains(expected), "missing expected bound: {cause:?}");
    assert!(cause.message.contains(actual), "missing actual bound: {cause:?}");
}

#[test]
fn input_signature_error_identifies_first_incompatible_field() {
    let declaration = "type WideInput = { basis: String<max=128>, key: String<max=65>, message: String<max=256>, };\n\n";
    let source = SOURCE.replace("type GreetingCreated", &format!("{declaration}type GreetingCreated"))
        .replace("input: hello.CreateGreetingInput", "input: WideInput");
    let control = source.replace("key: String<max=65>", "key: String<max=64>");
    compile(&control).expect("equivalent structural input compiles");
    assert_signature_cause(&source, "input.key", "String<max=64", "String<max=65");
}

#[test]
fn receipt_signature_error_identifies_first_incompatible_field() {
    let declaration = "type TightReceipt = { key: String<max=63>, };\n\n";
    let source = SOURCE.replace("type GreetingCreated", &format!("{declaration}type GreetingCreated"))
        .replace("let receipt: hello.GreetingReceipt", "let receipt: TightReceipt");
    let control = source.replace("key: String<max=63>", "key: String<max=64>");
    compile(&control).expect("equivalent receipt annotation compiles");
    assert_signature_cause(&source, "receipt.key", "String<max=63", "String<max=64");
}
