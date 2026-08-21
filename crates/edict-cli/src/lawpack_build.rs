//! Public filesystem boundary for deterministic lawpack authoring.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use edict_syntax::{
    author_lawpack, decode_lawpack_bundle, LawpackArtifactKind, LawpackAuthoringDefinition,
    LawpackAuthoringFailureKind, ValidatedLawpackBundle,
};
use serde::{Deserialize, Serialize};

const LAWPACK_BUILD_SCHEMA: &str = "edict.lawpack-build/v1";
const LAWPACK_OUTPUT_SCHEMA: &str = "edict.lawpack-output/v1";
const OUTPUT_INDEX_FILE: &str = "edict.lawpack-output.json";
const MAX_LAWPACK_DOCUMENT_BYTES: u64 = 1024 * 1024;
const MAX_LAWPACK_ARTIFACT_BYTES: u64 = 1024 * 1024;
const MAX_DEPENDENCY_BUNDLES: usize = 192;

#[derive(Debug)]
pub(crate) struct LawpackBuildFailure {
    pub(crate) kind: &'static str,
    pub(crate) message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LawpackBuildDocument {
    schema: String,
    output_directory: PathBuf,
    lawpack: LawpackAuthoringDefinition,
    #[serde(default)]
    dependency_bundles: Vec<LawpackDependencyBundle>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LawpackDependencyBundle {
    manifest: PathBuf,
    exports: PathBuf,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LawpackOutputIndex<'a> {
    schema: &'static str,
    lawpack_id: &'a str,
    lawpack_version: &'a str,
    artifacts: Vec<LawpackOutputIndexEntry<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LawpackOutputIndexEntry<'a> {
    path: &'a str,
    kind: &'static str,
    coordinate: &'a str,
    digest: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExistingOutputIndex {
    schema: String,
    lawpack_id: String,
    lawpack_version: String,
    artifacts: Vec<ExistingOutputIndexEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExistingOutputIndexEntry {
    path: String,
    kind: String,
    coordinate: String,
    digest: String,
}

/// Author and either publish or check one application-owned lawpack.
pub(crate) fn build_lawpack(
    document_path: &Path,
    check_only: bool,
) -> Result<(), LawpackBuildFailure> {
    let document_path = fs::canonicalize(document_path).map_err(|error| {
        failure(
            "LawpackConfigReadFailed",
            format!(
                "failed to resolve lawpack build document `{}`: {error}",
                document_path.display()
            ),
        )
    })?;
    let root = document_path.parent().ok_or_else(|| {
        failure(
            "InvalidLawpackConfig",
            "lawpack build document must have a parent directory".to_owned(),
        )
    })?;
    let document_bytes = read_bounded(
        &document_path,
        MAX_LAWPACK_DOCUMENT_BYTES,
        "lawpack build document",
        "LawpackConfigReadFailed",
    )?;
    let document =
        serde_json::from_slice::<LawpackBuildDocument>(&document_bytes).map_err(|error| {
            failure(
                "InvalidLawpackConfig",
                format!("invalid lawpack build document: {error}"),
            )
        })?;
    validate_document(&document)?;

    let output = resolve_output_directory(root, &document.output_directory)?;
    let dependencies = load_dependencies(root, &document.dependency_bundles, &output)?;
    let authored = author_lawpack(&document.lawpack, &dependencies).map_err(|failures| {
        let Some(first) = failures.first() else {
            return failure(
                "LawpackAuthoringFailed",
                "lawpack authoring failed without a diagnostic".to_owned(),
            );
        };
        failure(
            authoring_failure_kind(first.kind),
            format!("{}: {}", first.path, first.obligation),
        )
    })?;

    let mut files = authored
        .artifacts()
        .iter()
        .map(|artifact| (PathBuf::from(artifact.path()), artifact.bytes().to_vec()))
        .collect::<BTreeMap<_, _>>();
    let index = LawpackOutputIndex {
        schema: LAWPACK_OUTPUT_SCHEMA,
        lawpack_id: &document.lawpack.id,
        lawpack_version: &document.lawpack.version,
        artifacts: authored
            .artifacts()
            .iter()
            .map(|artifact| LawpackOutputIndexEntry {
                path: artifact.path(),
                kind: artifact_kind_name(artifact.kind()),
                coordinate: artifact.coordinate(),
                digest: artifact.digest(),
            })
            .collect(),
    };
    let index_bytes = encode_output_index(&index)?;
    if files
        .insert(PathBuf::from(OUTPUT_INDEX_FILE), index_bytes)
        .is_some()
    {
        return Err(failure(
            "LawpackAuthoringFailed",
            format!("authored artifact path `{OUTPUT_INDEX_FILE}` is reserved for ownership"),
        ));
    }

    if check_only {
        check_output(&output, &files)
    } else {
        publish_output(&output, &files)
    }
}

fn encode_output_index(index: &LawpackOutputIndex<'_>) -> Result<Vec<u8>, LawpackBuildFailure> {
    let mut bytes = serde_json::to_vec_pretty(index).map_err(|error| {
        failure(
            "LawpackAuthoringFailed",
            format!("failed to encode lawpack output index: {error}"),
        )
    })?;
    bytes.push(b'\n');
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_LAWPACK_ARTIFACT_BYTES {
        return Err(failure(
            "LawpackOutputTooLarge",
            format!("generated lawpack output index exceeds {MAX_LAWPACK_ARTIFACT_BYTES} bytes"),
        ));
    }
    Ok(bytes)
}

fn validate_document(document: &LawpackBuildDocument) -> Result<(), LawpackBuildFailure> {
    if document.schema != LAWPACK_BUILD_SCHEMA {
        return Err(failure(
            "InvalidLawpackConfig",
            format!("lawpack build schema must be `{LAWPACK_BUILD_SCHEMA}`"),
        ));
    }
    if document.dependency_bundles.len() > MAX_DEPENDENCY_BUNDLES {
        return Err(failure(
            "InvalidLawpackConfig",
            format!("dependencyBundles exceeds the maximum of {MAX_DEPENDENCY_BUNDLES}"),
        ));
    }
    validate_relative_path(&document.output_directory, "outputDirectory")
}

fn load_dependencies(
    root: &Path,
    definitions: &[LawpackDependencyBundle],
    output: &Path,
) -> Result<Vec<ValidatedLawpackBundle>, LawpackBuildFailure> {
    definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            reject_input_inside_output(
                root,
                &definition.manifest,
                output,
                &format!("dependencyBundles.{index}.manifest"),
            )?;
            reject_input_inside_output(
                root,
                &definition.exports,
                output,
                &format!("dependencyBundles.{index}.exports"),
            )?;
            let manifest = resolve_existing_input(
                root,
                &definition.manifest,
                &format!("dependencyBundles.{index}.manifest"),
            )?;
            let exports = resolve_existing_input(
                root,
                &definition.exports,
                &format!("dependencyBundles.{index}.exports"),
            )?;
            let manifest = read_bounded(
                &manifest,
                MAX_LAWPACK_ARTIFACT_BYTES,
                "dependency manifest",
                "LawpackArtifactReadFailed",
            )?;
            let exports = read_bounded(
                &exports,
                MAX_LAWPACK_ARTIFACT_BYTES,
                "dependency exports",
                "LawpackArtifactReadFailed",
            )?;
            decode_lawpack_bundle(&manifest, &exports).map_err(|failures| {
                failure(
                    "InvalidLawpackDependency",
                    format!("dependencyBundles.{index} failed validation: {failures:?}"),
                )
            })
        })
        .collect()
}

fn reject_input_inside_output(
    root: &Path,
    relative: &Path,
    output: &Path,
    field: &str,
) -> Result<(), LawpackBuildFailure> {
    validate_relative_path(relative, field)?;
    if root.join(relative).starts_with(output) {
        return Err(failure(
            "InvalidLawpackConfig",
            format!("{field} must be outside outputDirectory"),
        ));
    }
    Ok(())
}

fn read_bounded(
    path: &Path,
    limit: u64,
    subject: &str,
    kind: &'static str,
) -> Result<Vec<u8>, LawpackBuildFailure> {
    let file = File::open(path).map_err(|error| {
        failure(
            kind,
            format!("failed to open {subject} `{}`: {error}", path.display()),
        )
    })?;
    let metadata = file.metadata().map_err(|error| {
        failure(
            kind,
            format!("failed to inspect {subject} `{}`: {error}", path.display()),
        )
    })?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(failure(
            kind,
            format!(
                "{subject} `{}` must be a regular file no larger than {limit} bytes",
                path.display()
            ),
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            failure(
                kind,
                format!("failed to read {subject} `{}`: {error}", path.display()),
            )
        })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(failure(
            kind,
            format!("{subject} `{}` exceeds {limit} bytes", path.display()),
        ));
    }
    Ok(bytes)
}

fn resolve_existing_input(
    root: &Path,
    relative: &Path,
    field: &str,
) -> Result<PathBuf, LawpackBuildFailure> {
    validate_relative_path(relative, field)?;
    let resolved = fs::canonicalize(root.join(relative)).map_err(|error| {
        failure(
            "LawpackArtifactReadFailed",
            format!(
                "failed to resolve {field} `{}`: {error}",
                relative.display()
            ),
        )
    })?;
    if !resolved.starts_with(root) {
        return Err(failure(
            "LawpackPathOutsideRoot",
            format!("{field} resolves outside `{}`", root.display()),
        ));
    }
    Ok(resolved)
}

fn resolve_output_directory(root: &Path, relative: &Path) -> Result<PathBuf, LawpackBuildFailure> {
    validate_relative_path(relative, "outputDirectory")?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let Component::Normal(component) = component else {
            return Err(failure(
                "InvalidLawpackConfig",
                "outputDirectory must contain only normal relative components".to_owned(),
            ));
        };
        current.push(component);
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(failure(
                    "LawpackPathOutsideRoot",
                    format!(
                        "outputDirectory must not traverse symlink `{}`",
                        current.display()
                    ),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(failure(
                    "LawpackOutputWriteFailed",
                    format!(
                        "failed to inspect outputDirectory `{}`: {error}",
                        current.display()
                    ),
                ));
            }
        }
    }
    Ok(current)
}

