//! Typed Gate C admission-boundary checks.
//!
//! This module validates the Edict-owned parts of Continuum admission artifacts:
//! bundle-subject binding, operation requirement binding, hidden-input rejection,
//! receipt echoing, and invocation evidence shape. It does not authenticate
//! participants, evaluate participant policy, decide revocation, or execute
//! target lowerers.

use crate::{
    contract_bundle::{
        validate_contract_bundle_manifest, BundleSubject, BundleSubjectKind,
        ContractBundleManifest, ContractBundleValidationStatus,
    },
    core_ir::ResourceRef,
};

/// Continuum admission request ABI checked by this crate.
pub const ADMISSION_REQUEST_API_VERSION: &str = "continuum.admission-request/v1";

/// Continuum admission receipt body ABI checked by this crate.
pub const ADMISSION_RECEIPT_API_VERSION: &str = "continuum.admission-receipt-body/v1";

/// Authorship provenance for an artifact before admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoringProvenance {
    Human,
    Agent,
    LlmAgent,
}

/// Participant admission decision recorded in a receipt body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionDecision {
    Accepted,
    Rejected,
}

/// Runtime execution input class below the determinism boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionInputKind {
    CanonicalInput,
    WitnessedEvidence,
    AdmittedBasis,
    CapabilityPresentation,
    HiddenHostInput,
}

/// One execution input reference carried by an operation requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionInputRef {
    pub kind: ExecutionInputKind,
    pub artifact: ResourceRef,
}

/// Deterministic requirements for one operation under one bundle subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationRequirementRef {
    pub bundle_subject: BundleSubject,
    pub operation_coordinate: String,
    pub basis: ResourceRef,
    pub variables_digest: String,
    pub requirements_digest: String,
    pub execution_inputs: Vec<ExecutionInputRef>,
}

/// External admission evidence reference required by participant policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionEvidenceRef {
    pub artifact: ResourceRef,
}

/// Typed admission request at the Edict/Continuum boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionRequest {
    pub api_version: String,
    pub bundle_subject: BundleSubject,
    pub participant_descriptor: ResourceRef,
    pub catalog_snapshot: ResourceRef,
    pub admission_policy: ResourceRef,
    pub policy_epoch: String,
    pub requested_operations: Vec<OperationRequirementRef>,
    pub requested_capabilities: Vec<ResourceRef>,
    pub requested_runtime_budget: ResourceRef,
    pub admission_evidence: Vec<AdmissionEvidenceRef>,
}

/// Typed admission receipt body before its external signature envelope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionReceiptBody {
    pub api_version: String,
    pub admission_request_digest: String,
    pub bundle_subject: BundleSubject,
    pub participant: ResourceRef,
    pub decision: AdmissionDecision,
    pub admitted_operations: Vec<String>,
    pub admitted_capabilities: Vec<ResourceRef>,
    pub admitted_runtime_budget: ResourceRef,
    pub policy_epoch: String,
    pub rejection: Option<ResourceRef>,
    pub signing_envelope: Option<ResourceRef>,
}

/// Capability receipt class used by the invocation gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityReceiptKind {
    Registration,
    Invocation,
}

/// Participant capability receipt relevant to an invocation attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityReceipt {
    pub kind: CapabilityReceiptKind,
    pub issuer_bundle_subject: BundleSubject,
    pub participant: ResourceRef,
    pub operation_coordinate: String,
    pub scope: ResourceRef,
    pub policy_epoch: String,
}

/// Gate C invocation evidence checked before execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCInvocation {
    pub bundle: ContractBundleManifest,
    pub request: AdmissionRequest,
    pub operation_coordinate: String,
    pub receipt: Option<AdmissionReceiptBody>,
    pub capability_receipts: Vec<CapabilityReceipt>,
    pub authoring_provenance: AuthoringProvenance,
}

/// Overall admission-boundary validation classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionValidationStatus {
    Valid,
    Invalid,
}

