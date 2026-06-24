//! Canonical CBOR encoding for Edict Core IR.
//!
//! This module implements the `edict.canonical-cbor/v1` subset needed by the
//! current in-memory Core model. It emits deterministic bytes and can validate
//! those bytes by decoding to a canonical value and re-encoding. It deliberately
//! does not compute Core digests or provide reviewed golden fixtures.

use std::collections::BTreeSet;
use std::fmt;
use std::str;

use crate::core_ir::{
    CompareOp, CoreBlock, CoreBudget, CoreExpr, CoreImport, CoreIntent, CoreModule, CoreNode,
    CorePredicate, CoreType, CoreValue, InputConstraint, InputConstraintSource, LocalRef,
    ResourceRef,
};

/// Canonical encoding profile for Core artifacts.
pub const CORE_CANONICAL_ENCODING: &str = "edict.canonical-cbor/v1";

/// Stable canonical encoding error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalErrorKind {
    /// A Core value cannot be represented in the supported canonical CBOR subset.
    UnsupportedValue,
    /// A numeric Core value is outside the supported integer range or malformed.
    InvalidInteger,
    /// A digest-required Core reference has not been resolved.
    UnresolvedDigest,
    /// A digest string is not the expected review rendering.
    InvalidDigest,
    /// The byte stream ended before a complete value was decoded.
    UnexpectedEof,
    /// The byte stream contained extra data after one complete value.
    TrailingData,
    /// The byte stream uses a CBOR major type or additional-info form we reject.
    UnsupportedCbor,
    /// The byte stream decodes but is not in canonical form.
    NonCanonical,
    /// A decoded CBOR map contains duplicate keys.
    DuplicateMapKey,
    /// A decoded text string is not valid UTF-8.
    InvalidUtf8,
}

/// Canonical encoding failure with stable kind plus detail for diagnostics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalError {
    kind: CanonicalErrorKind,
    message: String,
}

