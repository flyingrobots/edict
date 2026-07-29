//! RED contract for typed external-action request values.
//!
//! The tests use only public compiler surfaces so the first failure is the
//! absence of request syntax/semantics, not a test compile error.

use std::collections::BTreeSet;

use edict_syntax::{
    compile_to_core, decode_canonical_cbor, digest_core_module, encode_core_module,
    encode_target_ir_artifact, lower_to_target_ir, parse_module, CanonicalValue, CompilerContext,
    CompilerErrorKind, CoreBudget, ResourceRef, TargetIrLoweringFacts, TargetLoweringStatus,
    ECHO_DPO_TARGET_PROFILE, ECHO_SPAN_IR_DOMAIN,
};

const OPERATION_DIGEST: char = 'a';
const INPUT_SCHEMA_DIGEST: char = 'b';
const SETTLEMENT_SCHEMA_DIGEST: char = 'c';
const RECONCILIATION_DIGEST: char = 'd';
const TARGET_PROFILE_DIGEST: char = 'e';
const PROPERTY_SEED: u64 = 0x4558_5452_4551_0001;

fn digest(hex: char) -> String {
    format!("sha256:{}", hex.to_string().repeat(64))
}

fn context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("workspace.read", "continuum.profile.read-only/v1")
        .with_budget(
            "workspace.tiny",
            CoreBudget {
                max_steps: 512,
                max_allocated_bytes: 256 * 1024,
                max_output_bytes: 128 * 1024,
            },
        )
}

fn target_facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: ECHO_DPO_TARGET_PROFILE.to_owned(),
            digest: Some(digest(TARGET_PROFILE_DIGEST)),
        },
        target_ir_domain: ECHO_SPAN_IR_DOMAIN.to_owned(),
        operation_profiles: vec!["continuum.profile.read-only/v1".to_owned()],
        obstruction_coordinates: Vec::new(),
        effect_lowerings: Vec::new(),
    }
}

fn request_source(
    capability_coordinate: &str,
    operation_alias: &str,
    operation_digest: &str,
    input_schema_digest: &str,
    settlement_schema_digest: &str,
    input_expr: &str,
    authority_expr: &str,
    basis_expr: &str,
    max_settlement_bytes: &str,
    max_attempts: &str,
    reconciliation_digest: &str,
) -> String {
    format!(
        r#"package examples.workspace_observer@1;

use capability {capability_coordinate} digest "{operation_digest}" as snapshot;

type ObserveInput = {{
  payload: Bytes<max=1024>,
  alternatePayload: Bytes<max=1024>,
  scope: Bytes<max=32>,
  alternateScope: Bytes<max=32>,
  basis: Bytes<max=32>,
  alternateBasis: Bytes<max=32>,
  maxSettlementBytes: U64,
  alternateMaxSettlementBytes: U64,
  maxAttempts: U32,
}};

intent observe(input: ObserveInput) returns ExternalActionRequest<Bytes<max=65536>>
  profile workspace.read
  basis input.basis
  budget <= workspace.tiny
{{
  request pending: ExternalActionRequest<Bytes<max=65536>> =
    {operation_alias}({input_expr})
    input schema workspace.snapshot.input@1 digest "{input_schema_digest}"
    settlement schema workspace.snapshot.settlement@1 digest "{settlement_schema_digest}"
    authority {authority_expr}
    basis {basis_expr}
    budget maxSettlementBytes {max_settlement_bytes} maxAttempts {max_attempts}
    reconcile workspace.snapshot.reconcile@1 digest "{reconciliation_digest}";
  return pending;
}}
"#
    )
}

fn baseline_source() -> String {
    request_source(
        "workspace.snapshot.observe@1",
        "snapshot",
        &digest(OPERATION_DIGEST),
        &digest(INPUT_SCHEMA_DIGEST),
        &digest(SETTLEMENT_SCHEMA_DIGEST),
        "input.payload",
        "input.scope",
        "input.basis",
        "input.maxSettlementBytes",
        "input.maxAttempts",
        &digest(RECONCILIATION_DIGEST),
    )
}

fn compile_source(source: &str) -> edict_syntax::CoreModule {
    let module = parse_module(source).expect("external-action source parses");
    compile_to_core(&module, &context()).expect("external-action source compiles")
}

