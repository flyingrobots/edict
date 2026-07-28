//! Canonical lawpack loading and dependency validation.
//!
//! These tests enter through exact canonical bytes and assert typed exports or
//! stable failure kinds. They do not construct an already-trusted manifest.

use edict_syntax::{
    compile_to_core, decode_canonical_cbor, decode_lawpack_adapter, decode_lawpack_bundle,
    digest_core_module, digest_target_ir_artifact, encode_canonical_cbor, encode_core_module,
    encode_target_ir_artifact, lower_to_target_ir, parse_module, prepare_lawpack_compilation,
    validate_lawpack_dependency_graph, CanonicalValue, LawpackAdapterFailureKind,
    LawpackExecutionClass, LawpackPureFunctionImplementation, LawpackValidationFailureKind,
    LawpackVerifierClass, TargetLoweringStatus, ValidatedLawpackBundle,
};
use sha2::{Digest, Sha256};

const DIGEST_FRAME: &str = "edict.digest/v1";
const EXPORTS_COORDINATE: &str = "hello.echo.exports/v1";
const ADAPTER_COORDINATE: &str = "hello.echo.echo-dpo-adapter/v1";
const MANIFEST_BYTES: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/manifest.cbor");
const EXPORTS_BYTES: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/exports.cbor");
const ADAPTER_BYTES: &[u8] = include_bytes!("../../../fixtures/lawpack/hello-echo/adapter.cbor");
const ADAPTER_DIGEST: &str = include_str!("../../../fixtures/lawpack/hello-echo/adapter.sha256");
const TARGET_CONFIGURATION_COORDINATE: &str = "hello.echo.echo-operation-configuration/v1";
const TARGET_CONFIGURATION_BYTES: &[u8] =
    include_bytes!("../../../fixtures/lawpack/hello-echo/echo-operation-configuration.cbor");
const TARGET_CONFIGURATION_DIGEST: &str =
    include_str!("../../../fixtures/lawpack/hello-echo/echo-operation-configuration.sha256");
const MANIFEST_DIGEST: &str = include_str!("../../../fixtures/lawpack/hello-echo/manifest.sha256");
const CREATE_GREETING_SOURCE: &str =
    include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.edict");
const CREATE_GREETING_CORE_BYTES: &[u8] =
    include_bytes!("../../../fixtures/lawpack/hello-echo/create-greeting.core.cbor");
const CREATE_GREETING_CORE_DIGEST: &str =
    include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.core.sha256");
const CREATE_GREETING_TARGET_IR_BYTES: &[u8] =
    include_bytes!("../../../fixtures/lawpack/hello-echo/create-greeting.target-ir.cbor");
const CREATE_GREETING_TARGET_IR_DIGEST: &str =
    include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.target-ir.sha256");

#[test]
fn hello_echo_lawpack_bundle_loads_from_exact_canonical_resources() {
    let bundle =
        decode_lawpack_bundle(MANIFEST_BYTES, EXPORTS_BYTES).expect("load Hello Echo lawpack");
    let source = parse_module(CREATE_GREETING_SOURCE).expect("parse createGreeting source");

    assert_eq!(bundle.manifest().id, "hello.echo");
    assert_eq!(bundle.manifest().version, "1");
    assert_eq!(bundle.exports().effects.len(), 1);
    assert_eq!(
        bundle.exports().effects[0].coordinate,
        "hello.echo@1.createGreeting"
    );
    assert_eq!(
        bundle.exports().effects[0].execution_class,
        LawpackExecutionClass::Runtime
    );
    assert_eq!(
        bundle.exports().effects[0]
            .effect_failures
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["alreadyExists"]
    );
    assert_eq!(
        source.imports[0].digest.as_deref(),
        Some(MANIFEST_DIGEST.trim())
    );
    assert_eq!(
        bundle.manifest_digest_review_string(),
        MANIFEST_DIGEST.trim()
    );
}

