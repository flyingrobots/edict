use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use edict_provider_host_wasmtime::{
    ProviderComponentHost, ProviderHostLimits, ResolvedProviderComponent,
};
use edict_provider_schema::{ProviderArtifactSchemaRegistry, ResolvedProviderSchemaArtifact};
use edict_syntax::{
    bind_target_provider_manifest, compile_to_core, decode_canonical_cbor, decode_lawpack_adapter,
    decode_lawpack_bundle, digest_canonical_artifact, encode_canonical_cbor, encode_core_module,
    encode_target_ir_artifact, lower_to_target_ir, parse_module, prepare_lawpack_compilation,
    select_provider_component, validate_lawpack_dependency_graph,
    validate_provider_lowering_request, validate_provider_verification_request,
    verify_result_projection, CanonicalValue, ProviderArtifact, ProviderArtifactBinding,
    ProviderArtifactKind, ProviderArtifactRef, ProviderBoundArtifact, ProviderDigest,
    ProviderDigestAlgorithm, ProviderInvocationKind, ProviderLoweringInvocationContract,
    ProviderLoweringOutputKind, ProviderLoweringOutputRequest, ProviderLoweringRequest,
    ProviderResourceRef, ProviderResponseLimits, ProviderSemanticInput,
    ProviderSemanticInputBinding, ProviderSemanticInputKind,
    ProviderVerificationInvocationContract, ProviderVerificationOutputKind,
    ProviderVerificationOutputRequest, ProviderVerificationRequest, ResultProjectionArtifact,
    TargetLoweringStatus, TargetProviderManifest, ValidatedLawpackBundle,
    ValidatedTargetProviderManifest, CORE_MODULE_DIGEST_DOMAIN, PROVIDER_LAWPACK_ARTIFACT_DOMAIN,
    RESULT_PROJECTION_DIGEST_DOMAIN, TARGET_IR_ARTIFACT_DIGEST_DOMAIN, TARGET_PROFILE_API_VERSION,
    TARGET_PROVIDER_PROTOCOL_VERSION,
};
use serde::Deserialize;

const APPLICATION_SCHEMA: &str = "edict.application/v1";
const SOURCE_DOMAIN: &str = "edict.source/v1";
const EXPORTS_DOMAIN: &str = "edict.lawpack-exports/v1";
const ADAPTER_DOMAIN: &str = "edict.lawpack-adapter/v1";
const PACKAGE_ROLE: &str = "executable-operation-package.echo";
const PACKAGE_DOMAIN: &str = "echo.operation-package/v1";
const VERIFICATION_REPORT_ROLE: &str = "verifier-report.echo-operation";
const VERIFICATION_REPORT_DOMAIN: &str = "echo.operation-package-verifier-report/v1";
const RESULT_PROJECTION_ROLE: &str = "07-result-projection";
const MAX_APPLICATION_ARTIFACT_BYTES: u64 = 1024 * 1024;

#[derive(Debug)]
pub(crate) struct ApplicationBuildFailure {
    pub(crate) kind: &'static str,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplicationManifest {
    schema: String,
    coordinate: String,
    sources: Vec<PathBuf>,
    lawpacks: Vec<ApplicationLawpack>,
    target: ApplicationTarget,
    output_directory: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplicationLawpack {
    manifest: PathBuf,
    exports: PathBuf,
    adapter: PathBuf,
    target_configuration: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplicationTarget {
    profile: String,
    provider_package: PathBuf,
}

struct LoadedLawpack {
    manifest_bytes: Vec<u8>,
    exports_bytes: Vec<u8>,
    adapter_bytes: Vec<u8>,
    configuration_bytes: Vec<u8>,
    bundle: ValidatedLawpackBundle,
}

struct ProviderInvocationContext<'a> {
    coordinate: &'a str,
    core_bytes: &'a [u8],
    target_profile: &'a ProviderBoundArtifact,
    loaded: &'a LoadedLawpack,
    adapter: &'a edict_syntax::ValidatedLawpackAdapter,
    source_bytes: &'a [u8],
    target_ir_bytes: &'a [u8],
    result_projection: &'a ResultProjectionArtifact,
}

#[allow(
    clippy::too_many_lines,
    reason = "the application build keeps its ordered authority-boundary crossing explicit"
)]
pub(crate) fn build_application(config_path: &Path) -> Result<(), ApplicationBuildFailure> {
    let config_bytes = read(
        config_path,
        "ApplicationConfigReadFailed",
        "application config",
    )?;
    let config = serde_json::from_slice::<ApplicationManifest>(&config_bytes).map_err(|error| {
        failure(
            "InvalidApplicationConfig",
            format!(
                "application config `{}` is not valid edict.application/v1 JSON: {error}",
                config_path.display()
            ),
        )
    })?;
    validate_application_manifest(&config)?;
    let root = canonical_application_root(config_path)?;

    let source = config.sources.first().ok_or_else(|| {
        failure(
            "InvalidApplicationConfig",
            "the executable-operation build requires exactly one Edict source",
        )
    })?;
    let source_path = confined_existing_path(
        &root,
        source,
        "sources",
        "ApplicationSourceReadFailed",
        "Edict source",
    )?;
    let source_bytes = read(&source_path, "ApplicationSourceReadFailed", "Edict source")?;
    let source = std::str::from_utf8(&source_bytes).map_err(|error| {
        failure(
            "InvalidApplicationSource",
            format!(
                "Edict source `{}` is not UTF-8: {error}",
                source_path.display()
            ),
        )
    })?;
    let module = parse_module(source).map_err(|error| {
        failure(
            "InvalidApplicationSource",
            format!(
                "Edict source `{}` did not parse: {error:?}",
                source_path.display()
            ),
        )
    })?;

    let mut loaded_lawpacks = Vec::with_capacity(config.lawpacks.len());
    for lawpack in &config.lawpacks {
        loaded_lawpacks.push(load_lawpack(&root, lawpack)?);
    }
    let bundles = loaded_lawpacks
        .iter()
        .map(|loaded| loaded.bundle.clone())
        .collect::<Vec<_>>();
    validate_lawpack_dependency_graph(&bundles).map_err(|failures| {
        failure(
            "InvalidLawpackClosure",
            format!("application lawpack dependency closure is invalid: {failures:?}"),
        )
    })?;
    let loaded = loaded_lawpacks.first().ok_or_else(|| {
        failure(
            "InvalidApplicationConfig",
            "the executable-operation build requires a root lawpack",
        )
    })?;

    let provider_root = confined_existing_path(
        &root,
        &config.target.provider_package,
        "target.providerPackage",
        "ProviderPackageReadFailed",
        "provider package",
    )?;
    let manifest_path = find_provider_manifest(&provider_root)?;
    let provider_manifest_bytes = read(
        &manifest_path,
        "ProviderPackageReadFailed",
        "provider manifest",
    )?;
    let provider_manifest = serde_json::from_slice::<TargetProviderManifest>(
        &provider_manifest_bytes,
    )
    .map_err(|error| {
        failure(
            "InvalidProviderPackage",
            format!(
                "provider manifest `{}` is not valid typed JSON: {error}",
                manifest_path.display()
            ),
        )
    })?;
    let provider_proof = bind_target_provider_manifest(&provider_manifest).map_err(|report| {
        failure(
            "InvalidProviderPackage",
            format!("provider manifest validation failed: {report:?}"),
        )
    })?;
    let target_profile_artifact = unique_artifact(
        &provider_manifest,
        ProviderArtifactKind::TargetProfile,
        Some(&config.target.profile),
    )?;
    let target_profile_bytes = read_provider_artifact(
        &provider_root,
        target_profile_artifact,
        ProviderArtifactKind::TargetProfile,
    )?;
    let target_profile = bound_artifact(
        &target_profile_artifact.resource.coordinate,
        TARGET_PROFILE_API_VERSION,
        &target_profile_bytes,
    )?;
    require_manifest_identity(target_profile_artifact, &target_profile)?;

    let schema_artifacts = provider_schema_artifacts(&provider_manifest)?;
    let resolved_schema_artifacts = schema_artifacts
        .into_iter()
        .map(|schema_artifact| {
            Ok(ResolvedProviderSchemaArtifact {
                role: schema_artifact.role.clone(),
                bytes: Arc::<[u8]>::from(read_provider_artifact(
                    &provider_root,
                    schema_artifact,
                    ProviderArtifactKind::ArtifactSchema,
                )?),
            })
        })
        .collect::<Result<Vec<_>, ApplicationBuildFailure>>()?;
    let required_domains = provider_manifest
        .schema_bindings
        .iter()
        .map(|binding| binding.domain.as_str());
    let registry = ProviderArtifactSchemaRegistry::from_manifest(
        &provider_proof,
        resolved_schema_artifacts,
        required_domains,
    )
    .map_err(|error| {
        failure(
            "InvalidProviderPackage",
            format!("provider artifact-schema registry failed: {error}"),
        )
    })?;

    let adapter = decode_lawpack_adapter(
        &loaded.bundle,
        &config.target.profile,
        &loaded.adapter_bytes,
    )
    .map_err(|failures| {
        failure(
            "InvalidLawpackAdapter",
            format!("lawpack target adapter is invalid: {failures:?}"),
        )
    })?;
    if adapter.target_profile().digest_review_string()
        != rendered_digest(&target_profile.reference.digest)
    {
        return Err(failure(
            "TargetProfileMismatch",
            "lawpack adapter target profile digest does not match the selected provider profile",
        ));
    }
    validate_target_configuration_binding(&adapter, &loaded.configuration_bytes)?;

    let preparation =
        prepare_lawpack_compilation(&module, &loaded.bundle, &adapter).map_err(|failures| {
            failure(
                "InvalidApplicationClosure",
                format!("source and lawpack closure do not corroborate: {failures:?}"),
            )
        })?;
    let core = compile_to_core(&module, preparation.compiler_context()).map_err(|error| {
        failure(
            "ApplicationCompilationFailed",
            format!("Edict application did not compile to Core: {error:?}"),
        )
    })?;
    if core.coordinate != config.coordinate {
        return Err(failure(
            "ApplicationCoordinateMismatch",
            format!(
                "application config coordinate `{}` does not match source coordinate `{}`",
                config.coordinate, core.coordinate
            ),
        ));
    }
    let target_ir_report = lower_to_target_ir(&core, preparation.target_ir_facts());
    if target_ir_report.status != TargetLoweringStatus::Lowered {
        return Err(failure(
            "TargetLoweringFailed",
            format!(
                "application did not lower through the selected target adapter: {:?}",
                target_ir_report.failures
            ),
        ));
    }
    let (result_intent, result_projection) = single_result_projection(
        &target_ir_report.result_projections,
        &target_ir_report.result_projection_failures,
    )?;
    let target_ir = target_ir_report.artifact.ok_or_else(|| {
        failure(
            "TargetLoweringFailed",
            "target lowering reported success without an artifact",
        )
    })?;
    verify_result_projection(
        &core,
        &target_ir,
        result_intent,
        result_projection.canonical_bytes(),
        result_projection.digest(),
    )
    .map_err(|error| {
        failure(
            "ResultProjectionVerificationFailed",
            format!("compiler result projection failed independent verification: {error}"),
        )
    })?;

    let core_bytes = encode_core_module(&core).map_err(|error| {
        failure(
            "ApplicationEncodingFailed",
            format!("Core canonical encoding failed: {error}"),
        )
    })?;
    let target_ir_bytes = encode_target_ir_artifact(&target_ir).map_err(|error| {
        failure(
            "ApplicationEncodingFailed",
            format!("Target IR canonical encoding failed: {error}"),
        )
    })?;
    let source_artifact_bytes = encode_canonical_cbor(&CanonicalValue::Bytes(source_bytes))
        .map_err(|error| {
            failure(
                "ApplicationEncodingFailed",
                format!("source canonical encoding failed: {error}"),
            )
        })?;

    let lowerer_artifact =
        unique_artifact(&provider_manifest, ProviderArtifactKind::Lowerer, None)?;
    let verifier_artifact =
        unique_artifact(&provider_manifest, ProviderArtifactKind::Verifier, None)?;
    let lowerer_bytes = read_provider_artifact(
        &provider_root,
        lowerer_artifact,
        ProviderArtifactKind::Lowerer,
    )?;
    let verifier_bytes = read_provider_artifact(
        &provider_root,
        verifier_artifact,
        ProviderArtifactKind::Verifier,
    )?;

    let host = ProviderComponentHost::new().map_err(|error| {
        failure(
            "ProviderHostFailed",
            format!("provider host configuration failed: {error}"),
        )
    })?;
    let invocation = ProviderInvocationContext {
        coordinate: &core.coordinate,
        core_bytes: &core_bytes,
        target_profile: &target_profile,
        loaded,
        adapter: &adapter,
        source_bytes: &source_artifact_bytes,
        target_ir_bytes: &target_ir_bytes,
        result_projection,
    };
    let package_bytes = invoke_lowerer(
        &host,
        provider_proof,
        &registry,
        lowerer_artifact,
        lowerer_bytes,
        &invocation,
    )?;
    let report_bytes = invoke_verifier(
        &host,
        provider_proof,
        &registry,
        verifier_artifact,
        verifier_bytes,
        &invocation,
        &package_bytes,
    )?;
    require_accepted_report(&report_bytes)?;

    let output_directory = prepare_output_directory(&root, &config.output_directory)?;
    write_outputs(&output_directory, &package_bytes, &report_bytes)
}

