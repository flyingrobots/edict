//! Runtime-neutral target-provider manifest validation.
//!
//! A provider package is an assembled boundary: generated lawpacks, target
//! profiles, authority facts, and review artifacts plus provider-owned lowerer
//! and verifier components. This module validates the generic manifest and
//! provenance envelope. It does not load provider files, execute WIT
//! components, run target verifiers, or interpret runtime-specific semantics.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::core_ir::ResourceRef;

/// Target-provider manifest ABI supported by this crate.
pub const TARGET_PROVIDER_MANIFEST_API_VERSION: &str = "edict.provider-manifest/v1";

/// Typed provider manifest value for `edict.provider-manifest/v1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetProviderManifest {
    pub api_version: String,
    pub provider: ResourceRef,
    pub artifacts: Vec<ProviderArtifactRef>,
}

/// One artifact or component exposed by a target-provider package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderArtifactRef {
    /// Unique package role slot, such as `lawpack.echo-dpo` or `lowerer.echo`.
    pub role: String,
    pub artifact_kind: ProviderArtifactKind,
    pub resource: ResourceRef,
    pub source: ProviderArtifactSource,
}

/// Runtime-neutral artifact categories Edict can route without interpreting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderArtifactKind {
    Lawpack,
    TargetProfile,
    AuthorityFacts,
    ProviderManifest,
    ReviewArtifact,
    GeneratedArtifactProfile,
    Lowerer,
    Verifier,
}

impl ProviderArtifactKind {
    const fn requires_generated_source(self) -> bool {
        matches!(
            self,
            Self::Lawpack
                | Self::TargetProfile
                | Self::AuthorityFacts
                | Self::ProviderManifest
                | Self::ReviewArtifact
                | Self::GeneratedArtifactProfile
        )
    }

    const fn requires_component_source(self) -> bool {
        matches!(self, Self::Lowerer | Self::Verifier)
    }
}

/// Provenance for a provider artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProviderArtifactSource {
    /// Generated from runtime-owned semantic source by a digest-locked generator.
    Generated {
        #[serde(rename = "semanticSource")]
        semantic_source: ResourceRef,
        generator: ResourceRef,
    },
    /// Provider-owned executable component, such as a lowerer or verifier.
    Component { component: ResourceRef },
}

/// Overall provider-manifest validation classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderManifestValidationStatus {
    Valid,
    Invalid,
}

/// Stable failure categories returned by provider-manifest validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderManifestValidationFailureKind {
    InvalidApiVersion,
    NonDigestLockedProvider,
    MissingArtifact,
    MissingRole,
    DuplicateArtifactRole,
    NonDigestLockedArtifact,
    NonDigestLockedGeneratedSource,
    NonDigestLockedGenerator,
    NonDigestLockedComponent,
    GeneratedRoleRequiresGeneratedSource,
    ComponentRoleRequiresComponentSource,
}

/// One failed provider-manifest validation obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderManifestValidationFailure {
    pub kind: ProviderManifestValidationFailureKind,
    pub field: String,
    pub obligation: String,
}

/// Complete provider-manifest validation report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderManifestValidationReport {
    pub status: ProviderManifestValidationStatus,
    pub provider: String,
    pub failures: Vec<ProviderManifestValidationFailure>,
}