#[test]
fn hello_echo_source_compiles_to_echo_target_ir_from_exact_lawpack_adapter() {
    let bundle =
        decode_lawpack_bundle(MANIFEST_BYTES, EXPORTS_BYTES).expect("load Hello Echo lawpack");
    let module = parse_module(CREATE_GREETING_SOURCE).expect("parse createGreeting source");
    let adapter =
        decode_lawpack_adapter(&bundle, "echo.dpo@1", ADAPTER_BYTES).expect("load exact adapter");
    assert_eq!(adapter.digest_review_string(), ADAPTER_DIGEST.trim());
    let effect = adapter
        .effects()
        .get("hello.echo@1.createGreeting")
        .expect("createGreeting adapter effect");
    assert_eq!(
        effect.target_configuration.id,
        TARGET_CONFIGURATION_COORDINATE
    );
    assert_eq!(
        effect.target_configuration.digest_review_string(),
        TARGET_CONFIGURATION_DIGEST.trim()
    );
    let target_configuration =
        decode_canonical_cbor(TARGET_CONFIGURATION_BYTES).expect("decode target configuration");
    assert_eq!(
        encode_canonical_cbor(&target_configuration).expect("re-encode target configuration"),
        TARGET_CONFIGURATION_BYTES
    );
    assert_eq!(
        digest_value(TARGET_CONFIGURATION_COORDINATE, &target_configuration),
        effect.target_configuration.digest
    );
    let preparation = prepare_lawpack_compilation(&module, &bundle, &adapter)
        .expect("derive compiler and target facts");
    let core = compile_to_core(&module, preparation.compiler_context())
        .expect("compile source-derived Core");
    let report = lower_to_target_ir(&core, preparation.target_ir_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    let artifact = report.artifact.expect("Echo Target IR artifact");
    assert_eq!(artifact.domain, "echo.span-ir/v1");
    assert_eq!(artifact.target_profile.coordinate, "echo.dpo@1");
    assert_eq!(
        artifact.target_profile.digest.as_deref(),
        Some("sha256:2e2494121aecf5e6a2d920f5fb85408825d394765fad41484c416397c920fb04")
    );
    let semantic_closure = artifact
        .semantic_closure
        .as_ref()
        .expect("lawpack-backed semantic closure");
    assert_eq!(semantic_closure.lawpacks.len(), 1);
    assert_eq!(semantic_closure.lawpacks[0].coordinate, "hello.echo@1");
    assert_eq!(
        semantic_closure.lawpacks[0].digest.as_deref(),
        Some(MANIFEST_DIGEST.trim())
    );
    let intent = artifact
        .intents
        .get("createGreeting")
        .expect("createGreeting intent");
    assert!(
        intent.basis.is_some(),
        "explicit basis must survive lowering"
    );
    assert_eq!(
        intent.steps[0].target_intrinsic,
        "echo.dpo@1.anchored-node-attachment-create-if-absent"
    );
    assert_eq!(intent.steps[0].obstruction_failures, vec!["alreadyExists"]);
    assert!(
        intent.steps[0]
            .obstruction_arms
            .contains_key("alreadyExists"),
        "typed failure arm must survive lowering"
    );
    assert_eq!(
        encode_core_module(&core).expect("encode createGreeting Core"),
        CREATE_GREETING_CORE_BYTES
    );
    assert_eq!(
        digest_core_module(&core)
            .expect("digest createGreeting Core")
            .to_review_string(),
        CREATE_GREETING_CORE_DIGEST.trim()
    );
    assert_eq!(
        encode_target_ir_artifact(&artifact).expect("encode createGreeting Target IR"),
        CREATE_GREETING_TARGET_IR_BYTES
    );
    assert_eq!(
        digest_target_ir_artifact(&artifact)
            .expect("digest createGreeting Target IR")
            .to_review_string(),
        CREATE_GREETING_TARGET_IR_DIGEST.trim()
    );
}

#[test]
fn lawpack_adapter_bytes_must_be_canonical_and_digest_bound() {
    let bundle =
        decode_lawpack_bundle(MANIFEST_BYTES, EXPORTS_BYTES).expect("load Hello Echo lawpack");
    let noncanonical = decode_lawpack_adapter(&bundle, "echo.dpo@1", &[0x18, 0x00])
        .expect_err("noncanonical adapter must reject");
    assert_eq!(
        adapter_failure_kinds(&noncanonical),
        vec![LawpackAdapterFailureKind::InvalidCanonicalCbor]
    );

    let mut substituted =
        decode_canonical_cbor(ADAPTER_BYTES).expect("decode canonical adapter fixture");
    replace_field(
        &mut substituted,
        "class",
        text("a different canonical adapter"),
    );
    let substituted_bytes =
        encode_canonical_cbor(&substituted).expect("encode substituted adapter");
    let failures = decode_lawpack_adapter(&bundle, "echo.dpo@1", &substituted_bytes)
        .expect_err("adapter digest substitution must reject");
    assert_eq!(
        adapter_failure_kinds(&failures),
        vec![LawpackAdapterFailureKind::AdapterDigestMismatch]
    );
}

#[test]
fn lawpack_adapter_requires_a_typed_target_configuration_reference() {
    let mut adapter = decode_canonical_cbor(ADAPTER_BYTES).expect("decode canonical adapter");
    let effect = first_map_value_mut(field_mut(&mut adapter, "effectImplementations"));
    let target_configuration = field_mut(effect, "targetConfiguration");
    let digest = field_mut(target_configuration, "digest");
    let CanonicalValue::Array(parts) = digest else {
        panic!("target configuration digest fixture must be an array");
    };
    let CanonicalValue::Bytes(bytes) = &mut parts[1] else {
        panic!("target configuration digest fixture must contain bytes");
    };
    bytes.pop();

    let bundle = bundle_with_adapter(&adapter);
    let bytes = encode_canonical_cbor(&adapter).expect("encode malformed adapter");
    let failures = decode_lawpack_adapter(&bundle, "echo.dpo@1", &bytes)
        .expect_err("malformed target configuration reference must reject");

    assert_eq!(
        adapter_failure_kinds(&failures),
        vec![LawpackAdapterFailureKind::InvalidTargetConfiguration]
    );
}

#[test]
fn lawpack_adapter_selection_requires_one_exact_target_profile() {
    let bundle =
        decode_lawpack_bundle(MANIFEST_BYTES, EXPORTS_BYTES).expect("load Hello Echo lawpack");
    let failures = decode_lawpack_adapter(&bundle, "echo.dpo@2", ADAPTER_BYTES)
        .expect_err("unselected target profile must reject");

    assert_eq!(
        adapter_failure_kinds(&failures),
        vec![LawpackAdapterFailureKind::MissingTargetAdapter]
    );
}

#[test]
fn lawpack_adapter_requires_complete_exported_effect_coverage() {
    let mut adapter = decode_canonical_cbor(ADAPTER_BYTES).expect("decode canonical adapter");
    map_mut(field_mut(&mut adapter, "effectImplementations")).clear();
    let bundle = bundle_with_adapter(&adapter);
    let bytes = encode_canonical_cbor(&adapter).expect("encode adapter");
    let failures = decode_lawpack_adapter(&bundle, "echo.dpo@1", &bytes)
        .expect_err("missing effect implementation must reject");

    assert_eq!(
        adapter_failure_kinds(&failures),
        vec![LawpackAdapterFailureKind::MissingEffectImplementation]
    );
}

#[test]
fn lawpack_adapter_corroborates_footprint_cost_and_failure_obligations() {
    for (field, replacement, expected) in [
        (
            "footprintObligation",
            text("hello.echo@1.someOtherFootprint"),
            LawpackAdapterFailureKind::ObligationMismatch,
        ),
        (
            "costObligation",
            text("hello.echo@1.someOtherBudget"),
            LawpackAdapterFailureKind::ObligationMismatch,
        ),
    ] {
        let mut adapter = decode_canonical_cbor(ADAPTER_BYTES).expect("decode canonical adapter");
        let effect = first_map_value_mut(field_mut(&mut adapter, "effectImplementations"));
        replace_field(effect, field, replacement);
        let bundle = bundle_with_adapter(&adapter);
        let bytes = encode_canonical_cbor(&adapter).expect("encode adapter");
        let failures = decode_lawpack_adapter(&bundle, "echo.dpo@1", &bytes)
            .expect_err("mismatched obligation must reject");
        assert_eq!(adapter_failure_kinds(&failures), vec![expected]);
    }

    let mut adapter = decode_canonical_cbor(ADAPTER_BYTES).expect("decode canonical adapter");
    let effect = first_map_value_mut(field_mut(&mut adapter, "effectImplementations"));
    map_mut(field_mut(effect, "failureMappings")).clear();
    let bundle = bundle_with_adapter(&adapter);
    let bytes = encode_canonical_cbor(&adapter).expect("encode adapter");
    let failures = decode_lawpack_adapter(&bundle, "echo.dpo@1", &bytes)
        .expect_err("incomplete failure mapping must reject");
    assert_eq!(
        adapter_failure_kinds(&failures),
        vec![LawpackAdapterFailureKind::FailureMappingMismatch]
    );
}

#[test]
fn lawpack_compilation_requires_the_exact_digest_locked_source_import() {
    let bundle =
        decode_lawpack_bundle(MANIFEST_BYTES, EXPORTS_BYTES).expect("load Hello Echo lawpack");
    let adapter =
        decode_lawpack_adapter(&bundle, "echo.dpo@1", ADAPTER_BYTES).expect("load exact adapter");
    let source = CREATE_GREETING_SOURCE.replace(
        MANIFEST_DIGEST.trim(),
        &format!("sha256:{}", "0".repeat(64)),
    );
    let module = parse_module(&source).expect("parse source with substituted import");
    let failures = prepare_lawpack_compilation(&module, &bundle, &adapter)
        .expect_err("substituted source import must reject");

    assert_eq!(
        adapter_failure_kinds(&failures),
        vec![LawpackAdapterFailureKind::SourceImportMismatch]
    );
}

#[test]
fn noncanonical_manifest_bytes_reject_before_shape_validation() {
    let failures = decode_lawpack_bundle(&[0x18, 0x00], EXPORTS_BYTES)
        .expect_err("noncanonical manifest must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::InvalidCanonicalCbor]
    );
}

#[test]
fn manifest_and_export_maps_are_closed() {
    let exports = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let mut manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    insert_field(
        &mut manifest,
        "displayName",
        text("not hash-significant here"),
    );
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");
    let failures = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .expect_err("unknown manifest field must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::UnexpectedField]
    );
    assert_eq!(failures[0].path, "manifest.displayName");
}

#[test]
fn export_digest_substitution_rejects() {
    let exports = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let manifest = hello_echo_manifest([0xff; 32]);
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");
    let failures = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .expect_err("substituted export digest must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::ExportsDigestMismatch]
    );
}

#[test]
fn runtime_effect_requires_at_least_one_target_adapter() {
    let exports = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let mut manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    remove_field(&mut manifest, "targetAdapters");
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");
    let failures = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .expect_err("runtime effect without an adapter must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::RuntimeEffectWithoutTargetAdapter]
    );
}

#[test]
fn proof_only_effect_does_not_invent_a_runtime_adapter_requirement() {
    let mut exports = hello_echo_exports();
    replace_field(
        first_array_item_mut(field_mut(&mut exports, "effects")),
        "executionClass",
        text("proofOnly"),
    );
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let mut manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    remove_field(&mut manifest, "targetAdapters");
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");

    let bundle = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .expect("proof-only lawpack does not require an adapter");

    assert_eq!(
        bundle.exports().effects[0].execution_class,
        LawpackExecutionClass::ProofOnly
    );
}

#[test]
fn executable_verifier_must_carry_component_sandbox_and_fuel() {
    let exports = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let mut manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    replace_field(
        &mut manifest,
        "verifier",
        map([
            ("class", text("executable")),
            (
                "component",
                resource_ref("hello.echo.verifier/v1", [0x77; 32]),
            ),
            (
                "sandbox",
                resource_ref("edict.wasm-component/v1", [0x88; 32]),
            ),
        ]),
    );
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");
    let failures = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .expect_err("unbounded executable verifier must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::MissingField]
    );
    assert_eq!(failures[0].path, "manifest.verifier.fuelModel");
}

#[test]
fn all_hash_bound_helper_and_verifier_variants_load() {
    let mut exports = hello_echo_exports();
    let identity_body = map([
        (
            "params",
            CanonicalValue::Array(vec![local_ref(
                "arg:0",
                "value",
                "hello.echo@1.GreetingKey",
            )]),
        ),
        (
            "body",
            map([
                ("locals", CanonicalValue::Array(Vec::new())),
                ("bindings", CanonicalValue::Array(Vec::new())),
                (
                    "result",
                    map([
                        ("kind", text("local")),
                        (
                            "ref",
                            local_ref("arg:0", "value", "hello.echo@1.GreetingKey"),
                        ),
                    ]),
                ),
            ]),
        ),
    ]);
    let pure_functions = array_mut(field_mut(&mut exports, "pureFunctions"));
    pure_functions.push(pure_function(
        "hello.echo@1.identity",
        "edict",
        ("body", identity_body),
    ));
    pure_functions.push(pure_function(
        "hello.echo@1.componentIdentity",
        "component",
        (
            "implementation",
            executable_component("hello.echo.component-identity/v1", 0x91),
        ),
    ));
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let mut manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    insert_field(
        &mut manifest,
        "helperComponent",
        executable_component("hello.echo.helpers/v1", 0x92),
    );
    replace_field(
        &mut manifest,
        "verifier",
        map([
            ("class", text("executable")),
            (
                "component",
                resource_ref("hello.echo.verifier/v1", [0x93; 32]),
            ),
            (
                "sandbox",
                resource_ref("edict.wasm-component/v1", [0x94; 32]),
            ),
            ("fuelModel", resource_ref("edict.fuel/v1", [0x95; 32])),
        ]),
    );
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");

    let bundle =
        decode_lawpack_bundle(&manifest_bytes, &exports_bytes).expect("full variant bundle");

    assert_eq!(
        bundle.manifest().verifier.class(),
        LawpackVerifierClass::Executable
    );
    assert!(bundle.manifest().helper_component.is_some());
    assert!(matches!(
        bundle.exports().pure_functions[0].implementation,
        LawpackPureFunctionImplementation::Edict { .. }
    ));
    assert!(matches!(
        bundle.exports().pure_functions[1].implementation,
        LawpackPureFunctionImplementation::Component { .. }
    ));
}

#[test]
fn edict_pure_helpers_reject_effectful_and_unresolved_callees() {
    for callee in ["hello.echo@1.createGreeting", "hello.echo@1.notExported"] {
        let mut exports = hello_echo_exports();
        let body = map([
            (
                "params",
                CanonicalValue::Array(vec![local_ref(
                    "arg:0",
                    "value",
                    "hello.echo@1.GreetingKey",
                )]),
            ),
            (
                "body",
                map([
                    ("locals", CanonicalValue::Array(Vec::new())),
                    ("bindings", CanonicalValue::Array(Vec::new())),
                    (
                        "result",
                        map([
                            ("kind", text("call")),
                            ("callee", text(callee)),
                            ("typeArgs", CanonicalValue::Array(Vec::new())),
                            (
                                "args",
                                CanonicalValue::Array(vec![map([
                                    ("kind", text("local")),
                                    (
                                        "ref",
                                        local_ref("arg:0", "value", "hello.echo@1.GreetingKey"),
                                    ),
                                ])]),
                            ),
                        ]),
                    ),
                ]),
            ),
        ]);
        array_mut(field_mut(&mut exports, "pureFunctions")).push(pure_function(
            "hello.echo@1.invalidCaller",
            "edict",
            ("body", body),
        ));
        let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
        let manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
        let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");

        let failures = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
            .expect_err("effectful or unresolved pure callee must reject");

        assert_eq!(
            failure_kinds(&failures),
            vec![LawpackValidationFailureKind::InvalidPureFunctionBody],
            "callee {callee}"
        );
    }
}

#[test]
fn typed_digests_and_target_adapter_selectors_are_exact() {
    let exports = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let mut malformed = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    let digest = field_mut(field_mut(&mut malformed, "exports"), "digest");
    *digest = CanonicalValue::Array(vec![text("sha256"), CanonicalValue::Bytes(vec![0x01; 31])]);
    let failures = decode_lawpack_bundle(
        &encode_canonical_cbor(&malformed).expect("encode malformed manifest"),
        &exports_bytes,
    )
    .expect_err("short digest must reject");
    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::InvalidDigest]
    );

    let mut duplicate = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    let adapters = array_mut(field_mut(&mut duplicate, "targetAdapters"));
    adapters.push(adapters[0].clone());
    let failures = decode_lawpack_bundle(
        &encode_canonical_cbor(&duplicate).expect("encode duplicate adapter manifest"),
        &exports_bytes,
    )
    .expect_err("duplicate exact target selector must reject");
    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::DuplicateIdentity]
    );
}

