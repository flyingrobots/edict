//! Provider manifest validation for generated-artifact provenance.
//!
//! These tests assert the public provider-boundary contract. They do not load
//! providers, execute lowerers, validate runtime-specific semantics, or inspect
//! documentation prose.

use edict_syntax::{
    validate_target_provider_manifest, ProviderArtifactSource,
    ProviderManifestValidationFailureKind, ProviderManifestValidationStatus, ResourceRef,
    TargetProviderManifest,
};

const ECHO_PROVIDER_FIXTURE: &str =
    include_str!("../../../fixtures/providers/echo-generated/provider-manifest.json");

fn digest_locked(coordinate: &str, digit: char) -> ResourceRef {
    ResourceRef {
        coordinate: coordinate.to_owned(),
        digest: Some(format!("sha256:{}", digit.to_string().repeat(64))),
    }
}

fn fixture_manifest() -> TargetProviderManifest {
    serde_json::from_str(ECHO_PROVIDER_FIXTURE).expect("provider fixture should deserialize")
}

fn failure_kinds(manifest: &TargetProviderManifest) -> Vec<ProviderManifestValidationFailureKind> {
    validate_target_provider_manifest(manifest)
        .failures
        .iter()
        .map(|failure| failure.kind)
        .collect()
}

#[test]
fn generated_provider_manifest_fixture_validates() {
    let manifest = fixture_manifest();

    let report = validate_target_provider_manifest(&manifest);

    assert_eq!(report.status, ProviderManifestValidationStatus::Valid);
    assert!(report.failures.is_empty());
}

#[test]
fn provider_manifest_rejects_unknown_api_version() {
    let mut manifest = fixture_manifest();
    manifest.api_version = "edict.provider-manifest/v2".to_owned();

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::InvalidApiVersion]
    );
}

#[test]
fn provider_manifest_rejects_unlocked_provider() {
    let mut manifest = fixture_manifest();
    manifest.provider.digest = None;

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::NonDigestLockedProvider]
    );
}

#[test]
fn provider_manifest_rejects_unlocked_generated_artifact() {
    let mut manifest = fixture_manifest();
    manifest.artifacts[0].resource.digest = None;

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::NonDigestLockedArtifact]
    );
}

#[test]
fn provider_manifest_rejects_unlocked_generated_provenance() {
    let mut manifest = fixture_manifest();
    let ProviderArtifactSource::Generated {
        semantic_source, ..
    } = &mut manifest.artifacts[0].source
    else {
        panic!("fixture lawpack artifact should be generated");
    };
    semantic_source.digest =
        Some("sha256:ABCDEFabcdefABCDEFabcdefABCDEFabcdefABCDEFabcdefABCDEFabcdefABCD".to_owned());

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::NonDigestLockedGeneratedSource]
    );
}

#[test]
fn provider_manifest_rejects_unlocked_generator_provenance() {
    let mut manifest = fixture_manifest();
    let ProviderArtifactSource::Generated { generator, .. } = &mut manifest.artifacts[0].source
    else {
        panic!("fixture lawpack artifact should be generated");
    };
    generator.digest = None;

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::NonDigestLockedGenerator]
    );
}

#[test]
fn provider_manifest_rejects_unlocked_component() {
    let mut manifest = fixture_manifest();
    let ProviderArtifactSource::Component { component } = &mut manifest.artifacts[4].source else {
        panic!("fixture lowerer artifact should be a component");
    };
    component.digest = None;

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::NonDigestLockedComponent]
    );
}

#[test]
fn provider_manifest_rejects_missing_artifacts() {
    let mut manifest = fixture_manifest();
    manifest.artifacts.clear();

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::MissingArtifact]
    );
}

#[test]
fn provider_manifest_rejects_generated_component_roles() {
    let mut manifest = fixture_manifest();
    manifest.artifacts[4].source = ProviderArtifactSource::Generated {
        semantic_source: digest_locked("echo.semantic-source/v1", '7'),
        generator: digest_locked("echo-wesley.provider-generator/v1", '8'),
    };

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::ComponentRoleRequiresComponentSource]
    );
}

#[test]
fn provider_manifest_rejects_component_metadata_roles() {
    let mut manifest = fixture_manifest();
    manifest.artifacts[0].source = ProviderArtifactSource::Component {
        component: digest_locked("echo.dpo.lowerer/component", '9'),
    };

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::GeneratedRoleRequiresGeneratedSource]
    );
}

#[test]
fn provider_manifest_rejects_duplicate_artifact_roles() {
    let mut manifest = fixture_manifest();
    let duplicate_role = manifest.artifacts[0].role.clone();
    manifest.artifacts[1].role = duplicate_role;

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::DuplicateArtifactRole]
    );
}

#[test]
fn provider_manifest_rejects_empty_artifact_role() {
    let mut manifest = fixture_manifest();
    manifest.artifacts[0].role.clear();

    assert_eq!(
        failure_kinds(&manifest),
        vec![ProviderManifestValidationFailureKind::MissingRole]
    );
}
