//! Canonical direct lawpack-adapter loading and compiler preparation.
//!
//! A direct adapter is declarative, selected by one exact digest-locked target
//! profile entry in a validated lawpack manifest, and complete over that
//! lawpack's runtime effects. It supplies compiler and Target IR facts; it does
//! not execute a runtime, emit an Echo package, or confer admission authority.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

use crate::ast::{ImportKind, Module};
use crate::canonical::{decode_canonical_cbor, encode_canonical_cbor, CanonicalValue};
use crate::compiler::CompilerContext;
use crate::core_ir::{CoreBudget, ResourceRef};
use crate::lawpack::{
    LawpackExecutionClass, LawpackResourceRef, LawpackSemanticEffect, LawpackTargetAdapter,
    ValidatedLawpackBundle,
};
use crate::lowerability::WriteClass;
use crate::target_ir::{TargetEffectLowering, TargetIrLoweringFacts};

/// Canonical direct lawpack-adapter ABI supported by this crate.
pub const LAWPACK_ADAPTER_API_VERSION: &str = "edict.lawpack-adapter/v1";

const DIGEST_FRAME: &str = "edict.digest/v1";
const ADAPTER_PATH: &str = "<lawpack-adapter-cbor>";

/// Stable direct-adapter failure classifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LawpackAdapterFailureKind {
    InvalidCanonicalCbor,
    InvalidShape,
    MissingField,
    UnexpectedField,
    InvalidApiVersion,
    UnsupportedClass,
    MissingTargetAdapter,
    AmbiguousTargetAdapter,
    AdapterDigestMismatch,
    MissingOperationProfile,
    UnknownOperationProfile,
    MissingEffectImplementation,
    UnknownEffectImplementation,
    DuplicateReference,
    ObligationMismatch,
    FailureMappingMismatch,
    MissingBudget,
    UnknownBudget,
    InvalidWriteClass,
    InvalidTargetIntrinsic,
    InvalidTargetConfiguration,
    SourceImportMismatch,
}

/// One failed direct-adapter validation or preparation obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackAdapterFailure {
    pub kind: LawpackAdapterFailureKind,
    pub path: String,
    pub obligation: String,
}

/// One operation profile discharged by a direct adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackAdapterOperationProfile {
    pub core: String,
    pub semantic_effects: Vec<String>,
}

/// One semantic effect discharged by a direct adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackAdapterEffect {
    pub target_intrinsic: String,
    pub target_configuration: LawpackResourceRef,
    pub write_class: WriteClass,
    pub footprint_obligation: String,
    pub cost_obligation: String,
    pub failure_mappings: BTreeMap<String, String>,
}

/// A canonical direct adapter corroborated against its owning lawpack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedLawpackAdapter {
    descriptor: LawpackTargetAdapter,
    operation_profiles: BTreeMap<String, LawpackAdapterOperationProfile>,
    effects: BTreeMap<String, LawpackAdapterEffect>,
    budgets: BTreeMap<String, CoreBudget>,
    digest: [u8; 32],
}

impl ValidatedLawpackAdapter {
    /// Exact target profile selected by the owning lawpack manifest.
    #[must_use]
    pub fn target_profile(&self) -> &LawpackResourceRef {
        &self.descriptor.accepted_target_profile
    }

    /// Exact Target IR selected by the owning lawpack manifest.
    #[must_use]
    pub fn target_ir(&self) -> &LawpackResourceRef {
        &self.descriptor.accepted_target_ir
    }

    /// Canonical operation-profile discharges.
    #[must_use]
    pub fn operation_profiles(&self) -> &BTreeMap<String, LawpackAdapterOperationProfile> {
        &self.operation_profiles
    }

    /// Canonical semantic-effect discharges.
    #[must_use]
    pub fn effects(&self) -> &BTreeMap<String, LawpackAdapterEffect> {
        &self.effects
    }

    /// Canonical budget discharges.
    #[must_use]
    pub fn budgets(&self) -> &BTreeMap<String, CoreBudget> {
        &self.budgets
    }