#[test]
fn operation_profile_optic_template_is_a_closed_typed_contract() {
    let mut exports = hello_echo_exports();
    let profiles = field_mut(&mut exports, "operationProfiles");
    let profile = first_map_value_mut(profiles);
    let optic = field_mut(profile, "opticTemplate");
    replace_field(
        field_mut(optic, "apertureRequirement"),
        "kind",
        text("whatever"),
    );
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    let failures = decode_lawpack_bundle(
        &encode_canonical_cbor(&manifest).expect("encode manifest"),
        &exports_bytes,
    )
    .expect_err("unknown aperture kind must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::InvalidDiscriminant]
    );
}

#[test]
fn manifest_must_accept_the_supported_core_abi() {
    let exports = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let mut manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    replace_field(
        &mut manifest,
        "acceptedCoreAbi",
        CanonicalValue::Array(vec![text("edict.core/v2")]),
    );
    let failures = decode_lawpack_bundle(
        &encode_canonical_cbor(&manifest).expect("encode manifest"),
        &exports_bytes,
    )
    .expect_err("unsupported Core ABI set must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::MissingAcceptedCoreAbi]
    );
}

#[test]
fn effect_failure_names_must_be_source_mappable_identifiers() {
    for (identifier, expected) in [
        (
            "not-source-mappable",
            LawpackValidationFailureKind::InvalidFailureIdentifier,
        ),
        (
            "else",
            LawpackValidationFailureKind::ReservedFailureIdentifier,
        ),
    ] {
        let mut exports = hello_echo_exports();
        let effect = first_array_item_mut(field_mut(&mut exports, "effects"));
        let failures = field_mut(effect, "effectFailures");
        rename_only_map_key(failures, identifier);
        let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
        let manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
        let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");
        let actual = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
            .expect_err("unmappable failure identifier must reject");

        assert_eq!(failure_kinds(&actual), vec![expected]);
    }
}

