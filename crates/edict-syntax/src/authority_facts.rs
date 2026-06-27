//! File-backed authority-fact loading for compiler context facts.
//!
//! This module deliberately loads only the first facts already modeled by
//! `CompilerContext`: operation profiles, profile write-class allowances,
//! effect write classes, and budgets. It does not validate full lawpack or
//! target-profile manifests, and it does not implement participant trust
//! policy.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::compiler::CompilerContext;
use crate::core_ir::{is_sha256_review_digest, CoreBudget};
use crate::lowerability::WriteClass;

/// Authority-facts document ABI supported by this crate.
pub const AUTHORITY_FACTS_API_VERSION: &str = "edict.authority-facts/v1";

/// Kind of digest-bound source that supplied authority facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthorityFactSourceKind {
    Lawpack,
    TargetProfile,
}

/// Digest-bound authority-fact source identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityFactSource {
    pub kind: AuthorityFactSourceKind,
    pub coordinate: String,
    pub digest: String,
}

/// Operation profile fact available to the compiler resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationProfileFact {
    pub source: String,
    pub core: String,
    pub allowed_write_classes: Vec<WriteClass>,
}

/// Effect write-class fact available to compiler profile/effect checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectWriteClassFact {
    pub effect: String,
    pub write_class: WriteClass,
}

/// Budget fact available to the compiler resolver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetFact {
    pub source: String,
    pub budget: CoreBudget,
}

/// One validated authority-facts document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityFactsDocument {
    pub api_version: String,
    pub source: AuthorityFactSource,
    pub operation_profiles: Vec<OperationProfileFact>,
    pub effect_write_classes: Vec<EffectWriteClassFact>,
    pub budgets: Vec<BudgetFact>,
}

/// Stable authority-facts load failure categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorityFactsLoadFailureKind {
    Io,
    InvalidJson,
    InvalidApiVersion,
    InvalidSourceKind,
    MissingCoordinate,
    InvalidCoordinate,
    NonDigestLockedSource,
    InvalidWriteClass,
    ConflictingFact,
}

/// One failed authority-facts loading obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityFactsLoadFailure {
    pub kind: AuthorityFactsLoadFailureKind,
    pub path: String,
    pub field: String,
    pub coordinate: String,
}

/// Load and validate one authority-facts JSON document.
///
/// # Errors
///
/// Returns stable load failures when the file cannot be read, cannot be parsed
/// as the supported JSON shape, lacks digest-bound source identity, or carries
/// invalid fact values.
pub fn load_authority_facts_file(
    path: impl AsRef<Path>,
) -> Result<AuthorityFactsDocument, Vec<AuthorityFactsLoadFailure>> {
    let path = path.as_ref();
    let path_display = path.display().to_string();
    let text = fs::read_to_string(path).map_err(|_err| {
        vec![failure(
            AuthorityFactsLoadFailureKind::Io,
            &path_display,
            "file",
            "",
        )]
    })?;
    let raw = serde_json::from_str::<RawAuthorityFactsDocument>(&text).map_err(|_err| {
        vec![failure(
            AuthorityFactsLoadFailureKind::InvalidJson,
            &path_display,
            "json",
            "",
        )]
    })?;
    validate_raw_document(raw, &path_display)
}

/// Load authority-facts files and merge them into a compiler context.
///
/// # Errors
///
/// Returns stable load failures when any input file is invalid or when the
/// loaded facts contain conflicting values for the same source coordinate.
pub fn load_compiler_context_from_authority_fact_files<I, P>(
    paths: I,
) -> Result<CompilerContext, Vec<AuthorityFactsLoadFailure>>
where
    I: IntoIterator<Item = P>,
    P: AsRef<Path>,
{
    let mut documents = Vec::new();
    let mut failures = Vec::new();

    for path in paths {
        match load_authority_facts_file(path) {
            Ok(document) => documents.push(document),
            Err(mut errs) => failures.append(&mut errs),
        }
    }
    if !failures.is_empty() {
        return Err(failures);
    }

    compiler_context_from_authority_facts(&documents)
}

/// Merge validated authority-facts documents into a compiler context.
///
/// # Errors
///
/// Returns stable load failures when repeated fact coordinates carry different
/// values.
pub fn compiler_context_from_authority_facts(
    documents: &[AuthorityFactsDocument],
) -> Result<CompilerContext, Vec<AuthorityFactsLoadFailure>> {
    let mut source_digests = BTreeMap::<(AuthorityFactSourceKind, String), String>::new();
    let mut profiles = BTreeMap::<String, (String, BTreeSet<WriteClass>)>::new();
    let mut effects = BTreeMap::<String, WriteClass>::new();
    let mut budgets = BTreeMap::<String, CoreBudget>::new();
    let mut failures = Vec::new();

    for document in documents {
        let path = format!(
            "{}@{}",
            source_kind_name(document.source.kind),
            document.source.coordinate
        );
        insert_source_digest(document, &path, &mut source_digests, &mut failures);
        for profile in &document.operation_profiles {
            insert_profile_fact(&mut profiles, profile, &path, &mut failures);
        }
        for effect in &document.effect_write_classes {
            insert_fact(
                &mut effects,
                &effect.effect,
                effect.write_class.clone(),
                "effectWriteClasses",
                &path,
                &mut failures,
            );
        }
        for budget in &document.budgets {
            insert_fact(
                &mut budgets,
                &budget.source,
                budget.budget.clone(),
                "budgets",
                &path,
                &mut failures,
            );
        }
    }

    if !failures.is_empty() {
        return Err(failures);
    }

    let mut context = CompilerContext::new();
    for (source, (core, write_classes)) in profiles {
        context = context
            .with_operation_profile(source.clone(), core)
            .with_operation_profile_write_classes(source, write_classes);
    }
    for (effect, write_class) in effects {
        context = context.with_effect_write_class(effect, write_class);
    }
    for (source, budget) in budgets {
        context = context.with_budget(source, budget);
    }
    Ok(context)
}