fn validate_relative_path(path: &Path, field: &str) -> Result<(), LawpackBuildFailure> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir
                    | Component::CurDir
                    | Component::RootDir
                    | Component::Prefix(_)
            )
        })
    {
        return Err(failure(
            "InvalidLawpackConfig",
            format!("{field} must be a non-empty confined relative path"),
        ));
    }
    Ok(())
}

fn check_output(
    output: &Path,
    expected: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
    let expected_owner = expected_output_owner(expected)?;
    let _lock = acquire_output_lock(output)?;
    validate_owned_output(output, false, &expected_owner)?;
    let mut permitted_directories = BTreeSet::new();
    for path in expected.keys() {
        let mut parent = path.parent();
        while let Some(directory) = parent {
            if directory.as_os_str().is_empty() {
                break;
            }
            permitted_directories.insert(directory.to_path_buf());
            parent = directory.parent();
        }
    }

    let mut seen = BTreeSet::new();
    let mut pending = vec![output.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| {
            failure(
                "LawpackOutputDrift",
                format!(
                    "failed to inspect output `{}`: {error}",
                    directory.display()
                ),
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                failure(
                    "LawpackOutputDrift",
                    format!("failed to inspect output entry: {error}"),
                )
            })?;
            let entry_path = entry.path();
            let relative = entry_path.strip_prefix(output).map_err(|error| {
                failure(
                    "LawpackOutputDrift",
                    format!("failed to relativize output entry: {error}"),
                )
            })?;
            let file_type = entry.file_type().map_err(|error| {
                failure(
                    "LawpackOutputDrift",
                    format!("failed to inspect `{}`: {error}", entry_path.display()),
                )
            })?;
            if file_type.is_dir() {
                if !permitted_directories.contains(relative) {
                    return Err(unexpected_output_path(output, relative));
                }
                pending.push(entry_path);
            } else if file_type.is_file() {
                let Some(expected_bytes) = expected.get(relative) else {
                    return Err(unexpected_output_path(output, relative));
                };
                let bytes = read_bounded(
                    &entry_path,
                    MAX_LAWPACK_ARTIFACT_BYTES,
                    "published lawpack artifact",
                    "LawpackOutputDrift",
                )?;
                if bytes != *expected_bytes {
                    return Err(failure(
                        "LawpackOutputDrift",
                        format!(
                            "published lawpack artifact `{}` differs from authored bytes",
                            relative.display()
                        ),
                    ));
                }
                seen.insert(relative.to_path_buf());
            } else {
                return Err(unexpected_output_path(output, relative));
            }
        }
    }
    if seen.len() != expected.len() {
        return Err(failure(
            "LawpackOutputDrift",
            format!(
                "lawpack output `{}` is missing one or more authored artifacts",
                output.display()
            ),
        ));
    }
    Ok(())
}