    /// Domain-framed digest of the exact canonical adapter bytes.
    #[must_use]
    pub fn digest_review_string(&self) -> String {
        sha256_review_string(&self.digest)
    }
}

/// Compiler and Target IR facts derived from an exact module/lawpack/adapter
/// closure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedLawpackCompilation {
    compiler_context: CompilerContext,
    target_ir_facts: TargetIrLoweringFacts,
}

impl PreparedLawpackCompilation {
    /// Compiler facts projected through the module-local lawpack alias.
    #[must_use]
    pub fn compiler_context(&self) -> &CompilerContext {
        &self.compiler_context
    }

    /// Target IR facts projected through the same module-local alias.
    #[must_use]
    pub fn target_ir_facts(&self) -> &TargetIrLoweringFacts {
        &self.target_ir_facts
    }
}

/// Decode one canonical direct adapter selected by a validated lawpack.
///
/// # Errors
///
/// Returns stable failures for canonical-shape errors, target selection
/// ambiguity, digest substitution, incomplete coverage, or mismatched
/// footprint, cost, and failure obligations.
pub fn decode_lawpack_adapter(
    bundle: &ValidatedLawpackBundle,
    target_profile: &str,
    bytes: &[u8],
) -> Result<ValidatedLawpackAdapter, Vec<LawpackAdapterFailure>> {
    let descriptor = select_descriptor(bundle, target_profile)?;
    let value = decode_canonical_cbor(bytes).map_err(|_| {
        one(failure(
            LawpackAdapterFailureKind::InvalidCanonicalCbor,
            ADAPTER_PATH,
            "exact canonical CBOR",
        ))
    })?;
    let digest = digest_value(&descriptor.adapter.id, &value)?;
    if digest != descriptor.adapter.digest {
        return Err(one(failure(
            LawpackAdapterFailureKind::AdapterDigestMismatch,
            "manifest.targetAdapters.adapter.digest",
            "digest of the exact canonical adapter bytes",
        )));
    }

    let (operation_profiles, effects, budgets) = parse_adapter(&value)?;
    validate_adapter_closure(bundle, &descriptor, &operation_profiles, &effects, &budgets)?;

    Ok(ValidatedLawpackAdapter {
        descriptor,
        operation_profiles,
        effects,
        budgets,
        digest,
    })
}

/// Derive compiler and target facts for one module from its exact imported
/// lawpack and selected direct adapter.
///
/// # Errors
///
/// Returns `SourceImportMismatch` unless the module contains exactly one
/// matching digest-locked lawpack import.
pub fn prepare_lawpack_compilation(
    module: &Module,
    bundle: &ValidatedLawpackBundle,
    adapter: &ValidatedLawpackAdapter,
) -> Result<PreparedLawpackCompilation, Vec<LawpackAdapterFailure>> {
    let alias = matching_import_alias(module, bundle)?;
    let prefix = format!("{}@{}.", bundle.manifest().id, bundle.manifest().version);
    let mut compiler_context = CompilerContext::new();
    let mut operation_profiles = BTreeSet::new();
    let mut obstruction_coordinates = BTreeSet::new();
    let mut effect_lowerings = Vec::new();

    for (coordinate, profile) in &adapter.operation_profiles {
        let local_profile = local_coordinate(&alias, &prefix, coordinate)?;
        let mut write_classes = BTreeSet::new();
        for effect_coordinate in &profile.semantic_effects {
            let effect = adapter.effects.get(effect_coordinate).ok_or_else(|| {
                one(failure(
                    LawpackAdapterFailureKind::MissingEffectImplementation,
                    format!("adapter.operationProfiles.{coordinate}.semanticEffects"),
                    effect_coordinate,
                ))
            })?;
            write_classes.insert(effect.write_class.clone());
        }
        compiler_context = compiler_context
            .with_operation_profile(local_profile.clone(), profile.core.clone())
            .with_operation_profile_write_classes(local_profile, write_classes);
        operation_profiles.insert(profile.core.clone());
    }

    for (coordinate, effect) in &adapter.effects {
        let local_effect = local_coordinate(&alias, &prefix, coordinate)?;
        compiler_context = compiler_context
            .with_effect_write_class(local_effect.clone(), effect.write_class.clone());
        obstruction_coordinates.extend(effect.failure_mappings.keys().cloned());
        effect_lowerings.push(TargetEffectLowering {
            effect: local_effect,
            target_intrinsic: effect.target_intrinsic.clone(),
        });
    }

    for (coordinate, budget) in &adapter.budgets {
        let local_budget = local_coordinate(&alias, &prefix, coordinate)?;
        compiler_context = compiler_context.with_budget(local_budget, budget.clone());
    }

    Ok(PreparedLawpackCompilation {
        compiler_context,
        target_ir_facts: TargetIrLoweringFacts {
            target_profile: ResourceRef {
                coordinate: adapter.target_profile().id.clone(),
                digest: Some(adapter.target_profile().digest_review_string()),
            },
            target_ir_domain: adapter.target_ir().id.clone(),
            operation_profiles: operation_profiles.into_iter().collect(),
            obstruction_coordinates: obstruction_coordinates.into_iter().collect(),
            effect_lowerings,
        },
    })
}

