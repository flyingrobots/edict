//! Canonical `edict.lawpack/v1` loading and dependency validation.
//!
//! Lawpacks are portable semantic modules. This boundary decodes their exact
//! canonical bytes, validates the closed manifest and export schemas, binds the
//! manifest to the supplied export surface, and validates complete dependency
//! sets before any export is exposed to compiler resolution. It does not
//! execute helpers, adapters, verifiers, or conformance fixtures.

use std::collections::{BTreeMap, BTreeSet};

use crate::canonical::{
    decode_canonical_cbor, digest_canonical_value, sha256_review_string, CanonicalErrorKind,
    CanonicalValue,
};
use crate::core_ir::CORE_API_VERSION;
use crate::parser::is_keyword;

/// Lawpack manifest ABI supported by this loader.
pub const LAWPACK_API_VERSION: &str = "edict.lawpack/v1";

const LAWPACK_CBOR_PATH: &str = "<lawpack-manifest-cbor>";
const EXPORTS_CBOR_PATH: &str = "<lawpack-exports-cbor>";

/// Stable lawpack validation failure categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LawpackValidationFailureKind {
    InvalidCanonicalCbor,
    InvalidShape,
    MissingField,
    UnexpectedField,
    InvalidApiVersion,
    EmptyIdentity,
    MissingAcceptedCoreAbi,
    InvalidDigest,
    ExportsDigestMismatch,
    InvalidDiscriminant,
    EmptyRequiredCollection,
    DuplicateIdentity,
    InvalidFailureIdentifier,
    ReservedFailureIdentifier,
    InvalidPureFunctionBody,
    RuntimeEffectWithoutTargetAdapter,
    SelfDependency,
    MissingDependency,
    DependencyDigestMismatch,
    DependencyCycle,
}

/// One failed lawpack loading or dependency obligation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackValidationFailure {
    pub kind: LawpackValidationFailureKind,
    pub path: String,
    pub obligation: String,
}

/// Typed SHA-256 resource reference from the canonical lawpack ABI.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LawpackResourceRef {
    pub id: String,
    pub digest: [u8; 32],
}

impl LawpackResourceRef {
    /// Lowercase review rendering for the typed wire digest.
    #[must_use]
    pub fn digest_review_string(&self) -> String {
        sha256_review_string(&self.digest)
    }
}

/// One digest-locked lawpack dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackDependency {
    pub id: String,
    pub version: String,
    pub digest: [u8; 32],
}

/// One bounded executable component reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackExecutableComponent {
    pub component: LawpackResourceRef,
    pub sandbox: LawpackResourceRef,
    pub fuel_model: LawpackResourceRef,
}

/// Verifier class discriminant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawpackVerifierClass {
    Declarative,
    Executable,
}

/// Declarative or bounded executable lawpack verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LawpackVerifier {
    Declarative {
        ruleset: LawpackResourceRef,
    },
    Executable {
        executable: LawpackExecutableComponent,
    },
}

impl LawpackVerifier {
    /// Return the verifier class without inspecting its implementation fields.
    #[must_use]
    pub fn class(&self) -> LawpackVerifierClass {
        match self {
            Self::Declarative { .. } => LawpackVerifierClass::Declarative,
            Self::Executable { .. } => LawpackVerifierClass::Executable,
        }
    }
}

/// Direct target-adapter descriptor selected by exact target-profile identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackTargetAdapter {
    pub accepted_target_profile: LawpackResourceRef,
    pub accepted_target_ir: LawpackResourceRef,
    pub adapter: LawpackResourceRef,
}

/// Typed `edict.lawpack/v1` manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackManifest {
    pub api_version: String,
    pub id: String,
    pub version: String,
    pub accepted_core_abi: Vec<String>,
    pub dependencies: Vec<LawpackDependency>,
    pub exports: LawpackResourceRef,
    pub target_adapters: Vec<LawpackTargetAdapter>,
    pub helper_component: Option<LawpackExecutableComponent>,
    pub verifier: LawpackVerifier,
    pub compatibility: LawpackResourceRef,
    pub conformance_fixture_corpus: LawpackResourceRef,
}

/// One exported type alias/reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackExportedType {
    pub coordinate: String,
    pub definition: String,
}

/// One typed exported constant. The value remains canonical and hash-significant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackExportedConstant {
    pub coordinate: String,
    pub ty: String,
    pub value: CanonicalValue,
}

/// Pure-helper determinism classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawpackDeterminismClass {
    Total,
    TotalWithTypedDiagnostic,
}

/// Hash-bound implementation of a pure lawpack helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LawpackPureFunctionImplementation {
    Edict {
        body: CanonicalValue,
    },
    Component {
        implementation: LawpackExecutableComponent,
    },
}

/// One exported pure helper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackPureFunction {
    pub coordinate: String,
    pub type_parameters: Vec<String>,
    pub parameter_types: Vec<String>,
    pub return_type: String,
    pub cost_template: String,
    pub determinism_class: LawpackDeterminismClass,
    pub implementation: LawpackPureFunctionImplementation,
}

/// Semantic-effect execution classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawpackExecutionClass {
    ProofOnly,
    Runtime,
}

/// Advisory semantic effect-kind classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawpackEffectKind {
    Read,
    Create,
    Ensure,
    Replace,
    Delete,
    Append,
    Reduce,
    SemanticEmit,
    Custom,
}

/// Authority owner for one effect failure or domain obstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LawpackAuthorityClass {
    DomainMappable,
    ParticipantOwned,
    IntegrityFault,
    ResourceFault,
    InternalFault,
}

/// One named low-level semantic-effect failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackEffectFailure {
    pub authority_class: LawpackAuthorityClass,
    pub payload_type: String,
}

/// One semantic effect exported by a lawpack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackSemanticEffect {
    pub coordinate: String,
    pub type_parameters: Vec<String>,
    pub input_type: String,
    pub output_type: String,
    pub execution_class: LawpackExecutionClass,
    pub effect_kind_hint: LawpackEffectKind,
    pub footprint_obligation: String,
    pub cost_obligation: String,
    pub effect_failures: BTreeMap<String, LawpackEffectFailure>,
    pub guard_support: bool,
}

/// One typed domain obstruction exported by a lawpack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackObstruction {
    pub coordinate: String,
    pub authority_class: LawpackAuthorityClass,
    pub payload_schema: String,
}

/// Typed aperture requirement supplied by an operation-profile optic template.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LawpackApertureRequirement {
    FootprintCeiling { reference: String },
    AbstractFootprintObligation { reference: String },
}

/// Runtime-neutral optic template from one operation profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackOpticTemplate {
    pub optic_kind: String,
    pub boundary_kind: String,
    pub support_policy: String,
    pub loss_disposition: String,
    pub basis_template: Option<String>,
    pub aperture_requirement: Option<LawpackApertureRequirement>,
}

/// One operation profile keyed by its export coordinate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackOperationProfile {
    pub optic_template: LawpackOpticTemplate,
    pub effect_predicate: String,
}

/// Typed lawpack export surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LawpackExports {
    pub types: Vec<LawpackExportedType>,
    pub constants: Vec<LawpackExportedConstant>,
    pub pure_functions: Vec<LawpackPureFunction>,
    pub effects: Vec<LawpackSemanticEffect>,
    pub obstructions: Vec<LawpackObstruction>,
    pub operation_profiles: BTreeMap<String, LawpackOperationProfile>,
}

/// A canonical manifest and export surface that passed all local obligations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedLawpackBundle {
    manifest: LawpackManifest,
    exports: LawpackExports,
    manifest_digest: [u8; 32],
    exports_digest: [u8; 32],
}

impl ValidatedLawpackBundle {
    /// Validated typed manifest.
    #[must_use]
    pub fn manifest(&self) -> &LawpackManifest {
        &self.manifest
    }