#[test]
fn duplicate_export_coordinates_reject_within_their_category() {
    let mut exports = hello_echo_exports();
    let effects = array_mut(field_mut(&mut exports, "effects"));
    effects.push(effects[0].clone());
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");
    let failures = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .expect_err("duplicate effect coordinate must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::DuplicateIdentity]
    );
}

#[test]
fn edict_pure_helper_body_must_match_the_closed_pure_core_schema() {
    let mut exports = hello_echo_exports();
    array_mut(field_mut(&mut exports, "pureFunctions")).push(map([
        ("coordinate", text("hello.echo@1.identity")),
        ("typeParameters", CanonicalValue::Array(Vec::new())),
        (
            "parameterTypes",
            CanonicalValue::Array(vec![text("hello.echo@1.GreetingKey")]),
        ),
        ("returnType", text("hello.echo@1.GreetingKey")),
        ("costTemplate", text("hello.echo@1.tiny")),
        ("determinismClass", text("total")),
        ("source", text("edict")),
        ("body", map([("opaque", text("not Core"))])),
    ]));
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let manifest = hello_echo_manifest(digest_value(EXPORTS_COORDINATE, &exports));
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");
    let failures = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .expect_err("opaque pure helper body must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::InvalidPureFunctionBody]
    );
}

#[test]
fn dependency_graph_requires_the_complete_exact_set_independent_of_input_order() {
    let dependency = bundle("hello.base", "1", &[]);
    let dependent = bundle(
        "hello.app",
        "1",
        &[("hello.base", "1", *dependency.manifest_digest())],
    );

    validate_lawpack_dependency_graph(&[dependent.clone(), dependency.clone()])
        .expect("reverse input order validates");
    validate_lawpack_dependency_graph(&[dependency, dependent]).expect("forward order validates");
}

