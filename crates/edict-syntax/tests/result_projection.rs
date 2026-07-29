//! Compiler-owned result-projection contract and independent verification.

use std::collections::BTreeMap;

use edict_syntax::{
    decode_canonical_cbor, decode_lawpack_adapter, decode_lawpack_bundle, decode_result_projection,
    digest_core_module, digest_result_projection, emit_result_projection, encode_canonical_cbor,
    encode_result_projection, lower_to_target_ir, parse_module, prepare_lawpack_compilation,
    verify_result_projection, CanonicalValue, CoreExpr, CoreModule, LocalRef, ResultProjection,
    ResultProjectionExpr, ResultProjectionFailureKind, ResultProjectionSource, TargetIrArtifact,
    TargetLoweringReport, TargetLoweringStatus, MAX_RESULT_PROJECTION_ARTIFACT_BYTES,
    MAX_RESULT_PROJECTION_NODES, MAX_RESULT_PROJECTION_PATH_SEGMENTS,
    MAX_RESULT_PROJECTION_TEXT_BYTES, RESULT_PROJECTION_API_VERSION,
};

const MANIFEST_BYTES: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/manifest.cbor");
const EXPORTS_BYTES: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/exports.cbor");
const ADAPTER_BYTES: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/adapter.cbor");
const SOURCE: &str = include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.edict");
const PROJECTION_BYTES: &[u8] =
    include_bytes!("../../../fixtures/lawpack/hello-echo/create-greeting.result-projection.cbor");
const PROJECTION_DIGEST: &str =
    include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.result-projection.sha256");
const PROPERTY_SEED: u64 = 0x1730_5eed_cafe_babe;

fn hello_echo_lowering() -> (CoreModule, TargetLoweringReport) {
    let bundle =
        decode_lawpack_bundle(MANIFEST_BYTES, EXPORTS_BYTES).expect("load Hello Echo lawpack");
    let adapter = decode_lawpack_adapter(&bundle, "echo.dpo@1", ADAPTER_BYTES)
        .expect("load Hello Echo adapter");
    let module = parse_module(SOURCE).expect("parse Hello Echo source");
    let preparation = prepare_lawpack_compilation(&module, &bundle, &adapter)
        .expect("prepare Hello Echo compilation");
    let core = edict_syntax::compile_to_core(&module, preparation.compiler_context())
        .expect("compile Hello Echo Core");
    let target = lower_to_target_ir(&core, preparation.target_ir_facts());
    assert_eq!(target.status, TargetLoweringStatus::Lowered);
    (core, target)
}

fn hello_echo() -> (CoreModule, TargetIrArtifact) {
    let (core, target) = hello_echo_lowering();
    (core, target.artifact.expect("lower Hello Echo Target IR"))
}

fn expected_expression() -> ResultProjectionExpr {
    ResultProjectionExpr::Record {
        fields: BTreeMap::from([
            (
                "key".to_owned(),
                ResultProjectionExpr::Source {
                    source: ResultProjectionSource::CapabilityResult {
                        step_id: "createGreeting.step.0".to_owned(),
                    },
                    path: vec!["key".to_owned()],
                },
            ),
            (
                "message".to_owned(),
                ResultProjectionExpr::Source {
                    source: ResultProjectionSource::ApplicationInput,
                    path: vec!["message".to_owned()],
                },
            ),
        ]),
    }
}

fn projection_with_fields(count: usize) -> ResultProjection {
    let fields = (0..count)
        .map(|index| {
            (
                format!("field{index:04}"),
                ResultProjectionExpr::Source {
                    source: ResultProjectionSource::ApplicationInput,
                    path: vec![format!("source{index:04}")],
                },
            )
        })
        .collect();
    ResultProjection {
        api_version: RESULT_PROJECTION_API_VERSION.to_owned(),
        operation_coordinate: "examples.boundary@1.project".to_owned(),
        output_type: "examples.boundary@1.Output".to_owned(),
        max_output_bytes: 65_536,
        expression: ResultProjectionExpr::Record { fields },
    }
}