    /// Validated typed export surface.
    #[must_use]
    pub fn exports(&self) -> &LawpackExports {
        &self.exports
    }

    /// Typed manifest digest bytes.
    #[must_use]
    pub fn manifest_digest(&self) -> &[u8; 32] {
        &self.manifest_digest
    }

    /// Typed export-surface digest bytes.
    #[must_use]
    pub fn exports_digest(&self) -> &[u8; 32] {
        &self.exports_digest
    }

    /// Lowercase review rendering used by source imports and dependency edges.
    #[must_use]
    pub fn manifest_digest_review_string(&self) -> String {
        sha256_review_string(&self.manifest_digest)
    }
}

/// Decode and validate one manifest together with its exact export bytes.
///
/// # Errors
///
/// Returns stable failures for non-canonical CBOR, values outside the closed
/// lawpack schemas, an export digest mismatch, or a manifest/export invariant
/// violation. Dependency existence and cycles are checked separately once the
/// caller supplies the complete dependency set.
pub fn decode_lawpack_bundle(
    manifest_bytes: &[u8],
    exports_bytes: &[u8],
) -> Result<ValidatedLawpackBundle, Vec<LawpackValidationFailure>> {
    let manifest_value = decode_lawpack_value(manifest_bytes, LAWPACK_CBOR_PATH)?;
    let exports_value = decode_lawpack_value(exports_bytes, EXPORTS_CBOR_PATH)?;
    let manifest = parse_manifest(&manifest_value)?;
    let exports = parse_exports(&exports_value)?;

    let exports_digest =
        digest_lawpack_value(&manifest.exports.id, &exports_value, EXPORTS_CBOR_PATH)?;
    if exports_digest != manifest.exports.digest {
        return Err(one(failure(
            LawpackValidationFailureKind::ExportsDigestMismatch,
            "manifest.exports.digest",
            "digest of the exact canonical export surface",
        )));
    }
    if exports
        .effects
        .iter()
        .any(|effect| effect.execution_class == LawpackExecutionClass::Runtime)
        && manifest.target_adapters.is_empty()
    {
        return Err(one(failure(
            LawpackValidationFailureKind::RuntimeEffectWithoutTargetAdapter,
            "manifest.targetAdapters",
            "at least one digest-locked target adapter for runtime effects",
        )));
    }

    let manifest_digest =
        digest_lawpack_value(LAWPACK_API_VERSION, &manifest_value, LAWPACK_CBOR_PATH)?;
    Ok(ValidatedLawpackBundle {
        manifest,
        exports,
        manifest_digest,
        exports_digest,
    })
}

