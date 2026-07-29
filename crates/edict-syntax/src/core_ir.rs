//! In-memory Edict Core IR value model for the compiler-spine stage.
//!
//! These Rust values mirror the `edict.core/v1` semantic shape closely enough
//! for source-to-Core lowering tests. They are not canonical bytes, do not carry
//! their own digest, and do not represent target IR or admission bundles.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The Core ABI identifier emitted by this crate.
pub const CORE_API_VERSION: &str = "edict.core/v1";

/// Compiler-owned local identity for the single application intent input.
pub(crate) const CORE_APPLICATION_INPUT_LOCAL_ID: &str = "arg.0";

/// A lowered in-memory Core module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreModule {
    pub api_version: String,
    pub coordinate: String,
    pub imports: Vec<CoreImport>,
    pub types: BTreeMap<String, CoreType>,
    pub intents: BTreeMap<String, CoreIntent>,
    pub required_core_capabilities: Vec<String>,
}

/// A Core import that survives source resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreImport {
    pub kind: CoreImportKind,
    pub resource: ResourceRef,
    pub alias: Option<String>,
}

/// Core import kinds. Shape imports are source-only and do not lower to Core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreImportKind {
    Lawpack,
    Target,
    Core,
}

impl CoreImportKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lawpack => "lawpack",
            Self::Target => "target",
            Self::Core => "core",
        }
    }
}

/// Digest-locked external artifact reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRef {
    pub coordinate: String,
    pub digest: Option<String>,
}

impl ResourceRef {
    #[must_use]
    pub(crate) fn is_digest_locked(&self) -> bool {
        !self.coordinate.is_empty() && self.digest.as_deref().is_some_and(is_sha256_review_digest)
    }
}

pub(crate) fn is_sha256_review_digest(digest: &str) -> bool {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn is_lowercase_sha256_review_digest(digest: &str) -> bool {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(crate) fn parse_core_integer(width: &str, value: &str) -> Option<i128> {
    match width {
        "I8" => parse_signed_integer(value, i8::MIN.into(), i8::MAX.into()),
        "I16" => parse_signed_integer(value, i16::MIN.into(), i16::MAX.into()),
        "I32" => parse_signed_integer(value, i32::MIN.into(), i32::MAX.into()),
        "I64" => parse_signed_integer(value, i64::MIN.into(), i64::MAX.into()),
        "U8" => parse_unsigned_integer(value, u8::MAX.into()),
        "U16" => parse_unsigned_integer(value, u16::MAX.into()),
        "U32" => parse_unsigned_integer(value, u32::MAX.into()),
        "U64" => parse_unsigned_integer(value, u64::MAX.into()),
        _ => None,
    }
}

fn parse_signed_integer(value: &str, min: i128, max: i128) -> Option<i128> {
    value
        .parse::<i128>()
        .ok()
        .filter(|value| (min..=max).contains(value))
}

fn parse_unsigned_integer(value: &str, max: u128) -> Option<i128> {
    value
        .parse::<u128>()
        .ok()
        .filter(|value| *value <= max)
        .and_then(|value| i128::try_from(value).ok())
}

/// Core type model for the initial compiler-spine subset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreType {
    Bool,
    Int {
        width: String,
    },
    String {
        max: u64,
        canonical: String,
    },
    Bytes {
        max: u64,
    },
    Record {
        fields: BTreeMap<String, String>,
    },
    Variant {
        cases: BTreeMap<String, Option<String>>,
    },
    Option {
        item: String,
    },
    List {
        item: String,
        max: u64,
    },
    Map {
        key: String,
        value: String,
        max: u64,
    },
    CapabilityRef {
        item: String,
    },
}

/// Alpha-stable local reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalRef {
    pub id: String,
    pub alpha_name: String,
    pub ty: String,
}

/// Literal Core value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreValue {
    Null,
    Bool(bool),
    Int { width: String, value: String },
    String(String),
    Bytes(Vec<u8>),
}

/// Core expression subset used by initial source-to-Core lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreExpr {
    Local {
        reference: LocalRef,
    },
    Const(CoreValue),
    Record {
        fields: BTreeMap<String, CoreExpr>,
    },
    Field {
        base: Box<CoreExpr>,
        field: String,
    },
    Call {
        callee: String,
        type_args: Vec<String>,
        args: Vec<CoreExpr>,
    },
}

/// Core predicate subset used by initial source-to-Core lowering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorePredicate {
    True,
    False,
    Not(Box<CorePredicate>),
    All(Vec<CorePredicate>),
    Any(Vec<CorePredicate>),
    Compare {
        op: CompareOp,
        left: CoreExpr,
        right: CoreExpr,
    },
}

/// Comparison operators in Core predicates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Source-origin input constraint lowered into Core.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputConstraint {
    pub coordinate: String,
    pub source: InputConstraintSource,
    pub predicate: CorePredicate,
}

/// Origin of a Core input constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputConstraintSource {
    Where,
    Compiler,
}

/// Core evaluation budget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreBudget {
    pub max_steps: u64,
    pub max_allocated_bytes: u64,
    pub max_output_bytes: u64,
}

/// Core intent shape for the initial lowerer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreIntent {
    pub input: String,
    pub output: String,
    pub required_operation_profile: String,
    /// Authored basis expression. Runtime basis resolution and
    /// admission remain target/runtime responsibilities.
    pub basis: Option<CoreExpr>,
    pub input_constraints: Vec<InputConstraint>,
    pub core_evaluation_budget: CoreBudget,
    pub body: CoreBlock,
}

/// Core block with alpha-stable locals and ordered nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreBlock {
    pub locals: Vec<LocalRef>,
    pub nodes: Vec<CoreNode>,
    pub result: CoreExpr,
}

/// Core node subset used by the first source-to-Core slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreNode {
    Let {
        binding: LocalRef,
        value: CoreExpr,
    },
    Require {
        predicate: CorePredicate,
        arm: CoreRequireFailureArm,
    },
    Effect {
        binding: LocalRef,
        effect: String,
        input: CoreExpr,
        obstruction_map: BTreeMap<String, CoreObstructionArm>,
    },
}

/// Core disposition for a failed `require` predicate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreRequireFailureArm {
    /// Terminal typed obstruction: the success path cannot continue.
    Terminal { reason: CoreObstructionReason },
    /// Preserved obstruction strand: the attempt continues as obstructed
    /// causal support rather than collapsing into terminal obstruction.
    ContinueObstructed { reason: CoreObstructionReason },
}

/// Closed Core reason envelope with opaque canonical payload fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreObstructionReason {
    pub kind: String,
    pub payload: BTreeMap<String, CoreExpr>,
}

/// Core obstruction arm for a semantic effect failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreObstructionArm {
    pub binder: LocalRef,
    pub value: CoreExpr,
}