fn mutate_results(core: &mut CoreModule, target: &mut TargetIrArtifact, expression: CoreExpr) {
    core.intents
        .get_mut("createGreeting")
        .expect("Core intent")
        .body
        .result = expression.clone();
    target
        .intents
        .get_mut("createGreeting")
        .expect("Target IR intent")
        .result = expression;
    repin_target_core(core, target);
}

fn repin_target_core(core: &CoreModule, target: &mut TargetIrArtifact) {
    target
        .semantic_closure
        .as_mut()
        .expect("semantic closure")
        .source_core
        .digest = Some(
        digest_core_module(core)
            .expect("digest mutated Core")
            .to_review_string(),
    );
}

fn remove_canonical_field(value: &mut CanonicalValue, field: &str) {
    let CanonicalValue::Map(entries) = value else {
        panic!("canonical projection root is a map");
    };
    entries.retain(|(key, _)| key != &CanonicalValue::Text(field.to_owned()));
}

#[test]
fn exact_core_and_target_ir_emit_the_typed_hello_echo_projection() {
    let (core, target) = hello_echo();
    let artifact = emit_result_projection(&core, &target, "createGreeting")
        .expect("emit exact result projection");

    assert_eq!(
        artifact.projection.api_version,
        RESULT_PROJECTION_API_VERSION
    );
    assert_eq!(
        artifact.projection.operation_coordinate,
        "examples.hello_echo@1.createGreeting"
    );
    assert_eq!(
        artifact.projection.output_type,
        "examples.hello_echo@1.GreetingCreated"
    );
    assert_eq!(artifact.projection.max_output_bytes, 512);
    assert_eq!(artifact.projection.expression, expected_expression());
    assert_eq!(
        decode_result_projection(&artifact.canonical_bytes).expect("decode emitted projection"),
        artifact.projection
    );
    assert_eq!(
        digest_result_projection(&artifact.projection).expect("digest projection"),
        artifact.digest
    );
    assert_eq!(artifact.canonical_bytes, PROJECTION_BYTES);
    assert_eq!(artifact.digest.to_review_string(), PROJECTION_DIGEST.trim());
}

#[test]
fn echo_target_lowering_emits_the_verified_result_projection() {
    let (core, report) = hello_echo_lowering();
    let artifact = report.artifact.as_ref().expect("lowered Target IR");
    let projection = report
        .result_projections
        .get("createGreeting")
        .expect("target lowerer emits the application result projection");

    assert_eq!(projection.canonical_bytes, PROJECTION_BYTES);
    assert_eq!(
        projection.digest.to_review_string(),
        PROJECTION_DIGEST.trim()
    );
    verify_result_projection(
        &core,
        artifact,
        "createGreeting",
        &projection.canonical_bytes,
        projection.digest,
    )
    .expect("independent verifier reconstructs the lowerer output");
}

#[test]
fn independent_verifier_reconstructs_the_authored_core_result() {
    let (core, target) = hello_echo();
    let artifact =
        emit_result_projection(&core, &target, "createGreeting").expect("emit projection");
    let verified = verify_result_projection(
        &core,
        &target,
        "createGreeting",
        &artifact.canonical_bytes,
        artifact.digest,
    )
    .expect("independent verifier accepts projection");

    assert_eq!(verified.projection(), &artifact.projection);
    assert_eq!(verified.digest(), artifact.digest);
}

#[test]
fn mutated_target_result_fails_closed() {
    let (core, mut target) = hello_echo();
    target
        .intents
        .get_mut("createGreeting")
        .expect("Target IR intent")
        .result = CoreExpr::Local {
        reference: LocalRef {
            id: "arg.0".to_owned(),
            alpha_name: "$arg0".to_owned(),
            ty: "examples.hello_echo@1.CreateGreetingInput".to_owned(),
        },
    };

    let failure = emit_result_projection(&core, &target, "createGreeting")
        .expect_err("mutated Target IR result must reject");
    assert_eq!(
        failure.kind(),
        ResultProjectionFailureKind::TargetResultMismatch
    );
}

