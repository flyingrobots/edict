//! Typed v1 contract bundle and assurance manifest checks.
//!
//! This module models the participant-neutral bundle boundary after Core and
//! target lowering have produced hash-addressed artifacts. It does not recompute
//! bundle digests, load files, run target verifiers, or perform admission.

use std::collections::BTreeSet;

use crate::core_ir::{is_sha256_review_digest, ResourceRef};

/// Contract bundle manifest ABI supported by this crate.
pub const CONTRACT_BUNDLE_API_VERSION: &str = "edict.contract-bundle/v1";

/// Which pre-admission bundle digest an artifact references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BundleSubjectKind {
    Semantic,
    Release,
}

/// Required assurance roles bound to a participant-neutral bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssuranceRole {
    Holmes,
    Watson,
    Moriarty,
}

/// Explicit subject reference used by assurance and admission-adjacent artifacts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleSubject {
    pub kind: BundleSubjectKind,
    pub digest: String,
}

/// Source artifact provenance recorded in the release bundle layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceArtifactRef {
    pub logical_path: String,
    pub artifact: ResourceRef,
}

/// Participant-neutral assurance evidence included with a bundle manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssuranceEvidenceRef {
    pub role: AssuranceRole,
    pub artifact: ResourceRef,
    pub subject: BundleSubject,
    pub target_profile_digest: String,
    pub target_ir_digest: String,
}

/// Typed contract checked before admission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractBundleManifest {
    pub api_version: String,
    pub semantic_bundle_digest: String,
    pub release_bundle_digest: String,
    pub source_artifacts: Vec<SourceArtifactRef>,
    pub source_profile_semantic_facts: ResourceRef,
    pub core_ir: ResourceRef,
    pub target_profile: ResourceRef,
    pub target_ir: ResourceRef,
    pub lawpacks: Vec<ResourceRef>,
    pub generated_artifacts: Vec<ResourceRef>,
    pub compiler: ResourceRef,
    pub lowerer: ResourceRef,
    pub verifier: ResourceRef,
    pub semantic_compile_options: ResourceRef,
    pub canonicalization_profile: ResourceRef,
    pub conformance_fixture_corpora: Vec<ResourceRef>,
    pub verifier_report: ResourceRef,
    pub compile_explanation: ResourceRef,
    pub assurance_evidence: Vec<AssuranceEvidenceRef>,
    pub admission_artifacts: Vec<ResourceRef>,
}

/// Overall contract bundle validation classification for v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractBundleValidationStatus {
    Valid,
    Invalid,
}

/// Stable failure categories returned by contract bundle checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractBundleValidationFailureKind {
    InvalidApiVersion,
    InvalidBundleDigest,
    EmptyArtifactSet,
    InvalidArtifactReference,
    InvalidSourcePath,
    MissingAssuranceRole,
    AssuranceSubjectMismatch,
    AssuranceTargetProfileMismatch,
    AssuranceTargetIrMismatch,
    AdmissionArtifactUnsupported,
}

/// One failed contract bundle validation obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractBundleValidationFailure {
    pub kind: ContractBundleValidationFailureKind,
    pub field: String,
    pub obligation: String,
}

/// Complete v1 contract bundle validation report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractBundleValidationReport {
    pub status: ContractBundleValidationStatus,
    pub semantic_bundle_digest: String,
    pub release_bundle_digest: String,
    pub failures: Vec<ContractBundleValidationFailure>,
}

/// Validate a typed contract bundle manifest against the v1 boundary.
#[must_use]
pub fn validate_contract_bundle_manifest(
    manifest: &ContractBundleManifest,
) -> ContractBundleValidationReport {
    let mut failures = Vec::new();

    check_manifest_identity(manifest, &mut failures);
    check_required_artifact_sets(manifest, &mut failures);
    check_source_artifacts(manifest, &mut failures);
    check_artifact_refs(manifest, &mut failures);
    check_assurance_evidence(manifest, &mut failures);
    check_admission_exclusion(manifest, &mut failures);

    let status = if failures.is_empty() {
        ContractBundleValidationStatus::Valid
    } else {
        ContractBundleValidationStatus::Invalid
    };

    ContractBundleValidationReport {
        status,
        semantic_bundle_digest: manifest.semantic_bundle_digest.clone(),
        release_bundle_digest: manifest.release_bundle_digest.clone(),
        failures,
    }
}

