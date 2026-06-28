//! `edict-syntax`: the Edict language front end.
//!
//! Scope is `edict.implementation/minimal-v1` (see SPEC - Edict Language v1).
//! Phase 1 parses: package and imports; `type` records and refined scalars;
//! `enum` declarations and `variant` types; `intent`s with their clauses;
//! `let`/`return`/`require`/`guarantee`/`assert`; the `if` family; bounded
//! `for`; calls and type-calls; variant-literal constructors; and `match`.
//! Phase 2 currently exposes the `validate_surface` compiler stage for
//! source-AST constraints that do not require import resolution, resolved typing,
//! target/lawpack facts, or Core IR: bounded runtime `String`/`Bytes`, required
//! intent operation-mode/budget/basis clauses, duplicate singleton intent
//! clauses, module namespace collisions, and source binder shadowing.
//! Phase 3 begins the executable compiler spine with `resolve_module`,
//! `type_check`, `lower_core`, and `compile_to_core`, currently covering the
//! initial pure local-record subset and producing in-memory Core IR only.
//! Authority-facts loading can build compiler context from explicit,
//! digest-bound JSON files for the first supported profile, budget, and
//! write-class facts.
//! The crate also exposes typed v1 target-profile conformance, lowerability, and
//! Echo/git-warp Target IR lowering, contract-bundle checks, and typed Gate C
//! admission-boundary checks.
//! Developer-tooling support begins with lexical highlighting roles for editor
//! adapters.
//! Conformance validates runtime-neutral target-profile manifests. Lowerability
//! checks `LoweringRequirements` against explicit target-profile facts and
//! classifies support as native, directly adapted, or unsupported.
//! Contract-bundle validation checks participant-neutral, SHA-locked bundle and
//! assurance evidence manifests. Admission-boundary checks validate Edict-owned
//! artifact and invocation evidence bindings without evaluating participant
//! policy.
//! Pure `fn`/`const` declarations, `record` semantic-effect statements,
//! list/map/unit expression literals, full source-language lowering, general
//! target lowering, and full admission execution tooling are deferred. The
//! crate exposes the reference canonical Core encoder for
//! `edict.canonical-cbor/v1` and the domain-separated `edict.core.module/v1`
//! Core digest used by reviewed golden fixtures.
//!
//! Assurance tooling (HOLMES / Watson / Moriarty) remains shared platform
//! machinery. This crate validates typed references to its evidence; it does not
//! execute those tools or make admission decisions.

pub mod admission;
pub mod ast;
pub mod authority_facts;
pub mod canonical;
pub mod compiler;
pub mod contract_bundle;
pub mod core_ir;
pub mod highlight;
pub mod lowerability;
pub mod parser;
pub mod semantic;
pub mod target_ir;
pub mod target_profile;
pub mod token;