#[test]
fn dependency_graph_rejects_missing_and_substituted_manifests() {
    let missing = bundle("hello.app", "1", &[("hello.base", "1", [0x99; 32])]);
    let failures =
        validate_lawpack_dependency_graph(&[missing]).expect_err("missing dependency must reject");
    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::MissingDependency]
    );

    let dependency = bundle("hello.base", "1", &[]);
    let substituted = bundle("hello.app", "1", &[("hello.base", "1", [0xaa; 32])]);
    let failures = validate_lawpack_dependency_graph(&[dependency, substituted])
        .expect_err("dependency digest substitution must reject");
    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::DependencyDigestMismatch]
    );
}

#[test]
fn dependency_graph_rejects_cycles_before_digest_corroboration() {
    let left = bundle("hello.left", "1", &[("hello.right", "1", [0x01; 32])]);
    let right = bundle("hello.right", "1", &[("hello.left", "1", [0x02; 32])]);
    let failures =
        validate_lawpack_dependency_graph(&[right, left]).expect_err("cycle must reject");

    assert_eq!(
        failure_kinds(&failures),
        vec![LawpackValidationFailureKind::DependencyCycle]
    );
}

fn hello_echo_manifest(exports_digest: [u8; 32]) -> CanonicalValue {
    manifest("hello.echo", "1", &[], exports_digest)
}