impl CanonicalError {
    fn new(kind: CanonicalErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Return the stable error category.
    #[must_use]
    pub const fn kind(&self) -> CanonicalErrorKind {
        self.kind
    }
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for CanonicalError {}

/// Decoded canonical CBOR value tree.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalValue {
    /// CBOR null.
    Null,
    /// CBOR boolean.
    Bool(bool),
    /// CBOR integer, represented across positive and negative major types.
    Integer(i128),
    /// CBOR byte string.
    Bytes(Vec<u8>),
    /// CBOR UTF-8 text string.
    Text(String),
    /// CBOR array.
    Array(Vec<CanonicalValue>),
    /// CBOR map. Encoding sorts keys by their canonical encoded bytes.
    Map(Vec<(CanonicalValue, CanonicalValue)>),
}

/// Encode a Core module as `edict.canonical-cbor/v1`.
///
/// # Errors
///
/// Returns an error if a Core integer or digest cannot be represented in the
/// supported canonical form.
pub fn encode_core_module(module: &CoreModule) -> Result<Vec<u8>, CanonicalError> {
    encode_canonical_cbor(&core_module_value(module)?)
}

/// Encode a decoded canonical value to canonical CBOR bytes.
///
/// # Errors
///
/// Returns an error if an integer is outside the supported CBOR range.
pub fn encode_canonical_cbor(value: &CanonicalValue) -> Result<Vec<u8>, CanonicalError> {
    let mut out = Vec::new();
    encode_value(value, &mut out)?;
    Ok(out)
}

/// Decode and validate canonical CBOR bytes.
///
/// The decoder accepts only the definite-length subset used by Edict Core. After
/// decoding, it re-encodes the value and requires an exact byte match, rejecting
/// non-minimal integers, unsorted maps, and other non-canonical encodings.
///
/// # Errors
///
/// Returns a stable [`CanonicalErrorKind`] for malformed, unsupported, or
/// non-canonical bytes.
pub fn decode_canonical_cbor(bytes: &[u8]) -> Result<CanonicalValue, CanonicalError> {
    let mut decoder = Decoder::new(bytes);
    let value = decoder.value()?;
    if decoder.remaining() != 0 {
        return Err(CanonicalError::new(
            CanonicalErrorKind::TrailingData,
            "canonical CBOR stream has trailing bytes",
        ));
    }
    let reencoded = encode_canonical_cbor(&value)?;
    if reencoded != bytes {
        return Err(CanonicalError::new(
            CanonicalErrorKind::NonCanonical,
            "decoded value does not re-encode to identical canonical bytes",
        ));
    }
    Ok(value)
}

fn core_module_value(module: &CoreModule) -> Result<CanonicalValue, CanonicalError> {
    Ok(map([
        ("apiVersion", text(&module.api_version)),
        ("coordinate", text(&module.coordinate)),
        (
            "imports",
            array_results(module.imports.iter().map(core_import_value))?,
        ),
        (
            "types",
            string_map_results(
                module
                    .types
                    .iter()
                    .map(|(name, ty)| Ok((name.as_str(), core_type_value(ty)))),
            )?,
        ),
        (
            "intents",
            string_map_results(
                module
                    .intents
                    .iter()
                    .map(|(name, intent)| Ok((name.as_str(), core_intent_value(intent)?))),
            )?,
        ),
        (
            "requiredCoreCapabilities",
            array(
                module
                    .required_core_capabilities
                    .iter()
                    .map(String::as_str)
                    .map(text),
            ),
        ),
    ]))
}

fn core_import_value(import: &CoreImport) -> Result<CanonicalValue, CanonicalError> {
    let mut entries = vec![
        ("kind", text(import.kind.as_str())),
        ("ref", resource_ref_value(&import.resource)?),
    ];
    if let Some(alias) = &import.alias {
        entries.push(("alias", text(alias)));
    }
    Ok(map(entries))
}

fn resource_ref_value(resource: &ResourceRef) -> Result<CanonicalValue, CanonicalError> {
    let mut entries = vec![("id", text(&resource.coordinate))];
    let Some(digest) = &resource.digest else {
        return Err(CanonicalError::new(
            CanonicalErrorKind::UnresolvedDigest,
            "Core import resource digest is unresolved",
        ));
    };
    entries.push(("digest", digest_value(digest)?));
    Ok(map(entries))
}

fn digest_value(digest: &str) -> Result<CanonicalValue, CanonicalError> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(CanonicalError::new(
            CanonicalErrorKind::InvalidDigest,
            "digest must use sha256 review rendering",
        ));
    };
    if hex.len() != 64 {
        return Err(CanonicalError::new(
            CanonicalErrorKind::InvalidDigest,
            "sha256 digest must contain 64 hex characters",
        ));
    }
    let mut bytes = Vec::with_capacity(32);
    for chunk in hex.as_bytes().chunks_exact(2) {
        let hi = hex_value(chunk[0])?;
        let lo = hex_value(chunk[1])?;
        bytes.push((hi << 4) | lo);
    }
    Ok(CanonicalValue::Array(vec![
        text("sha256"),
        CanonicalValue::Bytes(bytes),
    ]))
}

fn hex_value(byte: u8) -> Result<u8, CanonicalError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        _ => Err(CanonicalError::new(
            CanonicalErrorKind::InvalidDigest,
            "sha256 digest must be lowercase hex",
        )),
    }
}

