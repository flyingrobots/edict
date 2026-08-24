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
    Capability,
}

impl CoreImportKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Lawpack => "lawpack",
            Self::Target => "target",
            Self::Core => "core",
            Self::Capability => "capability",
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
        min: Option<u64>,
        max: u64,
    },
    Nominal {
        contract: String,
        representation: String,
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
    ExternalActionRequest {
        settlement: String,
    },
}

const MAX_CORE_TYPE_COMPATIBILITY_DEPTH: usize = 64;

pub(crate) fn core_type_fits(core: &CoreModule, source: &str, target: &str) -> bool {
    core_type_fits_at_depth(core, source, target, 0)
}

fn core_type_fits_at_depth(core: &CoreModule, source: &str, target: &str, depth: usize) -> bool {
    if depth > MAX_CORE_TYPE_COMPATIBILITY_DEPTH {
        return false;
    }
    let (Some(left), Some(right)) = (
        resolved_core_type(core, source),
        resolved_core_type(core, target),
    ) else {
        return false;
    };
    if let Some(fits) = nominal_types_fit(&left, &right) {
        return fits;
    }
    if let Some(fits) = scalar_types_fit(&left, &right) {
        return fits;
    }
    match (&left, &right) {
        (
            CoreType::Record {
                fields: left_fields,
            },
            CoreType::Record {
                fields: right_fields,
            },
        ) => {
            left_fields.keys().eq(right_fields.keys())
                && left_fields.iter().all(|(field, left_type)| {
                    core_type_fits_at_depth(
                        core,
                        left_type,
                        right_fields.get(field).expect("equal field keys"),
                        depth + 1,
                    )
                })
        }
        (CoreType::Variant { cases: left }, CoreType::Variant { cases: right }) => {
            left.keys().eq(right.keys())
                && left.iter().all(|(case, left_payload)| {
                    match (
                        left_payload,
                        right.get(case).expect("equal variant case keys"),
                    ) {
                        (None, None) => true,
                        (Some(left), Some(right)) => {
                            core_type_fits_at_depth(core, left, right, depth + 1)
                        }
                        (None, Some(_)) | (Some(_), None) => false,
                    }
                })
        }
        (CoreType::Option { item: left }, CoreType::Option { item: right })
        | (CoreType::CapabilityRef { item: left }, CoreType::CapabilityRef { item: right }) => {
            core_type_fits_at_depth(core, left, right, depth + 1)
        }
        (
            CoreType::ExternalActionRequest { settlement: left },
            CoreType::ExternalActionRequest { settlement: right },
        ) => core_type_fits_at_depth(core, left, right, depth + 1),
        (
            CoreType::List {
                item: left,
                max: left_max,
            },
            CoreType::List {
                item: right,
                max: right_max,
            },
        ) => left_max <= right_max && core_type_fits_at_depth(core, left, right, depth + 1),
        (
            CoreType::Map {
                key: left_key,
                value: left_value,
                max: left_max,
            },
            CoreType::Map {
                key: right_key,
                value: right_value,
                max: right_max,
            },
        ) => {
            left_max <= right_max
                && core_type_fits_at_depth(core, left_key, right_key, depth + 1)
                && core_type_fits_at_depth(core, left_value, right_value, depth + 1)
        }
        _ => false,
    }
}

pub(crate) fn resolved_core_type(core: &CoreModule, coordinate: &str) -> Option<CoreType> {
    core.types
        .get(coordinate)
        .or_else(|| {
            coordinate
                .strip_prefix(core.coordinate.as_str())
                .and_then(|relative| relative.strip_prefix('.'))
                .and_then(|relative| core.types.get(relative))
        })
        .cloned()
        .or_else(|| builtin_core_type(coordinate))
}