pub use admission::{
    check_gate_c_invocation, digest_admission_request, validate_admission_receipt,
    validate_admission_request, AdmissionDecision, AdmissionEvidenceRef, AdmissionReceiptBody,
    AdmissionRequest, AdmissionValidationFailure, AdmissionValidationFailureKind,
    AdmissionValidationReport, AdmissionValidationStatus, AuthoringProvenance, CapabilityReceipt,
    CapabilityReceiptKind, ExecutionInputKind, ExecutionInputRef, GateCInvocation,
    OperationRequirementRef, ADMISSION_RECEIPT_API_VERSION, ADMISSION_REQUEST_API_VERSION,
    ADMISSION_REQUEST_DIGEST_DOMAIN,
};
pub use authority_facts::{
    compiler_context_from_authority_facts, load_authority_facts_file,
    load_compiler_context_from_authority_fact_files, AuthorityFactSource, AuthorityFactSourceKind,
    AuthorityFactsDocument, AuthorityFactsLoadFailure, AuthorityFactsLoadFailureKind, BudgetFact,
    EffectWriteClassFact, OperationProfileFact, AUTHORITY_FACTS_API_VERSION,
};
pub use canonical::{
    decode_canonical_cbor, digest_core_module, encode_canonical_cbor, encode_core_module,
    CanonicalError, CanonicalErrorKind, CanonicalValue, CoreDigest, CORE_CANONICAL_ENCODING,
    CORE_DIGEST_FRAME, CORE_MODULE_DIGEST_DOMAIN,
};
pub use compiler::{
    compile_to_core, lower_core, resolve_module, type_check, CompilerContext, CompilerError,
    CompilerErrorKind, CompilerStage, ResolvedIntent, ResolvedModule, ResolvedTypeDecl,
    TypedIntent, TypedModule,
};
pub use contract_bundle::{
    validate_contract_bundle_manifest, AssuranceEvidenceRef, AssuranceRole, BundleSubject,
    BundleSubjectKind, ContractBundleManifest, ContractBundleValidationFailure,
    ContractBundleValidationFailureKind, ContractBundleValidationReport,
    ContractBundleValidationStatus, SourceArtifactRef, CONTRACT_BUNDLE_API_VERSION,
};
pub use core_ir::{
    CompareOp, CoreBlock, CoreBudget, CoreExpr, CoreImport, CoreImportKind, CoreIntent, CoreModule,
    CoreNode, CoreObstructionArm, CorePredicate, CoreType, CoreValue, InputConstraint,
    InputConstraintSource, LocalRef, ResourceRef, CORE_API_VERSION,
};
pub use highlight::{highlight_source, HighlightRole, HighlightToken};
pub use lowerability::{
    check_lowerability, AtomicityRequirement, DirectAdapterSupport, GuardKind,
    LowerabilityEffectResult, LowerabilityEffectStatus, LowerabilityFailure,
    LowerabilityFailureKind, LowerabilityReport, LowerabilityStatus, LoweringRequirements,
    NativeEffectSupport, SemanticEffectRequirement, TargetProfileFacts, WriteClass,
};
pub use parser::{parse_module, ParseError, ParseErrorKind};
pub use semantic::{validate_module, validate_surface, SemanticError, SemanticErrorKind};
pub use target_ir::{
    lower_to_target_ir, TargetEffectLowering, TargetIrArtifact, TargetIrIntent,
    TargetIrLoweringFacts, TargetIrStep, TargetLoweringFailure, TargetLoweringFailureKind,
    TargetLoweringReport, TargetLoweringStatus, ECHO_DPO_TARGET_PROFILE, ECHO_SPAN_IR_DOMAIN,
    GITWARP_COMMIT_REDUCER_IR_DOMAIN, GITWARP_REF_CRDT_TARGET_PROFILE,
};
pub use target_profile::{
    validate_target_profile_manifest, TargetProfileConformanceFailure,
    TargetProfileConformanceFailureKind, TargetProfileConformanceReport,
    TargetProfileConformanceStatus, TargetProfileManifest, CANONICAL_CBOR_ABI,
    TARGET_PROFILE_API_VERSION,
};
pub use token::{lex, IntSuffix, LexError, Span, Token, TokenKind};

#[cfg(doctest)]
mod topic_shelf_doctests {
    #[doc = include_str!("../../../docs/topics/authority-facts/README.md")]
    pub struct AuthorityFactsTopicDocs;

    #[doc = include_str!("../../../docs/topics/core-ir/README.md")]
    pub struct CoreIrTopicDocs;

    #[doc = include_str!("../../../docs/topics/compiler-spine/README.md")]
    pub struct CompilerSpineTopicDocs;

    #[doc = include_str!("../../../docs/topics/contract-bundles/README.md")]
    pub struct ContractBundlesTopicDocs;

    #[doc = include_str!("../../../docs/topics/admission/README.md")]
    pub struct AdmissionTopicDocs;

    #[doc = include_str!("../../../docs/topics/lowerability/README.md")]
    pub struct LowerabilityTopicDocs;

    #[doc = include_str!("../../../docs/topics/target-profiles/README.md")]
    pub struct TargetProfilesTopicDocs;

    #[doc = include_str!("../../../docs/topics/semantic-validation/README.md")]
    pub struct SemanticValidationTopicDocs;

    #[doc = include_str!("../../../docs/topics/syntax/README.md")]
    pub struct SyntaxTopicDocs;

    #[doc = include_str!("../../../docs/topics/developer-tooling/README.md")]
    pub struct DeveloperToolingTopicDocs;
}