fn validate_application_manifest(
    config: &ApplicationManifest,
) -> Result<(), ApplicationBuildFailure> {
    if config.schema != APPLICATION_SCHEMA {
        return Err(failure(
            "InvalidApplicationConfig",
            format!(
                "application schema must be `{APPLICATION_SCHEMA}`, got `{}`",
                config.schema
            ),
        ));
    }
    if config.coordinate.is_empty()
        || config.target.profile.is_empty()
        || config.target.provider_package.as_os_str().is_empty()
        || config.output_directory.as_os_str().is_empty()
    {
        return Err(failure(
            "InvalidApplicationConfig",
            "application coordinate, target profile, provider package, and output directory must be non-empty",
        ));
    }
    if config.sources.len() != 1 {
        return Err(failure(
            "InvalidApplicationConfig",
            "the executable-operation build currently requires exactly one Edict source",
        ));
    }
    if config.lawpacks.is_empty() {
        return Err(failure(
            "InvalidApplicationConfig",
            "the executable-operation build requires one root lawpack followed by its complete dependency closure",
        ));
    }
    let paths = config
        .sources
        .iter()
        .map(|path| ("sources", path.as_path()))
        .chain(config.lawpacks.iter().flat_map(|lawpack| {
            [
                ("lawpacks.manifest", lawpack.manifest.as_path()),
                ("lawpacks.exports", lawpack.exports.as_path()),
                ("lawpacks.adapter", lawpack.adapter.as_path()),
                (
                    "lawpacks.targetConfiguration",
                    lawpack.target_configuration.as_path(),
                ),
            ]
        }))
        .chain([
            (
                "target.providerPackage",
                config.target.provider_package.as_path(),
            ),
            ("outputDirectory", config.output_directory.as_path()),
        ]);
    for (field, path) in paths {
        validate_application_path(field, path)?;
    }
    Ok(())
}

fn validate_application_path(field: &str, path: &Path) -> Result<(), ApplicationBuildFailure> {
    if path.as_os_str().is_empty()
        || path.components().any(|component| {
            matches!(
                component,
                Component::Prefix(_) | Component::RootDir | Component::ParentDir
            )
        })
    {
        return Err(failure(
            "InvalidApplicationConfig",
            format!(
                "application field `{field}` must be a non-empty relative path without parent traversal"
            ),
        ));
    }
    Ok(())
}

fn load_lawpack(
    root: &Path,
    config: &ApplicationLawpack,
) -> Result<LoadedLawpack, ApplicationBuildFailure> {
    let manifest_path = confined_existing_path(
        root,
        &config.manifest,
        "lawpacks.manifest",
        "LawpackReadFailed",
        "lawpack manifest",
    )?;
    let manifest_bytes = read(&manifest_path, "LawpackReadFailed", "lawpack manifest")?;
    let exports_path = confined_existing_path(
        root,
        &config.exports,
        "lawpacks.exports",
        "LawpackReadFailed",
        "lawpack exports",
    )?;
    let exports_bytes = read(&exports_path, "LawpackReadFailed", "lawpack exports")?;
    let adapter_path = confined_existing_path(
        root,
        &config.adapter,
        "lawpacks.adapter",
        "LawpackReadFailed",
        "lawpack target adapter",
    )?;
    let adapter_bytes = read(&adapter_path, "LawpackReadFailed", "lawpack target adapter")?;
    let configuration_path = confined_existing_path(
        root,
        &config.target_configuration,
        "lawpacks.targetConfiguration",
        "LawpackReadFailed",
        "target configuration",
    )?;
    let configuration_bytes = read(
        &configuration_path,
        "LawpackReadFailed",
        "target configuration",
    )?;
    let bundle = decode_lawpack_bundle(&manifest_bytes, &exports_bytes).map_err(|failures| {
        failure(
            "InvalidLawpackClosure",
            format!("lawpack manifest and exports are invalid: {failures:?}"),
        )
    })?;
    Ok(LoadedLawpack {
        manifest_bytes,
        exports_bytes,
        adapter_bytes,
        configuration_bytes,
        bundle,
    })
}