fn unexpected_output_path(output: &Path, relative: &Path) -> LawpackBuildFailure {
    failure(
        "LawpackOutputDrift",
        format!(
            "lawpack output `{}` contains unexpected path `{}`",
            output.display(),
            relative.display()
        ),
    )
}

#[cfg(test)]
fn read_output_tree(output: &Path) -> Result<BTreeMap<PathBuf, Vec<u8>>, LawpackBuildFailure> {
    let mut files = BTreeMap::new();
    let mut pending = vec![output.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let entries = fs::read_dir(&directory).map_err(|error| {
            failure(
                "LawpackOutputDrift",
                format!(
                    "failed to inspect output `{}`: {error}",
                    directory.display()
                ),
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                failure(
                    "LawpackOutputDrift",
                    format!("failed to inspect output entry: {error}"),
                )
            })?;
            let file_type = entry.file_type().map_err(|error| {
                failure(
                    "LawpackOutputDrift",
                    format!("failed to inspect `{}`: {error}", entry.path().display()),
                )
            })?;
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                let entry_path = entry.path();
                let relative = entry_path.strip_prefix(output).map_err(|error| {
                    failure(
                        "LawpackOutputDrift",
                        format!("failed to relativize output entry: {error}"),
                    )
                })?;
                let bytes = read_bounded(
                    &entry_path,
                    MAX_LAWPACK_ARTIFACT_BYTES,
                    "published lawpack artifact",
                    "LawpackOutputDrift",
                )?;
                files.insert(relative.to_path_buf(), bytes);
            } else {
                return Err(failure(
                    "LawpackOutputDrift",
                    format!(
                        "output entry `{}` must be a regular file or directory",
                        entry.path().display()
                    ),
                ));
            }
        }
    }
    Ok(files)
}