fn manifest(
    id: &str,
    version: &str,
    dependencies: &[(&str, &str, [u8; 32])],
    exports_digest: [u8; 32],
) -> CanonicalValue {
    let mut value = decode_canonical_cbor(MANIFEST_BYTES).expect("decode fixture manifest");
    replace_field(&mut value, "id", text(id));
    replace_field(&mut value, "version", text(version));
    replace_field(
        &mut value,
        "dependencies",
        CanonicalValue::Array(
            dependencies
                .iter()
                .map(|(id, version, digest)| {
                    map([
                        ("id", text(id)),
                        ("version", text(version)),
                        (
                            "digest",
                            CanonicalValue::Array(vec![
                                text("sha256"),
                                CanonicalValue::Bytes(digest.to_vec()),
                            ]),
                        ),
                    ])
                })
                .collect(),
        ),
    );
    let exports = field_mut(&mut value, "exports");
    replace_field(
        exports,
        "digest",
        CanonicalValue::Array(vec![
            text("sha256"),
            CanonicalValue::Bytes(exports_digest.to_vec()),
        ]),
    );
    value
}

fn bundle(
    id: &str,
    version: &str,
    dependencies: &[(&str, &str, [u8; 32])],
) -> ValidatedLawpackBundle {
    let exports = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports).expect("encode exports");
    let manifest = manifest(
        id,
        version,
        dependencies,
        digest_value(EXPORTS_COORDINATE, &exports),
    );
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode manifest");
    decode_lawpack_bundle(&manifest_bytes, &exports_bytes).expect("decode test bundle")
}