fn find_provider_manifest(root: &Path) -> Result<PathBuf, ApplicationBuildFailure> {
    let entries = fs::read_dir(root).map_err(|error| {
        failure(
            "ProviderPackageReadFailed",
            format!(
                "failed to read provider package `{}`: {error}",
                root.display()
            ),
        )
    })?;
    let mut matches = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| {
            failure(
                "ProviderPackageReadFailed",
                format!(
                    "failed to enumerate provider package `{}`: {error}",
                    root.display()
                ),
            )
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with("provider-manifest.") && name.ends_with(".json") {
            matches.push(entry.path());
        }
    }
    matches.sort();
    match matches.as_slice() {
        [path] => confined_existing_path(
            root,
            path.strip_prefix(root).map_err(|error| {
                failure(
                    "ProviderPackageReadFailed",
                    format!(
                        "provider manifest `{}` is not beneath package root `{}`: {error}",
                        path.display(),
                        root.display()
                    ),
                )
            })?,
            "target.providerPackage.manifest",
            "ProviderPackageReadFailed",
            "provider manifest",
        ),
        _ => Err(failure(
            "InvalidProviderPackage",
            format!(
                "provider package `{}` must contain exactly one provider-manifest.*.json",
                root.display()
            ),
        )),
    }
}

fn unique_artifact<'a>(
    manifest: &'a TargetProviderManifest,
    kind: ProviderArtifactKind,
    coordinate: Option<&str>,
) -> Result<&'a ProviderArtifactRef, ApplicationBuildFailure> {
    let matches = manifest
        .artifacts
        .iter()
        .filter(|artifact| {
            artifact.artifact_kind == kind
                && coordinate.is_none_or(|expected| artifact.resource.coordinate == expected)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [artifact] => Ok(*artifact),
        _ => Err(failure(
            "InvalidProviderPackage",
            format!(
                "provider manifest must expose exactly one {kind:?} artifact{}",
                coordinate.map_or(String::new(), |value| format!(" for `{value}`"))
            ),
        )),
    }
}

fn provider_schema_artifacts(
    manifest: &TargetProviderManifest,
) -> Result<Vec<&ProviderArtifactRef>, ApplicationBuildFailure> {
    let bound_roles = manifest
        .schema_bindings
        .iter()
        .map(|binding| binding.schema_role.as_str())
        .collect::<BTreeSet<_>>();
    let mut artifacts = manifest
        .artifacts
        .iter()
        .filter(|artifact| {
            artifact.artifact_kind == ProviderArtifactKind::ArtifactSchema
                && bound_roles.contains(artifact.role.as_str())
        })
        .collect::<Vec<_>>();
    artifacts.sort_by(|left, right| left.role.cmp(&right.role));
    if artifacts.len() != bound_roles.len() || artifacts.is_empty() {
        return Err(failure(
            "InvalidProviderPackage",
            "provider manifest must expose every artifact schema named by its schema bindings",
        ));
    }
    Ok(artifacts)
}

fn read_provider_artifact(
    root: &Path,
    artifact: &ProviderArtifactRef,
    kind: ProviderArtifactKind,
) -> Result<Vec<u8>, ApplicationBuildFailure> {
    let relative = match kind {
        ProviderArtifactKind::Lowerer | ProviderArtifactKind::Verifier => {
            PathBuf::from("components").join(format!("{}.component.wasm", artifact.role))
        }
        ProviderArtifactKind::ArtifactSchema => PathBuf::from("generated")
            .join("primary")
            .join(format!("{}.cddl", artifact.role)),
        ProviderArtifactKind::TargetProfile => PathBuf::from("generated")
            .join("primary")
            .join(format!("{}.cbor", artifact.role)),
        _ => {
            return Err(failure(
                "InvalidProviderPackage",
                format!("unsupported provider artifact path kind {kind:?}"),
            ));
        }
    };
    let path = confined_existing_path(
        root,
        &relative,
        "target.providerPackage.artifact",
        "ProviderPackageReadFailed",
        "provider artifact",
    )?;
    read(&path, "ProviderPackageReadFailed", "provider artifact")
}

fn validate_target_configuration_binding(
    adapter: &edict_syntax::ValidatedLawpackAdapter,
    bytes: &[u8],
) -> Result<(), ApplicationBuildFailure> {
    let reference = single_unique_configuration(
        adapter
            .effects()
            .values()
            .map(|effect| &effect.target_configuration),
    )?;
    let digest = provider_digest(&reference.id, bytes)?;
    if reference.digest_review_string() != rendered_digest(&digest) {
        return Err(failure(
            "TargetConfigurationMismatch",
            "target configuration bytes do not reproduce the adapter's digest-locked reference",
        ));
    }
    Ok(())
}

fn invoke_lowerer(
    host: &ProviderComponentHost,
    provider_proof: ValidatedTargetProviderManifest<'_>,
    registry: &ProviderArtifactSchemaRegistry,
    component: &ProviderArtifactRef,
    component_bytes: Vec<u8>,
    invocation: &ProviderInvocationContext<'_>,
) -> Result<Vec<u8>, ApplicationBuildFailure> {
    let selected = select_provider_component(
        &provider_proof,
        &component.role,
        ProviderInvocationKind::Lowering,
    )
    .map_err(|error| {
        failure(
            "InvalidProviderPackage",
            format!("provider lowerer selection failed: {error}"),
        )
    })?;
    let resolved = ResolvedProviderComponent::new(selected, Arc::<[u8]>::from(component_bytes));
    let prepared = host.prepare(&resolved).map_err(|error| {
        failure(
            "ProviderLowererFailed",
            format!("provider lowerer preflight failed: {error}"),
        )
    })?;

    let core = bound_artifact(
        invocation.coordinate,
        CORE_MODULE_DIGEST_DOMAIN,
        invocation.core_bytes,
    )?;
    let semantic_inputs = lowering_inputs(
        invocation.loaded,
        invocation.adapter,
        invocation.coordinate,
        invocation.source_bytes,
        invocation.target_ir_bytes,
        invocation.result_projection,
    )?;
    let contract = lowering_contract(&core, invocation.target_profile, &semantic_inputs);
    let request = ProviderLoweringRequest {
        protocol_version: TARGET_PROVIDER_PROTOCOL_VERSION,
        core,
        target_profile: invocation.target_profile.clone(),
        semantic_inputs,
        requested_outputs: vec![ProviderLoweringOutputRequest {
            role: PACKAGE_ROLE.to_owned(),
            kind: ProviderLoweringOutputKind::GeneratedArtifact,
            domain: PACKAGE_DOMAIN.to_owned(),
        }],
        limits: response_limits(),
    };
    let request =
        validate_provider_lowering_request(registry, &contract, &request).map_err(|report| {
            failure(
                "InvalidProviderInvocation",
                format!("provider lowering request validation failed: {report:?}"),
            )
        })?;
    let outcome = host
        .invoke_lowerer(&prepared, &request, registry, host_limits())
        .map_err(|error| {
            failure(
                "ProviderLowererFailed",
                format!("provider lowerer invocation failed: {error}"),
            )
        })?;
    if let Some(refusal) = outcome.refusal() {
        return Err(failure(
            "ProviderLowererRefused",
            format!("provider lowerer refused application semantics: {refusal:?}"),
        ));
    }
    let response = outcome.response().ok_or_else(|| {
        failure(
            "ProviderLowererFailed",
            "provider lowerer returned neither a response nor a refusal",
        )
    })?;
    if !response.diagnostics.is_empty() {
        return Err(failure(
            "ProviderLowererFailed",
            format!(
                "provider lowerer returned diagnostics: {:?}",
                response.diagnostics
            ),
        ));
    }
    let [output] = response.outputs.as_slice() else {
        return Err(failure(
            "ProviderLowererFailed",
            "provider lowerer did not emit exactly one package",
        ));
    };
    Ok(output.artifact.bytes.clone())
}