type AdapterParts = (
    BTreeMap<String, LawpackAdapterOperationProfile>,
    BTreeMap<String, LawpackAdapterEffect>,
    BTreeMap<String, CoreBudget>,
);

fn parse_adapter(value: &CanonicalValue) -> Result<AdapterParts, Vec<LawpackAdapterFailure>> {
    let fields = closed_map(
        value,
        "adapter",
        &[
            "apiVersion",
            "class",
            "operationProfiles",
            "effectImplementations",
            "budgets",
        ],
    )?;
    let api_version = required_text(&fields, "apiVersion", "adapter")?;
    if api_version != LAWPACK_ADAPTER_API_VERSION {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidApiVersion,
            "adapter.apiVersion",
            LAWPACK_ADAPTER_API_VERSION,
        )));
    }
    if required_text(&fields, "class", "adapter")? != "declarative" {
        return Err(one(failure(
            LawpackAdapterFailureKind::UnsupportedClass,
            "adapter.class",
            "declarative",
        )));
    }
    Ok((
        parse_operation_profiles(required(&fields, "operationProfiles", "adapter")?)?,
        parse_effects(required(&fields, "effectImplementations", "adapter")?)?,
        parse_budgets(required(&fields, "budgets", "adapter")?)?,
    ))
}

fn parse_operation_profiles(
    value: &CanonicalValue,
) -> Result<BTreeMap<String, LawpackAdapterOperationProfile>, Vec<LawpackAdapterFailure>> {
    let values = text_map(value, "adapter.operationProfiles")?;
    let mut profiles = BTreeMap::new();
    for (coordinate, value) in values {
        let path = format!("adapter.operationProfiles.{coordinate}");
        let fields = closed_map(value, &path, &["core", "semanticEffects"])?;
        let core = required_nonempty_text(&fields, "core", &path)?;
        let semantic_effects = text_array(
            required(&fields, "semanticEffects", &path)?,
            &format!("{path}.semanticEffects"),
            true,
        )?;
        profiles.insert(
            coordinate,
            LawpackAdapterOperationProfile {
                core,
                semantic_effects,
            },
        );
    }
    Ok(profiles)
}

fn parse_effects(
    value: &CanonicalValue,
) -> Result<BTreeMap<String, LawpackAdapterEffect>, Vec<LawpackAdapterFailure>> {
    let values = text_map(value, "adapter.effectImplementations")?;
    let mut effects = BTreeMap::new();
    for (coordinate, value) in values {
        let path = format!("adapter.effectImplementations.{coordinate}");
        let fields = closed_map(
            value,
            &path,
            &[
                "targetIntrinsic",
                "targetConfiguration",
                "writeClass",
                "footprintObligation",
                "costObligation",
                "failureMappings",
            ],
        )?;
        let failure_values = text_map(
            required(&fields, "failureMappings", &path)?,
            &format!("{path}.failureMappings"),
        )?;
        let mut failure_mappings = BTreeMap::new();
        for (failure_coordinate, target_obstruction) in failure_values {
            failure_mappings.insert(
                failure_coordinate,
                nonempty_text(target_obstruction, &format!("{path}.failureMappings"))?,
            );
        }
        effects.insert(
            coordinate,
            LawpackAdapterEffect {
                target_intrinsic: required_nonempty_text(&fields, "targetIntrinsic", &path)?,
                target_configuration: parse_resource_ref(
                    required(&fields, "targetConfiguration", &path)?,
                    &format!("{path}.targetConfiguration"),
                )?,
                write_class: parse_write_class(&required_text(&fields, "writeClass", &path)?)?,
                footprint_obligation: required_nonempty_text(
                    &fields,
                    "footprintObligation",
                    &path,
                )?,
                cost_obligation: required_nonempty_text(&fields, "costObligation", &path)?,
                failure_mappings,
            },
        );
    }
    Ok(effects)
}