fn check_manifest_identity(
    manifest: &ContractBundleManifest,
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    if manifest.api_version != CONTRACT_BUNDLE_API_VERSION {
        push_failure(
            failures,
            ContractBundleValidationFailureKind::InvalidApiVersion,
            "api_version",
            CONTRACT_BUNDLE_API_VERSION,
        );
    }
    check_digest(
        "semantic_bundle_digest",
        &manifest.semantic_bundle_digest,
        failures,
    );
    check_digest(
        "release_bundle_digest",
        &manifest.release_bundle_digest,
        failures,
    );
}

fn check_required_artifact_sets(
    manifest: &ContractBundleManifest,
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    for (field, is_empty) in [
        ("source_artifacts", manifest.source_artifacts.is_empty()),
        ("lawpacks", manifest.lawpacks.is_empty()),
        (
            "generated_artifacts",
            manifest.generated_artifacts.is_empty(),
        ),
        (
            "conformance_fixture_corpora",
            manifest.conformance_fixture_corpora.is_empty(),
        ),
        ("assurance_evidence", manifest.assurance_evidence.is_empty()),
    ] {
        if is_empty {
            push_failure(
                failures,
                ContractBundleValidationFailureKind::EmptyArtifactSet,
                field,
                "at least one digest-locked artifact reference",
            );
        }
    }
}

fn check_source_artifacts(
    manifest: &ContractBundleManifest,
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    for source in &manifest.source_artifacts {
        if !is_logical_source_path(&source.logical_path) {
            push_failure(
                failures,
                ContractBundleValidationFailureKind::InvalidSourcePath,
                "source_artifacts.logical_path",
                "logical package-relative path",
            );
        }
        check_digest_locked_resource("source_artifacts.artifact", &source.artifact, failures);
    }
}

fn check_artifact_refs(
    manifest: &ContractBundleManifest,
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    for (field, resource) in [
        (
            "source_profile_semantic_facts",
            &manifest.source_profile_semantic_facts,
        ),
        ("core_ir", &manifest.core_ir),
        ("target_profile", &manifest.target_profile),
        ("target_ir", &manifest.target_ir),
        ("compiler", &manifest.compiler),
        ("lowerer", &manifest.lowerer),
        ("verifier", &manifest.verifier),
        (
            "semantic_compile_options",
            &manifest.semantic_compile_options,
        ),
        (
            "canonicalization_profile",
            &manifest.canonicalization_profile,
        ),
        ("verifier_report", &manifest.verifier_report),
        ("compile_explanation", &manifest.compile_explanation),
    ] {
        check_digest_locked_resource(field, resource, failures);
    }

    check_resource_list("lawpacks", &manifest.lawpacks, failures);
    check_resource_list(
        "generated_artifacts",
        &manifest.generated_artifacts,
        failures,
    );
    check_resource_list(
        "conformance_fixture_corpora",
        &manifest.conformance_fixture_corpora,
        failures,
    );
}

fn check_assurance_evidence(
    manifest: &ContractBundleManifest,
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    let present_roles = manifest
        .assurance_evidence
        .iter()
        .map(|evidence| evidence.role)
        .collect::<BTreeSet<_>>();
    for (role, field) in [
        (AssuranceRole::Holmes, "assurance_evidence.holmes"),
        (AssuranceRole::Watson, "assurance_evidence.watson"),
        (AssuranceRole::Moriarty, "assurance_evidence.moriarty"),
    ] {
        if !present_roles.contains(&role) {
            push_failure(
                failures,
                ContractBundleValidationFailureKind::MissingAssuranceRole,
                field,
                "HOLMES, Watson, and Moriarty evidence",
            );
        }
    }

    for evidence in &manifest.assurance_evidence {
        check_one_assurance_evidence(manifest, evidence, failures);
    }
}

