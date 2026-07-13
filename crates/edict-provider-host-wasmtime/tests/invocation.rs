use std::sync::Arc;

use edict_provider_host_wasmtime::{
    provider_lowering_input_bytes, PreparedProviderComponent, ProviderComponentHost,
    ProviderHostFailureKind, ProviderHostLimits, ResolvedProviderComponent,
};
use edict_provider_schema::{ProviderArtifactSchemaRegistry, ResolvedProviderSchemaArtifact};
use edict_syntax::{
    bind_target_provider_manifest, encode_canonical_cbor, select_provider_component,
    validate_provider_lowering_request, validate_provider_verification_request, CanonicalValue,
    ProviderArtifact, ProviderArtifactBinding, ProviderArtifactKind, ProviderArtifactRef,
    ProviderArtifactSource, ProviderBoundArtifact, ProviderDigest, ProviderDigestAlgorithm,
    ProviderInvocationKind, ProviderLoweringInvocationContract, ProviderLoweringOutputKind,
    ProviderLoweringOutputRequest, ProviderLoweringRequest, ProviderResourceRef,
    ProviderResponseLimits, ProviderSchemaBinding, ProviderSchemaFormat,
    ProviderVerificationInvocationContract, ProviderVerificationOutputKind,
    ProviderVerificationOutputRequest, ProviderVerificationRequest, ResourceRef,
    TargetProviderManifest, ValidatedProviderLoweringRequest, ValidatedProviderVerificationRequest,
    CORE_MODULE_DIGEST_DOMAIN, TARGET_IR_ARTIFACT_DIGEST_DOMAIN, TARGET_PROFILE_API_VERSION,
    TARGET_PROVIDER_ABI, TARGET_PROVIDER_MANIFEST_API_VERSION, TARGET_PROVIDER_PROTOCOL_VERSION,
};
use sha2::{Digest, Sha256};

const SCHEMA: &[u8] = b"artifact = null\n";
const NULL_BYTES: &[u8] = &[0xf6];
const LOWERER_BYTES: &[u8] =
    include_bytes!("../../../fixtures/providers/components/lowerer.component.wasm");
const VERIFIER_BYTES: &[u8] =
    include_bytes!("../../../fixtures/providers/components/verifier.component.wasm");
const MALFORMED_LOWERER_BYTES: &[u8] =
    include_bytes!("../../../fixtures/providers/components/malformed-lowerer.component.wasm");
const INSTANTIATION_FAILURE_LOWERER_BYTES: &[u8] = include_bytes!(
    "../../../fixtures/providers/components/instantiation-failure-lowerer.component.wasm"
);
const OUTPUT_DOMAIN: &str = "runtime.output/v1";

struct LowerHarness {
    host: ProviderComponentHost,
    prepared: PreparedProviderComponent<'static>,
    request: ValidatedProviderLoweringRequest<'static>,
    schema: &'static ProviderArtifactSchemaRegistry,
}

struct VerifyHarness {
    host: ProviderComponentHost,
    prepared: PreparedProviderComponent<'static>,
    request: ValidatedProviderVerificationRequest<'static>,
    schema: &'static ProviderArtifactSchemaRegistry,
}

fn sha256(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn manifest_resource(coordinate: &str, bytes: &[u8]) -> ResourceRef {
    ResourceRef {
        coordinate: coordinate.to_owned(),
        digest: Some(sha256(bytes)),
    }
}

fn locked_resource(coordinate: &str, digit: char) -> ResourceRef {
    ResourceRef {
        coordinate: coordinate.to_owned(),
        digest: Some(format!("sha256:{}", digit.to_string().repeat(64))),
    }
}

fn provider_manifest() -> &'static TargetProviderManifest {
    provider_manifest_with_lowerer(LOWERER_BYTES)
}