fn parse_budgets(
    value: &CanonicalValue,
) -> Result<BTreeMap<String, CoreBudget>, Vec<LawpackAdapterFailure>> {
    let values = text_map(value, "adapter.budgets")?;
    let mut budgets = BTreeMap::new();
    for (coordinate, value) in values {
        let path = format!("adapter.budgets.{coordinate}");
        let fields = closed_map(
            value,
            &path,
            &["maxSteps", "maxAllocatedBytes", "maxOutputBytes"],
        )?;
        budgets.insert(
            coordinate,
            CoreBudget {
                max_steps: required_u64(&fields, "maxSteps", &path)?,
                max_allocated_bytes: required_u64(&fields, "maxAllocatedBytes", &path)?,
                max_output_bytes: required_u64(&fields, "maxOutputBytes", &path)?,
            },
        );
    }
    Ok(budgets)
}

fn validate_adapter_closure(
    bundle: &ValidatedLawpackBundle,
    descriptor: &LawpackTargetAdapter,
    operation_profiles: &BTreeMap<String, LawpackAdapterOperationProfile>,
    effects: &BTreeMap<String, LawpackAdapterEffect>,
    budgets: &BTreeMap<String, CoreBudget>,
) -> Result<(), Vec<LawpackAdapterFailure>> {
    let exported_profiles = &bundle.exports().operation_profiles;
    exact_keys(
        exported_profiles.keys().map(String::as_str),
        operation_profiles.keys().map(String::as_str),
        LawpackAdapterFailureKind::MissingOperationProfile,
        LawpackAdapterFailureKind::UnknownOperationProfile,
        "adapter.operationProfiles",
    )?;

    let runtime_effects = bundle
        .exports()
        .effects
        .iter()
        .filter(|effect| effect.execution_class == LawpackExecutionClass::Runtime)
        .map(|effect| (effect.coordinate.as_str(), effect))
        .collect::<BTreeMap<_, _>>();
    exact_keys(
        runtime_effects.keys().copied(),
        effects.keys().map(String::as_str),
        LawpackAdapterFailureKind::MissingEffectImplementation,
        LawpackAdapterFailureKind::UnknownEffectImplementation,
        "adapter.effectImplementations",
    )?;

    let intrinsic_prefix = format!("{}.", descriptor.accepted_target_profile.id);
    let mut required_budgets = BTreeSet::new();
    for (coordinate, effect) in effects {
        let exported = runtime_effects.get(coordinate.as_str()).ok_or_else(|| {
            one(failure(
                LawpackAdapterFailureKind::UnknownEffectImplementation,
                format!("adapter.effectImplementations.{coordinate}"),
                "exported runtime semantic effect",
            ))
        })?;
        validate_effect(coordinate, exported, effect, &intrinsic_prefix)?;
        required_budgets.insert(effect.cost_obligation.as_str());
    }
    exact_keys(
        required_budgets,
        budgets.keys().map(String::as_str),
        LawpackAdapterFailureKind::MissingBudget,
        LawpackAdapterFailureKind::UnknownBudget,
        "adapter.budgets",
    )?;

    for (coordinate, profile) in operation_profiles {
        let mut seen = BTreeSet::new();
        for effect in &profile.semantic_effects {
            if !seen.insert(effect) {
                return Err(one(failure(
                    LawpackAdapterFailureKind::DuplicateReference,
                    format!("adapter.operationProfiles.{coordinate}.semanticEffects"),
                    "unique semantic effect references",
                )));
            }
            if !effects.contains_key(effect) {
                return Err(one(failure(
                    LawpackAdapterFailureKind::MissingEffectImplementation,
                    format!("adapter.operationProfiles.{coordinate}.semanticEffects"),
                    effect,
                )));
            }
        }
    }
    Ok(())
}