fn invoke_verifier(
    host: &ProviderComponentHost,
    provider_proof: ValidatedTargetProviderManifest<'_>,
    registry: &ProviderArtifactSchemaRegistry,
    component: &ProviderArtifactRef,
    component_bytes: Vec<u8>,
    invocation: &ProviderInvocationContext<'_>,
    package_bytes: &[u8],
) -> Result<Vec<u8>, ApplicationBuildFailure> {
    let selected = select_provider_component(
        &provider_proof,
        &component.role,
        ProviderInvocationKind::Verification,
    )
    .map_err(|error| {
        failure(
            "InvalidProviderPackage",
            format!("provider verifier selection failed: {error}"),
        )
    })?;
    let resolved = ResolvedProviderComponent::new(selected, Arc::<[u8]>::from(component_bytes));
    let prepared = host.prepare(&resolved).map_err(|error| {
        failure(
            "ProviderVerifierFailed",
            format!("provider verifier preflight failed: {error}"),
        )
    })?;

    let core = bound_artifact(
        invocation.coordinate,
        CORE_MODULE_DIGEST_DOMAIN,
        invocation.core_bytes,
    )?;
    let target_ir = bound_artifact(
        invocation.adapter.target_ir().id.as_str(),
        TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
        invocation.target_ir_bytes,
    )?;
    let semantic_inputs = verification_inputs(
        invocation.loaded,
        invocation.adapter,
        invocation.coordinate,
        invocation.source_bytes,
        package_bytes,
        invocation.result_projection,
    )?;
    let contract = verification_contract(
        &core,
        invocation.target_profile,
        &target_ir,
        &semantic_inputs,
    );
    let request = ProviderVerificationRequest {
        protocol_version: TARGET_PROVIDER_PROTOCOL_VERSION,
        core,
        target_profile: invocation.target_profile.clone(),
        target_ir,
        semantic_inputs,
        requested_outputs: vec![ProviderVerificationOutputRequest {
            role: VERIFICATION_REPORT_ROLE.to_owned(),
            kind: ProviderVerificationOutputKind::VerifierReport,
            domain: VERIFICATION_REPORT_DOMAIN.to_owned(),
        }],
        limits: response_limits(),
    };
    let request = validate_provider_verification_request(registry, &contract, &request).map_err(
        |report| {
            failure(
                "InvalidProviderInvocation",
                format!("provider verification request validation failed: {report:?}"),
            )
        },
    )?;
    let outcome = host
        .invoke_verifier(&prepared, &request, registry, host_limits())
        .map_err(|error| {
            failure(
                "ProviderVerifierFailed",
                format!("provider verifier invocation failed: {error}"),
            )
        })?;
    if let Some(refusal) = outcome.refusal() {
        return Err(failure(
            "ProviderVerifierRefused",
            format!("provider verifier refused application semantics: {refusal:?}"),
        ));
    }
    let response = outcome.response().ok_or_else(|| {
        failure(
            "ProviderVerifierFailed",
            "provider verifier returned neither a response nor a refusal",
        )
    })?;
    if !response.diagnostics.is_empty() {
        return Err(failure(
            "ProviderVerificationRejected",
            format!(
                "provider verifier rejected the package: {:?}",
                response.diagnostics
            ),
        ));
    }
    let [output] = response.outputs.as_slice() else {
        return Err(failure(
            "ProviderVerifierFailed",
            "provider verifier did not emit exactly one report",
        ));
    };
    Ok(output.artifact.bytes.clone())
}

fn lowering_inputs(
    loaded: &LoadedLawpack,
    adapter: &edict_syntax::ValidatedLawpackAdapter,
    coordinate: &str,
    source_bytes: &[u8],
    target_ir_bytes: &[u8],
    result_projection: &ResultProjectionArtifact,
) -> Result<Vec<ProviderSemanticInput>, ApplicationBuildFailure> {
    let configuration = single_configuration(adapter)?;
    let adapter_reference = selected_adapter_reference(
        &loaded.bundle.manifest().target_adapters,
        adapter.target_profile(),
    )?;
    with_result_projection_input(
        vec![
            semantic_input(
                "01-lawpack-adapter",
                ProviderSemanticInputKind::Auxiliary("lawpack-adapter".to_owned()),
                &adapter_reference.id,
                ADAPTER_DOMAIN,
                &loaded.adapter_bytes,
            )?,
            semantic_input(
                "02-lawpack-exports",
                ProviderSemanticInputKind::Auxiliary("lawpack-exports".to_owned()),
                &loaded.bundle.manifest().exports.id,
                EXPORTS_DOMAIN,
                &loaded.exports_bytes,
            )?,
            semantic_input(
                "03-lawpack",
                ProviderSemanticInputKind::Lawpack,
                &format!(
                    "{}@{}",
                    loaded.bundle.manifest().id,
                    loaded.bundle.manifest().version
                ),
                PROVIDER_LAWPACK_ARTIFACT_DOMAIN,
                &loaded.manifest_bytes,
            )?,
            semantic_input(
                "04-source",
                ProviderSemanticInputKind::Auxiliary("edict-source".to_owned()),
                coordinate,
                SOURCE_DOMAIN,
                source_bytes,
            )?,
            semantic_input(
                "05-target-configuration",
                ProviderSemanticInputKind::Auxiliary("target-configuration".to_owned()),
                &configuration.id,
                &configuration.id,
                &loaded.configuration_bytes,
            )?,
            semantic_input(
                "06-target-ir",
                ProviderSemanticInputKind::Auxiliary("target-ir".to_owned()),
                &adapter.target_ir().id,
                TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
                target_ir_bytes,
            )?,
        ],
        result_projection,
    )
}

fn verification_inputs(
    loaded: &LoadedLawpack,
    adapter: &edict_syntax::ValidatedLawpackAdapter,
    coordinate: &str,
    source_bytes: &[u8],
    package_bytes: &[u8],
    result_projection: &ResultProjectionArtifact,
) -> Result<Vec<ProviderSemanticInput>, ApplicationBuildFailure> {
    let configuration = single_configuration(adapter)?;
    let adapter_reference = selected_adapter_reference(
        &loaded.bundle.manifest().target_adapters,
        adapter.target_profile(),
    )?;
    with_result_projection_input(
        vec![
            semantic_input(
                "01-lawpack-adapter",
                ProviderSemanticInputKind::Auxiliary("lawpack-adapter".to_owned()),
                &adapter_reference.id,
                ADAPTER_DOMAIN,
                &loaded.adapter_bytes,
            )?,
            semantic_input(
                "02-executable-operation-package",
                ProviderSemanticInputKind::Auxiliary("executable-operation-package".to_owned()),
                PACKAGE_ROLE,
                PACKAGE_DOMAIN,
                package_bytes,
            )?,
            semantic_input(
                "03-lawpack-exports",
                ProviderSemanticInputKind::Auxiliary("lawpack-exports".to_owned()),
                &loaded.bundle.manifest().exports.id,
                EXPORTS_DOMAIN,
                &loaded.exports_bytes,
            )?,
            semantic_input(
                "04-lawpack",
                ProviderSemanticInputKind::Lawpack,
                &format!(
                    "{}@{}",
                    loaded.bundle.manifest().id,
                    loaded.bundle.manifest().version
                ),
                PROVIDER_LAWPACK_ARTIFACT_DOMAIN,
                &loaded.manifest_bytes,
            )?,
            semantic_input(
                "05-source",
                ProviderSemanticInputKind::Auxiliary("edict-source".to_owned()),
                coordinate,
                SOURCE_DOMAIN,
                source_bytes,
            )?,
            semantic_input(
                "06-target-configuration",
                ProviderSemanticInputKind::Auxiliary("target-configuration".to_owned()),
                &configuration.id,
                &configuration.id,
                &loaded.configuration_bytes,
            )?,
        ],
        result_projection,
    )
}

fn single_configuration(
    adapter: &edict_syntax::ValidatedLawpackAdapter,
) -> Result<&edict_syntax::LawpackResourceRef, ApplicationBuildFailure> {
    single_unique_configuration(
        adapter
            .effects()
            .values()
            .map(|effect| &effect.target_configuration),
    )
}

fn single_result_projection<'a>(
    projections: &'a BTreeMap<String, ResultProjectionArtifact>,
    failures: &BTreeMap<String, edict_syntax::ResultProjectionFailure>,
) -> Result<(&'a str, &'a ResultProjectionArtifact), ApplicationBuildFailure> {
    if !failures.is_empty() {
        return Err(failure(
            "ResultProjectionUnavailable",
            format!("application result projection failed closed: {failures:?}"),
        ));
    }
    let mut projections = projections.iter();
    match (projections.next(), projections.next()) {
        (Some((intent, projection)), None) => Ok((intent.as_str(), projection)),
        _ => Err(failure(
            "ResultProjectionUnavailable",
            "the executable-operation build requires exactly one compiler result projection",
        )),
    }
}