fn provider_manifest_with_lowerer(lowerer_bytes: &'static [u8]) -> &'static TargetProviderManifest {
    Box::leak(Box::new(TargetProviderManifest {
        api_version: TARGET_PROVIDER_MANIFEST_API_VERSION.to_owned(),
        provider_abi: TARGET_PROVIDER_ABI.to_owned(),
        provider: locked_resource("runtime.provider@1", '1'),
        artifacts: vec![
            ProviderArtifactRef {
                role: "lowerer.runtime".to_owned(),
                artifact_kind: ProviderArtifactKind::Lowerer,
                resource: manifest_resource("runtime.lowerer/component@1", lowerer_bytes),
                source: ProviderArtifactSource::Component {
                    component: manifest_resource("runtime.lowerer/component@1", lowerer_bytes),
                },
            },
            ProviderArtifactRef {
                role: "schema.runtime".to_owned(),
                artifact_kind: ProviderArtifactKind::ArtifactSchema,
                resource: manifest_resource("runtime.artifacts.cddl@1", SCHEMA),
                source: ProviderArtifactSource::Generated {
                    semantic_source: locked_resource("runtime.semantic-source@1", '2'),
                    generator: locked_resource("runtime.provider-generator@1", '3'),
                },
            },
            ProviderArtifactRef {
                role: "verifier.runtime".to_owned(),
                artifact_kind: ProviderArtifactKind::Verifier,
                resource: manifest_resource("runtime.verifier/component@1", VERIFIER_BYTES),
                source: ProviderArtifactSource::Component {
                    component: manifest_resource("runtime.verifier/component@1", VERIFIER_BYTES),
                },
            },
        ],
        schema_bindings: [
            CORE_MODULE_DIGEST_DOMAIN,
            TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
            TARGET_PROFILE_API_VERSION,
            OUTPUT_DOMAIN,
        ]
        .into_iter()
        .map(|domain| ProviderSchemaBinding {
            domain: domain.to_owned(),
            schema_role: "schema.runtime".to_owned(),
            format: ProviderSchemaFormat::SelfContainedCddlV1,
            root_rule: "artifact".to_owned(),
        })
        .collect(),
    }))
}

fn registry(manifest: &'static TargetProviderManifest) -> &'static ProviderArtifactSchemaRegistry {
    let validated = bind_target_provider_manifest(manifest).expect("manifest validates");
    Box::leak(Box::new(
        ProviderArtifactSchemaRegistry::from_manifest(
            &validated,
            [ResolvedProviderSchemaArtifact {
                role: "schema.runtime".to_owned(),
                bytes: Arc::from(SCHEMA),
            }],
            [
                CORE_MODULE_DIGEST_DOMAIN,
                TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
                TARGET_PROFILE_API_VERSION,
                OUTPUT_DOMAIN,
            ],
        )
        .expect("registry constructs"),
    ))
}

fn provider_digest(domain: &str) -> ProviderDigest {
    let frame = CanonicalValue::Array(vec![
        CanonicalValue::Text("edict.digest/v1".to_owned()),
        CanonicalValue::Text(domain.to_owned()),
        CanonicalValue::Null,
    ]);
    let bytes = encode_canonical_cbor(&frame).expect("digest frame encodes");
    ProviderDigest {
        algorithm: ProviderDigestAlgorithm::Sha256,
        bytes: Sha256::digest(bytes).to_vec(),
    }
}

fn binding(coordinate: &str, domain: &str) -> ProviderArtifactBinding {
    ProviderArtifactBinding {
        reference: ProviderResourceRef {
            coordinate: coordinate.to_owned(),
            digest: provider_digest(domain),
        },
        domain: domain.to_owned(),
    }
}

fn bound(coordinate: &str, domain: &str) -> ProviderBoundArtifact {
    ProviderBoundArtifact {
        reference: ProviderResourceRef {
            coordinate: coordinate.to_owned(),
            digest: provider_digest(domain),
        },
        artifact: ProviderArtifact {
            domain: domain.to_owned(),
            bytes: NULL_BYTES.to_vec(),
        },
    }
}

fn response_limits() -> ProviderResponseLimits {
    ProviderResponseLimits {
        max_output_count: 4,
        max_diagnostic_count: 4,
        max_total_response_bytes: 1024 * 1024,
    }
}

fn host_limits() -> ProviderHostLimits {
    ProviderHostLimits {
        max_input_bytes: 1024 * 1024,
        max_output_bytes: 3 * 1024 * 1024,
        max_diagnostic_bytes: 3 * 1024 * 1024,
        max_wasm_memory_bytes: 16 * 1024 * 1024,
        max_table_elements: 10_000,
        max_instances: 100,
        max_memories: 8,
        max_tables: 8,
        max_wasm_fuel: 50_000_000,
        max_hostcall_bytes: 4 * 1024 * 1024,
        max_host_diagnostic_bytes: 512,
    }
}

fn lower_harness(role: &str) -> LowerHarness {
    lower_harness_with_component(role, LOWERER_BYTES)
}

fn lower_harness_with_component(role: &str, component: &'static [u8]) -> LowerHarness {
    let manifest = provider_manifest_with_lowerer(component);
    let manifest_proof = Box::leak(Box::new(
        bind_target_provider_manifest(manifest).expect("manifest validates"),
    ));
    let selected = select_provider_component(
        manifest_proof,
        "lowerer.runtime",
        ProviderInvocationKind::Lowering,
    )
    .expect("lowerer selects");
    let resolved = ResolvedProviderComponent::new(selected, Arc::from(component));
    let host = ProviderComponentHost::new().expect("host configures");
    let prepared = host.prepare(&resolved).expect("lowerer prepares");
    let schema = registry(manifest);
    let contract = Box::leak(Box::new(ProviderLoweringInvocationContract {
        core: binding("core@1", CORE_MODULE_DIGEST_DOMAIN),
        target_profile: binding("profile@1", TARGET_PROFILE_API_VERSION),
        semantic_inputs: Vec::new(),
    }));
    let request = Box::leak(Box::new(ProviderLoweringRequest {
        protocol_version: TARGET_PROVIDER_PROTOCOL_VERSION,
        core: bound("core@1", CORE_MODULE_DIGEST_DOMAIN),
        target_profile: bound("profile@1", TARGET_PROFILE_API_VERSION),
        semantic_inputs: Vec::new(),
        requested_outputs: vec![ProviderLoweringOutputRequest {
            role: role.to_owned(),
            kind: ProviderLoweringOutputKind::GeneratedArtifact,
            domain: OUTPUT_DOMAIN.to_owned(),
        }],
        limits: response_limits(),
    }));
    let request = validate_provider_lowering_request(schema, contract, request)
        .expect("lowering request validates");
    LowerHarness {
        host,
        prepared,
        request,
        schema,
    }
}

fn verify_harness() -> VerifyHarness {
    let manifest = provider_manifest();
    let manifest_proof = Box::leak(Box::new(
        bind_target_provider_manifest(manifest).expect("manifest validates"),
    ));
    let selected = select_provider_component(
        manifest_proof,
        "verifier.runtime",
        ProviderInvocationKind::Verification,
    )
    .expect("verifier selects");
    let resolved = ResolvedProviderComponent::new(selected, Arc::from(VERIFIER_BYTES));
    let host = ProviderComponentHost::new().expect("host configures");
    let prepared = host.prepare(&resolved).expect("verifier prepares");
    let schema = registry(manifest);
    let contract = Box::leak(Box::new(ProviderVerificationInvocationContract {
        core: binding("core@1", CORE_MODULE_DIGEST_DOMAIN),
        target_profile: binding("profile@1", TARGET_PROFILE_API_VERSION),
        target_ir: binding("target-ir@1", TARGET_IR_ARTIFACT_DIGEST_DOMAIN),
        semantic_inputs: Vec::new(),
    }));
    let request = Box::leak(Box::new(ProviderVerificationRequest {
        protocol_version: TARGET_PROVIDER_PROTOCOL_VERSION,
        core: bound("core@1", CORE_MODULE_DIGEST_DOMAIN),
        target_profile: bound("profile@1", TARGET_PROFILE_API_VERSION),
        target_ir: bound("target-ir@1", TARGET_IR_ARTIFACT_DIGEST_DOMAIN),
        semantic_inputs: Vec::new(),
        requested_outputs: vec![ProviderVerificationOutputRequest {
            role: "report.runtime".to_owned(),
            kind: ProviderVerificationOutputKind::VerifierReport,
            domain: OUTPUT_DOMAIN.to_owned(),
        }],
        limits: response_limits(),
    }));
    let request = validate_provider_verification_request(schema, contract, request)
        .expect("verification request validates");
    VerifyHarness {
        host,
        prepared,
        request,
        schema,
    }
}

#[test]
fn conforming_lowerer_and_verifier_results_cross_complete_admission() {
    let lower = lower_harness("output.runtime");
    let outcome = lower
        .host
        .invoke_lowerer(&lower.prepared, &lower.request, lower.schema, host_limits())
        .expect("valid lowerer result is admitted");
    assert_eq!(
        outcome
            .manifest()
            .expect("success manifest")
            .outputs()
            .len(),
        1
    );

    let verify = verify_harness();
    let outcome = verify
        .host
        .invoke_verifier(
            &verify.prepared,
            &verify.request,
            verify.schema,
            host_limits(),
        )
        .expect("valid verifier result is admitted");
    assert_eq!(
        outcome
            .manifest()
            .expect("success manifest")
            .outputs()
            .len(),
        1
    );
}

#[test]
fn typed_provider_refusal_remains_distinct_from_host_failure() {
    let harness = lower_harness("fixture.refusal");
    let outcome = harness
        .host
        .invoke_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect("typed refusal crosses transport");
    assert!(outcome.refusal().is_some());
    assert!(outcome.manifest().is_none());
}

#[test]
fn input_and_authorized_output_limits_reject_before_invocation() {
    let harness = lower_harness("output.runtime");
    let exact_input =
        provider_lowering_input_bytes(&harness.request).expect("input size is representable");
    let mut limits = host_limits();
    limits.max_input_bytes = exact_input - 1;
    let failure = harness
        .host
        .invoke_lowerer(&harness.prepared, &harness.request, harness.schema, limits)
        .expect_err("zero input limit rejects");
    assert_eq!(failure.kind(), ProviderHostFailureKind::InputLimitExceeded);

    let mut exact_limits = host_limits();
    exact_limits.max_input_bytes = exact_input;
    harness
        .host
        .invoke_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            exact_limits,
        )
        .expect("the exact logical input-byte boundary passes");

    let mut limits = host_limits();
    limits.max_output_bytes = harness.request.request().limits.max_total_response_bytes - 1;
    let failure = harness
        .host
        .invoke_lowerer(&harness.prepared, &harness.request, harness.schema, limits)
        .expect_err("host output ceiling rejects larger request authority");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResponseLimitExceeded
    );
}