/// Validate that a complete supplied lawpack set satisfies every dependency
/// edge with an exact manifest digest and contains no cycle.
///
/// Input ordering does not affect validation: identities are canonicalized in
/// ordered maps before edges are checked.
///
/// # Errors
///
/// Returns stable failures for duplicate identities, missing or substituted
/// dependencies, self-dependencies, or dependency cycles.
pub fn validate_lawpack_dependency_graph(
    bundles: &[ValidatedLawpackBundle],
) -> Result<(), Vec<LawpackValidationFailure>> {
    let mut by_identity = BTreeMap::<(String, String), &ValidatedLawpackBundle>::new();
    for bundle in bundles {
        let key = (bundle.manifest.id.clone(), bundle.manifest.version.clone());
        if by_identity.insert(key.clone(), bundle).is_some() {
            return Err(one(failure(
                LawpackValidationFailureKind::DuplicateIdentity,
                format!("lawpacks.{}@{}", key.0, key.1),
                "one manifest per lawpack id and version",
            )));
        }
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    for identity in by_identity.keys() {
        visit_dependency(identity, &by_identity, &mut visiting, &mut visited)?;
    }

    for (identity, bundle) in &by_identity {
        for dependency in &bundle.manifest.dependencies {
            let dependency_identity = (dependency.id.clone(), dependency.version.clone());
            if dependency_identity == *identity {
                return Err(one(failure(
                    LawpackValidationFailureKind::SelfDependency,
                    format!("lawpacks.{}@{}.dependencies", identity.0, identity.1),
                    "a lawpack cannot depend on itself",
                )));
            }
            let Some(resolved) = by_identity.get(&dependency_identity) else {
                return Err(one(failure(
                    LawpackValidationFailureKind::MissingDependency,
                    format!(
                        "lawpacks.{}@{}.dependencies.{}@{}",
                        identity.0, identity.1, dependency.id, dependency.version
                    ),
                    "every dependency must be present in the supplied closed set",
                )));
            };
            if resolved.manifest_digest != dependency.digest {
                return Err(one(failure(
                    LawpackValidationFailureKind::DependencyDigestMismatch,
                    format!(
                        "lawpacks.{}@{}.dependencies.{}@{}",
                        identity.0, identity.1, dependency.id, dependency.version
                    ),
                    "dependency digest must equal the resolved manifest digest",
                )));
            }
        }
    }

    Ok(())
}

fn visit_dependency(
    identity: &(String, String),
    bundles: &BTreeMap<(String, String), &ValidatedLawpackBundle>,
    visiting: &mut BTreeSet<(String, String)>,
    visited: &mut BTreeSet<(String, String)>,
) -> Result<(), Vec<LawpackValidationFailure>> {
    if visited.contains(identity) {
        return Ok(());
    }
    if !visiting.insert(identity.clone()) {
        return Err(one(failure(
            LawpackValidationFailureKind::DependencyCycle,
            format!("lawpacks.{}@{}.dependencies", identity.0, identity.1),
            "acyclic dependency graph",
        )));
    }
    if let Some(bundle) = bundles.get(identity) {
        for dependency in &bundle.manifest.dependencies {
            let dependency_identity = (dependency.id.clone(), dependency.version.clone());
            if bundles.contains_key(&dependency_identity) {
                visit_dependency(&dependency_identity, bundles, visiting, visited)?;
            }
        }
    }
    visiting.remove(identity);
    visited.insert(identity.clone());
    Ok(())
}

fn parse_manifest(
    value: &CanonicalValue,
) -> Result<LawpackManifest, Vec<LawpackValidationFailure>> {
    let path = "manifest";
    let fields = closed_map(
        value,
        path,
        &[
            "apiVersion",
            "id",
            "version",
            "acceptedCoreAbi",
            "dependencies",
            "exports",
            "targetAdapters",
            "helperComponent",
            "verifier",
            "compatibility",
            "conformanceFixtureCorpus",
        ],
    )?;
    let (api_version, id, version, accepted_core_abi) = parse_manifest_identity(&fields, path)?;
    let dependencies = parse_dependencies(&fields, &id, &version, path)?;

    let exports = parse_resource_ref(required(&fields, "exports", path)?, "manifest.exports")?;
    let target_adapters = parse_target_adapters(&fields)?;
    let helper_component = fields
        .get("helperComponent")
        .map(|value| parse_executable_component(value, "manifest.helperComponent"))
        .transpose()?;
    let verifier = parse_verifier(required(&fields, "verifier", path)?, "manifest.verifier")?;
    let compatibility = parse_resource_ref(
        required(&fields, "compatibility", path)?,
        "manifest.compatibility",
    )?;
    let conformance_fixture_corpus = parse_resource_ref(
        required(&fields, "conformanceFixtureCorpus", path)?,
        "manifest.conformanceFixtureCorpus",
    )?;

    Ok(LawpackManifest {
        api_version,
        id,
        version,
        accepted_core_abi,
        dependencies,
        exports,
        target_adapters,
        helper_component,
        verifier,
        compatibility,
        conformance_fixture_corpus,
    })
}

fn parse_manifest_identity(
    fields: &BTreeMap<&str, &CanonicalValue>,
    path: &str,
) -> Result<(String, String, String, Vec<String>), Vec<LawpackValidationFailure>> {
    let api_version = required_text(fields, "apiVersion", path)?;
    if api_version != LAWPACK_API_VERSION {
        return Err(one(failure(
            LawpackValidationFailureKind::InvalidApiVersion,
            "manifest.apiVersion",
            LAWPACK_API_VERSION,
        )));
    }
    let id = required_nonempty_text(fields, "id", path)?;
    let version = required_nonempty_text(fields, "version", path)?;
    let accepted_core_abi = required_text_array(fields, "acceptedCoreAbi", path, true)?;
    if !accepted_core_abi.iter().any(|abi| abi == CORE_API_VERSION) {
        return Err(one(failure(
            LawpackValidationFailureKind::MissingAcceptedCoreAbi,
            "manifest.acceptedCoreAbi",
            CORE_API_VERSION,
        )));
    }
    Ok((api_version, id, version, accepted_core_abi))
}

fn parse_dependencies(
    fields: &BTreeMap<&str, &CanonicalValue>,
    manifest_id: &str,
    manifest_version: &str,
    path: &str,
) -> Result<Vec<LawpackDependency>, Vec<LawpackValidationFailure>> {
    let values = array(
        required(fields, "dependencies", path)?,
        "manifest.dependencies",
    )?;
    let mut dependencies = Vec::with_capacity(values.len());
    let mut identities = BTreeSet::new();
    for (index, dependency) in values.iter().enumerate() {
        let dependency_path = format!("manifest.dependencies[{index}]");
        let parsed = parse_dependency(dependency, &dependency_path)?;
        if !identities.insert((parsed.id.clone(), parsed.version.clone())) {
            return Err(one(failure(
                LawpackValidationFailureKind::DuplicateIdentity,
                dependency_path,
                "unique dependency id and version",
            )));
        }
        if parsed.id == manifest_id && parsed.version == manifest_version {
            return Err(one(failure(
                LawpackValidationFailureKind::SelfDependency,
                "manifest.dependencies",
                "a lawpack cannot depend on itself",
            )));
        }
        dependencies.push(parsed);
    }
    Ok(dependencies)
}

fn parse_target_adapters(
    fields: &BTreeMap<&str, &CanonicalValue>,
) -> Result<Vec<LawpackTargetAdapter>, Vec<LawpackValidationFailure>> {
    let Some(value) = fields.get("targetAdapters") else {
        return Ok(Vec::new());
    };
    let values = array(value, "manifest.targetAdapters")?;
    if values.is_empty() {
        return Err(one(failure(
            LawpackValidationFailureKind::EmptyRequiredCollection,
            "manifest.targetAdapters",
            "the optional field must contain at least one adapter when present",
        )));
    }
    let mut adapters = Vec::with_capacity(values.len());
    let mut selectors = BTreeSet::new();
    for (index, adapter) in values.iter().enumerate() {
        let adapter_path = format!("manifest.targetAdapters[{index}]");
        let parsed = parse_target_adapter(adapter, &adapter_path)?;
        let selector = (
            parsed.accepted_target_profile.id.clone(),
            parsed.accepted_target_profile.digest,
        );
        if !selectors.insert(selector) {
            return Err(one(failure(
                LawpackValidationFailureKind::DuplicateIdentity,
                adapter_path,
                "one adapter per exact accepted target-profile identity",
            )));
        }
        adapters.push(parsed);
    }
    Ok(adapters)
}

fn parse_dependency(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackDependency, Vec<LawpackValidationFailure>> {
    let fields = closed_map(value, path, &["id", "version", "digest"])?;
    Ok(LawpackDependency {
        id: required_nonempty_text(&fields, "id", path)?,
        version: required_nonempty_text(&fields, "version", path)?,
        digest: parse_digest(
            required(&fields, "digest", path)?,
            &format!("{path}.digest"),
        )?,
    })
}

fn parse_target_adapter(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackTargetAdapter, Vec<LawpackValidationFailure>> {
    let fields = closed_map(
        value,
        path,
        &["acceptedTargetProfile", "acceptedTargetIr", "adapter"],
    )?;
    Ok(LawpackTargetAdapter {
        accepted_target_profile: parse_resource_ref(
            required(&fields, "acceptedTargetProfile", path)?,
            &format!("{path}.acceptedTargetProfile"),
        )?,
        accepted_target_ir: parse_resource_ref(
            required(&fields, "acceptedTargetIr", path)?,
            &format!("{path}.acceptedTargetIr"),
        )?,
        adapter: parse_resource_ref(
            required(&fields, "adapter", path)?,
            &format!("{path}.adapter"),
        )?,
    })
}

fn parse_executable_component(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackExecutableComponent, Vec<LawpackValidationFailure>> {
    let fields = closed_map(value, path, &["component", "sandbox", "fuelModel"])?;
    Ok(LawpackExecutableComponent {
        component: parse_resource_ref(
            required(&fields, "component", path)?,
            &format!("{path}.component"),
        )?,
        sandbox: parse_resource_ref(
            required(&fields, "sandbox", path)?,
            &format!("{path}.sandbox"),
        )?,
        fuel_model: parse_resource_ref(
            required(&fields, "fuelModel", path)?,
            &format!("{path}.fuelModel"),
        )?,
    })
}

fn parse_verifier(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackVerifier, Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path)?;
    let class = required_text(&fields, "class", path)?;
    match class.as_str() {
        "declarative" => {
            ensure_allowed_fields(&fields, path, &["class", "ruleset"])?;
            Ok(LawpackVerifier::Declarative {
                ruleset: parse_resource_ref(
                    required(&fields, "ruleset", path)?,
                    &format!("{path}.ruleset"),
                )?,
            })
        }
        "executable" => {
            ensure_allowed_fields(
                &fields,
                path,
                &["class", "component", "sandbox", "fuelModel"],
            )?;
            Ok(LawpackVerifier::Executable {
                executable: LawpackExecutableComponent {
                    component: parse_resource_ref(
                        required(&fields, "component", path)?,
                        &format!("{path}.component"),
                    )?,
                    sandbox: parse_resource_ref(
                        required(&fields, "sandbox", path)?,
                        &format!("{path}.sandbox"),
                    )?,
                    fuel_model: parse_resource_ref(
                        required(&fields, "fuelModel", path)?,
                        &format!("{path}.fuelModel"),
                    )?,
                },
            })
        }
        _ => Err(one(failure(
            LawpackValidationFailureKind::InvalidDiscriminant,
            format!("{path}.class"),
            "declarative or executable",
        ))),
    }
}

fn parse_exports(value: &CanonicalValue) -> Result<LawpackExports, Vec<LawpackValidationFailure>> {
    let path = "exports";
    let fields = closed_map(
        value,
        path,
        &[
            "types",
            "constants",
            "pureFunctions",
            "effects",
            "obstructions",
            "operationProfiles",
        ],
    )?;
    let types = parse_array_field(&fields, "types", path, parse_exported_type)?;
    let constants = parse_array_field(&fields, "constants", path, parse_exported_constant)?;
    let pure_functions = parse_array_field(&fields, "pureFunctions", path, parse_pure_function)?;
    validate_pure_function_callees(&pure_functions)?;
    let effects = parse_array_field(&fields, "effects", path, parse_semantic_effect)?;
    let obstructions = parse_array_field(&fields, "obstructions", path, parse_obstruction)?;
    let operation_profiles = parse_operation_profiles(
        required(&fields, "operationProfiles", path)?,
        "exports.operationProfiles",
    )?;

    Ok(LawpackExports {
        types,
        constants,
        pure_functions,
        effects,
        obstructions,
        operation_profiles,
    })
}

fn validate_pure_function_callees(
    pure_functions: &[LawpackPureFunction],
) -> Result<(), Vec<LawpackValidationFailure>> {
    let pure_coordinates = pure_functions
        .iter()
        .map(|function| function.coordinate.as_str())
        .collect::<BTreeSet<_>>();
    for (index, function) in pure_functions.iter().enumerate() {
        if let LawpackPureFunctionImplementation::Edict { body } = &function.implementation {
            validate_pure_callees_in_value(
                body,
                &format!("exports.pureFunctions[{index}].body"),
                &pure_coordinates,
            )?;
        }
    }
    Ok(())
}

fn validate_pure_callees_in_value(
    value: &CanonicalValue,
    path: &str,
    pure_coordinates: &BTreeSet<&str>,
) -> Result<(), Vec<LawpackValidationFailure>> {
    match value {
        CanonicalValue::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                validate_pure_callees_in_value(
                    value,
                    &format!("{path}[{index}]"),
                    pure_coordinates,
                )?;
            }
        }
        CanonicalValue::Map(entries) => {
            let kind = entries.iter().find_map(|(key, value)| match (key, value) {
                (CanonicalValue::Text(key), CanonicalValue::Text(value)) if key == "kind" => {
                    Some(value.as_str())
                }
                _ => None,
            });
            if kind == Some("call") {
                let callee = entries.iter().find_map(|(key, value)| match (key, value) {
                    (CanonicalValue::Text(key), CanonicalValue::Text(value)) if key == "callee" => {
                        Some(value.as_str())
                    }
                    _ => None,
                });
                if callee.is_some_and(|callee| !pure_coordinates.contains(callee)) {
                    return Err(pure_body_failure(
                        &format!("{path}.callee"),
                        "the coordinate of an exported pure function",
                    ));
                }
            }
            for (index, (_, value)) in entries.iter().enumerate() {
                validate_pure_callees_in_value(
                    value,
                    &format!("{path}.entry[{index}]"),
                    pure_coordinates,
                )?;
            }
        }
        CanonicalValue::Null
        | CanonicalValue::Bool(_)
        | CanonicalValue::Integer(_)
        | CanonicalValue::Bytes(_)
        | CanonicalValue::Text(_) => {}
    }
    Ok(())
}

fn parse_array_field<T>(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    parent: &str,
    parse: fn(&CanonicalValue, &str) -> Result<T, Vec<LawpackValidationFailure>>,
) -> Result<Vec<T>, Vec<LawpackValidationFailure>> {
    let path = format!("{parent}.{field}");
    let values = array(required(fields, field, parent)?, &path)?;
    let mut parsed = Vec::with_capacity(values.len());
    let mut coordinates = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let item_path = format!("{path}[{index}]");
        let item = parse(value, &item_path)?;
        let coordinate = export_coordinate(value, &item_path)?;
        if !coordinates.insert(coordinate) {
            return Err(one(failure(
                LawpackValidationFailureKind::DuplicateIdentity,
                item_path,
                "unique export coordinate within the category",
            )));
        }
        parsed.push(item);
    }
    Ok(parsed)
}