fn validate_effect(
    coordinate: &str,
    exported: &LawpackSemanticEffect,
    effect: &LawpackAdapterEffect,
    intrinsic_prefix: &str,
) -> Result<(), Vec<LawpackAdapterFailure>> {
    let path = format!("adapter.effectImplementations.{coordinate}");
    if !effect.target_intrinsic.starts_with(intrinsic_prefix)
        || effect.target_intrinsic.len() == intrinsic_prefix.len()
    {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidTargetIntrinsic,
            format!("{path}.targetIntrinsic"),
            format!(
                "intrinsic below `{}`",
                intrinsic_prefix.trim_end_matches('.')
            ),
        )));
    }
    if effect.footprint_obligation != exported.footprint_obligation {
        return Err(one(failure(
            LawpackAdapterFailureKind::ObligationMismatch,
            format!("{path}.footprintObligation"),
            &exported.footprint_obligation,
        )));
    }
    if effect.cost_obligation != exported.cost_obligation {
        return Err(one(failure(
            LawpackAdapterFailureKind::ObligationMismatch,
            format!("{path}.costObligation"),
            &exported.cost_obligation,
        )));
    }
    let exported_failures = exported
        .effect_failures
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mapped_failures = effect
        .failure_mappings
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if exported_failures != mapped_failures {
        return Err(one(failure(
            LawpackAdapterFailureKind::FailureMappingMismatch,
            format!("{path}.failureMappings"),
            "exact exported named-failure set",
        )));
    }
    Ok(())
}

fn select_descriptor(
    bundle: &ValidatedLawpackBundle,
    target_profile: &str,
) -> Result<LawpackTargetAdapter, Vec<LawpackAdapterFailure>> {
    let matches = bundle
        .manifest()
        .target_adapters
        .iter()
        .filter(|adapter| adapter.accepted_target_profile.id == target_profile)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [descriptor] => Ok((*descriptor).clone()),
        [] => Err(one(failure(
            LawpackAdapterFailureKind::MissingTargetAdapter,
            "manifest.targetAdapters",
            format!("one adapter for `{target_profile}`"),
        ))),
        _ => Err(one(failure(
            LawpackAdapterFailureKind::AmbiguousTargetAdapter,
            "manifest.targetAdapters",
            format!("one adapter for `{target_profile}`"),
        ))),
    }
}

fn matching_import_alias(
    module: &Module,
    bundle: &ValidatedLawpackBundle,
) -> Result<String, Vec<LawpackAdapterFailure>> {
    let manifest_coordinate = format!("{}@{}", bundle.manifest().id, bundle.manifest().version);
    let manifest_digest = bundle.manifest_digest_review_string();
    let matches = module
        .imports
        .iter()
        .filter(|import| {
            import.kind == ImportKind::Lawpack
                && import.package.as_ref().is_some_and(|package| {
                    format!("{}@{}", package.path.join("."), package.version) == manifest_coordinate
                })
        })
        .collect::<Vec<_>>();
    let [import] = matches.as_slice() else {
        return Err(one(failure(
            LawpackAdapterFailureKind::SourceImportMismatch,
            "module.imports",
            format!("exactly one import of `{manifest_coordinate}`"),
        )));
    };
    if import.digest.as_deref() != Some(manifest_digest.as_str()) {
        return Err(one(failure(
            LawpackAdapterFailureKind::SourceImportMismatch,
            format!("module.imports.{}.digest", import.alias),
            manifest_digest,
        )));
    }
    Ok(import.alias.clone())
}