fn publish_output(
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
    publish_output_with_hook(output, files, || Ok(()))
}

fn publish_output_with_hook(
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    before_activation: impl FnOnce() -> Result<(), LawpackBuildFailure>,
) -> Result<(), LawpackBuildFailure> {
    publish_output_with_hooks(output, files, before_activation, |path| {
        fs::remove_dir_all(path)
    })
}

#[allow(
    clippy::too_many_lines,
    reason = "directory publication keeps staging and every rollback path explicit"
)]
fn publish_output_with_hooks(
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    before_activation: impl FnOnce() -> Result<(), LawpackBuildFailure>,
    remove_backup: impl FnOnce(&Path) -> std::io::Result<()>,
) -> Result<(), LawpackBuildFailure> {
    let parent = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a parent directory".to_owned(),
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        failure(
            "LawpackOutputWriteFailed",
            format!(
                "failed to create output parent `{}`: {error}",
                parent.display()
            ),
        )
    })?;
    let expected_owner = expected_output_owner(files)?;
    let _lock = acquire_output_lock(output)?;
    validate_owned_output(output, true, &expected_owner)?;

    let transaction = unique_sibling(output, "transaction")?;
    fs::create_dir(&transaction).map_err(|error| {
        failure(
            "LawpackOutputWriteFailed",
            format!(
                "failed to create output transaction `{}`: {error}",
                transaction.display()
            ),
        )
    })?;
    if let Err(error) = stage_files(&transaction, files) {
        let _ = fs::remove_dir_all(&transaction);
        return Err(error);
    }

    let existed = output.exists();
    let backup = match unique_sibling(output, "previous") {
        Ok(backup) => backup,
        Err(error) => {
            let _ = fs::remove_dir_all(&transaction);
            return Err(error);
        }
    };
    if existed {
        if let Err(error) = fs::rename(output, &backup) {
            let _ = fs::remove_dir_all(&transaction);
            return Err(failure(
                "LawpackOutputWriteFailed",
                format!(
                    "failed to preserve previous output `{}`: {error}",
                    output.display()
                ),
            ));
        }
    }

    if let Err(error) = before_activation() {
        let rollback = restore_output_directory(output, &backup, existed);
        let _ = fs::remove_dir_all(&transaction);
        return Err(rollback.err().unwrap_or(error));
    }
    if let Err(error) = fs::rename(&transaction, output) {
        let rollback = restore_output_directory(output, &backup, existed);
        let _ = fs::remove_dir_all(&transaction);
        return Err(rollback.err().unwrap_or_else(|| {
            failure(
                "LawpackOutputWriteFailed",
                format!("failed to activate output `{}`: {error}", output.display()),
            )
        }));
    }
    if existed {
        // Activation is the commit point. Backup cleanup is best effort so a
        // committed replacement is never reported as an unchanged failure.
        drop(remove_backup(&backup));
    }
    Ok(())
}