fn builtin_core_type(coordinate: &str) -> Option<CoreType> {
    if coordinate == "Bool" {
        return Some(CoreType::Bool);
    }
    if matches!(coordinate, "I32" | "I64" | "U32" | "U64") {
        return Some(CoreType::Int {
            width: coordinate.to_owned(),
        });
    }
    if let Some(item) = coordinate
        .strip_prefix("Option<")
        .and_then(|value| value.strip_suffix('>'))
    {
        return Some(CoreType::Option {
            item: item.to_owned(),
        });
    }
    if let Some(item) = coordinate
        .strip_prefix("CapabilityRef<")
        .and_then(|value| value.strip_suffix('>'))
    {
        return Some(CoreType::CapabilityRef {
            item: item.to_owned(),
        });
    }
    if let Some(settlement) = [
        "ExternalActionRequest<",
        "edict.external-action.request/v1<",
    ]
    .into_iter()
    .find_map(|prefix| coordinate.strip_prefix(prefix))
    .and_then(|value| value.strip_suffix('>'))
    {
        return Some(CoreType::ExternalActionRequest {
            settlement: settlement.to_owned(),
        });
    }
    if let Some(inner) = coordinate
        .strip_prefix("String<max=")
        .and_then(|value| value.strip_suffix('>'))
    {
        let (max, canonical) = inner.split_once(",canonical=")?;
        return Some(CoreType::String {
            max: max.parse().ok()?,
            canonical: canonical.to_owned(),
        });
    }
    if let Some(max) = coordinate
        .strip_prefix("Bytes<max=")
        .and_then(|value| value.strip_suffix('>'))
        .and_then(|max| max.parse().ok())
    {
        return Some(CoreType::Bytes { min: None, max });
    }
    coordinate
        .strip_prefix("Bytes<exact=")
        .and_then(|value| value.strip_suffix('>'))
        .and_then(|exact| exact.parse().ok())
        .map(|exact| CoreType::Bytes {
            min: Some(exact),
            max: exact,
        })
}

fn scalar_types_fit(left: &CoreType, right: &CoreType) -> Option<bool> {
    match (left, right) {
        (CoreType::Bool, CoreType::Bool) => Some(true),
        (CoreType::Int { width: left }, CoreType::Int { width: right }) => Some(left == right),
        (
            CoreType::String {
                max: left_max,
                canonical: left_canonical,
            },
            CoreType::String {
                max: right_max,
                canonical: right_canonical,
            },
        ) => Some(left_max <= right_max && left_canonical == right_canonical),
        (
            CoreType::Bytes {
                min: left_min,
                max: left_max,
            },
            CoreType::Bytes {
                min: right_min,
                max: right_max,
            },
        ) => Some(left_min.unwrap_or(0) >= right_min.unwrap_or(0) && left_max <= right_max),
        _ => None,
    }
}

fn nominal_types_fit(left: &CoreType, right: &CoreType) -> Option<bool> {
    match (left, right) {
        (
            CoreType::Nominal {
                contract: left_contract,
                representation: left_representation,
            },
            CoreType::Nominal {
                contract: right_contract,
                representation: right_representation,
            },
        ) => Some(left_contract == right_contract && left_representation == right_representation),
        (CoreType::Nominal { .. }, _) | (_, CoreType::Nominal { .. }) => Some(false),
        _ => None,
    }
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
    If {
        predicate: Box<CorePredicate>,
        then_value: Box<CoreExpr>,
        else_value: Box<CoreExpr>,
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

/// Static maximum carried by bounded Core control flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreBound {
    Literal(u64),
    Coordinate(String),
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
    ExternalActionRequest {
        binding: LocalRef,
        operation: ResourceRef,
        input_type: String,
        settlement_type: String,
        input_schema: ResourceRef,
        settlement_schema: ResourceRef,
        input: CoreExpr,
        authority_scope: Box<CoreExpr>,
        basis: Box<CoreExpr>,
        budget: Box<CoreExternalActionBudget>,
        reconciliation_law: ResourceRef,
    },
    For {
        binder: LocalRef,
        iter: CoreExpr,
        bound: CoreBound,
        body: CoreBlock,
    },
    Branch {
        /// When present, the selected block result becomes this local.
        /// Statement-only branches leave the binding absent.
        binding: Option<LocalRef>,
        predicate: CorePredicate,
        then_block: CoreBlock,
        else_block: CoreBlock,
    },
}

/// Runtime-valued bounds carried by a typed external-action request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreExternalActionBudget {
    pub max_settlement_bytes: CoreExpr,
    pub max_attempts: CoreExpr,
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
