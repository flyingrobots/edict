//! Canonical, bounded application-result projections derived from Edict Core.
//!
//! The projection is data. It does not execute application code, call a target,
//! or grant runtime authority. Emitters derive it from exact Core and Target IR;
//! verifiers reconstruct the authored Core result from the claimed projection.

use std::collections::BTreeMap;
use std::fmt;

use crate::{CoreDigest, CoreModule, TargetIrArtifact};

/// Semantic schema identifier for result-projection values.
pub const RESULT_PROJECTION_API_VERSION: &str = "edict.result-projection/v1";

/// Domain used to compute result-projection artifact identities.
pub const RESULT_PROJECTION_DIGEST_DOMAIN: &str = "edict.result-projection.artifact/v1";

/// CDDL root published for result-projection artifacts.
pub const RESULT_PROJECTION_CDDL_ROOT: &str = "result-projection";

/// Maximum expression nodes in one projection artifact.
pub const MAX_RESULT_PROJECTION_NODES: usize = 256;

/// Maximum field-path segments in one source reference.
pub const MAX_RESULT_PROJECTION_PATH_SEGMENTS: usize = 32;

/// Maximum UTF-8 bytes in any coordinate, field, step, or path segment.
pub const MAX_RESULT_PROJECTION_TEXT_BYTES: usize = 1_024;

/// Maximum canonical bytes in one projection artifact.
pub const MAX_RESULT_PROJECTION_ARTIFACT_BYTES: usize = 64 * 1_024;

/// Canonical result projection for one application operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultProjection {
    pub api_version: String,
    pub operation_coordinate: String,
    pub output_type: String,
    pub max_output_bytes: u64,
    pub expression: ResultProjectionExpr,
}

/// Closed projection-expression language.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultProjectionExpr {
    Record {
        fields: BTreeMap<String, ResultProjectionExpr>,
    },
    Source {
        source: ResultProjectionSource,
        path: Vec<String>,
    },
}

/// Declared data sources available to a result projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResultProjectionSource {
    ApplicationInput,
    CapabilityResult { step_id: String },
}

/// Emitted canonical bytes and their domain-framed identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultProjectionArtifact {
    pub projection: ResultProjection,
    pub canonical_bytes: Vec<u8>,
    pub digest: CoreDigest,
}

/// Projection accepted after independent semantic reconstruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedResultProjection {
    projection: ResultProjection,
    digest: CoreDigest,
}

impl VerifiedResultProjection {
    /// Borrow the accepted projection.
    #[must_use]
    pub const fn projection(&self) -> &ResultProjection {
        &self.projection
    }

    /// Return the accepted domain-framed identity.
    #[must_use]
    pub const fn digest(&self) -> CoreDigest {
        self.digest
    }
}

/// Stable result-projection rejection categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultProjectionFailureKind {
    NotImplemented,
    MissingIntent,
    CoreTargetMismatch,
    TargetResultMismatch,
    InvalidApplicationInput,
    UndeclaredProjectionSource,
    UnknownCapabilityStep,
    UnsupportedExpression,
    OutputShapeMismatch,
    InvalidOutputBound,
    ProjectionLimitExceeded,
    InvalidCanonicalArtifact,
    DigestMismatch,
}

/// Structured result-projection failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultProjectionFailure {
    kind: ResultProjectionFailureKind,
    subject: String,
}

impl ResultProjectionFailure {
    /// Return the stable rejection category.
    #[must_use]
    pub const fn kind(&self) -> ResultProjectionFailureKind {
        self.kind
    }

    /// Return the rejected semantic subject.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    fn not_implemented(subject: impl Into<String>) -> Self {
        Self {
            kind: ResultProjectionFailureKind::NotImplemented,
            subject: subject.into(),
        }
    }
}

impl fmt::Display for ResultProjectionFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.subject)
    }
}

impl std::error::Error for ResultProjectionFailure {}

/// Derive and emit one canonical projection artifact.
///
/// # Errors
///
/// Returns a structured failure when Core, Target IR, result sources, output
/// shape, or representation bounds do not admit the closed projection subset.
pub fn emit_result_projection(
    _core: &CoreModule,
    _target_ir: &TargetIrArtifact,
    _intent_name: &str,
) -> Result<ResultProjectionArtifact, ResultProjectionFailure> {
    Err(ResultProjectionFailure::not_implemented("emit"))
}

/// Independently verify claimed projection bytes against exact Core and Target IR.
///
/// # Errors
///
/// Returns a structured failure for malformed or non-canonical bytes, digest
/// mismatch, unsupported semantics, or failed reverse reconstruction.
pub fn verify_result_projection(
    _core: &CoreModule,
    _target_ir: &TargetIrArtifact,
    _intent_name: &str,
    _canonical_bytes: &[u8],
    _claimed_digest: CoreDigest,
) -> Result<VerifiedResultProjection, ResultProjectionFailure> {
    Err(ResultProjectionFailure::not_implemented("verify"))
}

/// Encode one projection using `edict.canonical-cbor/v1`.
///
/// # Errors
///
/// Returns a structured failure when the projection violates a representation
/// bound or cannot be represented canonically.
pub fn encode_result_projection(
    _projection: &ResultProjection,
) -> Result<Vec<u8>, ResultProjectionFailure> {
    Err(ResultProjectionFailure::not_implemented("encode"))
}

/// Decode and validate one canonical projection value.
///
/// # Errors
///
/// Returns a structured failure for malformed, non-canonical, incomplete,
/// unsupported, or unbounded projection bytes.
pub fn decode_result_projection(
    _bytes: &[u8],
) -> Result<ResultProjection, ResultProjectionFailure> {
    Err(ResultProjectionFailure::not_implemented("decode"))
}

/// Compute the domain-framed identity of one projection.
///
/// # Errors
///
/// Returns a structured failure when canonical encoding fails.
pub fn digest_result_projection(
    _projection: &ResultProjection,
) -> Result<CoreDigest, ResultProjectionFailure> {
    Err(ResultProjectionFailure::not_implemented("digest"))
}
