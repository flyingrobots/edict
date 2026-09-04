//! In-memory Edict Core IR value model for the compiler-spine stage.
//!
//! These Rust values mirror the `edict.core/v1` semantic shape closely enough
//! for source-to-Core lowering tests. They are not canonical bytes, do not carry
//! their own digest, and do not represent target IR or admission bundles.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};

/// The Core ABI identifier emitted by this crate.
pub const CORE_API_VERSION: &str = "edict.core/v1";

/// Compiler-owned local identity for the single application intent input.
pub(crate) const CORE_APPLICATION_INPUT_LOCAL_ID: &str = "arg.0";

/// Shared recursion ceiling for compiler type resolution and Core compatibility.
pub(crate) const MAX_CORE_TYPE_DEPTH: usize = 128;

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

/// A Core module proven to satisfy the complete type-integrity judgment.
///
/// The witness borrows the raw module, so safe Rust cannot mutate that module
/// while a consumer relies on the validation result. Its private field makes
/// successful [`validate_core_module_type_integrity`] the only constructor.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedCoreModule<'a> {
    module: &'a CoreModule,
}

impl<'a> ValidatedCoreModule<'a> {
    /// Borrow the exact raw module covered by this witness.
    #[must_use]
    pub const fn module(self) -> &'a CoreModule {
        self.module
    }
}

/// Stable categories for whole-module Core type-integrity failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreTypeIntegrityFailureKind {
    InvalidTableKey,
    InvalidDefinition,
    InvalidReference,
    UnresolvedNamedReference,
    NominalContractMismatch,
    ReferenceCycle,
    DepthExceeded,
}

/// Structured failure from the authoritative Core type-integrity judgment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreTypeIntegrityFailure {
    kind: CoreTypeIntegrityFailureKind,
    path: String,
}

impl CoreTypeIntegrityFailure {
    /// Return the stable rejection category.
    #[must_use]
    pub const fn kind(&self) -> CoreTypeIntegrityFailureKind {
        self.kind
    }

    /// Return the structural path to the invalid definition or reference.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    fn new(kind: CoreTypeIntegrityFailureKind, path: impl Into<String>) -> Self {
        Self {
            kind,
            path: path.into(),
        }
    }
}

impl fmt::Display for CoreTypeIntegrityFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.path)
    }
}

impl std::error::Error for CoreTypeIntegrityFailure {}

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
    Unit,
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

/// State-free classification of a Core type reference.
///
/// Intrinsic and structural references carry their own meaning. Named
/// references acquire meaning only from an exact entry in [`CoreModule::types`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CoreTypeReference {
    Intrinsic(CoreType),
    Structural(CoreType),
    Named,
}

/// Classify one canonical Core type reference without consulting module state.
pub(crate) fn classify_core_type_reference(reference: &str) -> Option<CoreTypeReference> {
    classify_core_type_reference_at_depth(reference, 0)
}

fn classify_core_type_reference_at_depth(
    reference: &str,
    depth: usize,
) -> Option<CoreTypeReference> {
    if depth > MAX_CORE_TYPE_DEPTH {
        return None;
    }
    if reference == "Bool" {
        return Some(CoreTypeReference::Intrinsic(CoreType::Bool));
    }
    if reference == "Unit" {
        return Some(CoreTypeReference::Intrinsic(CoreType::Unit));
    }
    if matches!(
        reference,
        "I8" | "I16" | "I32" | "I64" | "U8" | "U16" | "U32" | "U64"
    ) {
        return Some(CoreTypeReference::Intrinsic(CoreType::Int {
            width: reference.to_owned(),
        }));
    }
    if looks_like_structural_type_reference(reference) {
        let parsed = parse_structural_core_type_at_depth(reference, depth)?;
        return Some(CoreTypeReference::Structural(parsed));
    }
    if is_named_core_type_reference(reference) {
        Some(CoreTypeReference::Named)
    } else {
        None
    }
}

fn looks_like_structural_type_reference(reference: &str) -> bool {
    [
        "String<",
        "Bytes<",
        "Record<",
        "Option<",
        "List<",
        "Map<",
        "CapabilityRef<",
        "ExternalActionRequest<",
        "edict.external-action.request/v1<",
        "Nominal<",
        "Variant<",
    ]
    .into_iter()
    .any(|prefix| reference.starts_with(prefix))
}

fn is_named_core_type_reference(reference: &str) -> bool {
    !reference.is_empty()
        && reference != "anonymous.record"
        && !matches!(
            reference,
            "String"
                | "Bytes"
                | "Record"
                | "Option"
                | "List"
                | "Map"
                | "CapabilityRef"
                | "ExternalActionRequest"
                | "Nominal"
                | "Variant"
        )
        && reference.chars().all(|character| {
            !character.is_whitespace() && !matches!(character, '<' | '>' | ',' | ':' | '=')
        })
}

fn parse_canonical_u64(value: &str) -> Option<u64> {
    let parsed: u64 = value.parse().ok()?;
    (parsed.to_string() == value).then_some(parsed)
}

fn parse_structural_string_type(reference: &str) -> Option<CoreType> {
    let inner = reference.strip_prefix("String<max=")?.strip_suffix('>')?;
    let (max, canonical) = inner.split_once(",canonical=")?;
    if !matches!(canonical, "raw-utf8" | "unicode-scalar-nfc") {
        return None;
    }
    Some(CoreType::String {
        max: parse_canonical_u64(max)?,
        canonical: canonical.to_owned(),
    })
}