fn local_coordinate(
    alias: &str,
    canonical_prefix: &str,
    coordinate: &str,
) -> Result<String, Vec<LawpackAdapterFailure>> {
    let Some(suffix) = coordinate.strip_prefix(canonical_prefix) else {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            coordinate,
            format!(
                "coordinate below `{}`",
                canonical_prefix.trim_end_matches('.')
            ),
        )));
    };
    if suffix.is_empty() {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            coordinate,
            "non-empty exported coordinate suffix",
        )));
    }
    Ok(format!("{alias}.{suffix}"))
}

fn exact_keys<'a>(
    expected: impl IntoIterator<Item = &'a str>,
    actual: impl IntoIterator<Item = &'a str>,
    missing_kind: LawpackAdapterFailureKind,
    unknown_kind: LawpackAdapterFailureKind,
    path: &str,
) -> Result<(), Vec<LawpackAdapterFailure>> {
    let expected = expected.into_iter().collect::<BTreeSet<_>>();
    let actual = actual.into_iter().collect::<BTreeSet<_>>();
    if let Some(missing) = expected.difference(&actual).next() {
        return Err(one(failure(
            missing_kind,
            path,
            format!("include `{missing}`"),
        )));
    }
    if let Some(unknown) = actual.difference(&expected).next() {
        return Err(one(failure(
            unknown_kind,
            path,
            format!("exclude unknown `{unknown}`"),
        )));
    }
    Ok(())
}

fn parse_write_class(value: &str) -> Result<WriteClass, Vec<LawpackAdapterFailure>> {
    match value {
        "none" => Ok(WriteClass::None),
        "read" => Ok(WriteClass::Read),
        "create" => Ok(WriteClass::Create),
        "ensure" => Ok(WriteClass::Ensure),
        "append" => Ok(WriteClass::Append),
        "replace" => Ok(WriteClass::Replace),
        "delete" => Ok(WriteClass::Delete),
        "custom" => Ok(WriteClass::Custom("custom".to_owned())),
        _ => Err(one(failure(
            LawpackAdapterFailureKind::InvalidWriteClass,
            "adapter.effectImplementations.writeClass",
            "v1 authority write class",
        ))),
    }
}

fn parse_resource_ref(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackResourceRef, Vec<LawpackAdapterFailure>> {
    let fields = closed_map(value, path, &["id", "digest"])?;
    Ok(LawpackResourceRef {
        id: required_nonempty_text(&fields, "id", path)?,
        digest: parse_digest(
            required(&fields, "digest", path)?,
            &format!("{path}.digest"),
        )?,
    })
}

fn parse_digest(
    value: &CanonicalValue,
    path: &str,
) -> Result<[u8; 32], Vec<LawpackAdapterFailure>> {
    let CanonicalValue::Array(parts) = value else {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidTargetConfiguration,
            path,
            "['sha256', 32-byte bstr]",
        )));
    };
    let [CanonicalValue::Text(algorithm), CanonicalValue::Bytes(bytes)] = parts.as_slice() else {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidTargetConfiguration,
            path,
            "['sha256', 32-byte bstr]",
        )));
    };
    let Ok(digest) = <[u8; 32]>::try_from(bytes.as_slice()) else {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidTargetConfiguration,
            path,
            "['sha256', 32-byte bstr]",
        )));
    };
    if algorithm != "sha256" {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidTargetConfiguration,
            path,
            "['sha256', 32-byte bstr]",
        )));
    }
    Ok(digest)
}

fn digest_value(
    domain: &str,
    value: &CanonicalValue,
) -> Result<[u8; 32], Vec<LawpackAdapterFailure>> {
    let framed = CanonicalValue::Array(vec![
        CanonicalValue::Text(DIGEST_FRAME.to_owned()),
        CanonicalValue::Text(domain.to_owned()),
        value.clone(),
    ]);
    let bytes = encode_canonical_cbor(&framed).map_err(|_| {
        one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            ADAPTER_PATH,
            "canonically encodable adapter digest frame",
        ))
    })?;
    Ok(Sha256::digest(bytes).into())
}