#[test]
fn prepared_component_request_and_registry_must_share_one_authority() {
    let harness = lower_harness("output.runtime");
    let other_registry = registry(provider_manifest());
    let failure = harness
        .host
        .invoke_lowerer(
            &harness.prepared,
            &harness.request,
            other_registry,
            host_limits(),
        )
        .expect_err("a different registry instance cannot satisfy the request proof");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::HostInvariantViolated
    );
}

#[test]
fn fuel_resource_and_guest_traps_remain_distinct() {
    let looping = lower_harness("fixture.loop");
    let mut limits = host_limits();
    limits.max_wasm_fuel = 10_000;
    let failure = looping
        .host
        .invoke_lowerer(&looping.prepared, &looping.request, looping.schema, limits)
        .expect_err("loop must exhaust fuel");
    assert_eq!(failure.kind(), ProviderHostFailureKind::FuelExhausted);

    let memory = lower_harness("fixture.memory");
    let mut limits = host_limits();
    limits.max_wasm_memory_bytes = 4 * 1024 * 1024;
    let failure = memory
        .host
        .invoke_lowerer(&memory.prepared, &memory.request, memory.schema, limits)
        .expect_err("guest memory pressure must be denied");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResourceLimitExceeded
    );

    let instance_limited = lower_harness("output.runtime");
    let mut limits = host_limits();
    limits.max_instances = 0;
    let failure = instance_limited
        .host
        .invoke_lowerer(
            &instance_limited.prepared,
            &instance_limited.request,
            instance_limited.schema,
            limits,
        )
        .expect_err("store instance-count exhaustion must be a resource failure");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResourceLimitExceeded
    );

    let trapped = lower_harness("fixture.trap");
    let failure = trapped
        .host
        .invoke_lowerer(
            &trapped.prepared,
            &trapped.request,
            trapped.schema,
            host_limits(),
        )
        .expect_err("explicit guest trap rejects");
    assert_eq!(failure.kind(), ProviderHostFailureKind::GuestTrap);

    let trapped = lower_harness("fixture.trap");
    let mut limits = host_limits();
    limits.max_host_diagnostic_bytes = 0;
    let failure = trapped
        .host
        .invoke_lowerer(&trapped.prepared, &trapped.request, trapped.schema, limits)
        .expect_err("engine diagnostic retention must honor an exact zero bound");
    assert_eq!(failure.kind(), ProviderHostFailureKind::GuestTrap);
    assert!(failure.diagnostic().is_empty());
}