fn parse_structural_bytes_type(reference: &str) -> Option<CoreType> {
    let inner = reference.strip_prefix("Bytes<")?.strip_suffix('>')?;
    if let Some(bounds) = inner.strip_prefix("min=") {
        let (min, max) = bounds.split_once(",max=")?;
        let min = parse_canonical_u64(min)?;
        let max = parse_canonical_u64(max)?;
        return (min < max).then_some(CoreType::Bytes {
            min: Some(min),
            max,
        });
    }
    if let Some(max) = inner.strip_prefix("max=") {
        return Some(CoreType::Bytes {
            min: None,
            max: parse_canonical_u64(max)?,
        });
    }
    let exact = parse_canonical_u64(inner.strip_prefix("exact=")?)?;
    Some(CoreType::Bytes {
        min: Some(exact),
        max: exact,
    })
}

fn parse_structural_core_type_at_depth(reference: &str, depth: usize) -> Option<CoreType> {
    if reference.starts_with("String<") {
        return parse_structural_string_type(reference);
    }
    if reference.starts_with("Bytes<") {
        return parse_structural_bytes_type(reference);
    }
    if let Some(inner) = reference
        .strip_prefix("Record<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let mut fields = BTreeMap::new();
        let mut previous_name = None;
        for field in split_top_level_type_parts(inner)? {
            let (name, ty) = field.split_once(':')?;
            if !is_core_field_name(name)
                || ty.is_empty()
                || previous_name.is_some_and(|previous| previous >= name)
                || classify_core_type_reference_at_depth(ty, depth + 1).is_none()
                || fields.insert(name.to_owned(), ty.to_owned()).is_some()
            {
                return None;
            }
            previous_name = Some(name);
        }
        return Some(CoreType::Record { fields });
    }
    if let Some(item) = reference
        .strip_prefix("Option<")
        .and_then(|value| value.strip_suffix('>'))
    {
        classify_core_type_reference_at_depth(item, depth + 1)?;
        return Some(CoreType::Option {
            item: item.to_owned(),
        });
    }
    if let Some(inner) = reference
        .strip_prefix("List<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let parts = split_top_level_type_parts(inner)?;
        let [item, max] = parts.as_slice() else {
            return None;
        };
        classify_core_type_reference_at_depth(item, depth + 1)?;
        let max = max.strip_prefix("max=")?;
        let parsed_max: u64 = max.parse().ok()?;
        if parsed_max.to_string() != max {
            return None;
        }
        return Some(CoreType::List {
            item: (*item).to_owned(),
            max: parsed_max,
        });
    }
    if let Some(inner) = reference
        .strip_prefix("Map<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let parts = split_top_level_type_parts(inner)?;
        let [key, value, max] = parts.as_slice() else {
            return None;
        };
        classify_core_type_reference_at_depth(key, depth + 1)?;
        classify_core_type_reference_at_depth(value, depth + 1)?;
        let max = max.strip_prefix("max=")?;
        let parsed_max: u64 = max.parse().ok()?;
        if parsed_max.to_string() != max {
            return None;
        }
        return Some(CoreType::Map {
            key: (*key).to_owned(),
            value: (*value).to_owned(),
            max: parsed_max,
        });
    }
    if let Some(item) = reference
        .strip_prefix("CapabilityRef<")
        .and_then(|value| value.strip_suffix('>'))
    {
        classify_core_type_reference_at_depth(item, depth + 1)?;
        return Some(CoreType::CapabilityRef {
            item: item.to_owned(),
        });
    }
    let settlement = reference
        .strip_prefix("edict.external-action.request/v1<")?
        .strip_suffix('>')?;
    classify_core_type_reference_at_depth(settlement, depth + 1)?;
    Some(CoreType::ExternalActionRequest {
        settlement: settlement.to_owned(),
    })
}