fn insert_source_digest(
    document: &AuthorityFactsDocument,
    path: &str,
    source_digests: &mut BTreeMap<(AuthorityFactSourceKind, String), String>,
    failures: &mut Vec<AuthorityFactsLoadFailure>,
) {
    let key = (document.source.kind, document.source.coordinate.clone());
    if let Some(existing_digest) = source_digests.get(&key) {
        if existing_digest != &document.source.digest {
            failures.push(failure(
                AuthorityFactsLoadFailureKind::ConflictingFact,
                path,
                "source.digest",
                &document.source.coordinate,
            ));
        }
    } else {
        source_digests.insert(key, document.source.digest.clone());
    }
}

fn validate_raw_document(
    raw: RawAuthorityFactsDocument,
    path: &str,
) -> Result<AuthorityFactsDocument, Vec<AuthorityFactsLoadFailure>> {
    let mut failures = Vec::new();
    if raw.api_version != AUTHORITY_FACTS_API_VERSION {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::InvalidApiVersion,
            path,
            "apiVersion",
            &raw.api_version,
        ));
    }

    let source_kind = parse_source_kind(&raw.source.kind);
    if source_kind.is_none() {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::InvalidSourceKind,
            path,
            "source.kind",
            &raw.source.kind,
        ));
    }
    if raw.source.coordinate.is_empty() {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::MissingCoordinate,
            path,
            "source.coordinate",
            "",
        ));
    } else if !is_authority_coordinate(&raw.source.coordinate) {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::InvalidCoordinate,
            path,
            "source.coordinate",
            &raw.source.coordinate,
        ));
    }
    let digest = raw.source.digest.unwrap_or_default();
    if !is_sha256_review_digest(&digest) {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::NonDigestLockedSource,
            path,
            "source.digest",
            &raw.source.coordinate,
        ));
    }

    let operation_profiles = raw
        .operation_profiles
        .into_iter()
        .filter_map(|profile| validate_operation_profile(profile, path, &mut failures))
        .collect();
    let effect_write_classes = raw
        .effect_write_classes
        .into_iter()
        .filter_map(|effect| validate_effect_write_class(effect, path, &mut failures))
        .collect();
    let budgets = raw
        .budgets
        .into_iter()
        .filter_map(|budget| validate_budget(budget, path, &mut failures))
        .collect();

    if !failures.is_empty() {
        return Err(failures);
    }

    Ok(AuthorityFactsDocument {
        api_version: raw.api_version,
        source: AuthorityFactSource {
            kind: source_kind.expect("source kind already validated"),
            coordinate: raw.source.coordinate,
            digest,
        },
        operation_profiles,
        effect_write_classes,
        budgets,
    })
}

fn validate_operation_profile(
    raw: RawOperationProfileFact,
    path: &str,
    failures: &mut Vec<AuthorityFactsLoadFailure>,
) -> Option<OperationProfileFact> {
    let mut valid = true;
    if raw.source.is_empty() {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::MissingCoordinate,
            path,
            "operationProfiles.source",
            "",
        ));
        valid = false;
    } else if !is_authority_coordinate(&raw.source) {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::InvalidCoordinate,
            path,
            "operationProfiles.source",
            &raw.source,
        ));
        valid = false;
    }
    if raw.core.is_empty() {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::MissingCoordinate,
            path,
            "operationProfiles.core",
            &raw.source,
        ));
        valid = false;
    } else if !is_authority_coordinate(&raw.core) {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::InvalidCoordinate,
            path,
            "operationProfiles.core",
            &raw.source,
        ));
        valid = false;
    }

    let mut allowed_write_classes = Vec::new();
    for write_class in raw.allowed_write_classes {
        if let Some(value) = parse_write_class(&write_class) {
            allowed_write_classes.push(value);
        } else {
            failures.push(failure(
                AuthorityFactsLoadFailureKind::InvalidWriteClass,
                path,
                "operationProfiles.allowedWriteClasses",
                &raw.source,
            ));
            valid = false;
        }
    }

    valid.then_some(OperationProfileFact {
        source: raw.source,
        core: raw.core,
        allowed_write_classes,
    })
}