/// Stable failure categories returned by admission-boundary checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionValidationFailureKind {
    InvalidApiVersion,
    InvalidContractBundle,
    InvalidArtifactReference,
    InvalidBundleSubject,
    BundleSubjectMismatch,
    MissingPolicyEpoch,
    EmptyOperationSet,
    OperationRequirementMismatch,
    HiddenExecutionInput,
    AdmissionReceiptMismatch,
    ReceiptSignatureCycle,
    MissingAdmissionReceipt,
    MissingAcceptedAdmissionReceipt,
    RegistrationReceiptIsNotInvocationAuthority,
    MissingInvocationCapability,
}

/// One failed admission-boundary validation obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionValidationFailure {
    pub kind: AdmissionValidationFailureKind,
    pub field: String,
    pub obligation: String,
}

/// Complete admission-boundary validation report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionValidationReport {
    pub status: AdmissionValidationStatus,
    pub failures: Vec<AdmissionValidationFailure>,
}

/// Validate the Edict-owned fields of a Continuum admission request.
#[must_use]
pub fn validate_admission_request(
    bundle: &ContractBundleManifest,
    request: &AdmissionRequest,
) -> AdmissionValidationReport {
    let mut failures = Vec::new();

    if validate_contract_bundle_manifest(bundle).status != ContractBundleValidationStatus::Valid {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::InvalidContractBundle,
            "bundle",
            "valid participant-neutral contract bundle",
        );
    }

    if request.api_version != ADMISSION_REQUEST_API_VERSION {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::InvalidApiVersion,
            "api_version",
            ADMISSION_REQUEST_API_VERSION,
        );
    }

    check_bundle_subject(&request.bundle_subject, "bundle_subject", &mut failures);
    if is_valid_bundle_subject(&request.bundle_subject)
        && request.bundle_subject.digest
            != selected_bundle_digest(bundle, request.bundle_subject.kind)
    {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::BundleSubjectMismatch,
            "bundle_subject",
            "request subject digest matches the selected contract bundle digest",
        );
    }

    for (field, resource) in [
        ("participant_descriptor", &request.participant_descriptor),
        ("catalog_snapshot", &request.catalog_snapshot),
        ("admission_policy", &request.admission_policy),
        (
            "requested_runtime_budget",
            &request.requested_runtime_budget,
        ),
    ] {
        check_digest_locked_resource(field, resource, &mut failures);
    }

    if request.policy_epoch.is_empty() {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::MissingPolicyEpoch,
            "policy_epoch",
            "explicit participant policy epoch",
        );
    }

    if request.requested_operations.is_empty() {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::EmptyOperationSet,
            "requested_operations",
            "at least one requested operation requirement",
        );
    }
    for operation in &request.requested_operations {
        check_operation_requirement(request, operation, &mut failures);
    }

    check_resource_list(
        "requested_capabilities",
        &request.requested_capabilities,
        &mut failures,
    );
    for evidence in &request.admission_evidence {
        check_digest_locked_resource(
            "admission_evidence.artifact",
            &evidence.artifact,
            &mut failures,
        );
    }

    report(failures)
}

/// Validate an admission receipt body against its request.
#[must_use]
pub fn validate_admission_receipt(
    request: &AdmissionRequest,
    receipt: &AdmissionReceiptBody,
) -> AdmissionValidationReport {
    let mut failures = Vec::new();

    if receipt.api_version != ADMISSION_RECEIPT_API_VERSION {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::InvalidApiVersion,
            "api_version",
            ADMISSION_RECEIPT_API_VERSION,
        );
    }
    check_digest(
        "admission_request_digest",
        &receipt.admission_request_digest,
        &mut failures,
    );
    check_bundle_subject(&receipt.bundle_subject, "bundle_subject", &mut failures);
    if receipt.bundle_subject != request.bundle_subject
        || receipt.policy_epoch != request.policy_epoch
    {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::AdmissionReceiptMismatch,
            "receipt",
            "receipt body echoes request bundle subject and policy epoch",
        );
    }

    if receipt.signing_envelope.is_some() {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::ReceiptSignatureCycle,
            "signing_envelope",
            "receipt body is hashed before any external signing envelope",
        );
    }

    if receipt.decision == AdmissionDecision::Accepted && receipt.rejection.is_some() {
        push_failure(
            &mut failures,
            AdmissionValidationFailureKind::AdmissionReceiptMismatch,
            "rejection",
            "accepted receipt omits rejection evidence",
        );
    }
    for admitted_operation in &receipt.admitted_operations {
        if !request
            .requested_operations
            .iter()
            .any(|operation| operation.operation_coordinate == *admitted_operation)
        {
            push_failure(
                &mut failures,
                AdmissionValidationFailureKind::AdmissionReceiptMismatch,
                "admitted_operations",
                "receipt admitted operations are a subset of requested operations",
            );
        }
    }

    check_digest_locked_resource("participant", &receipt.participant, &mut failures);
    check_resource_list(
        "admitted_capabilities",
        &receipt.admitted_capabilities,
        &mut failures,
    );
    check_digest_locked_resource(
        "admitted_runtime_budget",
        &receipt.admitted_runtime_budget,
        &mut failures,
    );
    if let Some(rejection) = &receipt.rejection {
        check_digest_locked_resource("rejection", rejection, &mut failures);
    }

    report(failures)
}