fn acquire_output_lock(output: &Path) -> Result<File, LawpackBuildFailure> {
    let lock_path = output_lock_path(output);
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!(
                    "failed to open output lock `{}`: {error}",
                    lock_path.display()
                ),
            )
        })?;
    lock.try_lock().map_err(|error| {
        failure(
            "LawpackOutputWriteFailed",
            format!(
                "another lawpack build owns output `{}`: {error}",
                output.display()
            ),
        )
    })?;
    Ok(lock)
}

fn expected_output_owner(
    expected: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(String, String), LawpackBuildFailure> {
    let bytes = expected.get(Path::new(OUTPUT_INDEX_FILE)).ok_or_else(|| {
        failure(
            "LawpackOutputOwnershipFailed",
            "authored output is missing its ownership index".to_owned(),
        )
    })?;
    let index = serde_json::from_slice::<ExistingOutputIndex>(bytes).map_err(|error| {
        failure(
            "LawpackOutputOwnershipFailed",
            format!("invalid authored output ownership index: {error}"),
        )
    })?;
    validate_existing_index(&index)?;
    Ok((index.lawpack_id, index.lawpack_version))
}

fn validate_owned_output(
    output: &Path,
    allow_missing: bool,
    expected_owner: &(String, String),
) -> Result<(), LawpackBuildFailure> {
    let metadata = match fs::symlink_metadata(output) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound && allow_missing => return Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(failure(
                "LawpackOutputDrift",
                format!("lawpack output `{}` does not exist", output.display()),
            ));
        }
        Err(error) => {
            return Err(failure(
                "LawpackOutputOwnershipFailed",
                format!("failed to inspect output `{}`: {error}", output.display()),
            ));
        }
    };
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(failure(
            "LawpackOutputOwnershipFailed",
            format!(
                "lawpack output `{}` must be a real directory",
                output.display()
            ),
        ));
    }
    let mut entries = fs::read_dir(output).map_err(|error| {
        failure(
            "LawpackOutputOwnershipFailed",
            format!("failed to inspect output `{}`: {error}", output.display()),
        )
    })?;
    if entries.next().is_none() {
        return Ok(());
    }
    let index_path = output.join(OUTPUT_INDEX_FILE);
    let index_bytes = read_bounded(
        &index_path,
        MAX_LAWPACK_ARTIFACT_BYTES,
        "lawpack output ownership index",
        "LawpackOutputOwnershipFailed",
    )?;
    let index = serde_json::from_slice::<ExistingOutputIndex>(&index_bytes).map_err(|error| {
        failure(
            "LawpackOutputOwnershipFailed",
            format!("invalid output ownership index: {error}"),
        )
    })?;
    validate_existing_index(&index)?;
    if (&index.lawpack_id, &index.lawpack_version) != (&expected_owner.0, &expected_owner.1) {
        return Err(failure(
            "LawpackOutputOwnershipFailed",
            format!(
                "output is owned by {}@{}, not {}@{}",
                index.lawpack_id, index.lawpack_version, expected_owner.0, expected_owner.1
            ),
        ));
    }
    Ok(())
}

fn validate_existing_index(index: &ExistingOutputIndex) -> Result<(), LawpackBuildFailure> {
    let mut paths = BTreeSet::new();
    if index.schema != LAWPACK_OUTPUT_SCHEMA
        || index.lawpack_id.is_empty()
        || index.lawpack_version.is_empty()
        || index.artifacts.is_empty()
        || index.artifacts.iter().any(|artifact| {
            let path = Path::new(&artifact.path);
            artifact.path.is_empty()
                || path.is_absolute()
                || path
                    .components()
                    .any(|component| !matches!(component, Component::Normal(_)))
                || !paths.insert(artifact.path.as_str())
                || !is_artifact_kind(&artifact.kind)
                || artifact.coordinate.is_empty()
                || !is_lowercase_digest(&artifact.digest)
        })
    {
        return Err(failure(
            "LawpackOutputOwnershipFailed",
            "output ownership index does not match edict.lawpack-output/v1".to_owned(),
        ));
    }
    Ok(())
}