fn hello_echo_exports() -> CanonicalValue {
    decode_canonical_cbor(EXPORTS_BYTES).expect("decode fixture exports")
}

fn resource_ref(id: &str, digest: [u8; 32]) -> CanonicalValue {
    map([
        ("id", text(id)),
        (
            "digest",
            CanonicalValue::Array(vec![text("sha256"), CanonicalValue::Bytes(digest.to_vec())]),
        ),
    ])
}

fn executable_component(id: &str, digest_byte: u8) -> CanonicalValue {
    map([
        ("component", resource_ref(id, [digest_byte; 32])),
        (
            "sandbox",
            resource_ref("edict.wasm-component/v1", [digest_byte.wrapping_add(1); 32]),
        ),
        (
            "fuelModel",
            resource_ref("edict.fuel/v1", [digest_byte.wrapping_add(2); 32]),
        ),
    ])
}

fn pure_function(
    coordinate: &str,
    source: &str,
    implementation: (&str, CanonicalValue),
) -> CanonicalValue {
    map([
        ("coordinate", text(coordinate)),
        ("typeParameters", CanonicalValue::Array(Vec::new())),
        (
            "parameterTypes",
            CanonicalValue::Array(vec![text("hello.echo@1.GreetingKey")]),
        ),
        ("returnType", text("hello.echo@1.GreetingKey")),
        ("costTemplate", text("hello.echo@1.tiny")),
        ("determinismClass", text("total")),
        ("source", text(source)),
        implementation,
    ])
}