fn single_unique_configuration<'a>(
    references: impl IntoIterator<Item = &'a edict_syntax::LawpackResourceRef>,
) -> Result<&'a edict_syntax::LawpackResourceRef, ApplicationBuildFailure> {
    let references = references.into_iter().collect::<BTreeSet<_>>();
    let mut references = references.into_iter();
    match (references.next(), references.next()) {
        (Some(reference), None) => Ok(reference),
        _ => Err(failure(
            "InvalidLawpackAdapter",
            "the executable-operation adapter currently requires exactly one target configuration",
        )),
    }
}

fn selected_adapter_reference<'a>(
    adapters: &'a [edict_syntax::LawpackTargetAdapter],
    selected_target_profile: &edict_syntax::LawpackResourceRef,
) -> Result<&'a edict_syntax::LawpackResourceRef, ApplicationBuildFailure> {
    let mut matches = adapters
        .iter()
        .filter(|descriptor| descriptor.accepted_target_profile == *selected_target_profile);
    match (matches.next(), matches.next()) {
        (Some(descriptor), None) => Ok(&descriptor.adapter),
        _ => Err(failure(
            "InvalidLawpackAdapter",
            "the selected target profile must identify exactly one lawpack adapter",
        )),
    }
}

fn semantic_input(
    role: &str,
    kind: ProviderSemanticInputKind,
    coordinate: &str,
    domain: &str,
    bytes: &[u8],
) -> Result<ProviderSemanticInput, ApplicationBuildFailure> {
    Ok(ProviderSemanticInput {
        role: role.to_owned(),
        kind,
        artifact: bound_artifact(coordinate, domain, bytes)?,
    })
}

fn with_result_projection_input(
    mut inputs: Vec<ProviderSemanticInput>,
    projection: &ResultProjectionArtifact,
) -> Result<Vec<ProviderSemanticInput>, ApplicationBuildFailure> {
    let input = semantic_input(
        RESULT_PROJECTION_ROLE,
        ProviderSemanticInputKind::Auxiliary("result-projection".to_owned()),
        &projection.projection().operation_coordinate,
        RESULT_PROJECTION_DIGEST_DOMAIN,
        projection.canonical_bytes(),
    )?;
    if input.artifact.reference.digest.bytes != projection.digest().bytes() {
        return Err(failure(
            "ResultProjectionDigestMismatch",
            "provider input does not preserve the compiler result projection identity",
        ));
    }
    inputs.push(input);
    Ok(inputs)
}

fn lowering_contract(
    core: &ProviderBoundArtifact,
    target_profile: &ProviderBoundArtifact,
    inputs: &[ProviderSemanticInput],
) -> ProviderLoweringInvocationContract {
    ProviderLoweringInvocationContract {
        core: artifact_binding(core),
        target_profile: artifact_binding(target_profile),
        semantic_inputs: input_bindings(inputs),
    }
}

fn verification_contract(
    core: &ProviderBoundArtifact,
    target_profile: &ProviderBoundArtifact,
    target_ir: &ProviderBoundArtifact,
    inputs: &[ProviderSemanticInput],
) -> ProviderVerificationInvocationContract {
    ProviderVerificationInvocationContract {
        core: artifact_binding(core),
        target_profile: artifact_binding(target_profile),
        target_ir: artifact_binding(target_ir),
        semantic_inputs: input_bindings(inputs),
    }
}

fn input_bindings(inputs: &[ProviderSemanticInput]) -> Vec<ProviderSemanticInputBinding> {
    inputs
        .iter()
        .map(|input| ProviderSemanticInputBinding {
            role: input.role.clone(),
            kind: input.kind.clone(),
            artifact: artifact_binding(&input.artifact),
        })
        .collect()
}

fn artifact_binding(bound: &ProviderBoundArtifact) -> ProviderArtifactBinding {
    ProviderArtifactBinding {
        reference: bound.reference.clone(),
        domain: bound.artifact.domain.clone(),
    }
}

fn bound_artifact(
    coordinate: &str,
    domain: &str,
    bytes: &[u8],
) -> Result<ProviderBoundArtifact, ApplicationBuildFailure> {
    Ok(ProviderBoundArtifact {
        reference: ProviderResourceRef {
            coordinate: coordinate.to_owned(),
            digest: provider_digest(domain, bytes)?,
        },
        artifact: ProviderArtifact {
            domain: domain.to_owned(),
            bytes: bytes.to_vec(),
        },
    })
}

fn provider_digest(domain: &str, bytes: &[u8]) -> Result<ProviderDigest, ApplicationBuildFailure> {
    let digest = digest_canonical_artifact(domain, bytes).map_err(|error| {
        failure(
            "NonCanonicalApplicationArtifact",
            format!("artifact under `{domain}` is not canonical CBOR: {error}"),
        )
    })?;
    Ok(ProviderDigest {
        algorithm: ProviderDigestAlgorithm::Sha256,
        bytes: digest.bytes().to_vec(),
    })
}

fn require_manifest_identity(
    manifest_artifact: &ProviderArtifactRef,
    bound: &ProviderBoundArtifact,
) -> Result<(), ApplicationBuildFailure> {
    if manifest_artifact.resource.digest.as_deref()
        != Some(&rendered_digest(&bound.reference.digest))
    {
        return Err(failure(
            "ProviderArtifactDigestMismatch",
            format!(
                "provider artifact `{}` bytes do not reproduce the manifest digest",
                manifest_artifact.role
            ),
        ));
    }
    Ok(())
}

fn rendered_digest(digest: &ProviderDigest) -> String {
    let mut value = String::from("sha256:");
    for byte in &digest.bytes {
        use std::fmt::Write as _;
        let _ = write!(value, "{byte:02x}");
    }
    value
}

fn require_accepted_report(bytes: &[u8]) -> Result<(), ApplicationBuildFailure> {
    let value = decode_canonical_cbor(bytes).map_err(|error| {
        failure(
            "ProviderVerificationRejected",
            format!("verification report is not canonical CBOR: {error}"),
        )
    })?;
    let CanonicalValue::Map(entries) = value else {
        return Err(failure(
            "ProviderVerificationRejected",
            "verification report is not a canonical map",
        ));
    };
    let mut outcomes = entries
        .iter()
        .filter(|(key, _)| matches!(key, CanonicalValue::Text(key) if key == "outcome"));
    let accepted = matches!(
        (outcomes.next(), outcomes.next()),
        (Some((_, CanonicalValue::Text(outcome))), None) if outcome == "accepted"
    );
    if !accepted {
        return Err(failure(
            "ProviderVerificationRejected",
            "independent verifier report did not accept the emitted package",
        ));
    }
    Ok(())
}