fn lower_source(source: &str) -> (edict_syntax::CoreModule, edict_syntax::TargetIrArtifact) {
    let core = compile_source(source);
    let report = lower_to_target_ir(&core, &target_facts());
    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert_eq!(report.failures, Vec::new());
    (core, report.artifact.expect("external-action Target IR"))
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

fn array_field<'a>(value: &'a CanonicalValue, field: &str) -> &'a [CanonicalValue] {
    let CanonicalValue::Array(values) = map_field(value, field) else {
        panic!("canonical field {field:?} is not an array");
    };
    values
}

fn core_request_node(core: &edict_syntax::CoreModule) -> CanonicalValue {
    let encoded = encode_core_module(core).expect("Core request encodes");
    let value = decode_canonical_cbor(&encoded).expect("Core request decodes");
    let intent = map_field(map_field(&value, "intents"), "observe");
    array_field(map_field(intent, "body"), "nodes")[0].clone()
}

fn target_request_values(target: &edict_syntax::TargetIrArtifact) -> Vec<CanonicalValue> {
    let encoded = encode_target_ir_artifact(target).expect("Target IR request encodes");
    let value = decode_canonical_cbor(&encoded).expect("Target IR request decodes");
    let intent = map_field(map_field(&value, "intents"), "observe");
    array_field(intent, "externalActionRequests").to_vec()
}

#[test]
fn workspace_observation_request_compiles_as_non_callable_data() {
    let (core, target) = lower_source(&baseline_source());
    let core_value = decode_canonical_cbor(&encode_core_module(&core).expect("Core encodes"))
        .expect("Core CBOR");
    let imports = array_field(&core_value, "imports");
    assert_eq!(imports.len(), 1);
    assert_eq!(text_field(&imports[0], "kind"), "capability");
    assert_eq!(
        text_field(map_field(&imports[0], "ref"), "id"),
        "workspace.snapshot.observe@1"
    );

    let request = core_request_node(&core);
    assert_eq!(text_field(&request, "kind"), "externalActionRequest");
    assert_eq!(text_field(&request, "inputType"), "Bytes<max=1024>");
    assert_eq!(text_field(&request, "settlementType"), "Bytes<max=65536>");
    assert_eq!(text_field(&request, "state"), "awaitingSettlement");
    assert_eq!(
        text_field(&request, "settlementAdmission"),
        "schemaRequired"
    );

    let target_bytes = encode_target_ir_artifact(&target).expect("Target IR encodes");
    let target_value = decode_canonical_cbor(&target_bytes).expect("Target IR CBOR");
    let closure = map_field(&target_value, "semanticClosure");
    let capabilities = array_field(closure, "capabilities");
    assert_eq!(capabilities.len(), 1);
    assert_eq!(
        text_field(&capabilities[0], "id"),
        "workspace.snapshot.observe@1"
    );
    let intent = map_field(map_field(&target_value, "intents"), "observe");
    assert_eq!(array_field(intent, "steps"), []);
    let requests = array_field(intent, "externalActionRequests");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        text_field(map_field(&requests[0], "operation"), "id"),
        "workspace.snapshot.observe@1"
    );
    assert_eq!(
        text_field(&requests[0], "settlementAdmission"),
        "schemaRequired"
    );
    assert!(!target_bytes
        .windows("targetIntrinsic".len())
        .any(|window| window == b"targetIntrinsic"));
}

#[test]
fn undeclared_or_floating_operation_families_fail_closed() {
    let undeclared = baseline_source().replace("snapshot(input.payload)", "missing(input.payload)");
    let module = parse_module(&undeclared).expect("undeclared operation source parses");
    let errors = compile_to_core(&module, &context()).expect_err("undeclared operation rejects");
    assert_eq!(errors[0].kind, CompilerErrorKind::MissingContextFact);

    let floating =
        baseline_source().replace(&format!(" digest \"{}\"", digest(OPERATION_DIGEST)), "");
    let module = parse_module(&floating).expect("floating capability source parses");
    let core = compile_to_core(&module, &context()).expect("floating source reaches Core");
    assert_eq!(
        encode_core_module(&core)
            .expect_err("floating operation cannot become canonical")
            .kind(),
        edict_syntax::CanonicalErrorKind::UnresolvedDigest
    );
}

