use std::sync::Arc;

use edict_provider_schema::{
    ProviderArtifactSchemaRegistry, ProviderSchemaRegistryFailureKind,
    ResolvedProviderSchemaArtifact,
};
use edict_syntax::{
    bind_target_provider_manifest, CanonicalValue, ProviderArtifactKind, ProviderArtifactRef,
    ProviderArtifactSchemaValidationErrorKind, ProviderArtifactSchemaValidator,
    ProviderArtifactSource, ProviderSchemaBinding, ProviderSchemaFormat, ResourceRef,
    TargetProviderManifest, TARGET_PROVIDER_ABI, TARGET_PROVIDER_MANIFEST_API_VERSION,
};
use sha2::{Digest, Sha256};

const GENERATED_SCHEMA: &[u8] = br#"
generated-artifact = {
  kind: "generated",
  value: uint,
}
"#;

const REVIEW_SCHEMA: &[u8] = br#"
review-payload = { approved: bool }
verifier-report = { status: "valid" / "invalid" }
"#;

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn resource(coordinate: &str, digest: String) -> ResourceRef {
    ResourceRef {
        coordinate: coordinate.to_owned(),
        digest: Some(digest),
    }
}

fn schema_artifact(role: &str, coordinate: &str, bytes: &[u8]) -> ProviderArtifactRef {
    ProviderArtifactRef {
        role: role.to_owned(),
        artifact_kind: ProviderArtifactKind::ArtifactSchema,
        resource: resource(coordinate, digest(bytes)),
        source: ProviderArtifactSource::Generated {
            semantic_source: resource(
                "runtime.semantic-source@1",
                format!("sha256:{}", "1".repeat(64)),
            ),
            generator: resource(
                "runtime.provider-generator@1",
                format!("sha256:{}", "2".repeat(64)),
            ),
        },
    }
}

fn manifest(generated_schema: &[u8], review_schema: &[u8]) -> TargetProviderManifest {
    TargetProviderManifest {
        api_version: TARGET_PROVIDER_MANIFEST_API_VERSION.to_owned(),
        provider_abi: TARGET_PROVIDER_ABI.to_owned(),
        provider: resource("runtime.provider@1", format!("sha256:{}", "3".repeat(64))),
        artifacts: vec![
            schema_artifact(
                "schema.generated",
                "runtime.generated-artifact.cddl@1",
                generated_schema,
            ),
            schema_artifact(
                "schema.review",
                "runtime.review-artifacts.cddl@1",
                review_schema,
            ),
        ],
        schema_bindings: vec![
            ProviderSchemaBinding {
                domain: "runtime.generated-artifact/v1".to_owned(),
                schema_role: "schema.generated".to_owned(),
                format: ProviderSchemaFormat::SelfContainedCddlV1,
                root_rule: "generated-artifact".to_owned(),
            },
            ProviderSchemaBinding {
                domain: "runtime.review-payload/v1".to_owned(),
                schema_role: "schema.review".to_owned(),
                format: ProviderSchemaFormat::SelfContainedCddlV1,
                root_rule: "review-payload".to_owned(),
            },
            ProviderSchemaBinding {
                domain: "runtime.verifier-report/v1".to_owned(),
                schema_role: "schema.review".to_owned(),
                format: ProviderSchemaFormat::SelfContainedCddlV1,
                root_rule: "verifier-report".to_owned(),
            },
        ],
    }
}

fn resolved() -> Vec<ResolvedProviderSchemaArtifact> {
    vec![
        ResolvedProviderSchemaArtifact {
            role: "schema.generated".to_owned(),
            bytes: Arc::from(GENERATED_SCHEMA),
        },
        ResolvedProviderSchemaArtifact {
            role: "schema.review".to_owned(),
            bytes: Arc::from(REVIEW_SCHEMA),
        },
    ]
}

fn map(entries: &[(&str, CanonicalValue)]) -> CanonicalValue {
    CanonicalValue::Map(
        entries
            .iter()
            .map(|(key, value)| (CanonicalValue::Text((*key).to_owned()), value.clone()))
            .collect(),
    )
}

fn registry() -> ProviderArtifactSchemaRegistry {
    let manifest = Box::leak(Box::new(manifest(GENERATED_SCHEMA, REVIEW_SCHEMA)));
    let validated = bind_target_provider_manifest(manifest).expect("manifest validates");
    ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        resolved(),
        [
            "runtime.generated-artifact/v1",
            "runtime.review-payload/v1",
            "runtime.verifier-report/v1",
        ],
    )
    .expect("registry constructs")
}

#[test]
fn every_registered_domain_performs_real_schema_instance_validation() {
    let registry = registry();
    let cases = [
        (
            "runtime.generated-artifact/v1",
            map(&[
                ("kind", CanonicalValue::Text("generated".to_owned())),
                ("value", CanonicalValue::Integer(7)),
            ]),
            map(&[
                ("kind", CanonicalValue::Text("generated".to_owned())),
                ("value", CanonicalValue::Text("seven".to_owned())),
            ]),
        ),
        (
            "runtime.review-payload/v1",
            map(&[("approved", CanonicalValue::Bool(true))]),
            map(&[("approved", CanonicalValue::Text("yes".to_owned()))]),
        ),
        (
            "runtime.verifier-report/v1",
            map(&[("status", CanonicalValue::Text("valid".to_owned()))]),
            map(&[("status", CanonicalValue::Text("unknown".to_owned()))]),
        ),
    ];

    assert_eq!(registry.bindings().len(), cases.len());
    for (domain, valid, invalid) in cases {
        assert!(registry.supports_domain(domain));
        registry
            .validate_canonical_value(domain, &valid)
            .expect("valid instance passes");
        assert_eq!(
            registry.validate_canonical_value(domain, &invalid),
            Err(ProviderArtifactSchemaValidationErrorKind::SchemaMismatch)
        );
    }
    assert_eq!(
        registry.validate_canonical_value("runtime.absent/v1", &CanonicalValue::Null),
        Err(ProviderArtifactSchemaValidationErrorKind::UnsupportedDomain)
    );
}