fn export_coordinate(
    value: &CanonicalValue,
    path: &str,
) -> Result<String, Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path)?;
    required_nonempty_text(&fields, "coordinate", path)
}

fn parse_exported_type(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackExportedType, Vec<LawpackValidationFailure>> {
    let fields = closed_map(value, path, &["coordinate", "definition"])?;
    Ok(LawpackExportedType {
        coordinate: required_nonempty_text(&fields, "coordinate", path)?,
        definition: required_nonempty_text(&fields, "definition", path)?,
    })
}

fn parse_exported_constant(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackExportedConstant, Vec<LawpackValidationFailure>> {
    let fields = closed_map(value, path, &["coordinate", "type", "value"])?;
    Ok(LawpackExportedConstant {
        coordinate: required_nonempty_text(&fields, "coordinate", path)?,
        ty: required_nonempty_text(&fields, "type", path)?,
        value: required(&fields, "value", path)?.clone(),
    })
}

fn parse_pure_function(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackPureFunction, Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path)?;
    let source = required_text(&fields, "source", path)?;
    let implementation = match source.as_str() {
        "edict" => {
            ensure_allowed_fields(
                &fields,
                path,
                &[
                    "coordinate",
                    "typeParameters",
                    "parameterTypes",
                    "returnType",
                    "costTemplate",
                    "determinismClass",
                    "source",
                    "body",
                ],
            )?;
            let body = required(&fields, "body", path)?;
            validate_core_fn_body(body, &format!("{path}.body"))?;
            LawpackPureFunctionImplementation::Edict { body: body.clone() }
        }
        "component" => {
            ensure_allowed_fields(
                &fields,
                path,
                &[
                    "coordinate",
                    "typeParameters",
                    "parameterTypes",
                    "returnType",
                    "costTemplate",
                    "determinismClass",
                    "source",
                    "implementation",
                ],
            )?;
            LawpackPureFunctionImplementation::Component {
                implementation: parse_executable_component(
                    required(&fields, "implementation", path)?,
                    &format!("{path}.implementation"),
                )?,
            }
        }
        _ => {
            return Err(one(failure(
                LawpackValidationFailureKind::InvalidDiscriminant,
                format!("{path}.source"),
                "edict or component",
            )));
        }
    };
    let determinism_class = match required_text(&fields, "determinismClass", path)?.as_str() {
        "total" => LawpackDeterminismClass::Total,
        "total-with-typed-diagnostic" => LawpackDeterminismClass::TotalWithTypedDiagnostic,
        _ => {
            return Err(one(failure(
                LawpackValidationFailureKind::InvalidDiscriminant,
                format!("{path}.determinismClass"),
                "total or total-with-typed-diagnostic",
            )));
        }
    };
    Ok(LawpackPureFunction {
        coordinate: required_nonempty_text(&fields, "coordinate", path)?,
        type_parameters: required_text_array(&fields, "typeParameters", path, false)?,
        parameter_types: required_text_array(&fields, "parameterTypes", path, false)?,
        return_type: required_nonempty_text(&fields, "returnType", path)?,
        cost_template: required_nonempty_text(&fields, "costTemplate", path)?,
        determinism_class,
        implementation,
    })
}

fn parse_semantic_effect(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackSemanticEffect, Vec<LawpackValidationFailure>> {
    let fields = closed_map(
        value,
        path,
        &[
            "coordinate",
            "typeParameters",
            "inputType",
            "outputType",
            "executionClass",
            "effectKindHint",
            "footprintObligation",
            "costObligation",
            "effectFailures",
            "guardSupport",
        ],
    )?;
    let execution_class = match required_text(&fields, "executionClass", path)?.as_str() {
        "proofOnly" => LawpackExecutionClass::ProofOnly,
        "runtime" => LawpackExecutionClass::Runtime,
        _ => {
            return Err(one(failure(
                LawpackValidationFailureKind::InvalidDiscriminant,
                format!("{path}.executionClass"),
                "proofOnly or runtime",
            )));
        }
    };
    let effect_kind_hint = match required_text(&fields, "effectKindHint", path)?.as_str() {
        "read" => LawpackEffectKind::Read,
        "create" => LawpackEffectKind::Create,
        "ensure" => LawpackEffectKind::Ensure,
        "replace" => LawpackEffectKind::Replace,
        "delete" => LawpackEffectKind::Delete,
        "append" => LawpackEffectKind::Append,
        "reduce" => LawpackEffectKind::Reduce,
        "semantic.emit" => LawpackEffectKind::SemanticEmit,
        "custom" => LawpackEffectKind::Custom,
        _ => {
            return Err(one(failure(
                LawpackValidationFailureKind::InvalidDiscriminant,
                format!("{path}.effectKindHint"),
                "a supported semantic effect-kind hint",
            )));
        }
    };
    let effect_failures = parse_effect_failures(
        required(&fields, "effectFailures", path)?,
        &format!("{path}.effectFailures"),
    )?;
    Ok(LawpackSemanticEffect {
        coordinate: required_nonempty_text(&fields, "coordinate", path)?,
        type_parameters: required_text_array(&fields, "typeParameters", path, false)?,
        input_type: required_nonempty_text(&fields, "inputType", path)?,
        output_type: required_nonempty_text(&fields, "outputType", path)?,
        execution_class,
        effect_kind_hint,
        footprint_obligation: required_nonempty_text(&fields, "footprintObligation", path)?,
        cost_obligation: required_nonempty_text(&fields, "costObligation", path)?,
        effect_failures,
        guard_support: required_bool(&fields, "guardSupport", path)?,
    })
}

fn parse_effect_failures(
    value: &CanonicalValue,
    path: &str,
) -> Result<BTreeMap<String, LawpackEffectFailure>, Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path)?;
    let mut failures = BTreeMap::new();
    for (identifier, value) in fields {
        validate_failure_identifier(identifier, &format!("{path}.{identifier}"))?;
        let failure_path = format!("{path}.{identifier}");
        let body = closed_map(value, &failure_path, &["authorityClass", "payloadType"])?;
        failures.insert(
            identifier.to_owned(),
            LawpackEffectFailure {
                authority_class: parse_authority_class(
                    &required_text(&body, "authorityClass", &failure_path)?,
                    &format!("{failure_path}.authorityClass"),
                )?,
                payload_type: required_nonempty_text(&body, "payloadType", &failure_path)?,
            },
        );
    }
    Ok(failures)
}