/// Check whether a Gate C invocation has accepted admission and invocation
/// capability evidence for the requested operation.
#[must_use]
pub fn check_gate_c_invocation(packet: &GateCInvocation) -> AdmissionValidationReport {
    let request_report = validate_admission_request(&packet.bundle, &packet.request);
    if request_report.status == AdmissionValidationStatus::Invalid {
        return request_report;
    }

    let Some(receipt) = &packet.receipt else {
        return report(vec![failure(
            AdmissionValidationFailureKind::MissingAdmissionReceipt,
            "receipt",
            "accepted admission receipt for the bundle subject",
        )]);
    };

    let receipt_report = validate_admission_receipt(&packet.request, receipt);
    if receipt_report.status == AdmissionValidationStatus::Invalid {
        return receipt_report;
    }
    if receipt.decision != AdmissionDecision::Accepted {
        return report(vec![failure(
            AdmissionValidationFailureKind::MissingAcceptedAdmissionReceipt,
            "receipt.decision",
            "accepted admission receipt",
        )]);
    }

    let operation_coordinate = packet.operation_coordinate.as_str();
    if !packet
        .request
        .requested_operations
        .iter()
        .any(|operation| operation.operation_coordinate == operation_coordinate)
    {
        return report(vec![failure(
            AdmissionValidationFailureKind::OperationRequirementMismatch,
            "operation_coordinate",
            "invoked operation appears in requested operation requirements",
        )]);
    }

    if !receipt
        .admitted_operations
        .iter()
        .any(|admitted_operation| admitted_operation == operation_coordinate)
    {
        return report(vec![failure(
            AdmissionValidationFailureKind::MissingAcceptedAdmissionReceipt,
            "receipt.admitted_operations",
            "accepted admission receipt for the invoked operation",
        )]);
    }

    if has_matching_invocation_capability(packet, receipt, operation_coordinate) {
        return report(Vec::new());
    }
    if has_matching_registration_receipt(packet, receipt, operation_coordinate) {
        return report(vec![failure(
            AdmissionValidationFailureKind::RegistrationReceiptIsNotInvocationAuthority,
            "capability_receipts",
            "invocation capability receipt, not registration evidence",
        )]);
    }
    report(vec![failure(
        AdmissionValidationFailureKind::MissingInvocationCapability,
        "capability_receipts",
        "matching invocation capability receipt",
    )])
}

fn check_operation_requirement(
    request: &AdmissionRequest,
    operation: &OperationRequirementRef,
    failures: &mut Vec<AdmissionValidationFailure>,
) {
    if operation.bundle_subject != request.bundle_subject
        || operation.operation_coordinate.is_empty()
        || !is_digest_locked_resource(&operation.basis)
        || !is_review_digest(&operation.variables_digest)
        || !is_review_digest(&operation.requirements_digest)
    {
        push_failure(
            failures,
            AdmissionValidationFailureKind::OperationRequirementMismatch,
            "requested_operations",
            "operation requirement binds request subject, coordinate, basis, variables, and requirements",
        );
    }

    for input in &operation.execution_inputs {
        if input.kind == ExecutionInputKind::HiddenHostInput {
            push_failure(
                failures,
                AdmissionValidationFailureKind::HiddenExecutionInput,
                "requested_operations.execution_inputs",
                "runtime input materialized as canonical input, witnessed evidence, admitted basis, or capability presentation",
            );
        } else {
            check_digest_locked_resource(
                "requested_operations.execution_inputs",
                &input.artifact,
                failures,
            );
        }
    }
}