#[test]
fn construction_rejects_missing_ambiguous_and_digest_mismatched_schema_bytes() {
    let manifest = manifest(GENERATED_SCHEMA, REVIEW_SCHEMA);
    let validated = bind_target_provider_manifest(&manifest).expect("manifest validates");

    let missing = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        vec![resolved().remove(0)],
        ["runtime.generated-artifact/v1"],
    )
    .expect_err("missing schema rejects");
    assert_eq!(
        missing.kind(),
        ProviderSchemaRegistryFailureKind::SchemaArtifactMissing
    );

    let mut ambiguous_artifacts = resolved();
    ambiguous_artifacts.push(ambiguous_artifacts[0].clone());
    let ambiguous = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        ambiguous_artifacts,
        ["runtime.generated-artifact/v1"],
    )
    .expect_err("duplicate resolved role rejects");
    assert_eq!(
        ambiguous.kind(),
        ProviderSchemaRegistryFailureKind::SchemaArtifactAmbiguous
    );

    let mut mismatched = resolved();
    mismatched[0].bytes = Arc::from(b"generated-artifact = null".as_slice());
    let mismatch = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        mismatched,
        ["runtime.generated-artifact/v1"],
    )
    .expect_err("digest mismatch rejects");
    assert_eq!(
        mismatch.kind(),
        ProviderSchemaRegistryFailureKind::SchemaArtifactDigestMismatch
    );
}

#[test]
fn construction_requires_complete_domain_closure_and_compilable_roots() {
    let base_manifest = manifest(GENERATED_SCHEMA, REVIEW_SCHEMA);
    let validated = bind_target_provider_manifest(&base_manifest).expect("manifest validates");
    let missing_domain = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        resolved(),
        ["runtime.absent/v1"],
    )
    .expect_err("required unbound domain rejects");
    assert_eq!(
        missing_domain.kind(),
        ProviderSchemaRegistryFailureKind::RequiredDomainMissing
    );

    let malformed = b"generated-artifact = {";
    let malformed_manifest = manifest(malformed, REVIEW_SCHEMA);
    let validated =
        bind_target_provider_manifest(&malformed_manifest).expect("manifest envelope validates");
    let compile = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        vec![
            ResolvedProviderSchemaArtifact {
                role: "schema.generated".to_owned(),
                bytes: Arc::from(malformed.as_slice()),
            },
            resolved().remove(1),
        ],
        ["runtime.generated-artifact/v1"],
    )
    .expect_err("malformed CDDL rejects during construction");
    assert_eq!(
        compile.kind(),
        ProviderSchemaRegistryFailureKind::SchemaCompileFailed
    );

    let external = b"generated-artifact = absent-external-rule";
    let external_manifest = manifest(external, REVIEW_SCHEMA);
    let validated =
        bind_target_provider_manifest(&external_manifest).expect("manifest envelope validates");
    let external_reference = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        vec![
            ResolvedProviderSchemaArtifact {
                role: "schema.generated".to_owned(),
                bytes: Arc::from(external.as_slice()),
            },
            resolved().remove(1),
        ],
        ["runtime.generated-artifact/v1"],
    )
    .expect_err("external rule resolution rejects during construction");
    assert_eq!(
        external_reference.kind(),
        ProviderSchemaRegistryFailureKind::SchemaCompileFailed
    );

    let mut missing_root_manifest = manifest(GENERATED_SCHEMA, REVIEW_SCHEMA);
    missing_root_manifest.schema_bindings[0].root_rule = "absent-root".to_owned();
    let validated = bind_target_provider_manifest(&missing_root_manifest)
        .expect("nonempty root remains a valid manifest envelope");
    let missing_root = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        resolved(),
        ["runtime.generated-artifact/v1"],
    )
    .expect_err("missing root rejects during construction");
    assert_eq!(
        missing_root.kind(),
        ProviderSchemaRegistryFailureKind::SchemaRootRuleMissing
    );
}

#[test]
fn registry_receipt_and_behavior_are_independent_of_input_order() {
    let manifest = manifest(GENERATED_SCHEMA, REVIEW_SCHEMA);
    let validated = bind_target_provider_manifest(&manifest).expect("manifest validates");
    let first = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        resolved(),
        ["runtime.generated-artifact/v1", "runtime.review-payload/v1"],
    )
    .expect("first registry constructs");
    let mut reversed = resolved();
    reversed.reverse();
    let second = ProviderArtifactSchemaRegistry::from_manifest(
        &validated,
        reversed,
        ["runtime.review-payload/v1", "runtime.generated-artifact/v1"],
    )
    .expect("second registry constructs");

    assert_eq!(first.bindings(), second.bindings());
    let value = map(&[
        ("kind", CanonicalValue::Text("generated".to_owned())),
        ("value", CanonicalValue::Integer(7)),
    ]);
    assert_eq!(
        first.validate_canonical_value("runtime.generated-artifact/v1", &value),
        second.validate_canonical_value("runtime.generated-artifact/v1", &value)
    );
}