fn validate_failure_identifier(
    identifier: &str,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let mut bytes = identifier.bytes();
    let valid_start = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_');
    if !valid_start || !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
        return Err(one(failure(
            LawpackValidationFailureKind::InvalidFailureIdentifier,
            path,
            "a bare Edict identifier",
        )));
    }
    if is_keyword(identifier) {
        return Err(one(failure(
            LawpackValidationFailureKind::ReservedFailureIdentifier,
            path,
            "a non-keyword Edict identifier",
        )));
    }
    Ok(())
}

fn parse_obstruction(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackObstruction, Vec<LawpackValidationFailure>> {
    let fields = closed_map(
        value,
        path,
        &["coordinate", "authorityClass", "payloadSchema"],
    )?;
    Ok(LawpackObstruction {
        coordinate: required_nonempty_text(&fields, "coordinate", path)?,
        authority_class: parse_authority_class(
            &required_text(&fields, "authorityClass", path)?,
            &format!("{path}.authorityClass"),
        )?,
        payload_schema: required_nonempty_text(&fields, "payloadSchema", path)?,
    })
}

fn parse_authority_class(
    value: &str,
    path: &str,
) -> Result<LawpackAuthorityClass, Vec<LawpackValidationFailure>> {
    match value {
        "domainMappable" => Ok(LawpackAuthorityClass::DomainMappable),
        "participantOwned" => Ok(LawpackAuthorityClass::ParticipantOwned),
        "integrityFault" => Ok(LawpackAuthorityClass::IntegrityFault),
        "resourceFault" => Ok(LawpackAuthorityClass::ResourceFault),
        "internalFault" => Ok(LawpackAuthorityClass::InternalFault),
        _ => Err(one(failure(
            LawpackValidationFailureKind::InvalidDiscriminant,
            path,
            "a supported authority class",
        ))),
    }
}

fn parse_operation_profiles(
    value: &CanonicalValue,
    path: &str,
) -> Result<BTreeMap<String, LawpackOperationProfile>, Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path)?;
    let mut profiles = BTreeMap::new();
    for (coordinate, value) in fields {
        if coordinate.is_empty() {
            return Err(one(failure(
                LawpackValidationFailureKind::EmptyIdentity,
                path,
                "non-empty operation-profile coordinate",
            )));
        }
        let profile_path = format!("{path}.{coordinate}");
        let profile = closed_map(value, &profile_path, &["opticTemplate", "effectPredicate"])?;
        profiles.insert(
            coordinate.to_owned(),
            LawpackOperationProfile {
                optic_template: parse_optic_template(
                    required(&profile, "opticTemplate", &profile_path)?,
                    &format!("{profile_path}.opticTemplate"),
                )?,
                effect_predicate: required_nonempty_text(
                    &profile,
                    "effectPredicate",
                    &profile_path,
                )?,
            },
        );
    }
    Ok(profiles)
}