fn closed_map<'a>(
    value: &'a CanonicalValue,
    path: &str,
    allowed: &[&str],
) -> Result<BTreeMap<&'a str, &'a CanonicalValue>, Vec<LawpackAdapterFailure>> {
    let fields = text_map_ref(value, path)?;
    if let Some(unexpected) = fields.keys().find(|key| !allowed.contains(key)) {
        return Err(one(failure(
            LawpackAdapterFailureKind::UnexpectedField,
            format!("{path}.{unexpected}"),
            "closed map",
        )));
    }
    Ok(fields)
}

fn text_map<'a>(
    value: &'a CanonicalValue,
    path: &str,
) -> Result<BTreeMap<String, &'a CanonicalValue>, Vec<LawpackAdapterFailure>> {
    Ok(text_map_ref(value, path)?
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value))
        .collect())
}

fn text_map_ref<'a>(
    value: &'a CanonicalValue,
    path: &str,
) -> Result<BTreeMap<&'a str, &'a CanonicalValue>, Vec<LawpackAdapterFailure>> {
    let CanonicalValue::Map(entries) = value else {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            path,
            "map",
        )));
    };
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let CanonicalValue::Text(key) = key else {
            return Err(one(failure(
                LawpackAdapterFailureKind::InvalidShape,
                path,
                "text-keyed map",
            )));
        };
        fields.insert(key.as_str(), value);
    }
    Ok(fields)
}

fn required<'a>(
    fields: &BTreeMap<&str, &'a CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<&'a CanonicalValue, Vec<LawpackAdapterFailure>> {
    fields.get(field).copied().ok_or_else(|| {
        one(failure(
            LawpackAdapterFailureKind::MissingField,
            format!("{path}.{field}"),
            "required field",
        ))
    })
}

fn required_text(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<String, Vec<LawpackAdapterFailure>> {
    text(required(fields, field, path)?, &format!("{path}.{field}"))
}

fn required_nonempty_text(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<String, Vec<LawpackAdapterFailure>> {
    nonempty_text(required(fields, field, path)?, &format!("{path}.{field}"))
}

fn required_u64(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<u64, Vec<LawpackAdapterFailure>> {
    let value = required(fields, field, path)?;
    let CanonicalValue::Integer(value) = value else {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            format!("{path}.{field}"),
            "unsigned integer",
        )));
    };
    u64::try_from(*value).map_err(|_| {
        one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            format!("{path}.{field}"),
            "u64 value",
        ))
    })
}

fn text(value: &CanonicalValue, path: &str) -> Result<String, Vec<LawpackAdapterFailure>> {
    let CanonicalValue::Text(value) = value else {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            path,
            "text",
        )));
    };
    Ok(value.clone())
}

fn nonempty_text(value: &CanonicalValue, path: &str) -> Result<String, Vec<LawpackAdapterFailure>> {
    let value = text(value, path)?;
    if value.is_empty() {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            path,
            "non-empty text",
        )));
    }
    Ok(value)
}

fn text_array(
    value: &CanonicalValue,
    path: &str,
    nonempty: bool,
) -> Result<Vec<String>, Vec<LawpackAdapterFailure>> {
    let CanonicalValue::Array(values) = value else {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            path,
            "array",
        )));
    };
    if nonempty && values.is_empty() {
        return Err(one(failure(
            LawpackAdapterFailureKind::InvalidShape,
            path,
            "non-empty array",
        )));
    }
    values
        .iter()
        .enumerate()
        .map(|(index, value)| nonempty_text(value, &format!("{path}[{index}]")))
        .collect()
}

fn sha256_review_string(digest: &[u8; 32]) -> String {
    let mut output = String::with_capacity(71);
    output.push_str("sha256:");
    for byte in digest {
        use std::fmt::Write;
        write!(output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn failure(
    kind: LawpackAdapterFailureKind,
    path: impl Into<String>,
    obligation: impl Into<String>,
) -> LawpackAdapterFailure {
    LawpackAdapterFailure {
        kind,
        path: path.into(),
        obligation: obligation.into(),
    }
}

fn one(failure: LawpackAdapterFailure) -> Vec<LawpackAdapterFailure> {
    vec![failure]
}