#[test]
fn mutated_target_core_closure_fails_closed() {
    let (core, mut target) = hello_echo();
    target
        .semantic_closure
        .as_mut()
        .expect("semantic closure")
        .source_core
        .digest = Some(format!("sha256:{}", "0".repeat(64)));

    let failure = emit_result_projection(&core, &target, "createGreeting")
        .expect_err("mutated source Core identity must reject");
    assert_eq!(
        failure.kind(),
        ResultProjectionFailureKind::CoreTargetMismatch
    );
}

#[test]
fn undeclared_locals_and_unsupported_calls_fail_closed() {
    let (mut core, mut target) = hello_echo();
    mutate_results(
        &mut core,
        &mut target,
        CoreExpr::Local {
            reference: LocalRef {
                id: "local.999".to_owned(),
                alpha_name: "$999".to_owned(),
                ty: "examples.hello_echo@1.GreetingCreated".to_owned(),
            },
        },
    );
    let undeclared = emit_result_projection(&core, &target, "createGreeting")
        .expect_err("undeclared local must reject");
    assert_eq!(
        undeclared.kind(),
        ResultProjectionFailureKind::UndeclaredProjectionSource
    );

    let (mut core, mut target) = hello_echo();
    mutate_results(
        &mut core,
        &mut target,
        CoreExpr::Call {
            callee: "examples.hidden@1.callback".to_owned(),
            type_args: Vec::new(),
            args: Vec::new(),
        },
    );
    let call = emit_result_projection(&core, &target, "createGreeting")
        .expect_err("call expression must reject");
    assert_eq!(
        call.kind(),
        ResultProjectionFailureKind::UnsupportedExpression
    );
}

#[test]
fn incomplete_output_and_zero_output_bound_fail_closed() {
    let (mut core, mut target) = hello_echo();
    let key = core.intents["createGreeting"].body.result.clone();
    let CoreExpr::Record { mut fields } = key else {
        panic!("Hello Echo result is a record");
    };
    fields.remove("message");
    mutate_results(&mut core, &mut target, CoreExpr::Record { fields });
    let incomplete = emit_result_projection(&core, &target, "createGreeting")
        .expect_err("incomplete output must reject");
    assert_eq!(
        incomplete.kind(),
        ResultProjectionFailureKind::OutputShapeMismatch
    );

    let (mut core, mut target) = hello_echo();
    core.intents
        .get_mut("createGreeting")
        .expect("Core intent")
        .core_evaluation_budget
        .max_output_bytes = 0;
    target
        .intents
        .get_mut("createGreeting")
        .expect("Target intent")
        .core_evaluation_budget
        .max_output_bytes = 0;
    repin_target_core(&core, &mut target);
    let unbounded = emit_result_projection(&core, &target, "createGreeting")
        .expect_err("zero output bound must reject");
    assert_eq!(
        unbounded.kind(),
        ResultProjectionFailureKind::InvalidOutputBound
    );
}

#[test]
fn canonical_bytes_and_digest_are_independently_enforced() {
    let (core, target) = hello_echo();
    let artifact =
        emit_result_projection(&core, &target, "createGreeting").expect("emit projection");

    let mut malformed = artifact.canonical_bytes.clone();
    malformed.push(0);
    let malformed_failure = verify_result_projection(
        &core,
        &target,
        "createGreeting",
        &malformed,
        artifact.digest,
    )
    .expect_err("trailing data must reject");
    assert_eq!(
        malformed_failure.kind(),
        ResultProjectionFailureKind::InvalidCanonicalArtifact
    );

    let different = digest_result_projection(&projection_with_fields(1))
        .expect("digest different canonical projection");
    let digest_failure = verify_result_projection(
        &core,
        &target,
        "createGreeting",
        &artifact.canonical_bytes,
        different,
    )
    .expect_err("substituted digest must reject");
    assert_eq!(
        digest_failure.kind(),
        ResultProjectionFailureKind::DigestMismatch
    );
}