fn parse_optic_template(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackOpticTemplate, Vec<LawpackValidationFailure>> {
    let fields = closed_map(
        value,
        path,
        &[
            "opticKind",
            "boundaryKind",
            "supportPolicy",
            "lossDisposition",
            "basisTemplate",
            "apertureRequirement",
        ],
    )?;
    let optic_kind = required_text(&fields, "opticKind", path)?;
    if !matches!(optic_kind.as_str(), "revelation" | "affectReintegration") {
        return Err(one(failure(
            LawpackValidationFailureKind::InvalidDiscriminant,
            format!("{path}.opticKind"),
            "revelation or affectReintegration",
        )));
    }
    let boundary_kind = required_text(&fields, "boundaryKind", path)?;
    if !matches!(boundary_kind.as_str(), "projection" | "affect") {
        return Err(one(failure(
            LawpackValidationFailureKind::InvalidDiscriminant,
            format!("{path}.boundaryKind"),
            "projection or affect",
        )));
    }
    let basis_template = fields
        .get("basisTemplate")
        .map(|value| nonempty_text(value, &format!("{path}.basisTemplate")))
        .transpose()?;
    let aperture_requirement = fields
        .get("apertureRequirement")
        .map(|value| parse_aperture_requirement(value, &format!("{path}.apertureRequirement")))
        .transpose()?;
    Ok(LawpackOpticTemplate {
        optic_kind,
        boundary_kind,
        support_policy: required_nonempty_text(&fields, "supportPolicy", path)?,
        loss_disposition: required_nonempty_text(&fields, "lossDisposition", path)?,
        basis_template,
        aperture_requirement,
    })
}

fn parse_aperture_requirement(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackApertureRequirement, Vec<LawpackValidationFailure>> {
    let fields = closed_map(value, path, &["kind", "ref"])?;
    let reference = required_nonempty_text(&fields, "ref", path)?;
    match required_text(&fields, "kind", path)?.as_str() {
        "footprintCeiling" => Ok(LawpackApertureRequirement::FootprintCeiling { reference }),
        "abstractFootprintObligation" => {
            Ok(LawpackApertureRequirement::AbstractFootprintObligation { reference })
        }
        _ => Err(one(failure(
            LawpackValidationFailureKind::InvalidDiscriminant,
            format!("{path}.kind"),
            "footprintCeiling or abstractFootprintObligation",
        ))),
    }
}

fn validate_core_fn_body(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let fields = pure_map(value, path, &["params", "body"])?;
    let params = pure_array(
        required(&fields, "params", path)?,
        &format!("{path}.params"),
    )?;
    for (index, param) in params.iter().enumerate() {
        validate_local_ref(param, &format!("{path}.params[{index}]"))?;
    }
    validate_core_pure_block(required(&fields, "body", path)?, &format!("{path}.body"))
}

fn validate_core_pure_block(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let fields = pure_map(value, path, &["locals", "bindings", "result"])?;
    let locals = pure_array(
        required(&fields, "locals", path)?,
        &format!("{path}.locals"),
    )?;
    for (index, local) in locals.iter().enumerate() {
        validate_local_ref(local, &format!("{path}.locals[{index}]"))?;
    }
    let bindings = pure_array(
        required(&fields, "bindings", path)?,
        &format!("{path}.bindings"),
    )?;
    for (index, binding) in bindings.iter().enumerate() {
        let binding_path = format!("{path}.bindings[{index}]");
        let binding_fields = pure_map(binding, &binding_path, &["kind", "binding", "value"])?;
        pure_discriminant(&binding_fields, "kind", &binding_path, "let")?;
        validate_local_ref(
            required(&binding_fields, "binding", &binding_path)?,
            &format!("{binding_path}.binding"),
        )?;
        validate_core_expr(
            required(&binding_fields, "value", &binding_path)?,
            &format!("{binding_path}.value"),
        )?;
    }
    validate_core_expr(
        required(&fields, "result", path)?,
        &format!("{path}.result"),
    )
}

fn validate_local_ref(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let fields = pure_map(value, path, &["id", "alphaName", "type"])?;
    pure_nonempty_text(&fields, "id", path)?;
    pure_nonempty_text(&fields, "alphaName", path)?;
    pure_nonempty_text(&fields, "type", path)?;
    Ok(())
}

fn validate_core_expr(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path).map_err(as_pure_body_failure)?;
    let kind = required_text(&fields, "kind", path).map_err(as_pure_body_failure)?;
    validate_core_expr_fields(&fields, &kind, path)
}

fn validate_core_expr_fields(
    fields: &BTreeMap<&str, &CanonicalValue>,
    kind: &str,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    match kind {
        "local" => {
            ensure_allowed_fields(fields, path, &["kind", "ref"]).map_err(as_pure_body_failure)?;
            validate_local_ref(
                required(fields, "ref", path).map_err(as_pure_body_failure)?,
                &format!("{path}.ref"),
            )
        }
        "const" => {
            ensure_allowed_fields(fields, path, &["kind", "value"])
                .map_err(as_pure_body_failure)?;
            validate_core_value(
                required(fields, "value", path).map_err(as_pure_body_failure)?,
                &format!("{path}.value"),
            )
        }
        "record" => {
            ensure_allowed_fields(fields, path, &["kind", "fields"])
                .map_err(as_pure_body_failure)?;
            let values = string_keyed_map(
                required(fields, "fields", path).map_err(as_pure_body_failure)?,
                &format!("{path}.fields"),
            )
            .map_err(as_pure_body_failure)?;
            for (field, value) in values {
                validate_core_expr(value, &format!("{path}.fields.{field}"))?;
            }
            Ok(())
        }
        "field" => {
            ensure_allowed_fields(fields, path, &["kind", "base", "field"])
                .map_err(as_pure_body_failure)?;
            validate_core_expr(
                required(fields, "base", path).map_err(as_pure_body_failure)?,
                &format!("{path}.base"),
            )?;
            required_nonempty_text(fields, "field", path).map_err(as_pure_body_failure)?;
            Ok(())
        }
        "variant" => {
            ensure_allowed_fields(fields, path, &["kind", "type", "case", "payload"])
                .map_err(as_pure_body_failure)?;
            required_nonempty_text(fields, "type", path).map_err(as_pure_body_failure)?;
            required_nonempty_text(fields, "case", path).map_err(as_pure_body_failure)?;
            if let Some(payload) = fields.get("payload") {
                validate_core_expr(payload, &format!("{path}.payload"))?;
            }
            Ok(())
        }
        "match" => validate_match_expr(fields, path),
        "call" => {
            ensure_allowed_fields(fields, path, &["kind", "callee", "typeArgs", "args"])
                .map_err(as_pure_body_failure)?;
            required_nonempty_text(fields, "callee", path).map_err(as_pure_body_failure)?;
            validate_text_values(
                required(fields, "typeArgs", path).map_err(as_pure_body_failure)?,
                &format!("{path}.typeArgs"),
            )?;
            validate_expr_values(
                required(fields, "args", path).map_err(as_pure_body_failure)?,
                &format!("{path}.args"),
            )
        }
        "list" => {
            ensure_allowed_fields(fields, path, &["kind", "values"])
                .map_err(as_pure_body_failure)?;
            validate_expr_values(
                required(fields, "values", path).map_err(as_pure_body_failure)?,
                &format!("{path}.values"),
            )
        }
        "map" => {
            ensure_allowed_fields(fields, path, &["kind", "entries"])
                .map_err(as_pure_body_failure)?;
            validate_expr_entries(
                required(fields, "entries", path).map_err(as_pure_body_failure)?,
                &format!("{path}.entries"),
            )
        }
        "if" => {
            ensure_allowed_fields(fields, path, &["kind", "predicate", "then", "else"])
                .map_err(as_pure_body_failure)?;
            validate_core_predicate(
                required(fields, "predicate", path).map_err(as_pure_body_failure)?,
                &format!("{path}.predicate"),
            )?;
            validate_core_expr(
                required(fields, "then", path).map_err(as_pure_body_failure)?,
                &format!("{path}.then"),
            )?;
            validate_core_expr(
                required(fields, "else", path).map_err(as_pure_body_failure)?,
                &format!("{path}.else"),
            )
        }
        _ => Err(pure_body_failure(
            path,
            "a Core pure-expression discriminant",
        )),
    }
}

fn validate_match_expr(
    fields: &BTreeMap<&str, &CanonicalValue>,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    ensure_allowed_fields(fields, path, &["kind", "scrutinee", "arms"])
        .map_err(as_pure_body_failure)?;
    validate_core_expr(
        required(fields, "scrutinee", path).map_err(as_pure_body_failure)?,
        &format!("{path}.scrutinee"),
    )?;
    let arms = pure_array(
        required(fields, "arms", path).map_err(as_pure_body_failure)?,
        &format!("{path}.arms"),
    )?;
    if arms.is_empty() {
        return Err(pure_body_failure(path, "at least one match arm"));
    }
    for (index, arm) in arms.iter().enumerate() {
        let arm_path = format!("{path}.arms[{index}]");
        let arm_fields = pure_map(arm, &arm_path, &["case", "binder", "body"])?;
        pure_nonempty_text(&arm_fields, "case", &arm_path)?;
        if let Some(binder) = arm_fields.get("binder") {
            validate_local_ref(binder, &format!("{arm_path}.binder"))?;
        }
        validate_core_expr(
            required(&arm_fields, "body", &arm_path).map_err(as_pure_body_failure)?,
            &format!("{arm_path}.body"),
        )?;
    }
    Ok(())
}

fn validate_core_predicate(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path).map_err(as_pure_body_failure)?;
    let kind = required_text(&fields, "kind", path).map_err(as_pure_body_failure)?;
    match kind.as_str() {
        "true" | "false" => {
            ensure_allowed_fields(&fields, path, &["kind"]).map_err(as_pure_body_failure)
        }
        "not" => {
            ensure_allowed_fields(&fields, path, &["kind", "value"])
                .map_err(as_pure_body_failure)?;
            validate_core_predicate(
                required(&fields, "value", path).map_err(as_pure_body_failure)?,
                &format!("{path}.value"),
            )
        }
        "all" | "any" => {
            ensure_allowed_fields(&fields, path, &["kind", "values"])
                .map_err(as_pure_body_failure)?;
            let values = pure_array(
                required(&fields, "values", path).map_err(as_pure_body_failure)?,
                &format!("{path}.values"),
            )?;
            if values.is_empty() {
                return Err(pure_body_failure(path, "at least one predicate"));
            }
            for (index, predicate) in values.iter().enumerate() {
                validate_core_predicate(predicate, &format!("{path}.values[{index}]"))?;
            }
            Ok(())
        }
        "compare" => {
            ensure_allowed_fields(&fields, path, &["kind", "op", "left", "right"])
                .map_err(as_pure_body_failure)?;
            let op = required_text(&fields, "op", path).map_err(as_pure_body_failure)?;
            if !matches!(op.as_str(), "==" | "!=" | "<" | "<=" | ">" | ">=") {
                return Err(pure_body_failure(
                    &format!("{path}.op"),
                    "a Core comparison operator",
                ));
            }
            validate_core_expr(
                required(&fields, "left", path).map_err(as_pure_body_failure)?,
                &format!("{path}.left"),
            )?;
            validate_core_expr(
                required(&fields, "right", path).map_err(as_pure_body_failure)?,
                &format!("{path}.right"),
            )
        }
        "call" => {
            ensure_allowed_fields(&fields, path, &["kind", "predicate", "args"])
                .map_err(as_pure_body_failure)?;
            required_nonempty_text(&fields, "predicate", path).map_err(as_pure_body_failure)?;
            validate_expr_values(
                required(&fields, "args", path).map_err(as_pure_body_failure)?,
                &format!("{path}.args"),
            )
        }
        "obstruction" => {
            ensure_allowed_fields(&fields, path, &["kind", "coordinate", "payload"])
                .map_err(as_pure_body_failure)?;
            let coordinate = required_nonempty_text(&fields, "coordinate", path)
                .map_err(as_pure_body_failure)?;
            validate_failure_identifier(&coordinate, &format!("{path}.coordinate"))
                .map_err(as_pure_body_failure)?;
            validate_core_expr(
                required(&fields, "payload", path).map_err(as_pure_body_failure)?,
                &format!("{path}.payload"),
            )
        }
        _ => Err(pure_body_failure(path, "a Core predicate discriminant")),
    }
}