#[allow(
    clippy::too_many_lines,
    reason = "paired output publication keeps every rollback transition explicit"
)]
fn write_outputs(
    directory: &Path,
    package: &[u8],
    report: &[u8],
) -> Result<(), ApplicationBuildFailure> {
    fs::create_dir_all(directory).map_err(|error| {
        failure(
            "ApplicationOutputWriteFailed",
            format!(
                "failed to create application output directory `{}`: {error}",
                directory.display()
            ),
        )
    })?;

    let lock_path = output_lock_path(directory);
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|error| {
            failure(
                "ApplicationOutputWriteFailed",
                format!(
                    "failed to open application output lock `{}`: {error}",
                    lock_path.display()
                ),
            )
        })?;
    lock.try_lock().map_err(|error| {
        failure(
            "ApplicationOutputWriteFailed",
            format!(
                "another application build owns output directory `{}`: {error}",
                directory.display()
            ),
        )
    })?;

    let package_path = directory.join("executable-operation-package.cbor");
    let report_path = directory.join("verification-report.cbor");
    let package_existed = validate_output_target(&package_path)?;
    let report_existed = validate_output_target(&report_path)?;
    let transaction = create_output_transaction(directory)?;
    let package_temp = transaction.join("new-package.cbor");
    let report_temp = transaction.join("new-report.cbor");
    let package_backup = transaction.join("previous-package.cbor");
    let report_backup = transaction.join("previous-report.cbor");

    if let Err(error) = write_synced(&package_temp, package) {
        let _ = fs::remove_dir_all(&transaction);
        return Err(failure(
            "ApplicationOutputWriteFailed",
            format!("failed to stage `{}`: {error}", package_path.display()),
        ));
    }
    if let Err(error) = write_synced(&report_temp, report) {
        let _ = fs::remove_dir_all(&transaction);
        return Err(failure(
            "ApplicationOutputWriteFailed",
            format!("failed to stage `{}`: {error}", report_path.display()),
        ));
    }

    if package_existed {
        if let Err(error) = fs::rename(&package_path, &package_backup) {
            let _ = fs::remove_dir_all(&transaction);
            return Err(failure(
                "ApplicationOutputWriteFailed",
                format!(
                    "failed to preserve previous output `{}`: {error}",
                    package_path.display()
                ),
            ));
        }
    }
    if report_existed {
        if let Err(error) = fs::rename(&report_path, &report_backup) {
            let rollback = restore_output(&package_backup, &package_path, package_existed);
            if rollback.is_ok() {
                let _ = fs::remove_dir_all(&transaction);
            }
            return Err(output_publication_failure(
                &report_path,
                &error,
                rollback.err(),
            ));
        }
    }

    if let Err(error) = fs::rename(&report_temp, &report_path) {
        let rollback = restore_previous_outputs(&[
            OutputRecovery {
                destination: &package_path,
                backup: &package_backup,
                existed: package_existed,
                published: false,
            },
            OutputRecovery {
                destination: &report_path,
                backup: &report_backup,
                existed: report_existed,
                published: false,
            },
        ]);
        if rollback.is_ok() {
            let _ = fs::remove_dir_all(&transaction);
        }
        return Err(output_publication_failure(
            &report_path,
            &error,
            rollback.err(),
        ));
    }
    if let Err(error) = fs::rename(&package_temp, &package_path) {
        let rollback = restore_previous_outputs(&[
            OutputRecovery {
                destination: &package_path,
                backup: &package_backup,
                existed: package_existed,
                published: false,
            },
            OutputRecovery {
                destination: &report_path,
                backup: &report_backup,
                existed: report_existed,
                published: true,
            },
        ]);
        if rollback.is_ok() {
            let _ = fs::remove_dir_all(&transaction);
        }
        return Err(output_publication_failure(
            &package_path,
            &error,
            rollback.err(),
        ));
    }

    fs::remove_dir_all(&transaction).map_err(|error| {
        failure(
            "ApplicationOutputWriteFailed",
            format!(
                "published application outputs but failed to remove transaction `{}`: {error}",
                transaction.display()
            ),
        )
    })
}

fn output_lock_path(directory: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(
        directory
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("root")),
    );
    name.push(".edict-application-build.lock");
    directory
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(name)
}

fn write_synced(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

fn validate_output_target(path: &Path) -> Result<bool, ApplicationBuildFailure> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(true),
        Ok(_) => Err(failure(
            "ApplicationOutputWriteFailed",
            format!(
                "application output target `{}` must be absent or a regular file",
                path.display()
            ),
        )),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(failure(
            "ApplicationOutputWriteFailed",
            format!(
                "failed to inspect application output target `{}`: {error}",
                path.display()
            ),
        )),
    }
}

fn create_output_transaction(directory: &Path) -> Result<PathBuf, ApplicationBuildFailure> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            failure(
                "ApplicationOutputWriteFailed",
                format!("system clock cannot name an output transaction: {error}"),
            )
        })?
        .as_nanos();
    for attempt in 0..16 {
        let path = directory.join(format!(
            ".edict-application-build-{}-{timestamp}-{attempt}.transaction",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(failure(
                    "ApplicationOutputWriteFailed",
                    format!(
                        "failed to create output transaction `{}`: {error}",
                        path.display()
                    ),
                ));
            }
        }
    }
    Err(failure(
        "ApplicationOutputWriteFailed",
        format!(
            "failed to allocate a unique output transaction in `{}`",
            directory.display()
        ),
    ))
}

struct OutputRecovery<'a> {
    destination: &'a Path,
    backup: &'a Path,
    existed: bool,
    published: bool,
}