fn core_type_value(ty: &CoreType) -> CanonicalValue {
    match ty {
        CoreType::Bool => map([("kind", text("Bool"))]),
        CoreType::Int { width } => map([("kind", text(width))]),
        CoreType::String { max, canonical } => map([
            ("kind", text("String")),
            ("max", uint(*max)),
            ("canonical", text(canonical)),
        ]),
        CoreType::Bytes { max } => map([("kind", text("Bytes")), ("max", uint(*max))]),
        CoreType::Record { fields } => map([
            ("kind", text("Record")),
            (
                "fields",
                string_map(fields.iter().map(|(name, ty)| (name.as_str(), text(ty)))),
            ),
        ]),
        CoreType::Variant { cases } => map([
            ("kind", text("Variant")),
            (
                "cases",
                string_map(cases.iter().map(|(name, payload)| {
                    let body = if let Some(payload) = payload {
                        map([("payload", text(payload))])
                    } else {
                        CanonicalValue::Map(Vec::new())
                    };
                    (name.as_str(), body)
                })),
            ),
        ]),
        CoreType::Option { item } => map([("kind", text("Option")), ("item", text(item))]),
        CoreType::List { item, max } => map([
            ("kind", text("List")),
            ("item", text(item)),
            ("max", uint(*max)),
        ]),
        CoreType::Map { key, value, max } => map([
            ("kind", text("Map")),
            ("key", text(key)),
            ("value", text(value)),
            ("max", uint(*max)),
        ]),
        CoreType::CapabilityRef { item } => {
            map([("kind", text("CapabilityRef")), ("item", text(item))])
        }
    }
}

fn core_intent_value(intent: &CoreIntent) -> Result<CanonicalValue, CanonicalError> {
    Ok(map([
        ("input", text(&intent.input)),
        ("output", text(&intent.output)),
        (
            "requiredOperationProfile",
            text(&intent.required_operation_profile),
        ),
        (
            "inputConstraints",
            array_results(intent.input_constraints.iter().map(input_constraint_value))?,
        ),
        (
            "coreEvaluationBudget",
            core_budget_value(&intent.core_evaluation_budget),
        ),
        ("body", core_block_value(&intent.body)?),
    ]))
}

fn input_constraint_value(constraint: &InputConstraint) -> Result<CanonicalValue, CanonicalError> {
    Ok(map([
        ("coordinate", text(&constraint.coordinate)),
        (
            "source",
            text(input_constraint_source_str(constraint.source)),
        ),
        ("predicate", core_predicate_value(&constraint.predicate)?),
    ]))
}

const fn input_constraint_source_str(source: InputConstraintSource) -> &'static str {
    match source {
        InputConstraintSource::Where => "where",
        InputConstraintSource::Compiler => "compiler",
    }
}

fn core_budget_value(budget: &CoreBudget) -> CanonicalValue {
    map([
        ("maxSteps", uint(budget.max_steps)),
        ("maxAllocatedBytes", uint(budget.max_allocated_bytes)),
        ("maxOutputBytes", uint(budget.max_output_bytes)),
    ])
}

fn core_block_value(block: &CoreBlock) -> Result<CanonicalValue, CanonicalError> {
    Ok(map([
        ("locals", array(block.locals.iter().map(local_ref_value))),
        (
            "nodes",
            array_results(block.nodes.iter().map(core_node_value))?,
        ),
        ("result", core_expr_value(&block.result)?),
    ]))
}

fn core_node_value(node: &CoreNode) -> Result<CanonicalValue, CanonicalError> {
    match node {
        CoreNode::Let { binding, value } => Ok(map([
            ("kind", text("let")),
            ("binding", local_ref_value(binding)),
            ("value", core_expr_value(value)?),
        ])),
    }
}

fn core_expr_value(expr: &CoreExpr) -> Result<CanonicalValue, CanonicalError> {
    Ok(match expr {
        CoreExpr::Local { reference } => {
            map([("kind", text("local")), ("ref", local_ref_value(reference))])
        }
        CoreExpr::Const(value) => map([("kind", text("const")), ("value", core_value(value)?)]),
        CoreExpr::Record { fields } => map([
            ("kind", text("record")),
            (
                "fields",
                string_map_results(
                    fields
                        .iter()
                        .map(|(name, expr)| Ok((name.as_str(), core_expr_value(expr)?))),
                )?,
            ),
        ]),
        CoreExpr::Field { base, field } => map([
            ("kind", text("field")),
            ("base", core_expr_value(base)?),
            ("field", text(field)),
        ]),
        CoreExpr::Call {
            callee,
            type_args,
            args,
        } => map([
            ("kind", text("call")),
            ("callee", text(callee)),
            (
                "typeArgs",
                array(type_args.iter().map(String::as_str).map(text)),
            ),
            ("args", array_results(args.iter().map(core_expr_value))?),
        ]),
    })
}