#[test]
fn projection_node_limit_accepts_the_boundary_and_rejects_the_next_node() {
    let boundary = projection_with_fields(MAX_RESULT_PROJECTION_NODES - 1);
    encode_result_projection(&boundary).expect("exact node boundary encodes");

    let oversized = projection_with_fields(MAX_RESULT_PROJECTION_NODES);
    let failure =
        encode_result_projection(&oversized).expect_err("one node beyond boundary must reject");
    assert_eq!(
        failure.kind(),
        ResultProjectionFailureKind::ProjectionLimitExceeded
    );
}

#[test]
fn path_text_artifact_and_structure_bounds_fail_closed() {
    let mut path_projection = projection_with_fields(1);
    path_projection.expression = ResultProjectionExpr::Source {
        source: ResultProjectionSource::ApplicationInput,
        path: vec!["field".to_owned(); MAX_RESULT_PROJECTION_PATH_SEGMENTS + 1],
    };
    assert_eq!(
        encode_result_projection(&path_projection)
            .expect_err("overlong source path must reject")
            .kind(),
        ResultProjectionFailureKind::ProjectionLimitExceeded
    );

    let mut text_projection = projection_with_fields(1);
    text_projection.operation_coordinate = "x".repeat(MAX_RESULT_PROJECTION_TEXT_BYTES + 1);
    assert_eq!(
        encode_result_projection(&text_projection)
            .expect_err("overlong coordinate must reject")
            .kind(),
        ResultProjectionFailureKind::ProjectionLimitExceeded
    );

    assert_eq!(
        decode_result_projection(&vec![0; MAX_RESULT_PROJECTION_ARTIFACT_BYTES + 1])
            .expect_err("oversized artifact must reject before decoding")
            .kind(),
        ResultProjectionFailureKind::ProjectionLimitExceeded
    );

    let bytes =
        encode_result_projection(&projection_with_fields(1)).expect("encode complete projection");
    let mut incomplete = decode_canonical_cbor(&bytes).expect("decode projection value");
    remove_canonical_field(&mut incomplete, "outputType");
    let incomplete_bytes =
        encode_canonical_cbor(&incomplete).expect("encode incomplete canonical value");
    assert_eq!(
        decode_result_projection(&incomplete_bytes)
            .expect_err("incomplete projection must reject")
            .kind(),
        ResultProjectionFailureKind::InvalidCanonicalArtifact
    );
}

#[test]
fn canonical_encoding_is_insertion_order_independent_for_fixed_seed_cases() {
    let expected =
        encode_result_projection(&projection_with_fields(32)).expect("encode ordered projection");
    let mut state = PROPERTY_SEED;

    for _case in 0..64 {
        let mut indexes = (0..32).collect::<Vec<_>>();
        for index in (1..indexes.len()).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let selected = usize::try_from(state % u64::try_from(index + 1).expect("small index"))
                .expect("selected index");
            indexes.swap(index, selected);
        }
        let mut fields = BTreeMap::new();
        for index in indexes {
            fields.insert(
                format!("field{index:04}"),
                ResultProjectionExpr::Source {
                    source: ResultProjectionSource::ApplicationInput,
                    path: vec![format!("source{index:04}")],
                },
            );
        }
        let candidate = ResultProjection {
            expression: ResultProjectionExpr::Record { fields },
            ..projection_with_fields(0)
        };
        assert_eq!(
            encode_result_projection(&candidate).expect("encode property candidate"),
            expected
        );
    }
}

#[test]
fn repeated_emit_and_verify_is_stable_under_bounded_stress() {
    let (core, target) = hello_echo();
    let first = emit_result_projection(&core, &target, "createGreeting").expect("emit projection");
    for _attempt in 0..128 {
        let next =
            emit_result_projection(&core, &target, "createGreeting").expect("repeat emission");
        assert_eq!(next, first);
        verify_result_projection(
            &core,
            &target,
            "createGreeting",
            &next.canonical_bytes,
            next.digest,
        )
        .expect("repeat verification");
    }
}
