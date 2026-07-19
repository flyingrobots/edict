//! End-to-end compiler and Target IR evidence for executable-operation prerequisites.
//!
//! This fixture remains application-neutral. It proves Edict preserves exact
//! operation inputs and semantic-resource identity; it does not claim Echo
//! admission, execution, commitment, or receipt evidence.

use edict_syntax::{
    compile_to_core, decode_canonical_cbor, digest_core_module, digest_target_ir_artifact,
    encode_core_module, encode_target_ir_artifact, lower_to_target_ir, parse_module,
    CanonicalValue, CompilerContext, CompilerErrorKind, CompilerStage, CoreBudget, CoreImportKind,
    CoreType, ResourceRef, TargetEffectLowering, TargetIrArtifact, TargetIrLoweringFacts,
    TargetLoweringStatus, WriteClass, ECHO_DPO_TARGET_PROFILE, ECHO_SPAN_IR_DOMAIN,
};

const OPERATION_SOURCE: &str =
    include_str!("../../../fixtures/lang/operations/explicit-basis-u64.edict");

fn operation_context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("sequence.splice", "continuum.profile.write/v1")
        .with_operation_profile_write_classes("sequence.splice", [WriteClass::Replace])
        .with_effect_write_class("sequence.splice", WriteClass::Replace)
        .with_budget(
            "sequence.small",
            CoreBudget {
                max_steps: 64,
                max_allocated_bytes: 16 * 1024,
                max_output_bytes: 4096,
            },
        )
}

fn digest(hex: char) -> String {
    format!("sha256:{}", hex.to_string().repeat(64))
}

fn target_facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: ECHO_DPO_TARGET_PROFILE.to_owned(),
            digest: Some(digest('2')),
        },
        target_ir_domain: ECHO_SPAN_IR_DOMAIN.to_owned(),
        operation_profiles: vec!["continuum.profile.write/v1".to_owned()],
        obstruction_coordinates: vec!["rejected".to_owned()],
        effect_lowerings: vec![TargetEffectLowering {
            effect: "sequence.splice".to_owned(),
            target_intrinsic: "echo.dpo@1.splice".to_owned(),
        }],
    }
}

fn compile_operation(source: &str) -> edict_syntax::CoreModule {
    let module = parse_module(source).expect("operation prerequisite source parses");
    compile_to_core(&module, &operation_context()).expect("operation prerequisite compiles")
}

fn lower_operation(source: &str) -> (edict_syntax::CoreModule, TargetIrArtifact) {
    let core = compile_operation(source);
    let report = lower_to_target_ir(&core, &target_facts());
    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    let artifact = report.artifact.expect("operation prerequisite lowers");
    (core, artifact)
}

fn map_field<'a>(value: &'a CanonicalValue, field: &str) -> &'a CanonicalValue {
    let CanonicalValue::Map(entries) = value else {
        panic!("expected map while looking up {field:?}, got {value:?}");
    };
    entries
        .iter()
        .find_map(|(key, value)| match key {
            CanonicalValue::Text(key) if key == field => Some(value),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing canonical field {field:?}"))
}

fn text_field<'a>(value: &'a CanonicalValue, field: &str) -> &'a str {
    let CanonicalValue::Text(value) = map_field(value, field) else {
        panic!("canonical field {field:?} is not text");
    };
    value
}

fn digest_bytes(value: &CanonicalValue) -> &[u8] {
    let CanonicalValue::Array(parts) = value else {
        panic!("digest is not the canonical algorithm/payload pair");
    };
    let [CanonicalValue::Text(algorithm), CanonicalValue::Bytes(bytes)] = parts.as_slice() else {
        panic!("digest pair has the wrong canonical shape");
    };
    assert_eq!(algorithm, "sha256");
    bytes
}