#[test]
fn hostcall_diagnostic_and_envelope_limits_are_separate() {
    let output_flood = lower_harness("fixture.output-flood");
    let mut limits = host_limits();
    limits.max_hostcall_bytes = 64 * 1024;
    let failure = output_flood
        .host
        .invoke_lowerer(
            &output_flood.prepared,
            &output_flood.request,
            output_flood.schema,
            limits,
        )
        .expect_err("guest-to-host lifting must be bounded");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResponseLiftLimitExceeded,
        "{}",
        failure.diagnostic()
    );

    let output_flood = lower_harness("fixture.output-flood");
    let mut limits = host_limits();
    limits.max_hostcall_bytes = 64 * 1024;
    limits.max_host_diagnostic_bytes = 0;
    let failure = output_flood
        .host
        .invoke_lowerer(
            &output_flood.prepared,
            &output_flood.request,
            output_flood.schema,
            limits,
        )
        .expect_err("diagnostic truncation cannot change lifting failure identity");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResponseLiftLimitExceeded
    );
    assert!(failure.diagnostic().is_empty());

    let output_limit = lower_harness("fixture.output-flood");
    let failure = output_limit
        .host
        .invoke_lowerer(
            &output_limit.prepared,
            &output_limit.request,
            output_limit.schema,
            host_limits(),
        )
        .expect_err("lifted output must still pass the logical response bound");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResponseLimitExceeded
    );
    assert!(failure.validation_report().is_some());

    let diagnostic_flood = lower_harness("fixture.diagnostic-flood");
    let mut limits = host_limits();
    limits.max_diagnostic_bytes = 64;
    let failure = diagnostic_flood
        .host
        .invoke_lowerer(
            &diagnostic_flood.prepared,
            &diagnostic_flood.request,
            diagnostic_flood.schema,
            limits,
        )
        .expect_err("provider diagnostic bytes must be bounded");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::DiagnosticLimitExceeded
    );

    let bad_envelope = lower_harness("fixture.bad-envelope");
    let failure = bad_envelope
        .host
        .invoke_lowerer(
            &bad_envelope.prepared,
            &bad_envelope.request,
            bad_envelope.schema,
            host_limits(),
        )
        .expect_err("typed but unauthorized response rejects");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResponseEnvelopeInvalid
    );
    assert!(failure.validation_report().is_some());

    let schema_invalid = lower_harness("fixture.schema-invalid");
    let failure = schema_invalid
        .host
        .invoke_lowerer(
            &schema_invalid.prepared,
            &schema_invalid.request,
            schema_invalid.schema,
            host_limits(),
        )
        .expect_err("canonical guest artifact must satisfy its bound schema");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ResponseEnvelopeInvalid
    );
    assert!(failure
        .validation_report()
        .expect("schema rejection preserves the pure report")
        .failures
        .iter()
        .any(|failure| failure.kind
            == edict_syntax::ProviderInvocationValidationFailureKind::ArtifactSchemaMismatch));
}

#[test]
fn malformed_canonical_abi_result_is_not_a_guest_trap_or_envelope_failure() {
    let harness = lower_harness_with_component("output.runtime", MALFORMED_LOWERER_BYTES);
    let failure = harness
        .host
        .invoke_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect_err("invalid result discriminant must fail during lifting");
    assert_eq!(failure.kind(), ProviderHostFailureKind::MalformedResponse);
    assert!(failure.validation_report().is_none());
}

#[test]
fn component_instantiation_failure_is_stable() {
    let harness =
        lower_harness_with_component("output.runtime", INSTANTIATION_FAILURE_LOWERER_BYTES);
    let failure = harness
        .host
        .invoke_lowerer(
            &harness.prepared,
            &harness.request,
            harness.schema,
            host_limits(),
        )
        .expect_err("out-of-bounds active data must fail during instantiation");
    assert_eq!(
        failure.kind(),
        ProviderHostFailureKind::ComponentInstantiationFailed
    );
}
