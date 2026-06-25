//! Contract bundle checks for typed v1 bundle and assurance manifests.
//!
//! These tests assert public behavior: validation status and stable failure
//! kinds. They do not inspect diagnostic prose, serialized bytes, or repository
//! layout.

use edict_syntax::{
    validate_contract_bundle_manifest, AssuranceEvidenceRef, AssuranceRole, BundleSubject,
    BundleSubjectKind, ContractBundleManifest, ContractBundleValidationFailureKind,
    ContractBundleValidationReport, ContractBundleValidationStatus, ResourceRef, SourceArtifactRef,
    CONTRACT_BUNDLE_API_VERSION,
};

fn digest(hex: char) -> String {
    format!("sha256:{}", hex.to_string().repeat(64))
}

fn digest_locked(coordinate: &str, hex: char) -> ResourceRef {
    ResourceRef {
        coordinate: coordinate.to_owned(),
        digest: Some(digest(hex)),
    }
}

fn subject(kind: BundleSubjectKind, digest: &str) -> BundleSubject {
    BundleSubject {
        kind,
        digest: digest.to_owned(),
    }
}

fn evidence(
    role: AssuranceRole,
    semantic_digest: &str,
    target_profile_digest: &str,
    target_ir_digest: &str,
    artifact_hex: char,
) -> AssuranceEvidenceRef {
    AssuranceEvidenceRef {
        role,
        artifact: digest_locked(
            match role {
                AssuranceRole::Holmes => "holmes.lawfulness-certificate/v1",
                AssuranceRole::Watson => "watson.compile-explanation/v1",
                AssuranceRole::Moriarty => "moriarty.hash-impact-matrix/v1",
            },
            artifact_hex,
        ),
        subject: subject(BundleSubjectKind::Semantic, semantic_digest),
        target_profile_digest: target_profile_digest.to_owned(),
        target_ir_digest: target_ir_digest.to_owned(),
    }
}

fn echo_bundle() -> ContractBundleManifest {
    let semantic_bundle_digest = digest('a');
    let target_profile = digest_locked("echo.dpo@1", 'c');
    let target_ir = digest_locked("echo.span-ir/v1", 'd');
    let target_profile_digest = target_profile
        .digest
        .clone()
        .expect("target profile digest");
    let target_ir_digest = target_ir.digest.clone().expect("target IR digest");

    ContractBundleManifest {
        api_version: CONTRACT_BUNDLE_API_VERSION.to_owned(),
        semantic_bundle_digest: semantic_bundle_digest.clone(),
        release_bundle_digest: digest('b'),
        source_artifacts: vec![SourceArtifactRef {
            logical_path: "contracts/hello.edict".to_owned(),
            artifact: digest_locked("source.contracts.hello", 'e'),
        }],
        source_profile_semantic_facts: digest_locked("source-profile.hello/v1", 'f'),
        core_ir: digest_locked("edict.core/v1", '1'),
        target_profile,
        target_ir,
        lawpacks: vec![digest_locked("hello.optics@1", '2')],
        generated_artifacts: vec![digest_locked("echo.dpo.registration/v1", '3')],
        compiler: digest_locked("edict.compiler/v1", '4'),
        lowerer: digest_locked("echo.dpo.lowerer/v1", '5'),
        verifier: digest_locked("echo.dpo.verifier/v1", '6'),
        semantic_compile_options: digest_locked("edict.compile-options.semantic/v1", '7'),
        canonicalization_profile: digest_locked("edict.canonical-cbor/v1", '8'),
        conformance_fixture_corpora: vec![digest_locked("echo.dpo.fixtures/v1", '9')],
        verifier_report: digest_locked("echo.dpo.verifier-report/v1", 'a'),
        compile_explanation: digest_locked("watson.compile-explanation/v1", 'b'),
        assurance_evidence: vec![
            evidence(
                AssuranceRole::Holmes,
                &semantic_bundle_digest,
                &target_profile_digest,
                &target_ir_digest,
                'c',
            ),
            evidence(
                AssuranceRole::Watson,
                &semantic_bundle_digest,
                &target_profile_digest,
                &target_ir_digest,
                'd',
            ),
            evidence(
                AssuranceRole::Moriarty,
                &semantic_bundle_digest,
                &target_profile_digest,
                &target_ir_digest,
                'e',
            ),
        ],
        admission_artifacts: Vec::new(),
    }
}

fn kv_bundle() -> ContractBundleManifest {
    let mut bundle = echo_bundle();
    "contracts/kv/read.edict".clone_into(&mut bundle.source_artifacts[0].logical_path);
    bundle.source_artifacts[0].artifact = digest_locked("source.contracts.kv-read", 'f');
    bundle.target_profile = digest_locked("kv.transactional@1", '1');
    bundle.target_ir = digest_locked("kv.transactional.ir/v1", '2');
    bundle.generated_artifacts = vec![digest_locked("kv.transactional.plan/v1", '3')];
    bundle.lowerer = digest_locked("kv.transactional.lowerer/v1", '4');
    bundle.verifier = digest_locked("kv.transactional.verifier/v1", '5');
    bundle.conformance_fixture_corpora = vec![digest_locked("kv.transactional.fixtures/v1", '6')];

    let target_profile_digest = bundle
        .target_profile
        .digest
        .clone()
        .expect("target profile digest");
    let target_ir_digest = bundle.target_ir.digest.clone().expect("target IR digest");
    for evidence in &mut bundle.assurance_evidence {
        evidence
            .target_profile_digest
            .clone_from(&target_profile_digest);
        evidence.target_ir_digest.clone_from(&target_ir_digest);
    }

    bundle
}