#[test]
fn operation_prerequisite_fixture_preserves_fixed_width_basis_and_lawpack_closure() {
    let (core, target) = lower_operation(OPERATION_SOURCE);

    assert_eq!(
        core.types.get("SpliceInput.start"),
        Some(&CoreType::Int {
            width: "U64".to_owned()
        })
    );
    assert_eq!(
        core.types.get("SpliceInput.end"),
        Some(&CoreType::Int {
            width: "U64".to_owned()
        })
    );
    assert_eq!(
        core.types.get("SpliceInput.replacement"),
        Some(&CoreType::Bytes { max: 4096 })
    );
    let lawpack = core
        .imports
        .iter()
        .find(|import| import.kind == CoreImportKind::Lawpack)
        .expect("lawpack import reaches Core");
    assert_eq!(lawpack.resource.coordinate, "sequence.edit@1");
    let expected_lawpack_digest = digest('1');
    assert_eq!(
        lawpack.resource.digest.as_deref(),
        Some(expected_lawpack_digest.as_str())
    );

    let core_value = decode_canonical_cbor(&encode_core_module(&core).expect("Core encodes"))
        .expect("Core bytes decode");
    let core_intent = map_field(map_field(&core_value, "intents"), "splice");
    let core_basis = map_field(core_intent, "basis");
    assert_eq!(text_field(core_basis, "kind"), "field");
    assert_eq!(text_field(core_basis, "field"), "basis");

    let target_value =
        decode_canonical_cbor(&encode_target_ir_artifact(&target).expect("Target IR encodes"))
            .expect("Target IR bytes decode");
    let target_intent = map_field(map_field(&target_value, "intents"), "splice");
    assert_eq!(map_field(target_intent, "basis"), core_basis);

    let closure = map_field(&target_value, "semanticClosure");
    let source_core = map_field(closure, "sourceCore");
    assert_eq!(text_field(source_core, "id"), core.coordinate);
    assert_eq!(
        digest_bytes(map_field(source_core, "digest")),
        digest_core_module(&core).expect("Core digest").bytes()
    );
    let CanonicalValue::Array(lawpacks) = map_field(closure, "lawpacks") else {
        panic!("semantic closure lawpacks are not an array");
    };
    assert_eq!(lawpacks.len(), 1);
    assert_eq!(text_field(&lawpacks[0], "id"), "sequence.edit@1");
    assert_eq!(digest_bytes(map_field(&lawpacks[0], "digest")), &[0x11; 32]);
}

#[test]
fn out_of_range_u64_and_cross_width_values_reject_before_core() {
    for replacement in ["18446744073709551616u64", "1i64"] {
        let source = OPERATION_SOURCE.replace("18446744073709551615u64", replacement);
        let module = parse_module(&source).expect("mutated source parses");
        let errors = compile_to_core(&module, &operation_context())
            .expect_err("invalid fixed-width value must reject");
        assert_eq!(
            errors
                .iter()
                .map(|error| (error.stage, error.kind))
                .collect::<Vec<_>>(),
            vec![(CompilerStage::TypeCheck, CompilerErrorKind::TypeMismatch)]
        );
    }
}

#[test]
fn body_local_cannot_become_an_intent_basis() {
    let source = OPERATION_SOURCE.replace("basis input.basis", "basis receipt.version");
    let module = parse_module(&source).expect("mutated source parses");
    let errors = compile_to_core(&module, &operation_context())
        .expect_err("body-local basis must reject before Core");
    assert!(errors.iter().all(|error| {
        error.stage == CompilerStage::SurfaceValidation
            && error.kind == CompilerErrorKind::SurfaceValidation
    }));
}

#[test]
fn explicit_basis_and_semantic_input_mutations_move_target_identity() {
    let (_, baseline) = lower_operation(OPERATION_SOURCE);
    let baseline_digest = digest_target_ir_artifact(&baseline).expect("baseline Target IR digest");

    for mutated in [
        OPERATION_SOURCE.replace("basis input.basis", "basis input.replacement"),
        OPERATION_SOURCE.replace("sequence.edit@1", "sequence.alternate@1"),
        OPERATION_SOURCE.replace("18446744073709551615u64", "18446744073709551614u64"),
    ] {
        let (_, artifact) = lower_operation(&mutated);
        assert_ne!(
            digest_target_ir_artifact(&artifact).expect("mutated Target IR digest"),
            baseline_digest
        );
    }
}