fn has_matching_invocation_capability(
    packet: &GateCInvocation,
    admission_receipt: &AdmissionReceiptBody,
    operation_coordinate: &str,
) -> bool {
    packet.capability_receipts.iter().any(|capability| {
        capability.kind == CapabilityReceiptKind::Invocation
            && capability.issuer_bundle_subject == packet.request.bundle_subject
            && capability.participant == admission_receipt.participant
            && capability.operation_coordinate == operation_coordinate
            && capability.policy_epoch == packet.request.policy_epoch
            && is_digest_locked_resource(&capability.participant)
            && is_digest_locked_resource(&capability.scope)
    })
}

fn has_matching_registration_receipt(
    packet: &GateCInvocation,
    admission_receipt: &AdmissionReceiptBody,
    operation_coordinate: &str,
) -> bool {
    packet.capability_receipts.iter().any(|capability| {
        capability.kind == CapabilityReceiptKind::Registration
            && capability.issuer_bundle_subject == packet.request.bundle_subject
            && capability.participant == admission_receipt.participant
            && capability.operation_coordinate == operation_coordinate
            && capability.policy_epoch == packet.request.policy_epoch
    })
}

fn check_resource_list(
    field: &str,
    resources: &[ResourceRef],
    failures: &mut Vec<AdmissionValidationFailure>,
) {
    for resource in resources {
        check_digest_locked_resource(field, resource, failures);
    }
}

fn check_digest_locked_resource(
    field: &str,
    resource: &ResourceRef,
    failures: &mut Vec<AdmissionValidationFailure>,
) {
    if !is_digest_locked_resource(resource) {
        push_failure(
            failures,
            AdmissionValidationFailureKind::InvalidArtifactReference,
            field,
            "non-empty coordinate and lowercase sha256 digest",
        );
    }
}

fn check_bundle_subject(
    subject: &BundleSubject,
    field: &str,
    failures: &mut Vec<AdmissionValidationFailure>,
) {
    if !is_valid_bundle_subject(subject) {
        push_failure(
            failures,
            AdmissionValidationFailureKind::InvalidBundleSubject,
            field,
            "semantic or release bundle subject with lowercase sha256 digest",
        );
    }
}

fn check_digest(field: &str, digest: &str, failures: &mut Vec<AdmissionValidationFailure>) {
    if !is_review_digest(digest) {
        push_failure(
            failures,
            AdmissionValidationFailureKind::InvalidArtifactReference,
            field,
            "lowercase sha256 digest",
        );
    }
}

fn selected_bundle_digest(bundle: &ContractBundleManifest, kind: BundleSubjectKind) -> &str {
    match kind {
        BundleSubjectKind::Semantic => &bundle.semantic_bundle_digest,
        BundleSubjectKind::Release => &bundle.release_bundle_digest,
    }
}

fn is_valid_bundle_subject(subject: &BundleSubject) -> bool {
    is_review_digest(&subject.digest)
}

fn is_digest_locked_resource(resource: &ResourceRef) -> bool {
    !resource.coordinate.is_empty() && resource.digest.as_deref().is_some_and(is_review_digest)
}

fn is_review_digest(digest: &str) -> bool {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn report(failures: Vec<AdmissionValidationFailure>) -> AdmissionValidationReport {
    let status = if failures.is_empty() {
        AdmissionValidationStatus::Valid
    } else {
        AdmissionValidationStatus::Invalid
    };
    AdmissionValidationReport { status, failures }
}

fn push_failure(
    failures: &mut Vec<AdmissionValidationFailure>,
    kind: AdmissionValidationFailureKind,
    field: impl Into<String>,
    obligation: impl Into<String>,
) {
    let failure = failure(kind, field, obligation);
    if !failures.contains(&failure) {
        failures.push(failure);
    }
}

fn failure(
    kind: AdmissionValidationFailureKind,
    field: impl Into<String>,
    obligation: impl Into<String>,
) -> AdmissionValidationFailure {
    AdmissionValidationFailure {
        kind,
        field: field.into(),
        obligation: obligation.into(),
    }
}