fn validate_core_value(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path).map_err(as_pure_body_failure)?;
    let kind = required_text(&fields, "kind", path).map_err(as_pure_body_failure)?;
    validate_core_value_fields(&fields, &kind, path)
}

fn validate_core_value_fields(
    fields: &BTreeMap<&str, &CanonicalValue>,
    kind: &str,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    match kind {
        "null" | "bool" | "int" | "string" | "bytes" | "capability" => {
            validate_core_scalar_value(fields, kind, path)
        }
        "record" => {
            ensure_allowed_fields(fields, path, &["kind", "fields"])
                .map_err(as_pure_body_failure)?;
            let values = string_keyed_map(
                required(fields, "fields", path).map_err(as_pure_body_failure)?,
                &format!("{path}.fields"),
            )
            .map_err(as_pure_body_failure)?;
            for (field, value) in values {
                validate_core_value(value, &format!("{path}.fields.{field}"))?;
            }
            Ok(())
        }
        "variant" => {
            ensure_allowed_fields(fields, path, &["kind", "type", "case", "payload"])
                .map_err(as_pure_body_failure)?;
            required_nonempty_text(fields, "type", path).map_err(as_pure_body_failure)?;
            required_nonempty_text(fields, "case", path).map_err(as_pure_body_failure)?;
            if let Some(payload) = fields.get("payload") {
                validate_core_value(payload, &format!("{path}.payload"))?;
            }
            Ok(())
        }
        "list" => {
            ensure_allowed_fields(fields, path, &["kind", "values"])
                .map_err(as_pure_body_failure)?;
            let values = pure_array(
                required(fields, "values", path).map_err(as_pure_body_failure)?,
                &format!("{path}.values"),
            )?;
            for (index, value) in values.iter().enumerate() {
                validate_core_value(value, &format!("{path}.values[{index}]"))?;
            }
            Ok(())
        }
        "map" => {
            ensure_allowed_fields(fields, path, &["kind", "entries"])
                .map_err(as_pure_body_failure)?;
            let entries = pure_array(
                required(fields, "entries", path).map_err(as_pure_body_failure)?,
                &format!("{path}.entries"),
            )?;
            for (index, entry) in entries.iter().enumerate() {
                let entry_path = format!("{path}.entries[{index}]");
                let pair = pure_array(entry, &entry_path)?;
                if pair.len() != 2 {
                    return Err(pure_body_failure(&entry_path, "a key/value pair"));
                }
                validate_core_value(&pair[0], &format!("{entry_path}[0]"))?;
                validate_core_value(&pair[1], &format!("{entry_path}[1]"))?;
            }
            Ok(())
        }
        _ => Err(pure_body_failure(
            path,
            "a Core canonical-value discriminant",
        )),
    }
}

fn validate_core_scalar_value(
    fields: &BTreeMap<&str, &CanonicalValue>,
    kind: &str,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    match kind {
        "null" => ensure_allowed_fields(fields, path, &["kind"]).map_err(as_pure_body_failure),
        "bool" => {
            ensure_allowed_fields(fields, path, &["kind", "value"])
                .map_err(as_pure_body_failure)?;
            required_bool(fields, "value", path)
                .map(|_| ())
                .map_err(as_pure_body_failure)
        }
        "int" => {
            ensure_allowed_fields(fields, path, &["kind", "width", "value"])
                .map_err(as_pure_body_failure)?;
            required_nonempty_text(fields, "width", path).map_err(as_pure_body_failure)?;
            match required(fields, "value", path).map_err(as_pure_body_failure)? {
                CanonicalValue::Integer(_) => Ok(()),
                _ => Err(pure_body_failure(
                    &format!("{path}.value"),
                    "a Core integer",
                )),
            }
        }
        "string" => {
            ensure_allowed_fields(fields, path, &["kind", "value"])
                .map_err(as_pure_body_failure)?;
            required_text(fields, "value", path)
                .map(|_| ())
                .map_err(as_pure_body_failure)
        }
        "bytes" => {
            ensure_allowed_fields(fields, path, &["kind", "value"])
                .map_err(as_pure_body_failure)?;
            match required(fields, "value", path).map_err(as_pure_body_failure)? {
                CanonicalValue::Bytes(_) => Ok(()),
                _ => Err(pure_body_failure(
                    &format!("{path}.value"),
                    "Core byte string",
                )),
            }
        }
        "capability" => {
            ensure_allowed_fields(fields, path, &["kind", "receipt"])
                .map_err(as_pure_body_failure)?;
            parse_digest(
                required(fields, "receipt", path).map_err(as_pure_body_failure)?,
                &format!("{path}.receipt"),
            )
            .map(|_| ())
            .map_err(as_pure_body_failure)
        }
        _ => Err(pure_body_failure(path, "a scalar Core value")),
    }
}

fn validate_text_values(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let values = pure_array(value, path)?;
    for (index, value) in values.iter().enumerate() {
        nonempty_text(value, &format!("{path}[{index}]")).map_err(as_pure_body_failure)?;
    }
    Ok(())
}

fn validate_expr_values(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let values = pure_array(value, path)?;
    for (index, value) in values.iter().enumerate() {
        validate_core_expr(value, &format!("{path}[{index}]"))?;
    }
    Ok(())
}

fn validate_expr_entries(
    value: &CanonicalValue,
    path: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let entries = pure_array(value, path)?;
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = format!("{path}[{index}]");
        let pair = pure_array(entry, &entry_path)?;
        if pair.len() != 2 {
            return Err(pure_body_failure(&entry_path, "a key/value pair"));
        }
        validate_core_expr(&pair[0], &format!("{entry_path}[0]"))?;
        validate_core_expr(&pair[1], &format!("{entry_path}[1]"))?;
    }
    Ok(())
}

fn pure_map<'a>(
    value: &'a CanonicalValue,
    path: &str,
    allowed: &[&str],
) -> Result<BTreeMap<&'a str, &'a CanonicalValue>, Vec<LawpackValidationFailure>> {
    closed_map(value, path, allowed).map_err(as_pure_body_failure)
}

fn pure_array<'a>(
    value: &'a CanonicalValue,
    path: &str,
) -> Result<&'a [CanonicalValue], Vec<LawpackValidationFailure>> {
    array(value, path).map_err(as_pure_body_failure)
}