#[test]
fn request_operation_must_remain_in_core_and_target_capability_closure() {
    let (mut core, mut target) = lower_source(&baseline_source());
    core.imports.clear();
    assert_eq!(
        encode_core_module(&core)
            .expect_err("Core request without capability import rejects")
            .kind(),
        edict_syntax::CanonicalErrorKind::UnsupportedValue
    );

    target
        .semantic_closure
        .as_mut()
        .expect("request-bearing Target IR has a closure")
        .capabilities
        .clear();
    assert_eq!(
        encode_target_ir_artifact(&target)
            .expect_err("Target IR request without capability closure rejects")
            .kind(),
        edict_syntax::CanonicalErrorKind::UnsupportedValue
    );
}

#[test]
fn capability_import_cannot_be_called_as_an_effect() {
    let source = format!(
        r#"package examples.direct_call@1;
use capability workspace.snapshot.observe@1 digest "{}" as snapshot;
type Input = {{ payload: Bytes<max=1024>, }};
intent observe(input: Input) returns Bytes<max=65536>
  profile workspace.read
  basis none
  budget <= workspace.tiny {{
  let output: Bytes<max=65536> = snapshot(input.payload)
    else {{ rejected(reason) => workspace.Rejected }};
  return output;
}}
"#,
        digest(OPERATION_DIGEST)
    );
    let module = parse_module(&source).expect("direct call source parses");
    let errors = compile_to_core(&module, &context()).expect_err("direct call rejects");
    assert_eq!(errors[0].kind, CompilerErrorKind::MissingContextFact);
}

#[test]
fn ambient_operation_families_are_not_requestable() {
    for coordinate in [
        "filesystem.read@1",
        "process.spawn@1",
        "network.fetch@1",
        "git.push@1",
        "github.open_pull_request@1",
        "model.invoke@1",
        "shell.command@1",
    ] {
        let source = request_source(
            coordinate,
            "snapshot",
            &digest(OPERATION_DIGEST),
            &digest(INPUT_SCHEMA_DIGEST),
            &digest(SETTLEMENT_SCHEMA_DIGEST),
            "input.payload",
            "input.scope",
            "input.basis",
            "input.maxSettlementBytes",
            "input.maxAttempts",
            &digest(RECONCILIATION_DIGEST),
        );
        let module = parse_module(&source).expect("ambient-family source parses structurally");
        let errors =
            compile_to_core(&module, &context()).expect_err("ambient operation family rejects");
        assert_eq!(
            errors[0].kind,
            CompilerErrorKind::UnsupportedSourceShape,
            "{coordinate}"
        );
    }
}

#[test]
fn dynamic_admission_values_survive_without_compile_time_execution() {
    let core = compile_source(&baseline_source());
    let request = core_request_node(&core);
    assert_eq!(
        text_field(map_field(&request, "authorityScope"), "field"),
        "scope"
    );
    assert_eq!(text_field(map_field(&request, "basis"), "field"), "basis");
    let budget = map_field(&request, "budget");
    assert_eq!(
        text_field(map_field(budget, "maxSettlementBytes"), "field"),
        "maxSettlementBytes"
    );
    assert_eq!(
        text_field(map_field(budget, "maxAttempts"), "field"),
        "maxAttempts"
    );
}

#[test]
fn request_artifacts_are_reproducible() {
    let source = baseline_source();
    let (left_core, left_target) = lower_source(&source);
    let (right_core, right_target) = lower_source(&source);
    assert_eq!(
        encode_core_module(&left_core).expect("left Core"),
        encode_core_module(&right_core).expect("right Core")
    );
    assert_eq!(
        encode_target_ir_artifact(&left_target).expect("left Target IR"),
        encode_target_ir_artifact(&right_target).expect("right Target IR")
    );
}