fn local_ref(id: &str, alpha_name: &str, ty: &str) -> CanonicalValue {
    map([
        ("id", text(id)),
        ("alphaName", text(alpha_name)),
        ("type", text(ty)),
    ])
}

fn digest_value(domain: &str, value: &CanonicalValue) -> [u8; 32] {
    let framed = CanonicalValue::Array(vec![text(DIGEST_FRAME), text(domain), value.clone()]);
    let bytes = encode_canonical_cbor(&framed).expect("encode digest frame");
    Sha256::digest(bytes).into()
}

fn map<const N: usize>(entries: [(&str, CanonicalValue); N]) -> CanonicalValue {
    CanonicalValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn text(value: &str) -> CanonicalValue {
    CanonicalValue::Text(value.to_owned())
}

fn failure_kinds(
    failures: &[edict_syntax::LawpackValidationFailure],
) -> Vec<LawpackValidationFailureKind> {
    failures.iter().map(|failure| failure.kind).collect()
}

fn adapter_failure_kinds(
    failures: &[edict_syntax::LawpackAdapterFailure],
) -> Vec<LawpackAdapterFailureKind> {
    failures.iter().map(|failure| failure.kind).collect()
}

fn bundle_with_adapter(adapter: &CanonicalValue) -> ValidatedLawpackBundle {
    let mut manifest = decode_canonical_cbor(MANIFEST_BYTES).expect("decode canonical manifest");
    let descriptor = first_array_item_mut(field_mut(&mut manifest, "targetAdapters"));
    replace_field(
        descriptor,
        "adapter",
        resource_ref(
            ADAPTER_COORDINATE,
            digest_value(ADAPTER_COORDINATE, adapter),
        ),
    );
    let manifest_bytes = encode_canonical_cbor(&manifest).expect("encode rebound manifest");
    decode_lawpack_bundle(&manifest_bytes, EXPORTS_BYTES).expect("load rebound lawpack")
}

fn insert_field(value: &mut CanonicalValue, field: &str, replacement: CanonicalValue) {
    map_mut(value).push((text(field), replacement));
}

fn replace_field(value: &mut CanonicalValue, field: &str, replacement: CanonicalValue) {
    let target = field_mut(value, field);
    *target = replacement;
}

fn remove_field(value: &mut CanonicalValue, field: &str) {
    let entries = map_mut(value);
    let index = entries
        .iter()
        .position(|(key, _value)| key == &text(field))
        .expect("fixture field");
    entries.remove(index);
}

fn field_mut<'a>(value: &'a mut CanonicalValue, field: &str) -> &'a mut CanonicalValue {
    map_mut(value)
        .iter_mut()
        .find_map(|(key, value)| (key == &text(field)).then_some(value))
        .expect("fixture field")
}

fn rename_only_map_key(value: &mut CanonicalValue, replacement: &str) {
    let entries = map_mut(value);
    assert_eq!(entries.len(), 1);
    entries[0].0 = text(replacement);
}

fn first_array_item_mut(value: &mut CanonicalValue) -> &mut CanonicalValue {
    array_mut(value).first_mut().expect("fixture array item")
}

fn first_map_value_mut(value: &mut CanonicalValue) -> &mut CanonicalValue {
    map_mut(value)
        .first_mut()
        .map(|(_key, value)| value)
        .expect("fixture map entry")
}

fn array_mut(value: &mut CanonicalValue) -> &mut Vec<CanonicalValue> {
    let CanonicalValue::Array(values) = value else {
        panic!("fixture array");
    };
    values
}

fn map_mut(value: &mut CanonicalValue) -> &mut Vec<(CanonicalValue, CanonicalValue)> {
    let CanonicalValue::Map(entries) = value else {
        panic!("fixture map");
    };
    entries
}
