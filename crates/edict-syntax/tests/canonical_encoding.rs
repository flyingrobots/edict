//! Canonical Core encoding behavior for the v0.3 encoder slice.
//!
//! These tests assert public behavior: deterministic canonical bytes,
//! canonical-CBOR validation, mutation sensitivity, and alpha-stable source
//! lowering. They intentionally do not freeze reviewed Core golden fixtures or
//! exact digest values; that belongs to issue #22.

use edict_syntax::{
    compile_to_core, decode_canonical_cbor, encode_canonical_cbor, encode_core_module,
    parse_module, CanonicalErrorKind, CanonicalValue, CompilerContext, CoreBudget,
};

const BOUNDED_HELLO: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");

fn hello_context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("hello.readOnly", "continuum.profile.read-only/v1")
        .with_budget(
            "hello.tinyBudget",
            CoreBudget {
                max_steps: 64,
                max_allocated_bytes: 4096,
                max_output_bytes: 1024,
            },
        )
}

#[test]
fn canonical_core_bytes_are_independent_of_map_construction_order() {
    let module = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let core = compile_to_core(&module, &hello_context()).expect("fixture compiles to Core");

    let mut reordered = core.clone();
    reordered.types = core
        .types
        .iter()
        .rev()
        .map(|(name, ty)| (name.clone(), ty.clone()))
        .collect();
    reordered.intents = core
        .intents
        .iter()
        .rev()
        .map(|(name, intent)| (name.clone(), intent.clone()))
        .collect();

    assert_eq!(
        encode_core_module(&core).expect("canonical encoding succeeds"),
        encode_core_module(&reordered).expect("canonical encoding succeeds")
    );
}

#[test]
fn canonical_core_bytes_change_when_core_meaning_changes() {
    let module = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let core = compile_to_core(&module, &hello_context()).expect("fixture compiles to Core");
    let mut changed = core.clone();
    changed
        .intents
        .get_mut("sayHello")
        .expect("intent exists")
        .core_evaluation_budget
        .max_steps += 1;

    assert_ne!(
        encode_core_module(&core).expect("canonical encoding succeeds"),
        encode_core_module(&changed).expect("canonical encoding succeeds")
    );
}

#[test]
fn canonical_core_bytes_decode_and_reencode_stably() {
    let module = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let core = compile_to_core(&module, &hello_context()).expect("fixture compiles to Core");
    let bytes = encode_core_module(&core).expect("canonical encoding succeeds");

    let decoded = decode_canonical_cbor(&bytes).expect("canonical bytes decode");

    assert_eq!(
        encode_canonical_cbor(&decoded).expect("canonical value re-encodes"),
        bytes
    );
}

#[test]
fn canonical_core_rejects_unresolved_import_digest() {
    let module = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let mut core = compile_to_core(&module, &hello_context()).expect("fixture compiles to Core");
    core.imports
        .first_mut()
        .expect("fixture has an import")
        .resource
        .digest = None;

    let err = encode_core_module(&core).expect_err("unresolved import digest rejects");

    assert_eq!(err.kind(), CanonicalErrorKind::UnresolvedDigest);
}

#[test]
fn noncanonical_cbor_bytes_reject_with_stable_error_kind() {
    let err = decode_canonical_cbor(&[0x18, 0x00]).expect_err("non-minimal zero rejects");

    assert_eq!(err.kind(), CanonicalErrorKind::NonCanonical);
}

#[test]
fn canonical_cbor_rejects_duplicate_map_keys_on_encode() {
    let err = encode_canonical_cbor(&CanonicalValue::Map(vec![
        (
            CanonicalValue::Text("key".to_owned()),
            CanonicalValue::Integer(1),
        ),
        (
            CanonicalValue::Text("key".to_owned()),
            CanonicalValue::Integer(2),
        ),
    ]))
    .expect_err("duplicate map keys reject");

    assert_eq!(err.kind(), CanonicalErrorKind::DuplicateMapKey);
}

#[test]
fn canonical_cbor_integer_widths_are_platform_independent() {
    assert_eq!(
        encode_canonical_cbor(&CanonicalValue::Integer(23)).expect("integer encodes"),
        vec![0x17]
    );
    assert_eq!(
        encode_canonical_cbor(&CanonicalValue::Integer(24)).expect("integer encodes"),
        vec![0x18, 0x18]
    );
    assert_eq!(
        encode_canonical_cbor(&CanonicalValue::Integer(256)).expect("integer encodes"),
        vec![0x19, 0x01, 0x00]
    );
    assert_eq!(
        encode_canonical_cbor(&CanonicalValue::Integer(-1)).expect("integer encodes"),
        vec![0x20]
    );
}

#[test]
fn canonical_core_bytes_are_source_alpha_rename_invariant() {
    let renamed = BOUNDED_HELLO
        .replace("let message = ", "let greeting = ")
        .replace("return { message };", "return { message: greeting };");

    let original = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let renamed = parse_module(&renamed).expect("renamed source parses");
    let original_core =
        compile_to_core(&original, &hello_context()).expect("original compiles to Core");
    let renamed_core = compile_to_core(&renamed, &hello_context()).expect("renamed compiles");

    assert_eq!(
        encode_core_module(&original_core).expect("canonical encoding succeeds"),
        encode_core_module(&renamed_core).expect("canonical encoding succeeds")
    );
}