fn validate_effect_write_class(
    raw: RawEffectWriteClassFact,
    path: &str,
    failures: &mut Vec<AuthorityFactsLoadFailure>,
) -> Option<EffectWriteClassFact> {
    let mut valid = true;
    if raw.effect.is_empty() {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::MissingCoordinate,
            path,
            "effectWriteClasses.effect",
            "",
        ));
        valid = false;
    } else if !is_authority_coordinate(&raw.effect) {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::InvalidCoordinate,
            path,
            "effectWriteClasses.effect",
            &raw.effect,
        ));
        valid = false;
    }
    let write_class = if let Some(value) = parse_write_class(&raw.write_class) {
        value
    } else {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::InvalidWriteClass,
            path,
            "effectWriteClasses.writeClass",
            &raw.effect,
        ));
        valid = false;
        WriteClass::None
    };

    valid.then_some(EffectWriteClassFact {
        effect: raw.effect,
        write_class,
    })
}

fn validate_budget(
    raw: RawBudgetFact,
    path: &str,
    failures: &mut Vec<AuthorityFactsLoadFailure>,
) -> Option<BudgetFact> {
    if raw.source.is_empty() {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::MissingCoordinate,
            path,
            "budgets.source",
            "",
        ));
        return None;
    }
    if !is_authority_coordinate(&raw.source) {
        failures.push(failure(
            AuthorityFactsLoadFailureKind::InvalidCoordinate,
            path,
            "budgets.source",
            &raw.source,
        ));
        return None;
    }

    Some(BudgetFact {
        source: raw.source,
        budget: CoreBudget {
            max_steps: raw.max_steps,
            max_allocated_bytes: raw.max_allocated_bytes,
            max_output_bytes: raw.max_output_bytes,
        },
    })
}

fn insert_profile_fact(
    profiles: &mut BTreeMap<String, (String, BTreeSet<WriteClass>)>,
    profile: &OperationProfileFact,
    path: &str,
    failures: &mut Vec<AuthorityFactsLoadFailure>,
) {
    let value = (
        profile.core.clone(),
        profile.allowed_write_classes.iter().cloned().collect(),
    );
    insert_fact(
        profiles,
        &profile.source,
        value,
        "operationProfiles",
        path,
        failures,
    );
}

fn insert_fact<T>(
    map: &mut BTreeMap<String, T>,
    coordinate: &str,
    value: T,
    field: &str,
    path: &str,
    failures: &mut Vec<AuthorityFactsLoadFailure>,
) where
    T: PartialEq,
{
    if let Some(existing) = map.get(coordinate) {
        if existing != &value {
            failures.push(failure(
                AuthorityFactsLoadFailureKind::ConflictingFact,
                path,
                field,
                coordinate,
            ));
        }
    } else {
        map.insert(coordinate.to_owned(), value);
    }
}

fn parse_source_kind(value: &str) -> Option<AuthorityFactSourceKind> {
    match value {
        "lawpack" => Some(AuthorityFactSourceKind::Lawpack),
        "targetProfile" => Some(AuthorityFactSourceKind::TargetProfile),
        _ => None,
    }
}

fn source_kind_name(kind: AuthorityFactSourceKind) -> &'static str {
    match kind {
        AuthorityFactSourceKind::Lawpack => "lawpack",
        AuthorityFactSourceKind::TargetProfile => "targetProfile",
    }
}

fn is_authority_coordinate(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b'@' | b'/' | b':')
        })
}

fn parse_write_class(value: &str) -> Option<WriteClass> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Some(WriteClass::None),
        "read" => Some(WriteClass::Read),
        "create" => Some(WriteClass::Create),
        "ensure" => Some(WriteClass::Ensure),
        "append" => Some(WriteClass::Append),
        "replace" => Some(WriteClass::Replace),
        "delete" => Some(WriteClass::Delete),
        custom if custom.starts_with("custom:") && custom.len() > "custom:".len() => {
            Some(WriteClass::Custom(value["custom:".len()..].to_owned()))
        }
        _ => None,
    }
}

fn failure(
    kind: AuthorityFactsLoadFailureKind,
    path: &str,
    field: &str,
    coordinate: &str,
) -> AuthorityFactsLoadFailure {
    AuthorityFactsLoadFailure {
        kind,
        path: path.to_owned(),
        field: field.to_owned(),
        coordinate: coordinate.to_owned(),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawAuthorityFactsDocument {
    api_version: String,
    source: RawAuthorityFactSource,
    #[serde(default)]
    operation_profiles: Vec<RawOperationProfileFact>,
    #[serde(default)]
    effect_write_classes: Vec<RawEffectWriteClassFact>,
    #[serde(default)]
    budgets: Vec<RawBudgetFact>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawAuthorityFactSource {
    kind: String,
    coordinate: String,
    digest: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawOperationProfileFact {
    source: String,
    core: String,
    #[serde(default)]
    allowed_write_classes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawEffectWriteClassFact {
    effect: String,
    write_class: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawBudgetFact {
    source: String,
    max_steps: u64,
    max_allocated_bytes: u64,
    max_output_bytes: u64,
}