fn check_one_assurance_evidence(
    manifest: &ContractBundleManifest,
    evidence: &AssuranceEvidenceRef,
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    check_digest_locked_resource("assurance_evidence.artifact", &evidence.artifact, failures);
    if evidence.subject.digest != bundle_subject_digest(manifest, evidence.subject.kind) {
        push_failure(
            failures,
            ContractBundleValidationFailureKind::AssuranceSubjectMismatch,
            assurance_role_field(evidence.role),
            "evidence subject digest matches selected bundle digest",
        );
    }
    if digest_locked_value(&manifest.target_profile)
        .is_some_and(|digest| digest != evidence.target_profile_digest)
    {
        push_failure(
            failures,
            ContractBundleValidationFailureKind::AssuranceTargetProfileMismatch,
            assurance_role_field(evidence.role),
            "evidence target profile digest matches bundle target profile digest",
        );
    }
    if digest_locked_value(&manifest.target_ir)
        .is_some_and(|digest| digest != evidence.target_ir_digest)
    {
        push_failure(
            failures,
            ContractBundleValidationFailureKind::AssuranceTargetIrMismatch,
            assurance_role_field(evidence.role),
            "evidence target IR digest matches bundle target IR digest",
        );
    }
}

fn check_admission_exclusion(
    manifest: &ContractBundleManifest,
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    if !manifest.admission_artifacts.is_empty() {
        push_failure(
            failures,
            ContractBundleValidationFailureKind::AdmissionArtifactUnsupported,
            "admission_artifacts",
            "admission requests, receipts, policies, and signatures live outside the bundle",
        );
    }
}

fn check_resource_list(
    field: &str,
    resources: &[ResourceRef],
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    for resource in resources {
        check_digest_locked_resource(field, resource, failures);
    }
}

fn check_digest_locked_resource(
    field: &str,
    resource: &ResourceRef,
    failures: &mut Vec<ContractBundleValidationFailure>,
) {
    if !resource.is_digest_locked() {
        push_failure(
            failures,
            ContractBundleValidationFailureKind::InvalidArtifactReference,
            field,
            "non-empty coordinate and sha256 digest",
        );
    }
}

fn check_digest(field: &str, digest: &str, failures: &mut Vec<ContractBundleValidationFailure>) {
    if !is_sha256_review_digest(digest) {
        push_failure(
            failures,
            ContractBundleValidationFailureKind::InvalidBundleDigest,
            field,
            "sha256:<64 hex> digest",
        );
    }
}

fn digest_locked_value(resource: &ResourceRef) -> Option<&str> {
    resource
        .digest
        .as_deref()
        .filter(|digest| resource.is_digest_locked() && is_sha256_review_digest(digest))
}

fn bundle_subject_digest(manifest: &ContractBundleManifest, kind: BundleSubjectKind) -> &str {
    match kind {
        BundleSubjectKind::Semantic => &manifest.semantic_bundle_digest,
        BundleSubjectKind::Release => &manifest.release_bundle_digest,
    }
}

fn assurance_role_field(role: AssuranceRole) -> &'static str {
    match role {
        AssuranceRole::Holmes => "assurance_evidence.holmes",
        AssuranceRole::Watson => "assurance_evidence.watson",
        AssuranceRole::Moriarty => "assurance_evidence.moriarty",
    }
}

fn is_logical_source_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

fn push_failure(
    failures: &mut Vec<ContractBundleValidationFailure>,
    kind: ContractBundleValidationFailureKind,
    field: impl Into<String>,
    obligation: impl Into<String>,
) {
    let failure = ContractBundleValidationFailure {
        kind,
        field: field.into(),
        obligation: obligation.into(),
    };
    if !failures.contains(&failure) {
        failures.push(failure);
    }
}