fn stage_files(
    transaction: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
    for (relative, bytes) in files {
        validate_relative_path(relative, "authored artifact path")?;
        let path = transaction.join(relative);
        let parent = path.parent().ok_or_else(|| {
            failure(
                "LawpackOutputWriteFailed",
                format!("authored artifact `{}` has no parent", relative.display()),
            )
        })?;
        fs::create_dir_all(parent).map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!("failed to stage `{}`: {error}", relative.display()),
            )
        })?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|error| {
                failure(
                    "LawpackOutputWriteFailed",
                    format!("failed to stage `{}`: {error}", relative.display()),
                )
            })?;
        file.write_all(bytes).map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!("failed to write staged `{}`: {error}", relative.display()),
            )
        })?;
        file.sync_all().map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!("failed to sync staged `{}`: {error}", relative.display()),
            )
        })?;
    }
    Ok(())
}

fn restore_output_directory(
    output: &Path,
    backup: &Path,
    existed: bool,
) -> Result<(), LawpackBuildFailure> {
    if output.exists() {
        fs::remove_dir_all(output).map_err(|error| {
            failure(
                "LawpackOutputRollbackFailed",
                format!(
                    "failed to remove partial output `{}`: {error}",
                    output.display()
                ),
            )
        })?;
    }
    if existed {
        fs::rename(backup, output).map_err(|error| {
            failure(
                "LawpackOutputRollbackFailed",
                format!("failed to restore output `{}`: {error}", output.display()),
            )
        })?;
    }
    Ok(())
}

fn unique_sibling(output: &Path, role: &str) -> Result<PathBuf, LawpackBuildFailure> {
    let parent = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a parent directory".to_owned(),
        )
    })?;
    let name = output.file_name().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a final path component".to_owned(),
        )
    })?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!("system clock cannot name output transaction: {error}"),
            )
        })?
        .as_nanos();
    for attempt in 0..16 {
        let candidate = parent.join(format!(
            ".{}-edict-lawpack-{role}-{}-{timestamp}-{attempt}",
            name.to_string_lossy(),
            std::process::id()
        ));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }
    Err(failure(
        "LawpackOutputWriteFailed",
        format!(
            "failed to allocate a unique {role} path beside `{}`",
            output.display()
        ),
    ))
}

fn output_lock_path(output: &Path) -> PathBuf {
    let mut name = OsString::from(".");
    name.push(
        output
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("lawpack")),
    );
    name.push(".edict-lawpack-build.lock");
    output.parent().unwrap_or_else(|| Path::new(".")).join(name)
}

const fn artifact_kind_name(kind: LawpackArtifactKind) -> &'static str {
    match kind {
        LawpackArtifactKind::Manifest => "manifest",
        LawpackArtifactKind::ManifestDigest => "manifestDigest",
        LawpackArtifactKind::Exports => "exports",
        LawpackArtifactKind::ExportsDigest => "exportsDigest",
        LawpackArtifactKind::Adapter => "adapter",
        LawpackArtifactKind::AdapterDigest => "adapterDigest",
        LawpackArtifactKind::LocalResource => "localResource",
        LawpackArtifactKind::LocalResourceDigest => "localResourceDigest",
    }
}

const fn authoring_failure_kind(kind: LawpackAuthoringFailureKind) -> &'static str {
    match kind {
        LawpackAuthoringFailureKind::InvalidDefinition => "LawpackAuthoringInvalidDefinition",
        LawpackAuthoringFailureKind::InvalidDigest => "LawpackAuthoringInvalidDigest",
        LawpackAuthoringFailureKind::InvalidCanonicalValue => {
            "LawpackAuthoringInvalidCanonicalValue"
        }
        LawpackAuthoringFailureKind::MissingLocalResource => "LawpackAuthoringMissingLocalResource",
        LawpackAuthoringFailureKind::DuplicateIdentity => "LawpackAuthoringDuplicateIdentity",
        LawpackAuthoringFailureKind::InvalidOutputPath => "LawpackAuthoringInvalidOutputPath",
        LawpackAuthoringFailureKind::EncodingFailed => "LawpackAuthoringEncodingFailed",
        LawpackAuthoringFailureKind::InvalidLawpack => "LawpackAuthoringInvalidLawpack",
        LawpackAuthoringFailureKind::InvalidAdapter => "LawpackAuthoringInvalidAdapter",
        LawpackAuthoringFailureKind::MissingDependency => "LawpackAuthoringMissingDependency",
        LawpackAuthoringFailureKind::DependencyDigestMismatch => {
            "LawpackAuthoringDependencyDigestMismatch"
        }
        LawpackAuthoringFailureKind::InvalidDependencyClosure => {
            "LawpackAuthoringInvalidDependencyClosure"
        }
    }
}