/// Validate a typed target-provider manifest value against the v1 contract.
///
/// This check is deliberately runtime-neutral. It validates provider package
/// envelope, provenance, and digest-locking rules; it does not decide whether a
/// lawpack, target profile, lowerer, verifier, or generated artifact is
/// semantically correct for a runtime.
#[must_use]
pub fn validate_target_provider_manifest(
    manifest: &TargetProviderManifest,
) -> ProviderManifestValidationReport {
    let mut failures = Vec::new();

    if manifest.api_version != TARGET_PROVIDER_MANIFEST_API_VERSION {
        push_failure(
            &mut failures,
            ProviderManifestValidationFailureKind::InvalidApiVersion,
            "api_version",
            TARGET_PROVIDER_MANIFEST_API_VERSION,
        );
    }

    if !is_digest_locked_resource(&manifest.provider) {
        push_failure(
            &mut failures,
            ProviderManifestValidationFailureKind::NonDigestLockedProvider,
            "provider",
            "non-empty coordinate and lowercase sha256 digest",
        );
    }

    if manifest.artifacts.is_empty() {
        push_failure(
            &mut failures,
            ProviderManifestValidationFailureKind::MissingArtifact,
            "artifacts",
            "at least one provider artifact or component",
        );
    }

    check_artifacts(&manifest.artifacts, &mut failures);

    let status = if failures.is_empty() {
        ProviderManifestValidationStatus::Valid
    } else {
        ProviderManifestValidationStatus::Invalid
    };

    ProviderManifestValidationReport {
        status,
        provider: manifest.provider.coordinate.clone(),
        failures,
    }
}

fn check_artifacts(
    artifacts: &[ProviderArtifactRef],
    failures: &mut Vec<ProviderManifestValidationFailure>,
) {
    let mut roles = BTreeSet::new();

    for artifact in artifacts {
        if artifact.role.is_empty() {
            push_failure(
                failures,
                ProviderManifestValidationFailureKind::MissingRole,
                "artifacts.role",
                "non-empty unique artifact role",
            );
        } else if !roles.insert(artifact.role.as_str()) {
            push_failure(
                failures,
                ProviderManifestValidationFailureKind::DuplicateArtifactRole,
                "artifacts.role",
                "unique artifact role",
            );
        }

        if !is_digest_locked_resource(&artifact.resource) {
            push_failure(
                failures,
                ProviderManifestValidationFailureKind::NonDigestLockedArtifact,
                "artifacts.resource",
                "non-empty coordinate and lowercase sha256 digest",
            );
        }

        match &artifact.source {
            ProviderArtifactSource::Generated {
                semantic_source,
                generator,
            } => {
                if artifact.artifact_kind.requires_component_source() {
                    push_failure(
                        failures,
                        ProviderManifestValidationFailureKind::ComponentRoleRequiresComponentSource,
                        "artifacts.source",
                        "lowerer and verifier artifacts must use component provenance",
                    );
                }
                if !is_digest_locked_resource(semantic_source) {
                    push_failure(
                        failures,
                        ProviderManifestValidationFailureKind::NonDigestLockedGeneratedSource,
                        "artifacts.source.semantic_source",
                        "non-empty coordinate and lowercase sha256 digest",
                    );
                }
                if !is_digest_locked_resource(generator) {
                    push_failure(
                        failures,
                        ProviderManifestValidationFailureKind::NonDigestLockedGenerator,
                        "artifacts.source.generator",
                        "non-empty coordinate and lowercase sha256 digest",
                    );
                }
            }
            ProviderArtifactSource::Component { component } => {
                if artifact.artifact_kind.requires_generated_source() {
                    push_failure(
                        failures,
                        ProviderManifestValidationFailureKind::GeneratedRoleRequiresGeneratedSource,
                        "artifacts.source",
                        "metadata artifacts must use generated provenance",
                    );
                }
                if !is_digest_locked_resource(component) {
                    push_failure(
                        failures,
                        ProviderManifestValidationFailureKind::NonDigestLockedComponent,
                        "artifacts.source.component",
                        "non-empty coordinate and lowercase sha256 digest",
                    );
                }
            }
        }
    }
}

fn is_digest_locked_resource(resource: &ResourceRef) -> bool {
    !resource.coordinate.is_empty()
        && resource
            .digest
            .as_deref()
            .is_some_and(is_lowercase_sha256_review_digest)
}

fn is_lowercase_sha256_review_digest(digest: &str) -> bool {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn push_failure(
    failures: &mut Vec<ProviderManifestValidationFailure>,
    kind: ProviderManifestValidationFailureKind,
    field: &str,
    obligation: &str,
) {
    failures.push(ProviderManifestValidationFailure {
        kind,
        field: field.to_owned(),
        obligation: obligation.to_owned(),
    });
}