fn core_value(value: &CoreValue) -> Result<CanonicalValue, CanonicalError> {
    Ok(match value {
        CoreValue::Null => map([("kind", text("null"))]),
        CoreValue::Bool(value) => map([("kind", text("bool")), ("value", bool_value(*value))]),
        CoreValue::Int { width, value } => map([
            ("kind", text("int")),
            ("width", text(width)),
            ("value", int_text_value(value)?),
        ]),
        CoreValue::String(value) => map([("kind", text("string")), ("value", text(value))]),
        CoreValue::Bytes(value) => map([
            ("kind", text("bytes")),
            ("value", CanonicalValue::Bytes(value.clone())),
        ]),
    })
}

fn int_text_value(value: &str) -> Result<CanonicalValue, CanonicalError> {
    let value = value.parse::<i128>().map_err(|_| {
        CanonicalError::new(
            CanonicalErrorKind::InvalidInteger,
            "Core integer value is not a base-10 integer",
        )
    })?;
    Ok(CanonicalValue::Integer(value))
}

fn core_predicate_value(predicate: &CorePredicate) -> Result<CanonicalValue, CanonicalError> {
    Ok(match predicate {
        CorePredicate::True => map([("kind", text("true"))]),
        CorePredicate::False => map([("kind", text("false"))]),
        CorePredicate::Not(value) => map([
            ("kind", text("not")),
            ("value", core_predicate_value(value)?),
        ]),
        CorePredicate::All(values) => map([
            ("kind", text("all")),
            (
                "values",
                array_results(values.iter().map(core_predicate_value))?,
            ),
        ]),
        CorePredicate::Any(values) => map([
            ("kind", text("any")),
            (
                "values",
                array_results(values.iter().map(core_predicate_value))?,
            ),
        ]),
        CorePredicate::Compare { op, left, right } => map([
            ("kind", text("compare")),
            ("op", text(compare_op_str(*op))),
            ("left", core_expr_value(left)?),
            ("right", core_expr_value(right)?),
        ]),
    })
}

const fn compare_op_str(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "==",
        CompareOp::Ne => "!=",
        CompareOp::Lt => "<",
        CompareOp::Le => "<=",
        CompareOp::Gt => ">",
        CompareOp::Ge => ">=",
    }
}

fn local_ref_value(local: &LocalRef) -> CanonicalValue {
    map([
        ("id", text(&local.id)),
        ("alphaName", text(&local.alpha_name)),
        ("type", text(&local.ty)),
    ])
}