fn failure_kinds(
    report: &ContractBundleValidationReport,
) -> Vec<ContractBundleValidationFailureKind> {
    report.failures.iter().map(|failure| failure.kind).collect()
}

#[test]
fn echo_and_kv_bundles_validate_with_the_same_runtime_neutral_contract() {
    for bundle in [echo_bundle(), kv_bundle()] {
        let report = validate_contract_bundle_manifest(&bundle);

        assert_eq!(report.status, ContractBundleValidationStatus::Valid);
        assert!(report.failures.is_empty());
    }
}

#[test]
fn bundle_artifact_references_must_be_digest_locked() {
    let mut bundle = echo_bundle();
    bundle.target_profile.digest = None;

    let report = validate_contract_bundle_manifest(&bundle);

    assert_eq!(report.status, ContractBundleValidationStatus::Invalid);
    assert_eq!(
        failure_kinds(&report),
        vec![ContractBundleValidationFailureKind::InvalidArtifactReference]
    );
    assert_eq!(report.failures[0].field, "target_profile");
}

#[test]
fn malformed_bundle_artifact_digest_is_rejected() {
    let mut bundle = echo_bundle();
    bundle.target_profile.digest = Some("not-a-digest".to_owned());

    let report = validate_contract_bundle_manifest(&bundle);

    assert_eq!(report.status, ContractBundleValidationStatus::Invalid);
    assert_eq!(
        failure_kinds(&report),
        vec![ContractBundleValidationFailureKind::InvalidArtifactReference]
    );
    assert_eq!(report.failures[0].field, "target_profile");
}

#[test]
fn source_artifact_paths_must_be_logical_package_relative_paths() {
    for logical_path in [
        "",
        "/Users/james/git/edict/contracts/hello.edict",
        "contracts/../hello.edict",
        "contracts/./hello.edict",
        "C:/contracts/hello.edict",
        r"contracts\hello.edict",
    ] {
        let mut bundle = echo_bundle();
        bundle.source_artifacts = vec![SourceArtifactRef {
            logical_path: logical_path.to_owned(),
            artifact: digest_locked("source.contracts.hello", 'e'),
        }];

        let report = validate_contract_bundle_manifest(&bundle);

        assert_eq!(report.status, ContractBundleValidationStatus::Invalid);
        assert_eq!(
            failure_kinds(&report),
            vec![ContractBundleValidationFailureKind::InvalidSourcePath]
        );
    }
}

#[test]
fn assurance_evidence_must_match_bundle_subject() {
    let mut bundle = echo_bundle();
    bundle.assurance_evidence[0].subject.digest = digest('f');

    let report = validate_contract_bundle_manifest(&bundle);

    assert_eq!(report.status, ContractBundleValidationStatus::Invalid);
    assert_eq!(
        failure_kinds(&report),
        vec![ContractBundleValidationFailureKind::AssuranceSubjectMismatch]
    );
}

#[test]
fn assurance_evidence_must_match_target_profile_digest() {
    let mut bundle = echo_bundle();
    bundle.assurance_evidence[1].target_profile_digest = digest('f');

    let report = validate_contract_bundle_manifest(&bundle);

    assert_eq!(report.status, ContractBundleValidationStatus::Invalid);
    assert_eq!(
        failure_kinds(&report),
        vec![ContractBundleValidationFailureKind::AssuranceTargetProfileMismatch]
    );
}

#[test]
fn assurance_evidence_must_match_target_ir_digest() {
    let mut bundle = echo_bundle();
    bundle.assurance_evidence[2].target_ir_digest = digest('f');

    let report = validate_contract_bundle_manifest(&bundle);

    assert_eq!(report.status, ContractBundleValidationStatus::Invalid);
    assert_eq!(
        failure_kinds(&report),
        vec![ContractBundleValidationFailureKind::AssuranceTargetIrMismatch]
    );
}

#[test]
fn holmes_watson_and_moriarty_evidence_are_required() {
    let mut bundle = echo_bundle();
    bundle.assurance_evidence.retain(|evidence| {
        matches!(
            evidence.role,
            AssuranceRole::Holmes | AssuranceRole::Moriarty
        )
    });

    let report = validate_contract_bundle_manifest(&bundle);

    assert_eq!(report.status, ContractBundleValidationStatus::Invalid);
    assert_eq!(
        failure_kinds(&report),
        vec![ContractBundleValidationFailureKind::MissingAssuranceRole]
    );
    assert_eq!(report.failures[0].field, "assurance_evidence.watson");
}

#[test]
fn admission_artifacts_are_rejected_from_contract_bundles() {
    let mut bundle = echo_bundle();
    bundle
        .admission_artifacts
        .push(digest_locked("continuum.admission-receipt/v1", 'f'));

    let report = validate_contract_bundle_manifest(&bundle);

    assert_eq!(report.status, ContractBundleValidationStatus::Invalid);
    assert_eq!(
        failure_kinds(&report),
        vec![ContractBundleValidationFailureKind::AdmissionArtifactUnsupported]
    );
}
