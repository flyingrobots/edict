//! Curated public Rust facade for Edict.
//!
//! The [`check`] entry point is the supported one-call source check. Stable
//! failure classifications are grouped under [`diagnostic`], while canonical
//! semantic-artifact identity operations are grouped under [`artifact`]. The
//! implementation crate's broad module tree is intentionally not re-exported.
//!
//! ```
//! use edict::{check, CheckOutcome};
//!
//! assert_eq!(
//!     check("package examples.public_facade@1;\n"),
//!     CheckOutcome::Valid
//! );
//! ```
//!
//! Implementation modules are not part of this facade:
//!
//! ```compile_fail
//! use edict::parser::parse_module;
//! ```

pub use edict_syntax::{check, CheckOutcome};

/// Stable machine-usable failure classifications exposed by the public
/// facade.
pub mod diagnostic {
    pub use edict_syntax::{
        CanonicalError, CanonicalErrorKind, CompilerError, CompilerErrorKind, CompilerStage,
        ParseError, ParseErrorKind, ResultProjectionFailure, ResultProjectionFailureKind,
        SemanticError, SemanticErrorKind, TargetLoweringFailure, TargetLoweringFailureKind,
    };
}

/// Canonical semantic-artifact values, encoders, and domain-framed identity
/// operations.
pub mod artifact {
    pub use edict_syntax::{
        decode_canonical_cbor, decode_result_projection, digest_core_module,
        digest_result_projection, digest_target_ir_artifact, encode_core_module,
        encode_result_projection, encode_target_ir_artifact, verify_result_projection,
        CanonicalError, CanonicalErrorKind, CoreDigest, CoreModule, ResultProjection,
        ResultProjectionArtifact, ResultProjectionFailure, ResultProjectionFailureKind,
        TargetIrArtifact,
    };
}