#[test]
fn every_request_authority_field_moves_core_identity() {
    let baseline = baseline_source();
    let baseline_digest = digest_core_module(&compile_source(&baseline)).expect("baseline digest");
    let mutations = [
        baseline.replace(&digest(OPERATION_DIGEST), &digest('1')),
        baseline.replace(&digest(INPUT_SCHEMA_DIGEST), &digest('2')),
        baseline.replace(&digest(SETTLEMENT_SCHEMA_DIGEST), &digest('3')),
        baseline.replace("authority input.scope", "authority input.alternateScope"),
        baseline.replace("basis input.basis", "basis input.alternateBasis"),
        baseline.replace(
            "maxSettlementBytes input.maxSettlementBytes",
            "maxSettlementBytes input.alternateMaxSettlementBytes",
        ),
        baseline.replace(
            "snapshot(input.payload)",
            "snapshot(input.alternatePayload)",
        ),
        baseline.replace(&digest(RECONCILIATION_DIGEST), &digest('4')),
    ];
    for mutation in mutations {
        assert_ne!(
            digest_core_module(&compile_source(&mutation)).expect("mutated digest"),
            baseline_digest
        );
    }
}

#[test]
fn fixed_seed_request_identity_corpus_is_deterministic() {
    let mut state = PROPERTY_SEED;
    let mut observed = BTreeSet::new();
    for _ in 0..32 {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let mut hex = format!("{state:016x}");
        hex = hex.repeat(4);
        let operation_digest = format!("sha256:{hex}");
        let source = request_source(
            "workspace.snapshot.observe@1",
            "snapshot",
            &operation_digest,
            &digest(INPUT_SCHEMA_DIGEST),
            &digest(SETTLEMENT_SCHEMA_DIGEST),
            "input.payload",
            "input.scope",
            "input.basis",
            "input.maxSettlementBytes",
            "input.maxAttempts",
            &digest(RECONCILIATION_DIGEST),
        );
        let first = digest_core_module(&compile_source(&source)).expect("first property digest");
        let second = digest_core_module(&compile_source(&source)).expect("second property digest");
        assert_eq!(first, second);
        assert!(observed.insert(first));
    }
    assert_eq!(observed.len(), 32);
}

#[test]
fn sixty_four_requests_remain_bounded_non_callable_data() {
    let mut requests = String::new();
    for index in 0..64 {
        requests.push_str(&format!(
            r#"  request pending{index}: ExternalActionRequest<Bytes<max=65536>> =
    snapshot(input.payload)
    input schema workspace.snapshot.input@1 digest "{}"
    settlement schema workspace.snapshot.settlement@1 digest "{}"
    authority input.scope
    basis input.basis
    budget maxSettlementBytes input.maxSettlementBytes maxAttempts input.maxAttempts
    reconcile workspace.snapshot.reconcile@1 digest "{}";
"#,
            digest(INPUT_SCHEMA_DIGEST),
            digest(SETTLEMENT_SCHEMA_DIGEST),
            digest(RECONCILIATION_DIGEST),
        ));
    }
    let source = format!(
        r#"package examples.workspace_stress@1;
use capability workspace.snapshot.observe@1 digest "{}" as snapshot;
type ObserveInput = {{
  payload: Bytes<max=1024>,
  scope: Bytes<max=32>,
  basis: Bytes<max=32>,
  maxSettlementBytes: U64,
  maxAttempts: U32,
}};
intent observe(input: ObserveInput) returns ExternalActionRequest<Bytes<max=65536>>
  profile workspace.read
  basis input.basis
  budget <= workspace.tiny
{{
{requests}  return pending63;
}}
"#,
        digest(OPERATION_DIGEST),
    );
    let (core, target) = lower_source(&source);
    let core_value =
        decode_canonical_cbor(&encode_core_module(&core).expect("stress Core encodes"))
            .expect("stress Core decodes");
    let core_intent = map_field(map_field(&core_value, "intents"), "observe");
    assert_eq!(
        array_field(map_field(core_intent, "body"), "nodes").len(),
        64
    );
    assert_eq!(target_request_values(&target).len(), 64);

    let target_value = decode_canonical_cbor(
        &encode_target_ir_artifact(&target).expect("stress Target IR encodes"),
    )
    .expect("stress Target IR decodes");
    let target_intent = map_field(map_field(&target_value, "intents"), "observe");
    assert_eq!(array_field(target_intent, "steps"), []);
}