fn split_top_level_type_parts(value: &str) -> Option<Vec<&str>> {
    if value.is_empty() {
        return Some(Vec::new());
    }
    let mut depth = 0_usize;
    let mut start = 0_usize;
    let mut parts = Vec::new();
    for (index, character) in value.char_indices() {
        match character {
            '<' => depth = depth.checked_add(1)?,
            '>' => depth = depth.checked_sub(1)?,
            ',' if depth == 0 => {
                parts.push(&value[start..index]);
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    parts.push(&value[start..]);
    Some(parts)
}

fn is_core_field_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// Render an intrinsic or structural Core type as its canonical reference.
pub(crate) fn render_self_describing_core_type(ty: &CoreType) -> Option<String> {
    render_self_describing_core_type_at_depth(ty, 0)
}

fn render_self_describing_core_type_at_depth(ty: &CoreType, depth: usize) -> Option<String> {
    if depth > MAX_CORE_TYPE_DEPTH {
        return None;
    }
    match ty {
        CoreType::Bool => Some("Bool".to_owned()),
        CoreType::Unit => Some("Unit".to_owned()),
        CoreType::Int { width }
            if matches!(
                width.as_str(),
                "I8" | "I16" | "I32" | "I64" | "U8" | "U16" | "U32" | "U64"
            ) =>
        {
            Some(width.clone())
        }
        CoreType::String { max, canonical }
            if matches!(canonical.as_str(), "raw-utf8" | "unicode-scalar-nfc") =>
        {
            Some(format!("String<max={max},canonical={canonical}>"))
        }
        CoreType::Bytes { min, max } if min.is_none_or(|min| min <= *max) => match min {
            Some(min) if min == max => Some(format!("Bytes<exact={max}>")),
            Some(min) => Some(format!("Bytes<min={min},max={max}>")),
            None => Some(format!("Bytes<max={max}>")),
        },
        CoreType::Record { fields } => {
            let mut rendered = Vec::with_capacity(fields.len());
            for (name, ty) in fields {
                if !is_core_field_name(name) {
                    return None;
                }
                rendered.push(format!(
                    "{name}:{}",
                    canonical_core_type_reference_at_depth(ty, depth + 1)?
                ));
            }
            Some(format!("Record<{}>", rendered.join(",")))
        }
        CoreType::Option { item } => Some(format!(
            "Option<{}>",
            canonical_core_type_reference_at_depth(item, depth + 1)?
        )),
        CoreType::List { item, max } => Some(format!(
            "List<{},max={max}>",
            canonical_core_type_reference_at_depth(item, depth + 1)?
        )),
        CoreType::Map { key, value, max } => Some(format!(
            "Map<{},{},max={max}>",
            canonical_core_type_reference_at_depth(key, depth + 1)?,
            canonical_core_type_reference_at_depth(value, depth + 1)?
        )),
        CoreType::CapabilityRef { item } => Some(format!(
            "CapabilityRef<{}>",
            canonical_core_type_reference_at_depth(item, depth + 1)?
        )),
        CoreType::ExternalActionRequest { settlement } => Some(format!(
            "edict.external-action.request/v1<{}>",
            canonical_core_type_reference_at_depth(settlement, depth + 1)?
        )),
        CoreType::Int { .. }
        | CoreType::String { .. }
        | CoreType::Bytes { .. }
        | CoreType::Nominal { .. }
        | CoreType::Variant { .. } => None,
    }
}

fn canonical_core_type_reference_at_depth(reference: &str, depth: usize) -> Option<String> {
    match classify_core_type_reference_at_depth(reference, depth)? {
        CoreTypeReference::Intrinsic(ty) | CoreTypeReference::Structural(ty) => {
            render_self_describing_core_type_at_depth(&ty, depth)
        }
        CoreTypeReference::Named => Some(reference.to_owned()),
    }
}

/// Whether a Core type-table key is a valid named identity.
pub(crate) fn core_type_table_key_is_named(reference: &str) -> bool {
    matches!(
        classify_core_type_reference(reference),
        Some(CoreTypeReference::Named)
    )
}

pub(crate) fn core_type_fits(core: &CoreModule, source: &str, target: &str) -> bool {
    core_type_fits_at_depth(core, source, target, 0)
}

fn core_type_fits_at_depth(core: &CoreModule, source: &str, target: &str, depth: usize) -> bool {
    if depth > MAX_CORE_TYPE_DEPTH {
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
    match classify_core_type_reference(coordinate)? {
        CoreTypeReference::Intrinsic(ty) | CoreTypeReference::Structural(ty) => Some(ty),
        CoreTypeReference::Named => core
            .types
            .get(coordinate)
            .or_else(|| {
                coordinate
                    .strip_prefix(core.coordinate.as_str())
                    .and_then(|relative| relative.strip_prefix('.'))
                    .and_then(|relative| core.types.get(relative))
            })
            .cloned(),
    }
}

#[cfg(test)]
fn builtin_core_type(coordinate: &str) -> Option<CoreType> {
    match classify_core_type_reference(coordinate)? {
        CoreTypeReference::Intrinsic(ty) | CoreTypeReference::Structural(ty) => Some(ty),
        CoreTypeReference::Named => None,
    }
}

/// Reconstruct the exact reachable named type closure from signature roots.
///
/// Intrinsic and structural references are traversed transparently. Only named
/// definitions enter the returned closure, and every named coordinate must be
/// accepted by the caller-provided authority predicate.
pub(crate) fn named_core_type_closure<'a, I, F>(
    core: &CoreModule,
    roots: I,
    named_coordinate_allowed: F,
) -> Option<BTreeMap<String, CoreType>>
where
    I: IntoIterator<Item = &'a str>,
    F: Fn(&str) -> bool,
{
    let mut closure = BTreeMap::new();
    let mut visiting = std::collections::BTreeSet::new();
    for root in roots {
        collect_named_core_type_reference(
            core,
            root,
            &named_coordinate_allowed,
            &mut closure,
            &mut visiting,
            0,
        )?;
    }
    Some(closure)
}

fn collect_named_core_type_reference<F>(
    core: &CoreModule,
    reference: &str,
    named_coordinate_allowed: &F,
    closure: &mut BTreeMap<String, CoreType>,
    visiting: &mut std::collections::BTreeSet<String>,
    depth: usize,
) -> Option<()>
where
    F: Fn(&str) -> bool,
{
    if depth > MAX_CORE_TYPE_DEPTH {
        return None;
    }
    match classify_core_type_reference_at_depth(reference, depth)? {
        CoreTypeReference::Intrinsic(ty) | CoreTypeReference::Structural(ty) => {
            collect_named_core_type_children(
                core,
                &ty,
                named_coordinate_allowed,
                closure,
                visiting,
                depth,
            )
        }
        CoreTypeReference::Named => {
            if closure.contains_key(reference) {
                return Some(());
            }
            if !named_coordinate_allowed(reference) || !visiting.insert(reference.to_owned()) {
                return None;
            }
            let definition = core.types.get(reference)?.clone();
            if matches!(
                &definition,
                CoreType::Nominal { contract, .. } if contract != reference
            ) {
                return None;
            }
            collect_named_core_type_children(
                core,
                &definition,
                named_coordinate_allowed,
                closure,
                visiting,
                depth + 1,
            )?;
            visiting.remove(reference);
            closure.insert(reference.to_owned(), definition);
            Some(())
        }
    }
}

fn collect_named_core_type_children<F>(
    core: &CoreModule,
    ty: &CoreType,
    named_coordinate_allowed: &F,
    closure: &mut BTreeMap<String, CoreType>,
    visiting: &mut std::collections::BTreeSet<String>,
    depth: usize,
) -> Option<()>
where
    F: Fn(&str) -> bool,
{
    let mut collect = |reference: &str| {
        collect_named_core_type_reference(
            core,
            reference,
            named_coordinate_allowed,
            closure,
            visiting,
            depth + 1,
        )
    };
    match ty {
        CoreType::Bool
        | CoreType::Unit
        | CoreType::Int { .. }
        | CoreType::String { .. }
        | CoreType::Bytes { .. } => Some(()),
        CoreType::Nominal { representation, .. } => collect(representation),
        CoreType::Record { fields } => {
            for field in fields.values() {
                collect(field)?;
            }
            Some(())
        }
        CoreType::Variant { cases } => {
            for payload in cases.values().flatten() {
                collect(payload)?;
            }
            Some(())
        }
        CoreType::Option { item }
        | CoreType::List { item, .. }
        | CoreType::CapabilityRef { item } => collect(item),
        CoreType::Map { key, value, .. } => {
            collect(key)?;
            collect(value)
        }
        CoreType::ExternalActionRequest { settlement } => collect(settlement),
    }
}

/// Validate every Core type definition and graph-carried type reference.
///
/// This judgment is deliberately eager: valid unused named definitions remain
/// legal and hash-significant, while malformed unused definitions still reject
/// because they are part of the module's semantic preimage.
///
/// # Errors
///
/// Returns one deterministic structured failure for the first invalid table
/// key, definition, reference, cycle, or depth boundary in canonical map and
/// source order.
pub fn validate_core_module_type_integrity(
    module: &CoreModule,
) -> Result<ValidatedCoreModule<'_>, CoreTypeIntegrityFailure> {
    for key in module.types.keys() {
        if !core_type_table_key_is_named(key) {
            return Err(CoreTypeIntegrityFailure::new(
                CoreTypeIntegrityFailureKind::InvalidTableKey,
                format!("types.{key}"),
            ));
        }
    }

    let mut state = CoreTypeIntegrityState::default();
    for key in module.types.keys() {
        validate_named_core_type_definition(module, &mut state, key, &format!("types.{key}"), 0)?;
    }
    for (intent_name, intent) in &module.intents {
        validate_core_intent_types(
            module,
            &mut state,
            intent,
            &format!("intents.{intent_name}"),
        )?;
    }

    Ok(ValidatedCoreModule { module })
}

#[derive(Default)]
struct CoreTypeIntegrityState {
    validated_named: BTreeSet<String>,
    visiting_named: BTreeSet<String>,
}

fn validate_named_core_type_definition(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    table_key: &str,
    path: &str,
    depth: usize,
) -> Result<(), CoreTypeIntegrityFailure> {
    if depth > MAX_CORE_TYPE_DEPTH {
        return Err(CoreTypeIntegrityFailure::new(
            CoreTypeIntegrityFailureKind::DepthExceeded,
            path,
        ));
    }
    if state.validated_named.contains(table_key) {
        return Ok(());
    }
    if !state.visiting_named.insert(table_key.to_owned()) {
        return Err(CoreTypeIntegrityFailure::new(
            CoreTypeIntegrityFailureKind::ReferenceCycle,
            path,
        ));
    }
    let definition = module.types.get(table_key).ok_or_else(|| {
        CoreTypeIntegrityFailure::new(CoreTypeIntegrityFailureKind::UnresolvedNamedReference, path)
    })?;
    validate_core_type_definition(module, state, Some(table_key), definition, path, depth)?;
    state.visiting_named.remove(table_key);
    state.validated_named.insert(table_key.to_owned());
    Ok(())
}

fn validate_core_type_definition(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    table_key: Option<&str>,
    definition: &CoreType,
    path: &str,
    depth: usize,
) -> Result<(), CoreTypeIntegrityFailure> {
    if depth > MAX_CORE_TYPE_DEPTH {
        return Err(CoreTypeIntegrityFailure::new(
            CoreTypeIntegrityFailureKind::DepthExceeded,
            path,
        ));
    }
    match definition {
        CoreType::Bool | CoreType::Unit => Ok(()),
        CoreType::Int { width }
            if matches!(
                width.as_str(),
                "I8" | "I16" | "I32" | "I64" | "U8" | "U16" | "U32" | "U64"
            ) =>
        {
            Ok(())
        }
        CoreType::Int { .. } => Err(CoreTypeIntegrityFailure::new(
            CoreTypeIntegrityFailureKind::InvalidDefinition,
            format!("{path}.width"),
        )),
        CoreType::String { canonical, .. }
            if matches!(canonical.as_str(), "raw-utf8" | "unicode-scalar-nfc") =>
        {
            Ok(())
        }
        CoreType::String { .. } => Err(CoreTypeIntegrityFailure::new(
            CoreTypeIntegrityFailureKind::InvalidDefinition,
            format!("{path}.canonical"),
        )),
        CoreType::Bytes { min, max } if min.is_none_or(|min| min <= *max) => Ok(()),
        CoreType::Bytes { .. } => Err(CoreTypeIntegrityFailure::new(
            CoreTypeIntegrityFailureKind::InvalidDefinition,
            format!("{path}.bounds"),
        )),
        CoreType::Nominal {
            contract,
            representation,
        } => {
            if table_key != Some(contract.as_str()) {
                return Err(CoreTypeIntegrityFailure::new(
                    CoreTypeIntegrityFailureKind::NominalContractMismatch,
                    format!("{path}.contract"),
                ));
            }
            validate_core_type_reference(
                module,
                state,
                representation,
                &format!("{path}.representation"),
                depth + 1,
            )
        }
        CoreType::Record { fields } => {
            validate_core_record_definition(module, state, fields, path, depth)
        }
        CoreType::Variant { cases } => {
            validate_core_variant_definition(module, state, cases, path, depth)
        }
        CoreType::Option { item }
        | CoreType::List { item, .. }
        | CoreType::CapabilityRef { item } => {
            validate_core_type_reference(module, state, item, &format!("{path}.item"), depth + 1)
        }
        CoreType::Map { key, value, .. } => {
            validate_core_type_reference(module, state, key, &format!("{path}.key"), depth + 1)?;
            validate_core_type_reference(module, state, value, &format!("{path}.value"), depth + 1)
        }
        CoreType::ExternalActionRequest { settlement } => validate_core_type_reference(
            module,
            state,
            settlement,
            &format!("{path}.settlement"),
            depth + 1,
        ),
    }
}

fn validate_core_record_definition(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    fields: &BTreeMap<String, String>,
    path: &str,
    depth: usize,
) -> Result<(), CoreTypeIntegrityFailure> {
    for (field, reference) in fields {
        if !is_core_field_name(field) {
            return Err(CoreTypeIntegrityFailure::new(
                CoreTypeIntegrityFailureKind::InvalidDefinition,
                format!("{path}.fields.{field}"),
            ));
        }
        validate_core_type_reference(
            module,
            state,
            reference,
            &format!("{path}.fields.{field}"),
            depth + 1,
        )?;
    }
    Ok(())
}

fn validate_core_variant_definition(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    cases: &BTreeMap<String, Option<String>>,
    path: &str,
    depth: usize,
) -> Result<(), CoreTypeIntegrityFailure> {
    if cases.is_empty() {
        return Err(CoreTypeIntegrityFailure::new(
            CoreTypeIntegrityFailureKind::InvalidDefinition,
            format!("{path}.cases"),
        ));
    }
    for (case, payload) in cases {
        if let Some(reference) = payload {
            validate_core_type_reference(
                module,
                state,
                reference,
                &format!("{path}.cases.{case}.payload"),
                depth + 1,
            )?;
        }
    }
    Ok(())
}

fn validate_core_type_reference(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    reference: &str,
    path: &str,
    depth: usize,
) -> Result<(), CoreTypeIntegrityFailure> {
    if depth > MAX_CORE_TYPE_DEPTH {
        return Err(CoreTypeIntegrityFailure::new(
            CoreTypeIntegrityFailureKind::DepthExceeded,
            path,
        ));
    }
    let Some(classification) = classify_core_type_reference_at_depth(reference, depth) else {
        let kind = if structural_reference_exceeds_depth(reference, depth) {
            CoreTypeIntegrityFailureKind::DepthExceeded
        } else {
            CoreTypeIntegrityFailureKind::InvalidReference
        };
        return Err(CoreTypeIntegrityFailure::new(kind, path));
    };
    match classification {
        CoreTypeReference::Intrinsic(definition) | CoreTypeReference::Structural(definition) => {
            validate_core_type_definition(module, state, None, &definition, path, depth)
        }
        CoreTypeReference::Named => {
            let table_key =
                resolved_named_core_type_table_key(module, reference).ok_or_else(|| {
                    CoreTypeIntegrityFailure::new(
                        CoreTypeIntegrityFailureKind::UnresolvedNamedReference,
                        path,
                    )
                })?;
            validate_named_core_type_definition(module, state, table_key, path, depth)
        }
    }
}

fn structural_reference_exceeds_depth(reference: &str, starting_depth: usize) -> bool {
    let mut depth = starting_depth;
    for character in reference.chars() {
        match character {
            '<' => {
                let Some(next) = depth.checked_add(1) else {
                    return true;
                };
                depth = next;
                if depth > MAX_CORE_TYPE_DEPTH {
                    return true;
                }
            }
            '>' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

fn resolved_named_core_type_table_key<'a>(
    module: &'a CoreModule,
    reference: &str,
) -> Option<&'a str> {
    module
        .types
        .get_key_value(reference)
        .map(|(key, _)| key.as_str())
        .or_else(|| {
            reference
                .strip_prefix(module.coordinate.as_str())
                .and_then(|relative| relative.strip_prefix('.'))
                .and_then(|relative| module.types.get_key_value(relative))
                .map(|(key, _)| key.as_str())
        })
}

fn validate_core_intent_types(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    intent: &CoreIntent,
    path: &str,
) -> Result<(), CoreTypeIntegrityFailure> {
    validate_core_type_reference(module, state, &intent.input, &format!("{path}.input"), 0)?;
    validate_core_type_reference(module, state, &intent.output, &format!("{path}.output"), 0)?;
    if let Some(basis) = &intent.basis {
        validate_core_expression_types(module, state, basis, &format!("{path}.basis"))?;
    }
    for (index, constraint) in intent.input_constraints.iter().enumerate() {
        validate_core_predicate_types(
            module,
            state,
            &constraint.predicate,
            &format!("{path}.inputConstraints[{index}].predicate"),
        )?;
    }
    validate_core_block_types(module, state, &intent.body, &format!("{path}.body"))
}

fn validate_core_block_types(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    block: &CoreBlock,
    path: &str,
) -> Result<(), CoreTypeIntegrityFailure> {
    for (index, local) in block.locals.iter().enumerate() {
        validate_core_local_type(
            module,
            state,
            local,
            &format!("{path}.locals[{index}].type"),
        )?;
    }
    for (index, node) in block.nodes.iter().enumerate() {
        validate_core_node_types(module, state, node, &format!("{path}.nodes[{index}]"))?;
    }
    validate_core_expression_types(module, state, &block.result, &format!("{path}.result"))
}

fn validate_core_local_type(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    local: &LocalRef,
    path: &str,
) -> Result<(), CoreTypeIntegrityFailure> {
    validate_core_type_reference(module, state, &local.ty, path, 0)
}

fn validate_core_node_types(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    node: &CoreNode,
    path: &str,
) -> Result<(), CoreTypeIntegrityFailure> {
    match node {
        CoreNode::Let { binding, value } => {
            validate_core_local_type(module, state, binding, &format!("{path}.let.binding.type"))?;
            validate_core_expression_types(module, state, value, &format!("{path}.let.value"))
        }
        CoreNode::Require { predicate, arm } => {
            validate_core_predicate_types(
                module,
                state,
                predicate,
                &format!("{path}.require.predicate"),
            )?;
            let reason = match arm {
                CoreRequireFailureArm::Terminal { reason }
                | CoreRequireFailureArm::ContinueObstructed { reason } => reason,
            };
            validate_core_obstruction_reason_types(
                module,
                state,
                reason,
                &format!("{path}.require.reason"),
            )
        }
        CoreNode::Effect {
            binding,
            input,
            obstruction_map,
            ..
        } => {
            validate_core_local_type(
                module,
                state,
                binding,
                &format!("{path}.effect.binding.type"),
            )?;
            validate_core_expression_types(module, state, input, &format!("{path}.effect.input"))?;
            for (failure, arm) in obstruction_map {
                validate_core_local_type(
                    module,
                    state,
                    &arm.binder,
                    &format!("{path}.effect.obstructionMap.{failure}.binder.type"),
                )?;
                validate_core_expression_types(
                    module,
                    state,
                    &arm.value,
                    &format!("{path}.effect.obstructionMap.{failure}.value"),
                )?;
            }
            Ok(())
        }
        CoreNode::ExternalActionRequest { .. } => {
            validate_external_action_request_types(module, state, node, path)
        }
        CoreNode::For {
            binder, iter, body, ..
        } => {
            validate_core_local_type(module, state, binder, &format!("{path}.for.binder.type"))?;
            validate_core_expression_types(module, state, iter, &format!("{path}.for.iter"))?;
            validate_core_block_types(module, state, body, &format!("{path}.for.body"))
        }
        CoreNode::Branch {
            binding,
            predicate,
            then_block,
            else_block,
        } => {
            if let Some(binding) = binding {
                validate_core_local_type(
                    module,
                    state,
                    binding,
                    &format!("{path}.branch.binding.type"),
                )?;
            }
            validate_core_predicate_types(
                module,
                state,
                predicate,
                &format!("{path}.branch.predicate"),
            )?;
            validate_core_block_types(module, state, then_block, &format!("{path}.branch.then"))?;
            validate_core_block_types(module, state, else_block, &format!("{path}.branch.else"))
        }
    }
}

fn validate_external_action_request_types(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    node: &CoreNode,
    path: &str,
) -> Result<(), CoreTypeIntegrityFailure> {
    let CoreNode::ExternalActionRequest {
        binding,
        input_type,
        settlement_type,
        input,
        authority_scope,
        basis,
        budget,
        ..
    } = node
    else {
        unreachable!("caller selects the external-action request variant");
    };
    let request_path = format!("{path}.externalActionRequest");
    validate_core_local_type(
        module,
        state,
        binding,
        &format!("{request_path}.binding.type"),
    )?;
    validate_core_type_reference(
        module,
        state,
        input_type,
        &format!("{request_path}.inputType"),
        0,
    )?;
    validate_core_type_reference(
        module,
        state,
        settlement_type,
        &format!("{request_path}.settlementType"),
        0,
    )?;
    for (name, expression) in [
        ("input", input),
        ("authorityScope", authority_scope),
        ("basis", basis),
        ("budget.maxSettlementBytes", &budget.max_settlement_bytes),
        ("budget.maxAttempts", &budget.max_attempts),
    ] {
        validate_core_expression_types(
            module,
            state,
            expression,
            &format!("{request_path}.{name}"),
        )?;
    }
    Ok(())
}

fn validate_core_obstruction_reason_types(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    reason: &CoreObstructionReason,
    path: &str,
) -> Result<(), CoreTypeIntegrityFailure> {
    for (field, value) in &reason.payload {
        validate_core_expression_types(module, state, value, &format!("{path}.payload.{field}"))?;
    }
    Ok(())
}

fn validate_core_expression_types(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    expression: &CoreExpr,
    path: &str,
) -> Result<(), CoreTypeIntegrityFailure> {
    match expression {
        CoreExpr::Local { reference } => {
            validate_core_local_type(module, state, reference, &format!("{path}.local.type"))
        }
        CoreExpr::Const(_) => Ok(()),
        CoreExpr::Record { fields } => {
            for (field, value) in fields {
                validate_core_expression_types(
                    module,
                    state,
                    value,
                    &format!("{path}.record.{field}"),
                )?;
            }
            Ok(())
        }
        CoreExpr::Field { base, .. } => {
            validate_core_expression_types(module, state, base, &format!("{path}.field.base"))
        }
        CoreExpr::Call {
            type_args, args, ..
        } => {
            for (index, reference) in type_args.iter().enumerate() {
                validate_core_type_reference(
                    module,
                    state,
                    reference,
                    &format!("{path}.call.typeArgs[{index}]"),
                    0,
                )?;
            }
            for (index, argument) in args.iter().enumerate() {
                validate_core_expression_types(
                    module,
                    state,
                    argument,
                    &format!("{path}.call.args[{index}]"),
                )?;
            }
            Ok(())
        }
        CoreExpr::If {
            predicate,
            then_value,
            else_value,
        } => {
            validate_core_predicate_types(
                module,
                state,
                predicate,
                &format!("{path}.if.predicate"),
            )?;
            validate_core_expression_types(module, state, then_value, &format!("{path}.if.then"))?;
            validate_core_expression_types(module, state, else_value, &format!("{path}.if.else"))
        }
    }
}

fn validate_core_predicate_types(
    module: &CoreModule,
    state: &mut CoreTypeIntegrityState,
    predicate: &CorePredicate,
    path: &str,
) -> Result<(), CoreTypeIntegrityFailure> {
    match predicate {
        CorePredicate::True | CorePredicate::False => Ok(()),
        CorePredicate::Not(value) => {
            validate_core_predicate_types(module, state, value, &format!("{path}.not"))
        }
        CorePredicate::All(values) | CorePredicate::Any(values) => {
            for (index, value) in values.iter().enumerate() {
                validate_core_predicate_types(
                    module,
                    state,
                    value,
                    &format!("{path}.values[{index}]"),
                )?;
            }
            Ok(())
        }
        CorePredicate::Compare { left, right, .. } => {
            validate_core_expression_types(module, state, left, &format!("{path}.compare.left"))?;
            validate_core_expression_types(module, state, right, &format!("{path}.compare.right"))
        }
    }
}

fn scalar_types_fit(left: &CoreType, right: &CoreType) -> Option<bool> {
    match (left, right) {
        (CoreType::Bool, CoreType::Bool) | (CoreType::Unit, CoreType::Unit) => Some(true),
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

#[cfg(test)]
mod structural_type_reference_tests {
    use super::{
        builtin_core_type, render_self_describing_core_type, validate_core_module_type_integrity,
        CoreModule, CoreType, CoreTypeIntegrityFailureKind, CORE_API_VERSION, MAX_CORE_TYPE_DEPTH,
    };
    use std::collections::BTreeMap;

    fn module_with_types(types: BTreeMap<String, CoreType>) -> CoreModule {
        CoreModule {
            api_version: CORE_API_VERSION.to_owned(),
            coordinate: "integrity.test@1".to_owned(),
            imports: Vec::new(),
            types,
            intents: BTreeMap::new(),
            required_core_capabilities: Vec::new(),
        }
    }

    #[test]
    fn canonical_structural_type_references_round_trip() {
        let cases = [
            ("Unit", CoreType::Unit),
            (
                "String<max=7,canonical=unicode-scalar-nfc>",
                CoreType::String {
                    max: 7,
                    canonical: "unicode-scalar-nfc".to_owned(),
                },
            ),
            (
                "Bytes<min=1,max=3>",
                CoreType::Bytes {
                    min: Some(1),
                    max: 3,
                },
            ),
            (
                "Record<left:String<max=3,canonical=raw-utf8>,right:U64>",
                CoreType::Record {
                    fields: BTreeMap::from([
                        (
                            "left".to_owned(),
                            "String<max=3,canonical=raw-utf8>".to_owned(),
                        ),
                        ("right".to_owned(), "U64".to_owned()),
                    ]),
                },
            ),
            (
                "List<Record<inner:Record<value:U64>,tail:Bool>,max=3>",
                CoreType::List {
                    item: "Record<inner:Record<value:U64>,tail:Bool>".to_owned(),
                    max: 3,
                },
            ),
            (
                "Option<Record<value:U64>>",
                CoreType::Option {
                    item: "Record<value:U64>".to_owned(),
                },
            ),
            (
                "Map<String<max=2,canonical=raw-utf8>,CapabilityRef<U64>,max=4>",
                CoreType::Map {
                    key: "String<max=2,canonical=raw-utf8>".to_owned(),
                    value: "CapabilityRef<U64>".to_owned(),
                    max: 4,
                },
            ),
            (
                "edict.external-action.request/v1<Option<Record<value:U64>>>",
                CoreType::ExternalActionRequest {
                    settlement: "Option<Record<value:U64>>".to_owned(),
                },
            ),
        ];
        for (reference, expected) in cases {
            assert_eq!(
                builtin_core_type(reference),
                Some(expected.clone()),
                "{reference}"
            );
            assert_eq!(
                render_self_describing_core_type(&expected).as_deref(),
                Some(reference),
                "{reference}"
            );
        }

        for malformed in [
            "Record<right:U64,left:Bool>",
            "Record<value:U64,value:Bool>",
            "Record<value:>",
            "Record< value:U64>",
            "Record<value:U64",
            "Record<value:String<max=03,canonical=raw-utf8>>",
            "Bytes<min=3,max=3>",
        ] {
            assert_eq!(builtin_core_type(malformed), None, "{malformed}");
        }

        let at_depth = (0..MAX_CORE_TYPE_DEPTH).fold("U64".to_owned(), |inner, _| {
            format!("Record<value:{inner}>")
        });
        assert_eq!(
            builtin_core_type(&at_depth)
                .as_ref()
                .and_then(render_self_describing_core_type)
                .as_deref(),
            Some(at_depth.as_str())
        );

        let over_depth = format!("Record<value:{at_depth}>");
        assert_eq!(builtin_core_type(&over_depth), None);
    }

    #[test]
    fn whole_module_type_integrity_reports_stable_kinds_and_paths() {
        let over_depth =
            (0..=MAX_CORE_TYPE_DEPTH).fold("U64".to_owned(), |inner, _| format!("Option<{inner}>"));
        let mut cycle = BTreeMap::new();
        cycle.insert(
            "CycleA".to_owned(),
            CoreType::Option {
                item: "CycleB".to_owned(),
            },
        );
        cycle.insert(
            "CycleB".to_owned(),
            CoreType::Option {
                item: "CycleA".to_owned(),
            },
        );
        let cases = [
            (
                "table key",
                module_with_types(BTreeMap::from([("Bool".to_owned(), CoreType::Bool)])),
                CoreTypeIntegrityFailureKind::InvalidTableKey,
                "types.Bool",
            ),
            (
                "definition",
                module_with_types(BTreeMap::from([(
                    "Wide".to_owned(),
                    CoreType::Int {
                        width: "I128".to_owned(),
                    },
                )])),
                CoreTypeIntegrityFailureKind::InvalidDefinition,
                "types.Wide.width",
            ),
            (
                "reference",
                module_with_types(BTreeMap::from([(
                    "Bad".to_owned(),
                    CoreType::Option {
                        item: "List<U64,max=01>".to_owned(),
                    },
                )])),
                CoreTypeIntegrityFailureKind::InvalidReference,
                "types.Bad.item",
            ),
            (
                "unresolved",
                module_with_types(BTreeMap::from([(
                    "Bad".to_owned(),
                    CoreType::Option {
                        item: "Missing".to_owned(),
                    },
                )])),
                CoreTypeIntegrityFailureKind::UnresolvedNamedReference,
                "types.Bad.item",
            ),
            (
                "nominal",
                module_with_types(BTreeMap::from([(
                    "Handle".to_owned(),
                    CoreType::Nominal {
                        contract: "Other".to_owned(),
                        representation: "U64".to_owned(),
                    },
                )])),
                CoreTypeIntegrityFailureKind::NominalContractMismatch,
                "types.Handle.contract",
            ),
            (
                "cycle",
                module_with_types(cycle),
                CoreTypeIntegrityFailureKind::ReferenceCycle,
                "types.CycleA.item.item",
            ),
            (
                "depth",
                module_with_types(BTreeMap::from([(
                    "Deep".to_owned(),
                    CoreType::Option { item: over_depth },
                )])),
                CoreTypeIntegrityFailureKind::DepthExceeded,
                "types.Deep.item",
            ),
        ];

        for (case, module, expected_kind, expected_path) in cases {
            let failure = validate_core_module_type_integrity(&module)
                .expect_err("invalid module must not mint a witness");
            assert_eq!(failure.kind(), expected_kind, "{case}");
            assert_eq!(failure.path(), expected_path, "{case}");
        }
    }

    #[test]
    fn valid_unused_named_definition_mints_a_borrowed_witness() {
        let module = module_with_types(BTreeMap::from([(
            "Unused".to_owned(),
            CoreType::List {
                item: "U64".to_owned(),
                max: 4,
            },
        )]));

        let validated = validate_core_module_type_integrity(&module)
            .expect("valid unused authored definition remains legal");
        assert!(std::ptr::eq(validated.module(), &raw const module));
    }
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