fn restore_previous_outputs(outputs: &[OutputRecovery<'_>; 2]) -> Result<(), String> {
    let mut failures = Vec::new();
    for output in outputs.iter().filter(|output| output.published) {
        if let Err(error) = fs::remove_file(output.destination) {
            failures.push(format!(
                "failed to remove partial output `{}`: {error}",
                output.destination.display()
            ));
        }
    }
    for output in outputs {
        if let Err(error) = restore_output(output.backup, output.destination, output.existed) {
            failures.push(error);
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn restore_output(backup: &Path, destination: &Path, existed: bool) -> Result<(), String> {
    if !existed {
        return Ok(());
    }
    fs::rename(backup, destination).map_err(|error| {
        format!(
            "failed to restore previous output `{}`: {error}",
            destination.display()
        )
    })
}

fn output_publication_failure(
    path: &Path,
    error: &std::io::Error,
    rollback: Option<String>,
) -> ApplicationBuildFailure {
    let rollback = rollback.map_or(String::new(), |message| {
        format!("; rollback was incomplete: {message}")
    });
    failure(
        "ApplicationOutputWriteFailed",
        format!("failed to publish `{}`: {error}{rollback}", path.display()),
    )
}

const fn response_limits() -> ProviderResponseLimits {
    ProviderResponseLimits {
        max_output_count: 1,
        max_diagnostic_count: 8,
        max_total_response_bytes: 64 * 1024,
    }
}

const fn host_limits() -> ProviderHostLimits {
    ProviderHostLimits {
        max_input_bytes: 1024 * 1024,
        max_output_bytes: 3 * 1024 * 1024,
        max_diagnostic_bytes: 3 * 1024 * 1024,
        max_wasm_memory_bytes: 16 * 1024 * 1024,
        max_table_elements: 10_000,
        max_instances: 100,
        max_memories: 8,
        max_tables: 8,
        max_wasm_fuel: 50_000_000,
        max_hostcall_bytes: 4 * 1024 * 1024,
        max_host_diagnostic_bytes: 512,
    }
}

fn read(
    path: &Path,
    kind: &'static str,
    subject: &str,
) -> Result<Vec<u8>, ApplicationBuildFailure> {
    let mut file = OpenOptions::new().read(true).open(path).map_err(|error| {
        failure(
            kind,
            format!("failed to read {subject} `{}`: {error}", path.display()),
        )
    })?;
    let length = file
        .metadata()
        .map_err(|error| {
            failure(
                kind,
                format!("failed to inspect {subject} `{}`: {error}", path.display()),
            )
        })?
        .len();
    if length > MAX_APPLICATION_ARTIFACT_BYTES {
        return Err(artifact_too_large(path, subject));
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(MAX_APPLICATION_ARTIFACT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            failure(
                kind,
                format!("failed to read {subject} `{}`: {error}", path.display()),
            )
        })?;
    if u64::try_from(bytes.len()).map_or(true, |length| length > MAX_APPLICATION_ARTIFACT_BYTES) {
        return Err(artifact_too_large(path, subject));
    }
    Ok(bytes)
}

fn artifact_too_large(path: &Path, subject: &str) -> ApplicationBuildFailure {
    failure(
        "ApplicationArtifactTooLarge",
        format!(
            "{subject} `{}` exceeds the {} byte application-artifact limit",
            path.display(),
            MAX_APPLICATION_ARTIFACT_BYTES
        ),
    )
}

fn canonical_application_root(config_path: &Path) -> Result<PathBuf, ApplicationBuildFailure> {
    let root = config_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::canonicalize(root).map_err(|error| {
        failure(
            "ApplicationConfigReadFailed",
            format!(
                "failed to resolve application root `{}`: {error}",
                root.display()
            ),
        )
    })
}

fn confined_existing_path(
    root: &Path,
    relative: &Path,
    field: &str,
    read_failure_kind: &'static str,
    subject: &str,
) -> Result<PathBuf, ApplicationBuildFailure> {
    let candidate = root.join(relative);
    let canonical = fs::canonicalize(&candidate).map_err(|error| {
        failure(
            read_failure_kind,
            format!(
                "failed to resolve {subject} `{}`: {error}",
                candidate.display()
            ),
        )
    })?;
    require_path_beneath_root(root, &canonical, field)?;
    Ok(canonical)
}

fn prepare_output_directory(
    root: &Path,
    relative: &Path,
) -> Result<PathBuf, ApplicationBuildFailure> {
    let candidate = root.join(relative);
    let mut existing_ancestor = candidate.as_path();
    while !existing_ancestor.exists() {
        existing_ancestor = existing_ancestor.parent().ok_or_else(|| {
            failure(
                "ApplicationOutputWriteFailed",
                format!(
                    "application output `{}` has no existing ancestor",
                    candidate.display()
                ),
            )
        })?;
    }
    let canonical_ancestor = fs::canonicalize(existing_ancestor).map_err(|error| {
        failure(
            "ApplicationOutputWriteFailed",
            format!(
                "failed to resolve output ancestor `{}`: {error}",
                existing_ancestor.display()
            ),
        )
    })?;
    require_path_beneath_root(root, &canonical_ancestor, "outputDirectory")?;
    fs::create_dir_all(&candidate).map_err(|error| {
        failure(
            "ApplicationOutputWriteFailed",
            format!(
                "failed to create application output directory `{}`: {error}",
                candidate.display()
            ),
        )
    })?;
    let canonical = fs::canonicalize(&candidate).map_err(|error| {
        failure(
            "ApplicationOutputWriteFailed",
            format!(
                "failed to resolve application output directory `{}`: {error}",
                candidate.display()
            ),
        )
    })?;
    require_path_beneath_root(root, &canonical, "outputDirectory")?;
    Ok(canonical)
}

fn require_path_beneath_root(
    root: &Path,
    canonical: &Path,
    field: &str,
) -> Result<(), ApplicationBuildFailure> {
    if canonical.starts_with(root) {
        Ok(())
    } else {
        Err(failure(
            "ApplicationPathEscape",
            format!(
                "application field `{field}` resolves outside manifest root `{}`",
                root.display()
            ),
        ))
    }
}

fn failure(kind: &'static str, message: impl Into<String>) -> ApplicationBuildFailure {
    ApplicationBuildFailure {
        kind,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use edict_syntax::{
        compile_to_core, decode_lawpack_adapter, decode_lawpack_bundle, decode_result_projection,
        lower_to_target_ir, parse_module, prepare_lawpack_compilation, LawpackResourceRef,
        LawpackTargetAdapter, ProviderArtifactKind, ProviderArtifactRef, ProviderArtifactSource,
        ProviderSchemaBinding, ProviderSchemaFormat, ResourceRef, ResultProjectionArtifact,
        TargetProviderManifest, RESULT_PROJECTION_DIGEST_DOMAIN, TARGET_PROVIDER_ABI,
        TARGET_PROVIDER_MANIFEST_API_VERSION,
    };

    use super::{
        build_application, canonical_application_root, output_lock_path, provider_schema_artifacts,
        read, selected_adapter_reference, single_result_projection, single_unique_configuration,
        validate_application_manifest, with_result_projection_input, write_outputs,
        ApplicationLawpack, ApplicationManifest, ApplicationTarget, RESULT_PROJECTION_ROLE,
    };

    const STRESS_SEED: u64 = 0x5eed_1a77_c105_0a11;
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn application_manifest_accepts_one_root_and_its_complete_dependency_closure() {
        let config = application_manifest(64);

        test_ok(
            validate_application_manifest(&config),
            "one root plus a bounded dependency closure must be accepted",
        );
    }

    #[test]
    fn relative_application_config_uses_the_current_directory_as_root() {
        let actual = test_ok(
            canonical_application_root(PathBuf::from("edict.application.json").as_path()),
            "resolve relative application root",
        );
        let expected = test_ok(fs::canonicalize("."), "resolve current directory");

        assert_eq!(actual, expected);
    }

    #[test]
    fn application_manifest_rejects_absolute_and_parent_traversal_paths() {
        let mut source_escape = application_manifest(1);
        source_escape.sources = vec![PathBuf::from("../outside.edict")];
        let mut lawpack_escape = application_manifest(1);
        lawpack_escape.lawpacks[0].manifest = PathBuf::from("/outside/manifest.cbor");
        let mut provider_escape = application_manifest(1);
        provider_escape.target.provider_package = PathBuf::from("../provider");
        let mut output_escape = application_manifest(1);
        output_escape.output_directory = PathBuf::from("/outside/output");

        for config in [
            source_escape,
            lawpack_escape,
            provider_escape,
            output_escape,
        ] {
            assert!(
                validate_application_manifest(&config).is_err(),
                "application-owned paths must remain beneath the manifest root"
            );
        }
    }

    #[test]
    fn selected_adapter_reference_follows_the_selected_target_profile() {
        let first_profile = lawpack_ref("target.first@1", 0x11);
        let selected_profile = lawpack_ref("target.selected@1", 0x22);
        let first = adapter_descriptor(first_profile, "adapter.first@1", 0x31);
        let selected = adapter_descriptor(selected_profile.clone(), "adapter.selected@1", 0x32);
        let adapters = [first, selected];

        let actual = test_ok(
            selected_adapter_reference(&adapters, &selected_profile),
            "selected adapter",
        );

        assert_eq!(actual.id, "adapter.selected@1");
    }

    #[test]
    fn repeated_effect_references_to_one_configuration_are_deduplicated() {
        let configuration = lawpack_ref("target.configuration@1", 0x44);

        let actual = test_ok(
            single_unique_configuration([&configuration, &configuration]),
            "identical effect references name one configuration",
        );

        assert_eq!(actual, &configuration);
    }

    #[test]
    fn compiler_result_projection_is_bound_into_the_provider_closure() {
        let projection = result_projection_artifact();
        let bytes = projection.canonical_bytes().to_vec();
        let input = test_ok(
            with_result_projection_input(Vec::new(), &projection),
            "bind compiler result projection",
        );

        assert_eq!(input.len(), 1);
        assert_eq!(input[0].role, RESULT_PROJECTION_ROLE);
        assert_eq!(
            input[0].kind,
            edict_syntax::ProviderSemanticInputKind::Auxiliary("result-projection".to_owned())
        );
        assert_eq!(
            input[0].artifact.reference.coordinate,
            "examples.hello_echo@1.createGreeting"
        );
        assert_eq!(
            input[0].artifact.artifact.domain,
            RESULT_PROJECTION_DIGEST_DOMAIN
        );
        assert_eq!(input[0].artifact.artifact.bytes, bytes);
    }

    #[test]
    fn application_build_requires_one_projection_and_no_projection_failures() {
        let projection = result_projection_artifact();
        let empty = BTreeMap::new();
        let no_failures = BTreeMap::new();
        assert_eq!(
            single_result_projection(&empty, &no_failures)
                .expect_err("an application build without a projection must reject")
                .kind,
            "ResultProjectionUnavailable"
        );

        let multiple = BTreeMap::from([
            ("first".to_owned(), projection.clone()),
            ("second".to_owned(), projection.clone()),
        ]);
        assert_eq!(
            single_result_projection(&multiple, &no_failures)
                .expect_err("an application build with multiple projections must reject")
                .kind,
            "ResultProjectionUnavailable"
        );

        let admitted = BTreeMap::from([("createGreeting".to_owned(), projection)]);
        let projection_failure =
            decode_result_projection(&[]).expect_err("empty projection bytes must reject");
        let failures = BTreeMap::from([("createGreeting".to_owned(), projection_failure)]);
        assert_eq!(
            single_result_projection(&admitted, &failures)
                .expect_err("a recorded projection failure must reject the application build")
                .kind,
            "ResultProjectionUnavailable"
        );
    }

    #[test]
    fn provider_schema_selection_loads_every_bound_role_under_bounded_stress() {
        let mut manifest = provider_manifest_with_schema_count(32);
        let mut state = STRESS_SEED;
        for index in (1..manifest.artifacts.len()).rev() {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let bound = test_ok(u64::try_from(index + 1), "stress bound fits u64");
            let swap_with = test_ok(
                usize::try_from(state % bound),
                "stress permutation index fits usize",
            );
            manifest.artifacts.swap(index, swap_with);
        }

        let actual = test_ok(
            provider_schema_artifacts(&manifest),
            "every schema role bound by the provider must be loaded",
        );
        let roles = actual
            .iter()
            .map(|artifact| artifact.role.as_str())
            .collect::<BTreeSet<_>>();

        assert_eq!(actual.len(), 32);
        assert_eq!(roles.len(), 32);
    }

    fn result_projection_artifact() -> ResultProjectionArtifact {
        let manifest =
            include_bytes!("../../../fixtures/lawpack/hello-echo/manifest.cbor").as_slice();
        let exports =
            include_bytes!("../../../fixtures/lawpack/hello-echo/exports.cbor").as_slice();
        let adapter =
            include_bytes!("../../../fixtures/lawpack/hello-echo/adapter.cbor").as_slice();
        let source = include_str!("../../../fixtures/lawpack/hello-echo/create-greeting.edict");
        let bundle = test_ok(
            decode_lawpack_bundle(manifest, exports),
            "decode Hello Echo lawpack",
        );
        let adapter = test_ok(
            decode_lawpack_adapter(&bundle, "echo.dpo@1", adapter),
            "decode Hello Echo adapter",
        );
        let module = test_ok(parse_module(source), "parse Hello Echo source");
        let preparation = test_ok(
            prepare_lawpack_compilation(&module, &bundle, &adapter),
            "prepare Hello Echo compilation",
        );
        let core = test_ok(
            compile_to_core(&module, preparation.compiler_context()),
            "compile Hello Echo Core",
        );
        let mut report = lower_to_target_ir(&core, preparation.target_ir_facts());
        report
            .result_projections
            .remove("createGreeting")
            .expect("Hello Echo lowering emits its result projection")
    }

    #[test]
    fn failed_pair_publication_preserves_the_previous_package() {
        let root = temp_tree("pair-publication");
        let package_path = root.join("executable-operation-package.cbor");
        let report_path = root.join("verification-report.cbor");
        test_ok(
            fs::write(&package_path, b"previous-package"),
            "write prior package",
        );
        test_ok(
            fs::create_dir(&report_path),
            "create conflicting report directory",
        );

        let failure = test_err(
            write_outputs(&root, b"new-package", b"new-report"),
            "a non-file report target must reject the pair",
        );

        assert_eq!(failure.kind, "ApplicationOutputWriteFailed");
        assert_eq!(
            test_ok(fs::read(&package_path), "read preserved package"),
            b"previous-package"
        );
        test_ok(fs::remove_dir_all(root), "remove test-owned temp tree");
    }

    #[test]
    fn application_artifact_reads_are_bounded_before_allocation() {
        let root = temp_tree("bounded-read");
        let path = root.join("oversized.cbor");
        let file = test_ok(fs::File::create(&path), "create oversized artifact");
        test_ok(
            file.set_len(1024 * 1024 + 1),
            "size oversized artifact without allocating it",
        );

        let failure = test_err(
            read(&path, "ArtifactReadFailed", "test artifact"),
            "oversized artifact must reject",
        );

        assert_eq!(failure.kind, "ApplicationArtifactTooLarge");
        test_ok(fs::remove_dir_all(root), "remove bounded read temp tree");
    }

    #[cfg(unix)]
    #[test]
    fn application_build_rejects_source_symlink_escape() {
        use std::os::unix::fs::symlink;

        let root = temp_tree("source-symlink-root");
        let outside = temp_tree("source-symlink-outside");
        let outside_source = outside.join("operation.edict");
        test_ok(
            fs::write(&outside_source, "package examples.test@1;"),
            "write external source",
        );
        test_ok(
            symlink(&outside_source, root.join("operation.edict")),
            "link source outside application root",
        );
        let config_path = root.join("edict.application.json");
        let config = serde_json::json!({
            "schema": "edict.application/v1",
            "coordinate": "examples.test@1.operation",
            "sources": ["operation.edict"],
            "lawpacks": [{
                "manifest": "lawpack/manifest.cbor",
                "exports": "lawpack/exports.cbor",
                "adapter": "lawpack/adapter.cbor",
                "targetConfiguration": "lawpack/configuration.cbor"
            }],
            "target": {
                "profile": "target.test@1",
                "providerPackage": "provider"
            },
            "outputDirectory": ".build/application"
        });
        let config_bytes = test_ok(serde_json::to_vec(&config), "encode application config");
        test_ok(
            fs::write(&config_path, config_bytes),
            "write application config",
        );

        let failure = test_err(
            build_application(&config_path),
            "source symlink outside the manifest root must reject",
        );

        assert_eq!(failure.kind, "ApplicationPathEscape");
        test_ok(fs::remove_dir_all(root), "remove symlink root");
        test_ok(fs::remove_dir_all(outside), "remove symlink target");
    }

    #[test]
    fn successful_publication_leaves_no_lock_in_output_directory() {
        let root = temp_tree("output-lock-location");
        let lock_path = output_lock_path(&root);

        test_ok(
            write_outputs(&root, b"package", b"report"),
            "publish application outputs",
        );

        assert!(!root.join(".edict-application-build.lock").exists());
        test_ok(fs::remove_file(lock_path), "remove test output lock");
        test_ok(fs::remove_dir_all(root), "remove output lock temp tree");
    }

    fn application_manifest(lawpack_count: usize) -> ApplicationManifest {
        ApplicationManifest {
            schema: "edict.application/v1".to_owned(),
            coordinate: "examples.test@1.operation".to_owned(),
            sources: vec![PathBuf::from("operation.edict")],
            lawpacks: (0..lawpack_count)
                .map(|index| ApplicationLawpack {
                    manifest: PathBuf::from(format!("lawpacks/{index}/manifest.cbor")),
                    exports: PathBuf::from(format!("lawpacks/{index}/exports.cbor")),
                    adapter: PathBuf::from(format!("lawpacks/{index}/adapter.cbor")),
                    target_configuration: PathBuf::from(format!(
                        "lawpacks/{index}/configuration.cbor"
                    )),
                })
                .collect(),
            target: ApplicationTarget {
                profile: "target.test@1".to_owned(),
                provider_package: PathBuf::from("provider"),
            },
            output_directory: PathBuf::from(".build/application"),
        }
    }

    fn lawpack_ref(id: &str, digest_byte: u8) -> LawpackResourceRef {
        LawpackResourceRef {
            id: id.to_owned(),
            digest: [digest_byte; 32],
        }
    }

    fn adapter_descriptor(
        accepted_target_profile: LawpackResourceRef,
        adapter_id: &str,
        digest_byte: u8,
    ) -> LawpackTargetAdapter {
        LawpackTargetAdapter {
            accepted_target_profile,
            accepted_target_ir: lawpack_ref("target.ir/v1", digest_byte.wrapping_add(1)),
            adapter: lawpack_ref(adapter_id, digest_byte),
        }
    }

    fn provider_manifest_with_schema_count(count: usize) -> TargetProviderManifest {
        let artifacts = (0..count).map(schema_artifact).collect::<Vec<_>>();
        let schema_bindings = (0..count)
            .map(|index| ProviderSchemaBinding {
                domain: format!("example.schema-{index:02}/v1"),
                schema_role: format!("schema.example-{index:02}"),
                format: ProviderSchemaFormat::SelfContainedCddlV1,
                root_rule: format!("example-{index:02}"),
            })
            .collect();
        TargetProviderManifest {
            api_version: TARGET_PROVIDER_MANIFEST_API_VERSION.to_owned(),
            provider_abi: TARGET_PROVIDER_ABI.to_owned(),
            provider: resource("provider.example@1", 0x51),
            artifacts,
            schema_bindings,
        }
    }

    fn schema_artifact(index: usize) -> ProviderArtifactRef {
        ProviderArtifactRef {
            role: format!("schema.example-{index:02}"),
            artifact_kind: ProviderArtifactKind::ArtifactSchema,
            resource: resource(
                &format!("schema.example-{index:02}@1"),
                test_ok(u8::try_from(index), "schema fixture index fits u8"),
            ),
            source: ProviderArtifactSource::Generated {
                semantic_source: resource("source.example@1", 0x61),
                generator: resource("generator.example@1", 0x62),
            },
        }
    }

    fn resource(coordinate: &str, digest_byte: u8) -> ResourceRef {
        ResourceRef {
            coordinate: coordinate.to_owned(),
            digest: Some(format!(
                "sha256:{}",
                format!("{digest_byte:02x}").repeat(32)
            )),
        }
    }

    fn temp_tree(name: &str) -> PathBuf {
        let unique = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "edict-application-build-{name}-{}-{unique}",
            std::process::id()
        ));
        test_ok(fs::create_dir_all(&path), "create test temp tree");
        path
    }

    fn test_ok<T, E: std::fmt::Debug>(result: Result<T, E>, context: &str) -> T {
        match result {
            Ok(value) => value,
            Err(error) => panic!("{context}: {error:?}"),
        }
    }

    fn test_err<T: std::fmt::Debug, E>(result: Result<T, E>, context: &str) -> E {
        match result {
            Ok(value) => panic!("{context}: got {value:?}"),
            Err(error) => error,
        }
    }
}