fn is_lowercase_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_artifact_kind(value: &str) -> bool {
    matches!(
        value,
        "manifest"
            | "manifestDigest"
            | "exports"
            | "exportsDigest"
            | "adapter"
            | "adapterDigest"
            | "localResource"
            | "localResourceDigest"
    )
}

fn failure(kind: &'static str, message: String) -> LawpackBuildFailure {
    LawpackBuildFailure { kind, message }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;

    use super::{
        check_output, encode_output_index, load_dependencies, publish_output,
        publish_output_with_hook, publish_output_with_hooks, read_output_tree, LawpackBuildFailure,
        LawpackDependencyBundle, LawpackOutputIndex, LawpackOutputIndexEntry,
    };

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn publication_replaces_the_owned_tree_and_removes_stale_files() {
        let root = temp_tree("replace");
        let output = root.join("generated");
        let first = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("old", b"old"),
        ]);
        test_ok(publish_output(&output, &first), "publish initial set");

        let second = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"new"),
        ]);
        test_ok(publish_output(&output, &second), "replace owned set");

        assert_eq!(test_ok(read_output_tree(&output), "read output"), second);
        assert!(!output.join("old").exists());
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn injected_pre_activation_failure_restores_the_previous_tree() {
        let root = temp_tree("rollback");
        let output = root.join("generated");
        let original = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("old", b"old"),
        ]);
        test_ok(publish_output(&output, &original), "publish original");
        let replacement = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"new"),
        ]);

        let result = publish_output_with_hook(&output, &replacement, || {
            Err(LawpackBuildFailure {
                kind: "InjectedFailure",
                message: "injected before activation".to_owned(),
            })
        });

        assert_eq!(
            test_err(result, "injection rejects").kind,
            "InjectedFailure"
        );
        assert_eq!(test_ok(read_output_tree(&output), "read output"), original);
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn check_only_detects_exact_tree_drift() {
        let root = temp_tree("check");
        let output = root.join("generated");
        let expected = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);
        test_ok(publish_output(&output, &expected), "publish expected set");
        test_ok(check_output(&output, &expected), "exact output passes");
        test_ok(fs::write(output.join("one"), b"2"), "mutate output");

        assert_eq!(
            test_err(check_output(&output, &expected), "drift rejects").kind,
            "LawpackOutputDrift"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn check_only_observes_under_the_publication_lock() {
        let root = temp_tree("check-lock");
        let output = root.join("generated");
        let original = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);
        test_ok(publish_output(&output, &original), "publish original");
        let replacement = files(&[("edict.lawpack-output.json", valid_index()), ("two", b"2")]);
        let publisher_output = output.clone();
        let publisher_files = replacement.clone();
        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let publisher = std::thread::spawn(move || {
            publish_output_with_hook(&publisher_output, &publisher_files, || {
                test_ok(locked_tx.send(()), "announce held publication lock");
                test_ok(release_rx.recv(), "wait for check attempt");
                Ok(())
            })
        });
        test_ok(locked_rx.recv(), "wait for held publication lock");

        let error = test_err(
            check_output(&output, &replacement),
            "check cannot race an active publication",
        );
        assert_eq!(error.kind, "LawpackOutputWriteFailed");
        test_ok(release_tx.send(()), "release publisher");
        test_ok(
            test_ok(publisher.join(), "join publisher"),
            "finish publication",
        );
        test_ok(check_output(&output, &replacement), "new tree checks");
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn generated_ownership_index_rejects_its_own_read_limit() {
        let index = LawpackOutputIndex {
            schema: "edict.lawpack-output/v1",
            lawpack_id: "test",
            lawpack_version: "1",
            artifacts: (0..10_000)
                .map(|_| LawpackOutputIndexEntry {
                    path: "resources/one.cbor",
                    kind: "localResource",
                    coordinate: "example.resource/v1",
                    digest:
                        "sha256:1111111111111111111111111111111111111111111111111111111111111111",
                })
                .collect(),
        };

        assert_eq!(
            test_err(encode_output_index(&index), "oversized index rejects").kind,
            "LawpackOutputTooLarge"
        );
    }

    #[test]
    fn check_only_rejects_an_unexpected_file_before_reading_it() {
        let root = temp_tree("unexpected");
        let output = root.join("generated");
        let expected = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);
        test_ok(publish_output(&output, &expected), "publish expected set");
        test_ok(
            fs::write(output.join("unexpected"), vec![0; 1024 * 1024 + 1]),
            "write oversized unexpected file",
        );

        let error = test_err(check_output(&output, &expected), "unexpected path rejects");
        assert_eq!(error.kind, "LawpackOutputDrift");
        assert!(error.message.contains("unexpected path"));
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn post_activation_cleanup_failure_keeps_the_committed_success() {
        let root = temp_tree("cleanup");
        let output = root.join("generated");
        let original = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("old", b"old"),
        ]);
        test_ok(publish_output(&output, &original), "publish original");
        let replacement = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"new"),
        ]);

        test_ok(
            publish_output_with_hooks(
                &output,
                &replacement,
                || Ok(()),
                |_| Err(std::io::Error::other("injected cleanup failure")),
            ),
            "activation remains successful",
        );
        assert_eq!(
            test_ok(read_output_tree(&output), "read committed output"),
            replacement
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn publication_rejects_an_output_owned_by_another_lawpack() {
        let root = temp_tree("foreign-owner");
        let output = root.join("generated");
        let foreign_index = valid_index_for("other", "9");
        let foreign = files(&[
            ("edict.lawpack-output.json", foreign_index.as_slice()),
            ("manifest.cbor", b"foreign"),
        ]);
        test_ok(fs::create_dir_all(&output), "create foreign output");
        for (path, bytes) in &foreign {
            test_ok(fs::write(output.join(path), bytes), "write foreign output");
        }
        let expected = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("manifest.cbor", b"expected"),
        ]);

        let error = test_err(
            publish_output(&output, &expected),
            "foreign-owned output rejects",
        );
        assert_eq!(error.kind, "LawpackOutputOwnershipFailed");
        assert_eq!(
            test_ok(read_output_tree(&output), "read foreign tree"),
            foreign
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn dependency_inputs_inside_the_owned_output_tree_reject_before_reading() {
        let root = temp_tree("dependency-output");
        let output = root.join("generated");
        test_ok(fs::create_dir_all(&output), "create output");
        test_ok(
            fs::write(output.join("manifest.cbor"), b"not cbor"),
            "write manifest",
        );
        test_ok(
            fs::write(output.join("exports.cbor"), b"not cbor"),
            "write exports",
        );
        let definitions = [LawpackDependencyBundle {
            manifest: PathBuf::from("generated/manifest.cbor"),
            exports: PathBuf::from("generated/exports.cbor"),
        }];

        assert_eq!(
            test_err(
                load_dependencies(&root, &definitions, &output),
                "dependency inside output rejects",
            )
            .kind,
            "InvalidLawpackConfig"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    fn files(entries: &[(&str, &[u8])]) -> BTreeMap<PathBuf, Vec<u8>> {
        entries
            .iter()
            .map(|(path, bytes)| (PathBuf::from(path), bytes.to_vec()))
            .collect()
    }

    fn valid_index() -> &'static [u8] {
        br#"{"schema":"edict.lawpack-output/v1","lawpackId":"test","lawpackVersion":"1","artifacts":[{"path":"manifest.cbor","kind":"manifest","coordinate":"edict.lawpack/v1","digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111"}]}"#
    }

    fn valid_index_for(id: &str, version: &str) -> Vec<u8> {
        format!(
            r#"{{"schema":"edict.lawpack-output/v1","lawpackId":"{id}","lawpackVersion":"{version}","artifacts":[{{"path":"manifest.cbor","kind":"manifest","coordinate":"edict.lawpack/v1","digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111"}}]}}"#
        )
        .into_bytes()
    }

    fn temp_tree(name: &str) -> PathBuf {
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "edict-lawpack-build-{name}-{}-{unique}",
            std::process::id()
        ));
        test_ok(fs::create_dir_all(&path), "create test tree");
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
            Ok(value) => panic!("{context}: expected error, got {value:?}"),
            Err(error) => error,
        }
    }
}