fn map<'a>(entries: impl IntoIterator<Item = (&'a str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn string_map<'a>(entries: impl IntoIterator<Item = (&'a str, CanonicalValue)>) -> CanonicalValue {
    CanonicalValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn string_map_results<'a>(
    entries: impl IntoIterator<Item = Result<(&'a str, CanonicalValue), CanonicalError>>,
) -> Result<CanonicalValue, CanonicalError> {
    entries
        .into_iter()
        .map(|entry| entry.map(|(key, value)| (text(key), value)))
        .collect::<Result<Vec<_>, _>>()
        .map(CanonicalValue::Map)
}

fn array(entries: impl IntoIterator<Item = CanonicalValue>) -> CanonicalValue {
    CanonicalValue::Array(entries.into_iter().collect())
}

fn array_results(
    entries: impl IntoIterator<Item = Result<CanonicalValue, CanonicalError>>,
) -> Result<CanonicalValue, CanonicalError> {
    entries
        .into_iter()
        .collect::<Result<Vec<_>, _>>()
        .map(CanonicalValue::Array)
}

fn text(value: &str) -> CanonicalValue {
    CanonicalValue::Text(value.to_owned())
}

const fn uint(value: u64) -> CanonicalValue {
    CanonicalValue::Integer(value as i128)
}

const fn bool_value(value: bool) -> CanonicalValue {
    CanonicalValue::Bool(value)
}

fn encode_value(value: &CanonicalValue, out: &mut Vec<u8>) -> Result<(), CanonicalError> {
    match value {
        CanonicalValue::Null => out.push(0xf6),
        CanonicalValue::Bool(false) => out.push(0xf4),
        CanonicalValue::Bool(true) => out.push(0xf5),
        CanonicalValue::Integer(value) => encode_integer(*value, out)?,
        CanonicalValue::Bytes(value) => {
            encode_type_value(2, usize_to_u64(value.len())?, out);
            out.extend(value);
        }
        CanonicalValue::Text(value) => {
            encode_type_value(3, usize_to_u64(value.len())?, out);
            out.extend(value.as_bytes());
        }
        CanonicalValue::Array(values) => {
            encode_type_value(4, usize_to_u64(values.len())?, out);
            for value in values {
                encode_value(value, out)?;
            }
        }
        CanonicalValue::Map(entries) => {
            let mut encoded = Vec::with_capacity(entries.len());
            let mut keys = BTreeSet::new();
            for (key, value) in entries {
                let key_bytes = encode_canonical_cbor(key)?;
                if !keys.insert(key_bytes.clone()) {
                    return Err(CanonicalError::new(
                        CanonicalErrorKind::DuplicateMapKey,
                        "CBOR map contains duplicate keys",
                    ));
                }
                encoded.push((key_bytes, value));
            }
            encoded.sort_by(|(left, _), (right, _)| left.cmp(right));
            encode_type_value(5, usize_to_u64(encoded.len())?, out);
            for (key_bytes, value) in encoded {
                out.extend(key_bytes);
                encode_value(value, out)?;
            }
        }
    }
    Ok(())
}

fn encode_integer(value: i128, out: &mut Vec<u8>) -> Result<(), CanonicalError> {
    if value >= 0 {
        let value = u64::try_from(value).map_err(|_| {
            CanonicalError::new(
                CanonicalErrorKind::InvalidInteger,
                "positive integer exceeds CBOR uint range",
            )
        })?;
        encode_type_value(0, value, out);
    } else {
        let magnitude = (-1i128).checked_sub(value).ok_or_else(|| {
            CanonicalError::new(
                CanonicalErrorKind::InvalidInteger,
                "negative integer cannot be converted to CBOR range",
            )
        })?;
        let magnitude = u64::try_from(magnitude).map_err(|_| {
            CanonicalError::new(
                CanonicalErrorKind::InvalidInteger,
                "negative integer exceeds CBOR negative range",
            )
        })?;
        encode_type_value(1, magnitude, out);
    }
    Ok(())
}

fn encode_type_value(major: u8, value: u64, out: &mut Vec<u8>) {
    let prefix = major << 5;
    match value {
        0..=23 => out.push(prefix | u8::try_from(value).expect("value <= 23")),
        24..=0xff => {
            out.push(prefix | 0x18);
            out.push(u8::try_from(value).expect("value <= u8::MAX"));
        }
        0x100..=0xffff => {
            out.push(prefix | 0x19);
            out.extend(
                u16::try_from(value)
                    .expect("value <= u16::MAX")
                    .to_be_bytes(),
            );
        }
        0x1_0000..=0xffff_ffff => {
            out.push(prefix | 0x1a);
            out.extend(
                u32::try_from(value)
                    .expect("value <= u32::MAX")
                    .to_be_bytes(),
            );
        }
        _ => {
            out.push(prefix | 0x1b);
            out.extend(value.to_be_bytes());
        }
    }
}

fn usize_to_u64(value: usize) -> Result<u64, CanonicalError> {
    u64::try_from(value).map_err(|_| {
        CanonicalError::new(
            CanonicalErrorKind::UnsupportedValue,
            "collection length cannot fit in CBOR uint range",
        )
    })
}

struct Decoder<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    const fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn value(&mut self) -> Result<CanonicalValue, CanonicalError> {
        let initial = self.byte()?;
        let major = initial >> 5;
        let additional = initial & 0x1f;
        match major {
            0 => Ok(CanonicalValue::Integer(i128::from(
                self.argument(additional)?,
            ))),
            1 => Ok(CanonicalValue::Integer(
                -1 - i128::from(self.argument(additional)?),
            )),
            2 => {
                let len = self.length(additional)?;
                Ok(CanonicalValue::Bytes(self.take(len)?.to_vec()))
            }
            3 => {
                let len = self.length(additional)?;
                let bytes = self.take(len)?;
                let value = str::from_utf8(bytes).map_err(|_| {
                    CanonicalError::new(
                        CanonicalErrorKind::InvalidUtf8,
                        "CBOR text string is not valid UTF-8",
                    )
                })?;
                Ok(CanonicalValue::Text(value.to_owned()))
            }
            4 => {
                let len = self.length(additional)?;
                let mut values = Vec::with_capacity(len);
                for _ in 0..len {
                    values.push(self.value()?);
                }
                Ok(CanonicalValue::Array(values))
            }
            5 => {
                let len = self.length(additional)?;
                let mut entries = Vec::with_capacity(len);
                let mut keys = BTreeSet::new();
                for _ in 0..len {
                    let key = self.value()?;
                    let encoded_key = encode_canonical_cbor(&key)?;
                    if !keys.insert(encoded_key) {
                        return Err(CanonicalError::new(
                            CanonicalErrorKind::DuplicateMapKey,
                            "CBOR map contains duplicate keys",
                        ));
                    }
                    let value = self.value()?;
                    entries.push((key, value));
                }
                Ok(CanonicalValue::Map(entries))
            }
            7 => match additional {
                20 => Ok(CanonicalValue::Bool(false)),
                21 => Ok(CanonicalValue::Bool(true)),
                22 => Ok(CanonicalValue::Null),
                _ => Err(CanonicalError::new(
                    CanonicalErrorKind::UnsupportedCbor,
                    "unsupported CBOR simple value",
                )),
            },
            _ => Err(CanonicalError::new(
                CanonicalErrorKind::UnsupportedCbor,
                "unsupported CBOR major type",
            )),
        }
    }

    fn argument(&mut self, additional: u8) -> Result<u64, CanonicalError> {
        match additional {
            0..=23 => Ok(u64::from(additional)),
            24 => Ok(u64::from(self.byte()?)),
            25 => {
                let bytes = self.take_array::<2>()?;
                Ok(u64::from(u16::from_be_bytes(bytes)))
            }
            26 => {
                let bytes = self.take_array::<4>()?;
                Ok(u64::from(u32::from_be_bytes(bytes)))
            }
            27 => Ok(u64::from_be_bytes(self.take_array::<8>()?)),
            _ => Err(CanonicalError::new(
                CanonicalErrorKind::UnsupportedCbor,
                "indefinite or reserved CBOR length is unsupported",
            )),
        }
    }

    fn length(&mut self, additional: u8) -> Result<usize, CanonicalError> {
        usize::try_from(self.argument(additional)?).map_err(|_| {
            CanonicalError::new(
                CanonicalErrorKind::UnsupportedCbor,
                "CBOR collection length does not fit usize",
            )
        })
    }

    fn byte(&mut self) -> Result<u8, CanonicalError> {
        let Some(value) = self.bytes.get(self.pos).copied() else {
            return Err(CanonicalError::new(
                CanonicalErrorKind::UnexpectedEof,
                "expected another CBOR byte",
            ));
        };
        self.pos += 1;
        Ok(value)
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], CanonicalError> {
        let end = self.pos.checked_add(len).ok_or_else(|| {
            CanonicalError::new(
                CanonicalErrorKind::UnexpectedEof,
                "CBOR length overflowed input position",
            )
        })?;
        let Some(slice) = self.bytes.get(self.pos..end) else {
            return Err(CanonicalError::new(
                CanonicalErrorKind::UnexpectedEof,
                "CBOR value extends past end of input",
            ));
        };
        self.pos = end;
        Ok(slice)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], CanonicalError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }
}