fn pure_nonempty_text(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<String, Vec<LawpackValidationFailure>> {
    required_nonempty_text(fields, field, path).map_err(as_pure_body_failure)
}

fn pure_discriminant(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
    expected: &str,
) -> Result<(), Vec<LawpackValidationFailure>> {
    let actual = required_text(fields, field, path).map_err(as_pure_body_failure)?;
    if actual == expected {
        Ok(())
    } else {
        Err(pure_body_failure(&format!("{path}.{field}"), expected))
    }
}

fn as_pure_body_failure(failures: Vec<LawpackValidationFailure>) -> Vec<LawpackValidationFailure> {
    failures
        .into_iter()
        .map(|failure| LawpackValidationFailure {
            kind: LawpackValidationFailureKind::InvalidPureFunctionBody,
            path: failure.path,
            obligation: failure.obligation,
        })
        .collect()
}

fn pure_body_failure(path: &str, obligation: &str) -> Vec<LawpackValidationFailure> {
    one(failure(
        LawpackValidationFailureKind::InvalidPureFunctionBody,
        path,
        obligation,
    ))
}

fn decode_lawpack_value(
    bytes: &[u8],
    path: &str,
) -> Result<CanonicalValue, Vec<LawpackValidationFailure>> {
    decode_canonical_cbor(bytes).map_err(|error| {
        let obligation = if error.kind() == CanonicalErrorKind::DuplicateMapKey {
            "canonical CBOR with unique map keys"
        } else {
            "edict.canonical-cbor/v1"
        };
        one(failure(
            LawpackValidationFailureKind::InvalidCanonicalCbor,
            path,
            obligation,
        ))
    })
}

fn digest_lawpack_value(
    domain: &str,
    value: &CanonicalValue,
    path: &str,
) -> Result<[u8; 32], Vec<LawpackValidationFailure>> {
    digest_canonical_value(domain, value).map_err(|_error| {
        one(failure(
            LawpackValidationFailureKind::InvalidCanonicalCbor,
            path,
            "domain-framed canonical SHA-256 digest",
        ))
    })
}

fn parse_resource_ref(
    value: &CanonicalValue,
    path: &str,
) -> Result<LawpackResourceRef, Vec<LawpackValidationFailure>> {
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
) -> Result<[u8; 32], Vec<LawpackValidationFailure>> {
    let CanonicalValue::Array(parts) = value else {
        return Err(one(failure(
            LawpackValidationFailureKind::InvalidDigest,
            path,
            "['sha256', 32-byte bstr]",
        )));
    };
    let [CanonicalValue::Text(algorithm), CanonicalValue::Bytes(bytes)] = parts.as_slice() else {
        return Err(one(failure(
            LawpackValidationFailureKind::InvalidDigest,
            path,
            "['sha256', 32-byte bstr]",
        )));
    };
    if algorithm != "sha256" || bytes.len() != 32 {
        return Err(one(failure(
            LawpackValidationFailureKind::InvalidDigest,
            path,
            "['sha256', 32-byte bstr]",
        )));
    }
    let mut digest = [0u8; 32];
    digest.copy_from_slice(bytes);
    Ok(digest)
}

fn required<'a>(
    fields: &BTreeMap<&str, &'a CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<&'a CanonicalValue, Vec<LawpackValidationFailure>> {
    fields.get(field).copied().ok_or_else(|| {
        one(failure(
            LawpackValidationFailureKind::MissingField,
            format!("{path}.{field}"),
            "required field",
        ))
    })
}

fn required_text(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<String, Vec<LawpackValidationFailure>> {
    text_value(required(fields, field, path)?, &format!("{path}.{field}"))
}

fn required_nonempty_text(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<String, Vec<LawpackValidationFailure>> {
    nonempty_text(required(fields, field, path)?, &format!("{path}.{field}"))
}

fn required_bool(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
) -> Result<bool, Vec<LawpackValidationFailure>> {
    match required(fields, field, path)? {
        CanonicalValue::Bool(value) => Ok(*value),
        _ => Err(one(failure(
            LawpackValidationFailureKind::InvalidShape,
            format!("{path}.{field}"),
            "boolean",
        ))),
    }
}

fn required_text_array(
    fields: &BTreeMap<&str, &CanonicalValue>,
    field: &str,
    path: &str,
    nonempty: bool,
) -> Result<Vec<String>, Vec<LawpackValidationFailure>> {
    let field_path = format!("{path}.{field}");
    let values = array(required(fields, field, path)?, &field_path)?;
    if nonempty && values.is_empty() {
        return Err(one(failure(
            LawpackValidationFailureKind::EmptyRequiredCollection,
            field_path,
            "at least one value",
        )));
    }
    let mut result = Vec::with_capacity(values.len());
    let mut unique = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let item_path = format!("{field_path}[{index}]");
        let item = nonempty_text(value, &item_path)?;
        if !unique.insert(item.clone()) {
            return Err(one(failure(
                LawpackValidationFailureKind::DuplicateIdentity,
                item_path,
                "unique text values",
            )));
        }
        result.push(item);
    }
    Ok(result)
}

fn text_value(value: &CanonicalValue, path: &str) -> Result<String, Vec<LawpackValidationFailure>> {
    match value {
        CanonicalValue::Text(value) => Ok(value.clone()),
        _ => Err(one(failure(
            LawpackValidationFailureKind::InvalidShape,
            path,
            "text string",
        ))),
    }
}

fn nonempty_text(
    value: &CanonicalValue,
    path: &str,
) -> Result<String, Vec<LawpackValidationFailure>> {
    let text = text_value(value, path)?;
    if text.is_empty() {
        Err(one(failure(
            LawpackValidationFailureKind::EmptyIdentity,
            path,
            "non-empty text string",
        )))
    } else {
        Ok(text)
    }
}

fn array<'a>(
    value: &'a CanonicalValue,
    path: &str,
) -> Result<&'a [CanonicalValue], Vec<LawpackValidationFailure>> {
    match value {
        CanonicalValue::Array(values) => Ok(values),
        _ => Err(one(failure(
            LawpackValidationFailureKind::InvalidShape,
            path,
            "array",
        ))),
    }
}

fn closed_map<'a>(
    value: &'a CanonicalValue,
    path: &str,
    allowed: &[&str],
) -> Result<BTreeMap<&'a str, &'a CanonicalValue>, Vec<LawpackValidationFailure>> {
    let fields = string_keyed_map(value, path)?;
    ensure_allowed_fields(&fields, path, allowed)?;
    Ok(fields)
}

fn string_keyed_map<'a>(
    value: &'a CanonicalValue,
    path: &str,
) -> Result<BTreeMap<&'a str, &'a CanonicalValue>, Vec<LawpackValidationFailure>> {
    let CanonicalValue::Map(entries) = value else {
        return Err(one(failure(
            LawpackValidationFailureKind::InvalidShape,
            path,
            "map with text keys",
        )));
    };
    let mut fields = BTreeMap::new();
    for (key, value) in entries {
        let CanonicalValue::Text(key) = key else {
            return Err(one(failure(
                LawpackValidationFailureKind::InvalidShape,
                path,
                "map with text keys",
            )));
        };
        if fields.insert(key.as_str(), value).is_some() {
            return Err(one(failure(
                LawpackValidationFailureKind::DuplicateIdentity,
                format!("{path}.{key}"),
                "unique map key",
            )));
        }
    }
    Ok(fields)
}

fn ensure_allowed_fields(
    fields: &BTreeMap<&str, &CanonicalValue>,
    path: &str,
    allowed: &[&str],
) -> Result<(), Vec<LawpackValidationFailure>> {
    if let Some(field) = fields
        .keys()
        .find(|field| !allowed.iter().any(|allowed| field == &allowed))
    {
        Err(one(failure(
            LawpackValidationFailureKind::UnexpectedField,
            format!("{path}.{field}"),
            "closed schema with no unknown fields",
        )))
    } else {
        Ok(())
    }
}

fn failure(
    kind: LawpackValidationFailureKind,
    path: impl Into<String>,
    obligation: impl Into<String>,
) -> LawpackValidationFailure {
    LawpackValidationFailure {
        kind,
        path: path.into(),
        obligation: obligation.into(),
    }
}

fn one(failure: LawpackValidationFailure) -> Vec<LawpackValidationFailure> {
    vec![failure]
}
