//! Public filesystem boundary for deterministic lawpack authoring.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{ErrorKind, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use cap_fs_ext::{DirExt as _, FollowSymlinks, OpenOptionsFollowExt as _};
use cap_std::ambient_authority;
use cap_std::fs::{Dir, OpenOptions as CapOpenOptions};
use edict_syntax::{
    author_lawpack, decode_lawpack_bundle, preflight_lawpack_authoring_paths, LawpackArtifactKind,
    LawpackAuthoringDefinition, LawpackAuthoringFailure, LawpackAuthoringFailureKind,
    ValidatedLawpackBundle,
};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

const LAWPACK_BUILD_SCHEMA: &str = "edict.lawpack-build/v1";
const LAWPACK_OUTPUT_SCHEMA: &str = "edict.lawpack-output/v1";
const OUTPUT_INDEX_FILE: &str = "edict.lawpack-output.json";
const INTERNAL_PUBLICATION_PREFIX: &str = ".edict-lawpack-";
const OUTPUT_LOCK_SUFFIX: &str = ".edict-lawpack-build.lock";
const MAX_LAWPACK_DOCUMENT_BYTES: u64 = 1024 * 1024;
const MAX_LAWPACK_ARTIFACT_BYTES: u64 = 1024 * 1024;
const MAX_DEPENDENCY_BUNDLES: usize = 192;
const MAX_OUTPUT_DIRECTORY_COMPONENT_BYTES: usize = 229;
const MAX_OUTPUT_DIRECTORY_PATH_BYTES: usize = 1022;
static PUBLICATION_NAME_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub(crate) struct LawpackBuildFailure {
    pub(crate) kind: &'static str,
    pub(crate) message: String,
}

#[derive(Debug)]
struct PublicationAuthority {
    output_parent: Dir,
    #[allow(
        dead_code,
        reason = "lock handles keep ancestor intents held for the authority lifetime"
    )]
    ancestor_locks: Vec<File>,
}

struct OpenedDependencyInput {
    file: File,
    display: PathBuf,
}

#[derive(Clone, Copy)]
enum OutputLockMode {
    SharedIntent,
    ExclusiveOutput,
}

#[cfg(test)]
impl PublicationAuthority {
    fn len(&self) -> usize {
        self.ancestor_locks.len()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LawpackBuildDocument {
    schema: String,
    output_directory: String,
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

struct UniqueJsonValue(Value);

impl<'de> Deserialize<'de> for UniqueJsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(UniqueJsonVisitor)
    }
}

struct UniqueJsonVisitor;

impl<'de> Visitor<'de> for UniqueJsonVisitor {
    type Value = UniqueJsonValue;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("JSON without duplicate object keys")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Null))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Null))
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Number(value.into())))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::Number(value.into())))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        serde_json::Number::from_f64(value)
            .map(Value::Number)
            .map(UniqueJsonValue)
            .ok_or_else(|| E::custom("non-finite JSON number"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(UniqueJsonValue(Value::String(value)))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(UniqueJsonValue(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(UniqueJsonValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut values = serde_json::Map::new();
        while let Some(key) = map.next_key::<String>()? {
            if values.contains_key(&key) {
                return Err(de::Error::custom(format!("duplicate JSON key `{key}`")));
            }
            let UniqueJsonValue(value) = map.next_value()?;
            values.insert(key, value);
        }
        Ok(UniqueJsonValue(Value::Object(values)))
    }
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
    let document = decode_lawpack_document(&document_bytes)?;
    validate_document(&document)?;
    preflight_lawpack_authoring_paths(&document.lawpack).map_err(first_authoring_failure)?;
    let output = resolve_output_directory(root, &document.output_directory)?;
    let dependencies = load_dependencies(root, &document.dependency_bundles, &output)?;
    let authored =
        author_lawpack(&document.lawpack, &dependencies).map_err(first_authoring_failure)?;

    let mut files = BTreeMap::new();
    for artifact in authored.artifacts() {
        validate_generated_artifact_size(artifact.path(), artifact.bytes())?;
        files.insert(PathBuf::from(artifact.path()), artifact.bytes().to_vec());
    }
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
        check_output_in_root(root, &output, &files)
    } else {
        publish_output_in_root(root, &output, &files)
    }
}

fn decode_lawpack_document(bytes: &[u8]) -> Result<LawpackBuildDocument, LawpackBuildFailure> {
    let UniqueJsonValue(document_value) = serde_json::from_slice::<UniqueJsonValue>(bytes)
        .map_err(|error| {
            failure(
                "InvalidLawpackConfig",
                format!("invalid lawpack build document: {error}"),
            )
        })?;
    serde_json::from_value::<LawpackBuildDocument>(document_value).map_err(|error| {
        failure(
            "InvalidLawpackConfig",
            format!("invalid lawpack build document: {error}"),
        )
    })
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

fn validate_generated_artifact_size(path: &str, bytes: &[u8]) -> Result<(), LawpackBuildFailure> {
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_LAWPACK_ARTIFACT_BYTES {
        return Err(failure(
            "LawpackOutputTooLarge",
            format!(
                "generated lawpack artifact `{path}` exceeds {MAX_LAWPACK_ARTIFACT_BYTES} bytes"
            ),
        ));
    }
    Ok(())
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
    validate_output_directory_path(&document.output_directory)
}

fn load_dependencies(
    root: &Path,
    definitions: &[LawpackDependencyBundle],
    output: &Path,
) -> Result<Vec<ValidatedLawpackBundle>, LawpackBuildFailure> {
    load_dependencies_with_hook(root, definitions, output, || {})
}

fn load_dependencies_with_hook(
    root: &Path,
    definitions: &[LawpackDependencyBundle],
    output: &Path,
    after_resolve: impl FnOnce(),
) -> Result<Vec<ValidatedLawpackBundle>, LawpackBuildFailure> {
    let root_dir = open_publication_root(root)?;
    let relative_output = output.strip_prefix(root).map_err(|error| {
        failure(
            "LawpackPathOutsideRoot",
            format!(
                "output `{}` resolves outside `{}`: {error}",
                output.display(),
                root.display()
            ),
        )
    })?;
    let resolved = definitions
        .iter()
        .enumerate()
        .map(|(index, definition)| {
            let manifest = open_dependency_input(
                &root_dir,
                root,
                &definition.manifest,
                relative_output,
                &format!("dependencyBundles.{index}.manifest"),
            )?;
            let exports = open_dependency_input(
                &root_dir,
                root,
                &definition.exports,
                relative_output,
                &format!("dependencyBundles.{index}.exports"),
            )?;
            Ok((manifest, exports))
        })
        .collect::<Result<Vec<_>, LawpackBuildFailure>>()?;
    after_resolve();
    resolved
        .into_iter()
        .enumerate()
        .map(|(index, (manifest, exports))| {
            let manifest = read_bounded_file(
                manifest.file,
                &manifest.display,
                MAX_LAWPACK_ARTIFACT_BYTES,
                "dependency manifest",
                "LawpackArtifactReadFailed",
            )?;
            let exports = read_bounded_file(
                exports.file,
                &exports.display,
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

fn open_dependency_input(
    root_dir: &Dir,
    root: &Path,
    relative: &Path,
    relative_output: &Path,
    field: &str,
) -> Result<OpenedDependencyInput, LawpackBuildFailure> {
    open_dependency_input_with_hook(root_dir, root, relative, relative_output, field, || {})
}

fn open_dependency_input_with_hook(
    root_dir: &Dir,
    root: &Path,
    relative: &Path,
    relative_output: &Path,
    field: &str,
    after_inspection: impl FnOnce(),
) -> Result<OpenedDependencyInput, LawpackBuildFailure> {
    validate_relative_path(relative, field)?;
    if relative.starts_with(relative_output) {
        return Err(failure(
            "InvalidLawpackConfig",
            format!("{field} must be outside outputDirectory"),
        ));
    }
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    let (directory, mut display) = open_dependency_parent(root_dir, root, parent, field)?;
    let name = relative.file_name().ok_or_else(|| {
        failure(
            "InvalidLawpackConfig",
            format!("{field} must have a final path component"),
        )
    })?;
    display.push(name);
    let metadata = directory.symlink_metadata(name).map_err(|error| {
        failure(
            "LawpackArtifactReadFailed",
            format!("failed to inspect {field} `{}`: {error}", display.display()),
        )
    })?;
    if metadata.is_symlink() {
        return Err(failure(
            "InvalidLawpackConfig",
            format!(
                "{field} `{}` must not be a symbolic link",
                display.display()
            ),
        ));
    }
    if !metadata.is_file() {
        return Err(failure(
            "LawpackPathOutsideRoot",
            format!(
                "{field} `{}` must be a real regular file",
                display.display()
            ),
        ));
    }
    after_inspection();
    let mut options = CapOpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = directory
        .open_with(name, &options)
        .map(cap_std::fs::File::into_std)
        .map_err(|error| {
            failure(
                "LawpackArtifactReadFailed",
                format!("failed to pin {field} `{}`: {error}", display.display()),
            )
        })?;
    Ok(OpenedDependencyInput { file, display })
}

fn open_dependency_parent(
    root_dir: &Dir,
    root: &Path,
    parent: &Path,
    field: &str,
) -> Result<(Dir, PathBuf), LawpackBuildFailure> {
    let mut directory = root_dir.try_clone().map_err(|error| {
        failure(
            "LawpackArtifactReadFailed",
            format!("failed to pin lawpack root for {field}: {error}"),
        )
    })?;
    let mut display = root.to_path_buf();
    for component in parent.components() {
        let Component::Normal(name) = component else {
            return Err(failure(
                "LawpackPathOutsideRoot",
                format!("{field} must contain only normal relative components"),
            ));
        };
        display.push(name);
        let metadata = directory.symlink_metadata(name).map_err(|error| {
            failure(
                "LawpackArtifactReadFailed",
                format!(
                    "failed to inspect {field} ancestor `{}`: {error}",
                    display.display()
                ),
            )
        })?;
        if metadata.is_symlink() {
            return Err(failure(
                "InvalidLawpackConfig",
                format!(
                    "{field} ancestor `{}` must not be a symbolic link",
                    display.display()
                ),
            ));
        }
        if !metadata.is_dir() {
            return Err(failure(
                "LawpackPathOutsideRoot",
                format!(
                    "{field} ancestor `{}` must be a real directory",
                    display.display()
                ),
            ));
        }
        directory = directory.open_dir_nofollow(name).map_err(|error| {
            failure(
                "LawpackArtifactReadFailed",
                format!(
                    "failed to pin {field} ancestor `{}`: {error}",
                    display.display()
                ),
            )
        })?;
    }
    Ok((directory, display))
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
    read_bounded_file(file, path, limit, subject, kind)
}

fn read_bounded_file(
    file: File,
    display: &Path,
    limit: u64,
    subject: &str,
    kind: &'static str,
) -> Result<Vec<u8>, LawpackBuildFailure> {
    let metadata = file.metadata().map_err(|error| {
        failure(
            kind,
            format!(
                "failed to inspect {subject} `{}`: {error}",
                display.display()
            ),
        )
    })?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(failure(
            kind,
            format!(
                "{subject} `{}` must be a regular file no larger than {limit} bytes",
                display.display()
            ),
        ));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            failure(
                kind,
                format!("failed to read {subject} `{}`: {error}", display.display()),
            )
        })?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
        return Err(failure(
            kind,
            format!("{subject} `{}` exceeds {limit} bytes", display.display()),
        ));
    }
    Ok(bytes)
}

fn read_bounded_in(
    directory: &Dir,
    path: &Path,
    limit: u64,
    subject: &str,
    kind: &'static str,
) -> Result<Vec<u8>, LawpackBuildFailure> {
    let mut options = CapOpenOptions::new();
    options.read(true).follow(FollowSymlinks::No);
    let file = directory.open_with(path, &options).map_err(|error| {
        failure(
            kind,
            format!("failed to open {subject} `{}`: {error}", path.display()),
        )
    })?;
    read_bounded_file(file.into_std(), path, limit, subject, kind)
}

fn resolve_output_directory(root: &Path, relative: &str) -> Result<PathBuf, LawpackBuildFailure> {
    validate_output_directory_path(relative)?;
    let relative = Path::new(relative);
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
    reject_owned_output_ancestor(root, &current)?;
    Ok(current)
}

fn reject_owned_output_ancestor(root: &Path, output: &Path) -> Result<(), LawpackBuildFailure> {
    let mut ancestor = output.parent();
    while let Some(directory) = ancestor {
        if !directory.starts_with(root) {
            break;
        }
        match fs::symlink_metadata(directory) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(failure(
                    "LawpackPathOutsideRoot",
                    format!(
                        "output ancestor `{}` must be a real directory",
                        directory.display()
                    ),
                ));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(failure(
                    "LawpackOutputWriteFailed",
                    format!(
                        "failed to inspect output ancestor `{}`: {error}",
                        directory.display()
                    ),
                ));
            }
        }
        let ownership_index = directory.join(OUTPUT_INDEX_FILE);
        match fs::symlink_metadata(&ownership_index) {
            Ok(_) => {
                return Err(failure(
                    "LawpackOutputOwnershipFailed",
                    format!(
                        "output `{}` is nested inside owned lawpack tree `{}`",
                        output.display(),
                        directory.display()
                    ),
                ));
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(failure(
                    "LawpackOutputOwnershipFailed",
                    format!(
                        "failed to inspect ancestor ownership index `{}`: {error}",
                        ownership_index.display()
                    ),
                ));
            }
        }
        if directory == root {
            break;
        }
        ancestor = directory.parent();
    }
    Ok(())
}

#[cfg(test)]
fn validate_check_output_parent_chain(
    root: &Path,
    output: &Path,
) -> Result<(), LawpackBuildFailure> {
    let root_dir = open_check_root(root)?;
    open_check_output_parent_in_root(&root_dir, root, output).map(drop)
}

fn open_check_root(root: &Path) -> Result<Dir, LawpackBuildFailure> {
    open_check_root_with_hook(root, || {})
}

fn open_check_root_with_hook(
    root: &Path,
    after_inspection: impl FnOnce(),
) -> Result<Dir, LawpackBuildFailure> {
    let metadata = fs::symlink_metadata(root).map_err(|error| {
        let kind = if error.kind() == ErrorKind::NotFound {
            "LawpackOutputDrift"
        } else {
            "LawpackOutputOwnershipFailed"
        };
        failure(
            kind,
            format!(
                "failed to inspect output ancestor `{}`: {error}",
                root.display()
            ),
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(failure(
            "LawpackOutputOwnershipFailed",
            format!(
                "output ancestor `{}` must remain a real directory",
                root.display()
            ),
        ));
    }
    after_inspection();
    let (Some(parent), Some(name)) = (root.parent(), root.file_name()) else {
        return Dir::open_ambient_dir(root, ambient_authority()).map_err(|error| {
            failure(
                "LawpackOutputOwnershipFailed",
                format!(
                    "failed to pin output ancestor `{}`: {error}",
                    root.display()
                ),
            )
        });
    };
    let parent_dir = Dir::open_ambient_dir(parent, ambient_authority()).map_err(|error| {
        failure(
            "LawpackOutputOwnershipFailed",
            format!(
                "failed to pin parent of output ancestor `{}`: {error}",
                root.display()
            ),
        )
    })?;
    parent_dir.open_dir_nofollow(name).map_err(|error| {
        failure(
            "LawpackOutputOwnershipFailed",
            format!(
                "failed to pin output ancestor `{}`: {error}",
                root.display()
            ),
        )
    })
}

fn open_check_output_parent_in_root(
    root_dir: &Dir,
    root: &Path,
    output: &Path,
) -> Result<Dir, LawpackBuildFailure> {
    let parent = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputDrift",
            "lawpack output must have a parent directory".to_owned(),
        )
    })?;
    let relative = parent.strip_prefix(root).map_err(|error| {
        failure(
            "LawpackPathOutsideRoot",
            format!(
                "output parent `{}` resolves outside `{}`: {error}",
                parent.display(),
                root.display()
            ),
        )
    })?;
    let mut current_path = root.to_path_buf();
    let mut current = root_dir.try_clone().map_err(|error| {
        failure(
            "LawpackOutputOwnershipFailed",
            format!("failed to retain output root `{}`: {error}", root.display()),
        )
    })?;
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(failure(
                "LawpackPathOutsideRoot",
                "output parent must contain only normal relative components".to_owned(),
            ));
        };
        current_path.push(name);
        let metadata = match current.symlink_metadata(name) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Err(failure(
                    "LawpackOutputDrift",
                    format!(
                        "lawpack output ancestor `{}` does not exist",
                        current_path.display()
                    ),
                ));
            }
            Err(error) => {
                return Err(failure(
                    "LawpackOutputOwnershipFailed",
                    format!(
                        "failed to inspect output ancestor `{}`: {error}",
                        current_path.display()
                    ),
                ));
            }
        };
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(failure(
                "LawpackOutputOwnershipFailed",
                format!(
                    "output ancestor `{}` must remain a real directory",
                    current_path.display()
                ),
            ));
        }
        current = current.open_dir_nofollow(name).map_err(|error| {
            failure(
                "LawpackOutputOwnershipFailed",
                format!(
                    "failed to pin output ancestor `{}`: {error}",
                    current_path.display()
                ),
            )
        })?;
    }
    Ok(current)
}

fn acquire_output_ancestor_locks(
    root: &Path,
    output: &Path,
) -> Result<PublicationAuthority, LawpackBuildFailure> {
    let parent = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a parent directory".to_owned(),
        )
    })?;
    let relative = parent.strip_prefix(root).map_err(|error| {
        failure(
            "LawpackPathOutsideRoot",
            format!(
                "output `{}` resolves outside `{}`: {error}",
                output.display(),
                root.display()
            ),
        )
    })?;
    let mut directory = open_publication_root(root)?;
    let mut display = root.to_path_buf();
    let mut locks = Vec::new();
    reject_owned_output_ancestor_directory(&directory, &display, output)?;
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(failure(
                "LawpackPathOutsideRoot",
                "output parent must contain only normal relative components".to_owned(),
            ));
        };
        display.push(name);
        locks.push(acquire_output_lock_in(
            &directory,
            name,
            &display,
            OutputLockMode::SharedIntent,
        )?);
        match directory.create_dir(name) {
            Ok(()) => {}
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(failure(
                    "LawpackOutputWriteFailed",
                    format!(
                        "failed to prepare output ancestor `{}`: {error}",
                        display.display()
                    ),
                ));
            }
        }
        let metadata = directory.symlink_metadata(name).map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!(
                    "failed to inspect output ancestor `{}`: {error}",
                    display.display()
                ),
            )
        })?;
        if !metadata.is_dir() || metadata.is_symlink() {
            return Err(failure(
                "LawpackPathOutsideRoot",
                format!(
                    "output ancestor `{}` must be a real directory",
                    display.display()
                ),
            ));
        }
        directory = directory.open_dir(name).map_err(|error| {
            failure(
                "LawpackPathOutsideRoot",
                format!(
                    "failed to pin output ancestor `{}`: {error}",
                    display.display()
                ),
            )
        })?;
        reject_owned_output_ancestor_directory(&directory, &display, output)?;
    }
    Ok(PublicationAuthority {
        output_parent: directory,
        ancestor_locks: locks,
    })
}

fn reject_owned_output_ancestor_directory(
    directory: &Dir,
    display: &Path,
    output: &Path,
) -> Result<(), LawpackBuildFailure> {
    match directory.symlink_metadata(OUTPUT_INDEX_FILE) {
        Ok(_) => Err(failure(
            "LawpackOutputOwnershipFailed",
            format!(
                "output `{}` is nested inside owned lawpack tree `{}`",
                output.display(),
                display.display()
            ),
        )),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(failure(
            "LawpackOutputOwnershipFailed",
            format!(
                "failed to inspect ancestor ownership index `{}`: {error}",
                display.join(OUTPUT_INDEX_FILE).display()
            ),
        )),
    }
}

fn open_publication_root(root: &Path) -> Result<Dir, LawpackBuildFailure> {
    Dir::open_ambient_dir(root, ambient_authority()).map_err(|error| {
        failure(
            "LawpackOutputWriteFailed",
            format!(
                "failed to open lawpack publication root `{}`: {error}",
                root.display()
            ),
        )
    })
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

fn validate_output_directory_path(path: &str) -> Result<(), LawpackBuildFailure> {
    let raw_components = path.split('/').collect::<Vec<_>>();
    let raw_grammar_is_portable = !path.is_empty()
        && path.len() <= MAX_OUTPUT_DIRECTORY_PATH_BYTES
        && !path.as_bytes().contains(&b'\\')
        && raw_components.iter().all(|component| {
            !component.is_empty()
                && component.len() <= MAX_OUTPUT_DIRECTORY_COMPONENT_BYTES
                && !matches!(*component, "." | "..")
                && !component.ends_with('.')
                && component
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
                && !is_windows_reserved_output_component(component)
        });
    if !raw_grammar_is_portable {
        return Err(failure(
            "InvalidLawpackConfig",
            "outputDirectory must use bounded portable non-empty `/`-separated components"
                .to_owned(),
        ));
    }
    validate_relative_path(Path::new(path), "outputDirectory")?;
    if raw_components.iter().any(|component| {
        let folded = component.to_ascii_lowercase();
        folded.starts_with(INTERNAL_PUBLICATION_PREFIX)
            || (folded.starts_with('.') && folded.ends_with(OUTPUT_LOCK_SUFFIX))
    }) {
        return Err(failure(
            "InvalidLawpackConfig",
            "outputDirectory must not use Edict's reserved publication namespace".to_owned(),
        ));
    }
    Ok(())
}

fn is_windows_reserved_output_component(component: &str) -> bool {
    let stem = component
        .split_once('.')
        .map_or(component, |(stem, _)| stem)
        .to_ascii_lowercase();
    matches!(stem.as_str(), "con" | "prn" | "aux" | "nul" | "clock$")
        || stem
            .strip_prefix("com")
            .or_else(|| stem.strip_prefix("lpt"))
            .is_some_and(|suffix| {
                matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
}

fn check_output_in_root(
    root: &Path,
    output: &Path,
    expected: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
    check_output_in_root_with_hooks(root, output, expected, || {}, || {})
}

#[cfg(test)]
fn check_output(
    output: &Path,
    expected: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
    let root = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputDrift",
            "lawpack output has no parent directory".to_owned(),
        )
    })?;
    check_output_in_root(root, output, expected)
}

#[cfg(test)]
fn check_output_with_hook(
    output: &Path,
    expected: &BTreeMap<PathBuf, Vec<u8>>,
    after_traversal: impl FnOnce(),
) -> Result<(), LawpackBuildFailure> {
    let root = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputDrift",
            "lawpack output has no parent directory".to_owned(),
        )
    })?;
    check_output_in_root_with_hooks(root, output, expected, || {}, after_traversal)
}

fn check_output_in_root_with_hooks(
    root: &Path,
    output: &Path,
    expected: &BTreeMap<PathBuf, Vec<u8>>,
    after_parent_pin: impl FnOnce(),
    after_traversal: impl FnOnce(),
) -> Result<(), LawpackBuildFailure> {
    let expected_owner = expected_output_owner(expected)?;
    let root_dir = open_check_root(root)?;
    let parent_dir = open_check_output_parent_in_root(&root_dir, root, output)?;
    let parent_identity = directory_identity(&parent_dir, output)?;
    after_parent_pin();
    let output_name = output.file_name().ok_or_else(|| {
        failure(
            "LawpackOutputDrift",
            "lawpack output has no final path component".to_owned(),
        )
    })?;
    let output_dir = open_check_output_dir(&parent_dir, output_name, output)?;
    let observed_identity = directory_identity(&output_dir, output)?;
    let basis = validate_owned_output_dir(&output_dir, output, &expected_owner)?;
    validate_output_tree(output, output_dir, expected)?;
    after_traversal();
    let current_parent = open_check_output_parent_in_root(&root_dir, root, output)?;
    if directory_identity(&current_parent, output)? != parent_identity {
        return Err(failure(
            "LawpackOutputDrift",
            format!(
                "lawpack output parent for `{}` changed while it was being checked",
                output.display()
            ),
        ));
    }
    let current_output = open_check_output_dir(&current_parent, output_name, output)?;
    if directory_identity(&current_output, output)? != observed_identity
        || validate_owned_output_dir(&current_output, output, &expected_owner)? != basis
    {
        return Err(failure(
            "LawpackOutputDrift",
            format!(
                "lawpack output `{}` changed while it was being checked",
                output.display()
            ),
        ));
    }
    validate_output_tree(output, current_output, expected)?;
    Ok(())
}

fn validate_output_tree(
    output: &Path,
    output_dir: Dir,
    expected: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
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
    let mut pending = vec![(output_dir, PathBuf::new())];
    while let Some((directory, relative_directory)) = pending.pop() {
        let entries = directory.entries().map_err(|error| {
            failure(
                "LawpackOutputDrift",
                format!(
                    "failed to inspect output `{}`: {error}",
                    output.join(&relative_directory).display()
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
            let relative = relative_directory.join(entry.file_name());
            let entry_path = output.join(&relative);
            let file_type = entry.file_type().map_err(|error| {
                failure(
                    "LawpackOutputDrift",
                    format!("failed to inspect `{}`: {error}", entry_path.display()),
                )
            })?;
            if file_type.is_dir() {
                if !permitted_directories.contains(&relative) {
                    return Err(unexpected_output_path(output, &relative));
                }
                let child = entry.open_dir().map_err(|error| {
                    failure(
                        "LawpackOutputDrift",
                        format!("failed to pin `{}`: {error}", entry_path.display()),
                    )
                })?;
                pending.push((child, relative));
            } else if file_type.is_file() {
                let Some(expected_bytes) = expected.get(&relative) else {
                    return Err(unexpected_output_path(output, &relative));
                };
                let file = entry
                    .open()
                    .map(cap_std::fs::File::into_std)
                    .map_err(|error| {
                        failure(
                            "LawpackOutputDrift",
                            format!("failed to pin `{}`: {error}", entry_path.display()),
                        )
                    })?;
                let bytes = read_bounded_file(
                    file,
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
                seen.insert(relative);
            } else {
                return Err(unexpected_output_path(output, &relative));
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

fn open_check_output_dir(
    parent: &Dir,
    output_name: &std::ffi::OsStr,
    output: &Path,
) -> Result<Dir, LawpackBuildFailure> {
    let metadata = parent.symlink_metadata(output_name).map_err(|error| {
        failure(
            "LawpackOutputDrift",
            format!("failed to inspect output `{}`: {error}", output.display()),
        )
    })?;
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err(failure(
            "LawpackOutputOwnershipFailed",
            format!(
                "lawpack output `{}` must be a real directory",
                output.display()
            ),
        ));
    }
    parent.open_dir(output_name).map_err(|error| {
        failure(
            "LawpackOutputDrift",
            format!("failed to pin output `{}`: {error}", output.display()),
        )
    })
}

#[cfg(any(unix, target_os = "wasi", target_os = "vxworks"))]
fn directory_identity(directory: &Dir, output: &Path) -> Result<(u64, u64), LawpackBuildFailure> {
    use cap_std::fs::MetadataExt as _;

    let metadata = directory.dir_metadata().map_err(|error| {
        failure(
            "LawpackOutputDrift",
            format!(
                "failed to identify output directory `{}`: {error}",
                output.display()
            ),
        )
    })?;
    Ok((metadata.dev(), metadata.ino()))
}

#[cfg(windows)]
fn directory_identity(directory: &Dir, output: &Path) -> Result<(u64, u64), LawpackBuildFailure> {
    use std::os::windows::fs::MetadataExt as _;

    let metadata = directory
        .try_clone()
        .map(Dir::into_std_file)
        .and_then(|file| file.metadata())
        .map_err(|error| {
            failure(
                "LawpackOutputDrift",
                format!(
                    "failed to identify output directory `{}`: {error}",
                    output.display()
                ),
            )
        })?;
    let volume = metadata.volume_serial_number().ok_or_else(|| {
        failure(
            "LawpackOutputDrift",
            format!(
                "output directory `{}` has no stable volume identity",
                output.display()
            ),
        )
    })?;
    let index = metadata.file_index().ok_or_else(|| {
        failure(
            "LawpackOutputDrift",
            format!(
                "output directory `{}` has no stable file identity",
                output.display()
            ),
        )
    })?;
    Ok((u64::from(volume), index))
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

#[cfg(test)]
fn publish_output(
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
    let root = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a parent directory".to_owned(),
        )
    })?;
    publish_output_in_root(root, output, files)
}

fn publish_output_in_root(
    root: &Path,
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
    publish_output_with_hooks_in_root(
        root,
        output,
        files,
        || Ok(()),
        || Ok(()),
        Dir::remove_open_dir_all,
    )
}

#[cfg(test)]
fn publish_output_with_hook(
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    before_activation: impl FnOnce() -> Result<(), LawpackBuildFailure>,
) -> Result<(), LawpackBuildFailure> {
    let root = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a parent directory".to_owned(),
        )
    })?;
    publish_output_with_hooks_in_root(
        root,
        output,
        files,
        || Ok(()),
        before_activation,
        Dir::remove_open_dir_all,
    )
}

#[cfg(test)]
fn publish_output_with_capture_hook(
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    before_capture: impl FnOnce() -> Result<(), LawpackBuildFailure>,
) -> Result<(), LawpackBuildFailure> {
    let root = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a parent directory".to_owned(),
        )
    })?;
    publish_output_with_hooks_in_root(
        root,
        output,
        files,
        before_capture,
        || Ok(()),
        Dir::remove_open_dir_all,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "directory publication keeps staging and every rollback path explicit"
)]
#[cfg(test)]
fn publish_output_with_hooks(
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    before_activation: impl FnOnce() -> Result<(), LawpackBuildFailure>,
    remove_backup: impl FnOnce(Dir) -> std::io::Result<()>,
) -> Result<(), LawpackBuildFailure> {
    let root = output.parent().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a parent directory".to_owned(),
        )
    })?;
    publish_output_with_hooks_in_root(
        root,
        output,
        files,
        || Ok(()),
        before_activation,
        remove_backup,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "directory publication keeps staging and every rollback path explicit"
)]
fn publish_output_with_hooks_in_root(
    root: &Path,
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    before_capture: impl FnOnce() -> Result<(), LawpackBuildFailure>,
    before_activation: impl FnOnce() -> Result<(), LawpackBuildFailure>,
    remove_backup: impl FnOnce(Dir) -> std::io::Result<()>,
) -> Result<(), LawpackBuildFailure> {
    let authority = acquire_output_ancestor_locks(root, output)?;
    publish_output_with_hooks_in_authority(
        &authority,
        output,
        files,
        before_capture,
        before_activation,
        || Ok(()),
        remove_backup,
    )
}

#[allow(
    clippy::too_many_lines,
    reason = "directory publication keeps staging and every rollback path explicit"
)]
fn publish_output_with_hooks_in_authority(
    authority: &PublicationAuthority,
    output: &Path,
    files: &BTreeMap<PathBuf, Vec<u8>>,
    before_capture: impl FnOnce() -> Result<(), LawpackBuildFailure>,
    before_activation: impl FnOnce() -> Result<(), LawpackBuildFailure>,
    after_activation: impl FnOnce() -> Result<(), LawpackBuildFailure>,
    remove_backup: impl FnOnce(Dir) -> std::io::Result<()>,
) -> Result<(), LawpackBuildFailure> {
    let parent_dir = &authority.output_parent;
    let output_name = output.file_name().ok_or_else(|| {
        failure(
            "LawpackOutputWriteFailed",
            "lawpack output must have a final path component".to_owned(),
        )
    })?;
    let expected_owner = expected_output_owner(files)?;
    let _lock = acquire_output_lock_in(
        parent_dir,
        output_name,
        output,
        OutputLockMode::ExclusiveOutput,
    )?;

    let transaction = unique_sibling_in(parent_dir, output, "transaction")?;
    parent_dir.create_dir(&transaction).map_err(|error| {
        failure(
            "LawpackOutputWriteFailed",
            format!(
                "failed to create output transaction `{}`: {error}",
                transaction.display()
            ),
        )
    })?;
    let transaction_dir = parent_dir.open_dir(&transaction).map_err(|error| {
        failure(
            "LawpackOutputWriteFailed",
            format!(
                "failed to pin output transaction beside `{}`: {error}",
                output.display()
            ),
        )
    })?;
    if let Err(error) = stage_files_in(&transaction_dir, files) {
        drop(Dir::remove_open_dir_all(transaction_dir));
        return Err(error);
    }
    let staged_identity = match directory_identity(&transaction_dir, output) {
        Ok(identity) => identity,
        Err(error) => {
            drop(Dir::remove_open_dir_all(transaction_dir));
            return Err(failure(
                "LawpackOutputWriteFailed",
                format!(
                    "failed to retain staged output identity for `{}`: {}",
                    output.display(),
                    error.message
                ),
            ));
        }
    };

    if let Err(error) = before_capture() {
        drop(Dir::remove_open_dir_all(transaction_dir));
        return Err(error);
    }
    let existed = entry_exists(parent_dir, output_name)?;
    let backup = match unique_sibling_in(parent_dir, output, "previous") {
        Ok(backup) => backup,
        Err(error) => {
            drop(Dir::remove_open_dir_all(transaction_dir));
            return Err(error);
        }
    };
    if existed {
        if let Err(error) = parent_dir.rename(output_name, parent_dir, &backup) {
            drop(Dir::remove_open_dir_all(transaction_dir));
            return Err(failure(
                "LawpackOutputWriteFailed",
                format!(
                    "failed to preserve previous output `{}`: {error}",
                    output.display()
                ),
            ));
        }
    }
    let captured = if existed {
        let captured = match parent_dir.open_dir(&backup) {
            Ok(captured) => captured,
            Err(error) => {
                let rollback =
                    restore_output_directory_in(parent_dir, output_name, &backup, output, true);
                drop(Dir::remove_open_dir_all(transaction_dir));
                return Err(rollback.err().unwrap_or_else(|| {
                    failure(
                        "LawpackOutputOwnershipFailed",
                        format!(
                            "failed to pin captured output `{}`: {error}",
                            output.display()
                        ),
                    )
                }));
            }
        };
        if let Err(error) = validate_owned_output_dir(&captured, output, &expected_owner) {
            drop(captured);
            let rollback =
                restore_output_directory_in(parent_dir, output_name, &backup, output, true);
            drop(Dir::remove_open_dir_all(transaction_dir));
            return Err(rollback.err().unwrap_or(error));
        }
        Some(captured)
    } else {
        None
    };

    if let Err(error) = before_activation() {
        drop(captured);
        let rollback =
            restore_output_directory_in(parent_dir, output_name, &backup, output, existed);
        drop(Dir::remove_open_dir_all(transaction_dir));
        return Err(rollback.err().unwrap_or(error));
    }
    if let Err(error) = parent_dir.rename(&transaction, parent_dir, output_name) {
        drop(captured);
        let rollback =
            restore_output_directory_in(parent_dir, output_name, &backup, output, existed);
        drop(Dir::remove_open_dir_all(transaction_dir));
        return Err(rollback.err().unwrap_or_else(|| {
            failure(
                "LawpackOutputWriteFailed",
                format!("failed to activate output `{}`: {error}", output.display()),
            )
        }));
    }
    if let Err(error) = after_activation() {
        drop(captured);
        let rollback = restore_after_substituted_activation_in(
            parent_dir,
            output_name,
            &transaction,
            &backup,
            output,
            existed,
        );
        drop(Dir::remove_open_dir_all(transaction_dir));
        return Err(rollback.err().unwrap_or(error));
    }
    let activated = open_check_output_dir(parent_dir, output_name, output);
    let activation_matches = activated
        .as_ref()
        .ok()
        .and_then(|directory| directory_identity(directory, output).ok())
        .is_some_and(|identity| identity == staged_identity);
    if !activation_matches {
        drop(activated);
        drop(captured);
        let rollback = restore_after_substituted_activation_in(
            parent_dir,
            output_name,
            &transaction,
            &backup,
            output,
            existed,
        );
        drop(Dir::remove_open_dir_all(transaction_dir));
        return Err(rollback.err().unwrap_or_else(|| {
            failure(
                "LawpackOutputWriteFailed",
                format!(
                    "activated output `{}` did not match the retained staged transaction identity",
                    output.display()
                ),
            )
        }));
    }
    if let Some(captured) = captured {
        // Activation is the commit point. Backup cleanup is best effort so a
        // committed replacement is never reported as an unchanged failure.
        drop(remove_backup(captured));
    }
    Ok(())
}

fn restore_after_substituted_activation_in(
    parent: &Dir,
    output_name: &std::ffi::OsStr,
    transaction: &Path,
    backup: &Path,
    output: &Path,
    existed: bool,
) -> Result<(), LawpackBuildFailure> {
    if entry_exists(parent, transaction.as_os_str())? {
        return Err(failure(
            "LawpackOutputRollbackFailed",
            format!(
                "refused to overwrite a concurrently installed transaction while restoring `{}`",
                output.display()
            ),
        ));
    }
    if !entry_exists(parent, output_name)? {
        return restore_output_directory_in(parent, output_name, backup, output, existed);
    }
    parent
        .rename(output_name, parent, transaction)
        .map_err(|error| {
            failure(
                "LawpackOutputRollbackFailed",
                format!(
                    "failed to preserve a substituted activation for `{}`: {error}",
                    output.display()
                ),
            )
        })?;
    restore_output_directory_in(parent, output_name, backup, output, existed)
}

fn acquire_output_lock_in(
    parent: &Dir,
    output_name: &std::ffi::OsStr,
    output: &Path,
    mode: OutputLockMode,
) -> Result<File, LawpackBuildFailure> {
    let lock_name = output_lock_name(output_name);
    let mut options = CapOpenOptions::new();
    options.create(true).read(true).write(true);
    let open_subject = match mode {
        OutputLockMode::SharedIntent => "output-intent lock",
        OutputLockMode::ExclusiveOutput => "output lock",
    };
    let lock = parent
        .open_with(&lock_name, &options)
        .map(cap_std::fs::File::into_std)
        .map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!(
                    "failed to open {open_subject} for `{}`: {error}",
                    output.display()
                ),
            )
        })?;
    match mode {
        OutputLockMode::SharedIntent => lock.try_lock_shared().map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!(
                    "an overlapping lawpack build owns output ancestor `{}`: {error}",
                    output.display()
                ),
            )
        })?,
        OutputLockMode::ExclusiveOutput => lock.try_lock().map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!(
                    "another lawpack build owns output `{}`: {error}",
                    output.display()
                ),
            )
        })?,
    }
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

fn validate_owned_output_dir(
    output_dir: &Dir,
    output: &Path,
    expected_owner: &(String, String),
) -> Result<Option<Vec<u8>>, LawpackBuildFailure> {
    validate_owned_output_dir_with_hook(output_dir, output, expected_owner, || {})
}

fn validate_owned_output_dir_with_hook(
    output_dir: &Dir,
    output: &Path,
    expected_owner: &(String, String),
    after_index_inspection: impl FnOnce(),
) -> Result<Option<Vec<u8>>, LawpackBuildFailure> {
    let mut entries = output_dir.entries().map_err(|error| {
        failure(
            "LawpackOutputOwnershipFailed",
            format!("failed to inspect output `{}`: {error}", output.display()),
        )
    })?;
    if entries.next().is_none() {
        return Ok(None);
    }
    let index_path = Path::new(OUTPUT_INDEX_FILE);
    let index_metadata = output_dir.symlink_metadata(index_path).map_err(|error| {
        failure(
            "LawpackOutputOwnershipFailed",
            format!("failed to inspect output ownership index: {error}"),
        )
    })?;
    if !index_metadata.is_file() || index_metadata.is_symlink() {
        return Err(failure(
            "LawpackOutputOwnershipFailed",
            "output ownership index must be a real regular file".to_owned(),
        ));
    }
    after_index_inspection();
    let index_bytes = read_bounded_in(
        output_dir,
        index_path,
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
    Ok(Some(index_bytes))
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

fn stage_files_in(
    transaction: &Dir,
    files: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<(), LawpackBuildFailure> {
    for (relative, bytes) in files {
        validate_relative_path(relative, "authored artifact path")?;
        let parent = relative.parent().ok_or_else(|| {
            failure(
                "LawpackOutputWriteFailed",
                format!("authored artifact `{}` has no parent", relative.display()),
            )
        })?;
        transaction.create_dir_all(parent).map_err(|error| {
            failure(
                "LawpackOutputWriteFailed",
                format!("failed to stage `{}`: {error}", relative.display()),
            )
        })?;
        let mut options = CapOpenOptions::new();
        options.create_new(true).write(true);
        let mut file = transaction.open_with(relative, &options).map_err(|error| {
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

fn restore_output_directory_in(
    parent: &Dir,
    output_name: &std::ffi::OsStr,
    backup: &Path,
    output: &Path,
    existed: bool,
) -> Result<(), LawpackBuildFailure> {
    if existed {
        if entry_exists(parent, output_name)? {
            return Err(failure(
                "LawpackOutputRollbackFailed",
                format!(
                    "refused to replace a concurrently installed output while restoring `{}`",
                    output.display()
                ),
            ));
        }
        parent
            .rename(backup, parent, output_name)
            .map_err(|error| {
                failure(
                    "LawpackOutputRollbackFailed",
                    format!("failed to restore output `{}`: {error}", output.display()),
                )
            })?;
    }
    Ok(())
}

fn unique_sibling_in(
    parent: &Dir,
    output: &Path,
    role: &str,
) -> Result<PathBuf, LawpackBuildFailure> {
    let sequence = PUBLICATION_NAME_COUNTER.fetch_add(1, Ordering::Relaxed);
    for attempt in 0..16 {
        let candidate = PathBuf::from(format!(
            ".edict-lawpack-{role}-{:08x}-{sequence:016x}-{attempt:02x}",
            std::process::id(),
        ));
        match parent.symlink_metadata(&candidate) {
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(candidate),
            Ok(_) => {}
            Err(error) => {
                return Err(failure(
                    "LawpackOutputWriteFailed",
                    format!(
                        "failed to inspect a candidate {role} path beside `{}`: {error}",
                        output.display()
                    ),
                ));
            }
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

fn entry_exists(parent: &Dir, name: &std::ffi::OsStr) -> Result<bool, LawpackBuildFailure> {
    match parent.symlink_metadata(name) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(failure(
            "LawpackOutputWriteFailed",
            format!("failed to inspect publication entry: {error}"),
        )),
    }
}

#[cfg(test)]
fn output_lock_path(output: &Path) -> PathBuf {
    let name = output_lock_name(
        output
            .file_name()
            .unwrap_or_else(|| std::ffi::OsStr::new("lawpack")),
    );
    output.parent().unwrap_or_else(|| Path::new(".")).join(name)
}

fn output_lock_name(output_name: &std::ffi::OsStr) -> OsString {
    let mut name = OsString::from(".");
    name.push(output_name);
    name.push(OUTPUT_LOCK_SUFFIX);
    name
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

fn first_authoring_failure(failures: Vec<LawpackAuthoringFailure>) -> LawpackBuildFailure {
    let Some(first) = failures.into_iter().next() else {
        return failure(
            "LawpackAuthoringFailed",
            "lawpack authoring failed without a diagnostic".to_owned(),
        );
    };
    failure(
        authoring_failure_kind(first.kind),
        format!("{}: {}", first.path, first.obligation),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        acquire_output_ancestor_locks, build_lawpack, check_output,
        check_output_in_root_with_hooks, check_output_with_hook, decode_lawpack_document,
        encode_output_index, load_dependencies, load_dependencies_with_hook,
        open_check_root_with_hook, open_dependency_input_with_hook, output_lock_path,
        publish_output, publish_output_with_capture_hook, publish_output_with_hook,
        publish_output_with_hooks, publish_output_with_hooks_in_authority,
        publish_output_with_hooks_in_root, read_output_tree, resolve_output_directory,
        validate_check_output_parent_chain, validate_generated_artifact_size,
        validate_owned_output_dir_with_hook, LawpackBuildFailure, LawpackDependencyBundle,
        LawpackOutputIndex, LawpackOutputIndexEntry, MAX_OUTPUT_DIRECTORY_COMPONENT_BYTES,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn nested_output_rejects_an_ancestor_owned_lawpack_tree() {
        let root = temp_tree("nested-owner");
        let owner = root.join("generated");
        test_ok(fs::create_dir(&owner), "create owner tree");
        test_ok(
            fs::write(owner.join("edict.lawpack-output.json"), valid_index()),
            "write ancestor ownership index",
        );
        let nested = owner.join("child");

        let failure = test_err(
            resolve_output_directory(&root, "generated/child"),
            "nested owner rejects",
        );
        assert_eq!(failure.kind, "LawpackOutputOwnershipFailed");
        assert!(!nested.exists());
        assert!(!output_lock_path(&nested).exists());
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn output_directory_rejects_internal_publication_namespaces() {
        let root = temp_tree("reserved-output-name");
        for relative in [
            ".foo.edict-lawpack-build.lock",
            ".foo.EDICT-LAWPACK-BUILD.LOCK",
            ".edict-lawpack-transaction-00000001-0000000000000000-00",
            ".EDICT-LAWPACK-TRANSACTION-00000001-0000000000000000-00",
            ".edict-lawpack-previous-00000001-0000000000000000-00",
        ] {
            let failure = test_err(
                resolve_output_directory(&root, relative),
                "internal publication path rejects as an output",
            );
            assert_eq!(failure.kind, "InvalidLawpackConfig", "{relative}");
        }
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn output_directory_bounds_derived_lock_names() {
        let root = temp_tree("bounded-output-name");
        let exact = "x".repeat(MAX_OUTPUT_DIRECTORY_COMPONENT_BYTES);
        assert_eq!(
            test_ok(
                resolve_output_directory(&root, &exact),
                "exact lock-name boundary succeeds",
            ),
            root.join(&exact)
        );
        let overlong = "x".repeat(MAX_OUTPUT_DIRECTORY_COMPONENT_BYTES + 1);
        let failure = test_err(
            resolve_output_directory(&root, &overlong),
            "overlong derived lock name rejects",
        );
        assert_eq!(failure.kind, "InvalidLawpackConfig");
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn output_directory_rejects_nonportable_raw_paths() {
        let root = temp_tree("portable-output-path");
        for relative in [
            "generated\\child",
            "generated//child",
            "generated/./child",
            "con/output",
            "generated/bad:name",
        ] {
            let failure = test_err(
                resolve_output_directory(&root, relative),
                "nonportable output path rejects",
            );
            assert_eq!(failure.kind, "InvalidLawpackConfig", "{relative}");
        }
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn nested_output_intent_conflicts_with_parent_publication() {
        let root = temp_tree("nested-race");
        let owner = root.join("generated");
        let nested = owner.join("child");
        assert_eq!(
            test_ok(
                resolve_output_directory(&root, "generated/child"),
                "resolve nested output",
            ),
            nested
        );

        let guards = test_ok(
            acquire_output_ancestor_locks(&root, &nested),
            "lock nested ancestors",
        );
        let expected = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);
        assert_eq!(
            test_err(
                publish_output(&owner, &expected),
                "ancestor publication while nested build is active",
            )
            .kind,
            "LawpackOutputWriteFailed"
        );

        drop(guards);
        test_ok(
            publish_output(&owner, &expected),
            "publish ancestor after nested build window",
        );
        assert_eq!(
            test_err(
                acquire_output_ancestor_locks(&root, &nested),
                "nested output rechecks ancestor ownership",
            )
            .kind,
            "LawpackOutputOwnershipFailed"
        );
        assert!(!nested.exists());
        assert!(!output_lock_path(&nested).exists());
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn sibling_output_intents_share_ancestors_without_serializing() {
        let root = temp_tree("sibling-intents");
        let first = root.join("generated/first");
        let second = root.join("generated/second");
        let first_guards = test_ok(
            acquire_output_ancestor_locks(&root, &first),
            "acquire first sibling intent",
        );
        let second_guards = test_ok(
            acquire_output_ancestor_locks(&root, &second),
            "acquire compatible second sibling intent",
        );
        assert_eq!(first_guards.len(), 1);
        assert_eq!(second_guards.len(), 1);
        drop(second_guards);
        drop(first_guards);
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn publication_keeps_the_locked_root_identity_after_real_directory_substitution() {
        let root = temp_tree("root-identity");
        let admitted_root = root.with_extension("admitted");
        let output = root.join("parent/generated");
        let authority = test_ok(
            acquire_output_ancestor_locks(&root, &output),
            "lock admitted output ancestors",
        );
        test_ok(fs::rename(&root, &admitted_root), "move admitted root");

        let substituted_output = root.join("parent/generated");
        test_ok(
            fs::create_dir_all(&substituted_output),
            "create substituted output",
        );
        let substituted = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("victim", b"substituted"),
        ]);
        for (relative, bytes) in &substituted {
            test_ok(
                fs::write(substituted_output.join(relative), bytes),
                "write substituted output",
            );
        }
        let expected = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"admitted"),
        ]);

        test_ok(
            publish_output_with_hooks_in_authority(
                &authority,
                &output,
                &expected,
                || Ok(()),
                || Ok(()),
                || Ok(()),
                cap_std::fs::Dir::remove_open_dir_all,
            ),
            "publish through admitted root identity",
        );

        assert_eq!(
            test_ok(
                read_output_tree(&substituted_output),
                "read substituted output",
            ),
            substituted
        );
        assert_eq!(
            test_ok(
                read_output_tree(&admitted_root.join("parent/generated")),
                "read admitted output",
            ),
            expected
        );
        drop(authority);
        test_ok(fs::remove_dir_all(root), "remove substituted root");
        test_ok(fs::remove_dir_all(admitted_root), "remove admitted root");
    }

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
    fn publication_authorizes_the_tree_it_captures_for_replacement() {
        let root = temp_tree("capture-owner");
        let output = root.join("generated");
        let displaced = root.join("displaced-owned");
        let foreign_source = root.join("foreign-source");
        let original = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("manifest.cbor", b"owned"),
        ]);
        test_ok(publish_output(&output, &original), "publish owned output");

        let foreign_index = valid_index_for("other", "9");
        let foreign = files(&[
            ("edict.lawpack-output.json", foreign_index.as_slice()),
            ("manifest.cbor", b"foreign"),
        ]);
        test_ok(fs::create_dir(&foreign_source), "create foreign output");
        for (relative, bytes) in &foreign {
            test_ok(
                fs::write(foreign_source.join(relative), bytes),
                "write foreign output",
            );
        }
        let replacement = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("manifest.cbor", b"replacement"),
        ]);

        let error = test_err(
            publish_output_with_capture_hook(&output, &replacement, || {
                fs::rename(&output, &displaced).map_err(|error| {
                    super::failure(
                        "InjectedFailure",
                        format!("failed to displace owned output: {error}"),
                    )
                })?;
                fs::rename(&foreign_source, &output).map_err(|error| {
                    super::failure(
                        "InjectedFailure",
                        format!("failed to substitute foreign output: {error}"),
                    )
                })?;
                Ok(())
            }),
            "substituted foreign output rejects",
        );

        assert_eq!(error.kind, "LawpackOutputOwnershipFailed");
        assert_eq!(
            test_ok(read_output_tree(&output), "read foreign tree"),
            foreign
        );
        assert_eq!(
            test_ok(read_output_tree(&displaced), "read displaced owned tree"),
            original
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn publication_bounds_internal_names_for_long_output_components() {
        let root = temp_tree("long-output-component");
        let output = root.join("x".repeat(MAX_OUTPUT_DIRECTORY_COMPONENT_BYTES));
        let expected = files(&[("edict.lawpack-output.json", valid_index())]);

        test_ok(
            publish_output(&output, &expected),
            "publish beside a long portable output component",
        );
        assert_eq!(test_ok(read_output_tree(&output), "read output"), expected);
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn injected_pre_activation_failure_restores_the_previous_tree() {
        let root = temp_tree("rollback");
        let output = root.join("a/b/generated");
        let original = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("old", b"old"),
        ]);
        test_ok(
            publish_output_with_hooks_in_root(
                &root,
                &output,
                &original,
                || Ok(()),
                || Ok(()),
                cap_std::fs::Dir::remove_open_dir_all,
            ),
            "publish original nested output",
        );
        assert!(root.join("a/b").is_dir());
        let replacement = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"new"),
        ]);

        let result = publish_output_with_hooks_in_root(
            &root,
            &output,
            &replacement,
            || Ok(()),
            || {
                Err(LawpackBuildFailure {
                    kind: "InjectedFailure",
                    message: "injected before activation".to_owned(),
                })
            },
            cap_std::fs::Dir::remove_open_dir_all,
        );

        assert_eq!(
            test_err(result, "injection rejects").kind,
            "InjectedFailure"
        );
        assert_eq!(
            test_ok(read_output_tree(&output), "read restored nested output"),
            original
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn rollback_refuses_to_delete_a_concurrently_installed_output() {
        let root = temp_tree("rollback-concurrent-output");
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
            fs::create_dir(&output).map_err(|error| {
                super::failure(
                    "InjectedFailure",
                    format!("failed to install concurrent output: {error}"),
                )
            })?;
            fs::write(output.join("victim"), b"keep").map_err(|error| {
                super::failure(
                    "InjectedFailure",
                    format!("failed to write concurrent output: {error}"),
                )
            })?;
            Err(super::failure(
                "InjectedFailure",
                "stop before activation".to_owned(),
            ))
        });

        assert_eq!(
            test_err(result, "concurrent output blocks rollback").kind,
            "LawpackOutputRollbackFailed"
        );
        assert_eq!(
            test_ok(fs::read(output.join("victim")), "read concurrent output"),
            b"keep"
        );
        let backup = test_ok(fs::read_dir(&root), "read publication root")
            .map(|entry| test_ok(entry, "read publication entry"))
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".edict-lawpack-previous-")
            })
            .map_or_else(
                || panic!("captured output backup must remain recoverable"),
                |entry| entry.path(),
            );
        assert_eq!(
            test_ok(read_output_tree(&backup), "read captured output backup"),
            original
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn publication_rejects_a_substituted_staged_transaction() {
        let root = temp_tree("staged-transaction-identity");
        let output = root.join("generated");
        let displaced_transaction = root.join("displaced-transaction");
        let original = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("old", b"old"),
        ]);
        test_ok(publish_output(&output, &original), "publish original");
        let replacement = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"new"),
        ]);

        let error = test_err(
            publish_output_with_hook(&output, &replacement, || {
                let transaction = test_ok(fs::read_dir(&root), "read publication root")
                    .map(|entry| test_ok(entry, "read publication entry"))
                    .find(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with(".edict-lawpack-transaction-")
                    })
                    .map_or_else(
                        || panic!("staged transaction must exist before activation"),
                        |entry| entry.path(),
                    );
                fs::rename(&transaction, &displaced_transaction).map_err(|error| {
                    super::failure(
                        "InjectedFailure",
                        format!("failed to displace staged transaction: {error}"),
                    )
                })?;
                fs::create_dir(&transaction).map_err(|error| {
                    super::failure(
                        "InjectedFailure",
                        format!("failed to install substitute transaction: {error}"),
                    )
                })?;
                fs::write(transaction.join("substitute"), b"unadmitted").map_err(|error| {
                    super::failure(
                        "InjectedFailure",
                        format!("failed to write substitute transaction: {error}"),
                    )
                })?;
                Ok(())
            }),
            "substituted staged transaction rejects",
        );

        assert_eq!(error.kind, "LawpackOutputWriteFailed");
        assert_eq!(
            test_ok(read_output_tree(&output), "read restored output"),
            original
        );
        assert!(!displaced_transaction.exists());
        let preserved_substitute = test_ok(fs::read_dir(&root), "read publication root")
            .map(|entry| test_ok(entry, "read publication entry"))
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".edict-lawpack-transaction-")
            })
            .map_or_else(
                || panic!("substituted transaction must remain recoverable"),
                |entry| entry.path(),
            );
        assert_eq!(
            test_ok(
                fs::read(preserved_substitute.join("substitute")),
                "read preserved substitute transaction",
            ),
            b"unadmitted"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn failed_activation_cleans_the_retained_transaction_not_its_reused_name() {
        let root = temp_tree("failed-activation-transaction-identity");
        let output = root.join("generated");
        let displaced_transaction = root.join("displaced-transaction");
        let original = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("old", b"old"),
        ]);
        test_ok(publish_output(&output, &original), "publish original");
        let replacement = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"new"),
        ]);

        let error = test_err(
            publish_output_with_hook(&output, &replacement, || {
                let transaction = test_ok(fs::read_dir(&root), "read publication root")
                    .map(|entry| test_ok(entry, "read publication entry"))
                    .find(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with(".edict-lawpack-transaction-")
                    })
                    .map_or_else(
                        || panic!("staged transaction must exist before activation"),
                        |entry| entry.path(),
                    );
                test_ok(
                    fs::rename(&transaction, &displaced_transaction),
                    "displace staged transaction",
                );
                test_ok(
                    fs::create_dir(&transaction),
                    "install substitute transaction",
                );
                test_ok(
                    fs::write(transaction.join("substitute"), b"unadmitted"),
                    "write substitute transaction",
                );
                test_ok(fs::create_dir(&output), "install concurrent output");
                test_ok(
                    fs::write(output.join("concurrent"), b"keep"),
                    "write concurrent output",
                );
                Ok(())
            }),
            "failed activation rejects",
        );

        assert_eq!(error.kind, "LawpackOutputRollbackFailed");
        assert!(!displaced_transaction.exists());
        let substitute = test_ok(fs::read_dir(&root), "read publication root")
            .map(|entry| test_ok(entry, "read publication entry"))
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".edict-lawpack-transaction-")
            })
            .map_or_else(
                || panic!("substituted transaction must remain recoverable"),
                |entry| entry.path(),
            );
        assert_eq!(
            test_ok(fs::read(substitute.join("substitute")), "read substitute"),
            b"unadmitted"
        );
        assert_eq!(
            test_ok(
                fs::read(output.join("concurrent")),
                "read concurrent output"
            ),
            b"keep"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn vanished_activation_restores_the_captured_output() {
        let root = temp_tree("vanished-activation");
        let output = root.join("generated");
        let displaced_activation = root.join("displaced-activation");
        let original = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("old", b"old"),
        ]);
        test_ok(publish_output(&output, &original), "publish original");
        let replacement = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"new"),
        ]);
        let authority = test_ok(
            acquire_output_ancestor_locks(&root, &output),
            "acquire publication authority",
        );

        let error = test_err(
            publish_output_with_hooks_in_authority(
                &authority,
                &output,
                &replacement,
                || Ok(()),
                || Ok(()),
                || {
                    fs::rename(&output, &displaced_activation).map_err(|error| {
                        super::failure(
                            "InjectedFailure",
                            format!("failed to displace activated output: {error}"),
                        )
                    })
                },
                cap_std::fs::Dir::remove_open_dir_all,
            ),
            "vanished activation rejects",
        );

        assert_eq!(error.kind, "LawpackOutputWriteFailed");
        assert_eq!(
            test_ok(read_output_tree(&output), "read restored output"),
            original
        );
        assert!(!displaced_activation.exists());
        drop(authority);
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
    fn check_only_rejects_output_directory_identity_substitution() {
        let root = temp_tree("check-directory-identity");
        let output = root.join("generated");
        let observed = root.join("observed");
        let substitute = root.join("substitute");
        let expected = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);
        test_ok(publish_output(&output, &expected), "publish expected set");
        test_ok(fs::create_dir(&substitute), "create substitute output");
        for (relative, bytes) in &expected {
            let bytes = if relative.as_os_str() == "one" {
                b"drift".as_slice()
            } else {
                bytes.as_slice()
            };
            test_ok(
                fs::write(substitute.join(relative), bytes),
                "write substitute output",
            );
        }

        let error = test_err(
            check_output_with_hook(&output, &expected, || {
                test_ok(fs::rename(&output, &observed), "move observed output");
                test_ok(
                    fs::rename(&substitute, &output),
                    "install substitute output",
                );
            }),
            "directory substitution rejects",
        );

        assert_eq!(error.kind, "LawpackOutputDrift");
        assert_eq!(
            test_ok(fs::read(output.join("one")), "read substitute artifact"),
            b"drift"
        );
        assert_eq!(
            test_ok(read_output_tree(&observed), "read observed output"),
            expected
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn check_only_rejects_in_place_tree_mutation_after_traversal() {
        let root = temp_tree("check-in-place-mutation");
        let output = root.join("generated");
        let expected = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);
        test_ok(publish_output(&output, &expected), "publish expected set");

        let error = test_err(
            check_output_with_hook(&output, &expected, || {
                test_ok(
                    fs::create_dir(output.join("unexpected")),
                    "create in-place unexpected directory",
                );
            }),
            "in-place post-traversal mutation rejects",
        );

        assert_eq!(error.kind, "LawpackOutputDrift");
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[cfg(unix)]
    #[test]
    fn check_only_rejects_a_post_validation_ancestor_substitution() {
        use std::os::unix::fs::symlink;

        let container = temp_tree("check-parent-identity-container");
        let root = container.join("root");
        let nested = root.join("nested");
        let displaced_nested = root.join("nested-displaced");
        let output = nested.join("generated");
        let outside = temp_tree("check-parent-identity-outside");
        let outside_output = outside.join("generated");
        let expected = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);
        test_ok(fs::create_dir_all(&output), "create admitted output");
        test_ok(
            fs::create_dir_all(&outside_output),
            "create external substitute output",
        );
        for (relative, bytes) in &expected {
            test_ok(
                fs::write(output.join(relative), bytes),
                "write admitted output",
            );
            test_ok(
                fs::write(outside_output.join(relative), bytes),
                "write external substitute output",
            );
        }

        test_ok(
            validate_check_output_parent_chain(&root, &output),
            "validate admitted parent chain",
        );
        let result = check_output_in_root_with_hooks(
            &root,
            &output,
            &expected,
            || {
                test_ok(
                    fs::rename(&nested, &displaced_nested),
                    "displace admitted parent",
                );
                test_ok(
                    symlink(&outside, &nested),
                    "install external parent symlink",
                );
            },
            || {},
        );

        test_ok(fs::remove_file(&nested), "remove external parent symlink");
        test_ok(
            fs::rename(&displaced_nested, &nested),
            "restore admitted parent",
        );
        assert!(result.is_err(), "post-validation ancestor swap must reject");
        test_ok(fs::remove_dir_all(container), "remove admitted tree");
        test_ok(fs::remove_dir_all(outside), "remove external tree");
    }

    #[test]
    fn check_only_accepts_an_exact_tree_without_creating_coordination_state() {
        let root = temp_tree("check-read-only");
        let output = root.join("generated");
        let expected = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);
        test_ok(fs::create_dir(&output), "create output");
        for (relative, bytes) in &expected {
            test_ok(
                fs::write(output.join(relative), bytes),
                "write expected file",
            );
        }
        let before = (
            test_ok(read_output_tree(&root), "snapshot files before check"),
            directory_set(&root),
        );

        test_ok(check_output(&output, &expected), "exact output passes");

        assert_eq!(
            (
                test_ok(read_output_tree(&root), "snapshot files after check"),
                directory_set(&root),
            ),
            before
        );
        assert!(!output_lock_path(&output).exists());
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[cfg(unix)]
    #[test]
    fn check_root_substituted_with_a_symlink_after_inspection_rejects() {
        use std::os::unix::fs::symlink;

        let container = temp_tree("check-root-inspection-race");
        let root = container.join("root");
        let displaced_root = container.join("displaced-root");
        let outside = temp_tree("check-root-inspection-race-outside");
        test_ok(fs::create_dir(&root), "create admitted root");

        let result = open_check_root_with_hook(&root, || {
            test_ok(fs::rename(&root, &displaced_root), "displace admitted root");
            test_ok(symlink(&outside, &root), "substitute root symlink");
        });

        test_ok(fs::remove_file(&root), "remove substituted root symlink");
        test_ok(fs::rename(&displaced_root, &root), "restore admitted root");
        assert!(result.is_err(), "post-inspection root symlink must reject");
        test_ok(fs::remove_dir_all(container), "remove admitted tree");
        test_ok(fs::remove_dir_all(outside), "remove outside tree");
    }

    #[cfg(unix)]
    #[test]
    fn check_only_parent_chain_rejects_a_symlink_substitution() {
        use std::os::unix::fs::symlink;

        let root = temp_tree("publish-symlink-swap");
        let outside = temp_tree("publish-symlink-target");
        let parent = root.join("parent");
        let output = parent.join("generated");
        test_ok(fs::create_dir(&parent), "create original parent");
        assert_eq!(
            test_ok(
                resolve_output_directory(&root, "parent/generated"),
                "resolve output before swap",
            ),
            output
        );
        test_ok(fs::remove_dir(&parent), "remove original parent");
        test_ok(symlink(&outside, &parent), "replace parent with symlink");

        let failure = test_err(
            validate_check_output_parent_chain(&root, &output),
            "symlink swap rejects during check-only validation",
        );
        assert_eq!(failure.kind, "LawpackOutputOwnershipFailed");
        assert!(test_ok(fs::read_dir(&outside), "read outside tree")
            .next()
            .is_none());

        test_ok(fs::remove_file(parent), "remove parent symlink");
        test_ok(fs::remove_dir_all(root), "remove test tree");
        test_ok(fs::remove_dir_all(outside), "remove outside tree");
    }

    #[test]
    fn check_only_parent_chain_uses_read_only_failure_kinds() {
        let root = temp_tree("check-parent-kinds");
        let missing_output = root.join("missing/generated");
        assert_eq!(
            test_err(
                validate_check_output_parent_chain(&root, &missing_output),
                "missing check-only ancestor drifts",
            )
            .kind,
            "LawpackOutputDrift"
        );

        let file_parent = root.join("file-parent");
        test_ok(
            fs::write(&file_parent, b"not a directory"),
            "write file parent",
        );
        assert_eq!(
            test_err(
                validate_check_output_parent_chain(&root, &file_parent.join("generated")),
                "non-directory check-only ancestor rejects ownership",
            )
            .kind,
            "LawpackOutputOwnershipFailed"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn check_only_parent_chain_rejects_non_normal_components() {
        let root = temp_tree("check-parent-components");
        test_ok(
            fs::create_dir(root.join("nested")),
            "create nested ancestor",
        );
        let output = root.join("nested/../generated");

        assert_eq!(
            test_err(
                validate_check_output_parent_chain(&root, &output),
                "non-normal parent component rejects",
            )
            .kind,
            "LawpackPathOutsideRoot"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[cfg(unix)]
    #[test]
    fn post_revalidation_parent_swap_cannot_escape_root() {
        use std::os::unix::fs::symlink;

        let container = temp_tree("publish-parent-swap-container");
        let root = container.join("root");
        let displaced = container.join("root-displaced");
        let outside = temp_tree("publish-parent-swap-outside");
        let output = root.join("generated");
        let outside_output = outside.join("generated");
        test_ok(fs::create_dir(&root), "create publication root");
        test_ok(
            fs::create_dir(&outside_output),
            "create external victim tree",
        );
        test_ok(
            fs::write(outside_output.join("victim"), b"untouched"),
            "write external victim",
        );
        let original = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("old", b"old"),
        ]);
        test_ok(publish_output(&output, &original), "publish original");
        let replacement = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("new", b"new"),
        ]);

        let failure = test_err(
            publish_output_with_hook(&output, &replacement, || {
                fs::rename(&root, &displaced).map_err(|error| {
                    super::failure(
                        "InjectedFailure",
                        format!("failed to displace publication root: {error}"),
                    )
                })?;
                symlink(&outside, &root).map_err(|error| {
                    super::failure(
                        "InjectedFailure",
                        format!("failed to replace publication root: {error}"),
                    )
                })?;
                Err(super::failure(
                    "InjectedFailure",
                    "stop before activation".to_owned(),
                ))
            }),
            "parent swap must refuse without escaping",
        );
        let outside_survived = fs::read(outside_output.join("victim")).ok();

        test_ok(fs::remove_file(&root), "remove replacement symlink");
        test_ok(fs::rename(&displaced, &root), "restore publication root");
        test_ok(fs::remove_dir_all(container), "remove publication tree");
        test_ok(fs::remove_dir_all(outside), "remove outside tree");

        assert_eq!(outside_survived.as_deref(), Some(b"untouched".as_slice()));
        assert_eq!(failure.kind, "InjectedFailure");
    }

    #[test]
    fn check_only_reports_missing_nested_output_as_drift_without_creating_parent() {
        let root = temp_tree("check-missing-parent");
        let output = root.join("missing-parent/generated");
        let expected = files(&[("edict.lawpack-output.json", valid_index()), ("one", b"1")]);

        assert_eq!(
            test_err(check_output(&output, &expected), "missing output drifts").kind,
            "LawpackOutputDrift"
        );
        assert!(!root.join("missing-parent").exists());
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn check_only_reports_drift_during_atomic_publication() {
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
        assert_eq!(error.kind, "LawpackOutputDrift");
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
    fn generated_artifacts_reject_their_own_read_limit() {
        test_ok(
            validate_generated_artifact_size("boundary.cbor", &vec![0; 1024 * 1024]),
            "artifact at read boundary passes",
        );
        assert_eq!(
            test_err(
                validate_generated_artifact_size("oversized.cbor", &vec![0; 1024 * 1024 + 1]),
                "oversized generated artifact rejects",
            )
            .kind,
            "LawpackOutputTooLarge"
        );
    }

    #[test]
    fn raw_authoring_json_rejects_duplicate_hash_significant_keys() {
        let document = br#"{
          "schema":"edict.lawpack-build/v1",
          "outputDirectory":"generated",
          "lawpack":{
            "schema":"edict.lawpack-authoring/v1",
            "id":"example.duplicate",
            "version":"1",
            "acceptedCoreAbi":["edict.core/v1"],
            "dependencies":[],
            "exportsCoordinate":"example.duplicate.exports/v1",
            "exports":{"types":[],"constants":[],"pureFunctions":[],"effects":[],"obstructions":[],"operationProfiles":{}},
            "targetAdapters":[],
            "verifier":{"class":"declarative","ruleset":{"id":"example.rules/v1","digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111"}},
            "compatibility":{"id":"example.compat/v1","digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222"},
            "conformanceFixtureCorpus":{"id":"example.fixtures/v1","digest":"sha256:3333333333333333333333333333333333333333333333333333333333333333"},
            "localResources":[{"name":"config","coordinate":"example.config/v1","output":"config.cbor","value":{"limit":1,"limit":2}}]
          },
          "dependencyBundles":[]
        }"#;

        assert_eq!(
            test_err(
                decode_lawpack_document(document),
                "duplicate review key rejects",
            )
            .kind,
            "InvalidLawpackConfig"
        );
    }

    #[test]
    fn build_preflights_nul_and_collisions_before_missing_dependency_io() {
        let root = temp_tree("nul-before-dependency");
        let document_path = root.join("edict.lawpack.json");
        let document = r#"{
          "schema":"edict.lawpack-build/v1",
          "outputDirectory":"generated",
          "lawpack":{
            "schema":"edict.lawpack-authoring/v1",
            "id":"example.nul",
            "version":"1",
            "acceptedCoreAbi":["edict.core/v1"],
            "dependencies":[],
            "exportsCoordinate":"example.nul.exports/v1",
            "exports":{"types":[],"constants":[],"pureFunctions":[],"effects":[],"obstructions":[],"operationProfiles":{}},
            "targetAdapters":[],
            "verifier":{"class":"declarative","ruleset":{"id":"example.rules/v1","digest":"sha256:1111111111111111111111111111111111111111111111111111111111111111"}},
            "compatibility":{"id":"example.compat/v1","digest":"sha256:2222222222222222222222222222222222222222222222222222222222222222"},
            "conformanceFixtureCorpus":{"id":"example.fixtures/v1","digest":"sha256:3333333333333333333333333333333333333333333333333333333333333333"},
            "localResources":[{"name":"config","coordinate":"example.config/v1","output":"bad\u0000.cbor","value":{}}]
          },
          "dependencyBundles":[{"manifest":"missing-manifest.cbor","exports":"missing-exports.cbor"}]
        }"#;
        test_ok(
            fs::write(&document_path, document.as_bytes()),
            "write build document",
        );

        assert_eq!(
            test_err(
                build_lawpack(&document_path, false),
                "NUL path rejects before missing dependency I/O",
            )
            .kind,
            "LawpackAuthoringInvalidOutputPath"
        );
        assert!(!root.join("generated").exists());

        let collision = document.replace("bad\\u0000.cbor", "manifest.cbor");
        test_ok(
            fs::write(&document_path, collision.as_bytes()),
            "write colliding build document",
        );
        assert_eq!(
            test_err(
                build_lawpack(&document_path, false),
                "fixed artifact collision rejects before missing dependency I/O",
            )
            .kind,
            "LawpackAuthoringDuplicateIdentity"
        );
        assert!(!root.join("generated").exists());
        test_ok(fs::remove_dir_all(root), "remove test tree");
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
        let canonical_root = test_ok(fs::canonicalize(&root), "canonicalize root");
        let canonical_output = test_ok(fs::canonicalize(&output), "canonicalize output");

        assert_eq!(
            test_err(
                load_dependencies(&canonical_root, &definitions, &canonical_output),
                "dependency inside output rejects",
            )
            .kind,
            "InvalidLawpackConfig"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[test]
    fn dependency_reads_keep_the_identity_that_passed_output_confinement() {
        let root = temp_tree("dependency-identity");
        let dependencies = root.join("dependencies");
        let admitted_dependencies = root.join("admitted-dependencies");
        let output = root.join("generated");
        let substituted_dependencies = output.join("substituted-dependencies");
        test_ok(
            fs::create_dir(&dependencies),
            "create admitted dependencies",
        );
        test_ok(
            fs::create_dir_all(&substituted_dependencies),
            "create substituted dependencies",
        );
        test_ok(
            fs::write(
                dependencies.join("manifest.cbor"),
                include_bytes!("../../../fixtures/lawpack/workspace-snapshot/manifest.cbor"),
            ),
            "write admitted manifest",
        );
        test_ok(
            fs::write(
                dependencies.join("exports.cbor"),
                include_bytes!("../../../fixtures/lawpack/workspace-snapshot/exports.cbor"),
            ),
            "write admitted exports",
        );
        test_ok(
            fs::write(
                substituted_dependencies.join("manifest.cbor"),
                include_bytes!("../../../fixtures/lawpack/hello-echo/manifest.cbor"),
            ),
            "write substituted manifest",
        );
        test_ok(
            fs::write(
                substituted_dependencies.join("exports.cbor"),
                include_bytes!("../../../fixtures/lawpack/hello-echo/exports.cbor"),
            ),
            "write substituted exports",
        );
        let definitions = [LawpackDependencyBundle {
            manifest: PathBuf::from("dependencies/manifest.cbor"),
            exports: PathBuf::from("dependencies/exports.cbor"),
        }];
        let canonical_root = test_ok(fs::canonicalize(&root), "canonicalize root");
        let canonical_output = test_ok(fs::canonicalize(&output), "canonicalize output");

        let loaded = test_ok(
            load_dependencies_with_hook(&canonical_root, &definitions, &canonical_output, || {
                test_ok(
                    fs::rename(&dependencies, &admitted_dependencies),
                    "move admitted dependencies",
                );
                test_ok(
                    fs::rename(&substituted_dependencies, &dependencies),
                    "substitute dependencies from output",
                );
            }),
            "load admitted dependency identity",
        );

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].manifest().id, "workspace.snapshot");
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[cfg(unix)]
    #[test]
    fn dependency_file_substituted_with_a_symlink_after_inspection_rejects() {
        use std::os::unix::fs::symlink;

        let root = temp_tree("dependency-file-inspection-race");
        let dependencies = root.join("dependencies");
        let manifest = dependencies.join("manifest.cbor");
        let displaced_manifest = dependencies.join("displaced-manifest.cbor");
        let alternate_manifest = dependencies.join("alternate-manifest.cbor");
        test_ok(fs::create_dir(&dependencies), "create dependency directory");
        test_ok(fs::write(&manifest, b"admitted"), "write admitted manifest");
        test_ok(
            fs::write(&alternate_manifest, b"substitute"),
            "write alternate manifest",
        );
        let root_dir = test_ok(
            cap_std::fs::Dir::open_ambient_dir(&root, cap_std::ambient_authority()),
            "open dependency root",
        );

        let result = open_dependency_input_with_hook(
            &root_dir,
            &root,
            std::path::Path::new("dependencies/manifest.cbor"),
            std::path::Path::new("generated"),
            "dependencyBundles.0.manifest",
            || {
                test_ok(
                    fs::rename(&manifest, &displaced_manifest),
                    "displace admitted manifest",
                );
                test_ok(
                    symlink("alternate-manifest.cbor", &manifest),
                    "substitute manifest symlink",
                );
            },
        );

        assert!(
            result.is_err(),
            "post-inspection dependency link must reject"
        );
        assert_eq!(
            test_ok(fs::read(&displaced_manifest), "read displaced manifest"),
            b"admitted"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_dependency_inputs_resolving_inside_output_reject() {
        use std::os::unix::fs::symlink;

        let root = temp_tree("dependency-symlink-output");
        let output = root.join("generated");
        let links = root.join("dependencies");
        test_ok(fs::create_dir_all(&output), "create output");
        test_ok(fs::create_dir_all(&links), "create dependency links");
        test_ok(
            fs::write(output.join("manifest.cbor"), b"not cbor"),
            "write manifest",
        );
        test_ok(
            fs::write(output.join("exports.cbor"), b"not cbor"),
            "write exports",
        );
        test_ok(
            symlink(output.join("manifest.cbor"), links.join("manifest.cbor")),
            "link manifest into output",
        );
        test_ok(
            symlink(output.join("exports.cbor"), links.join("exports.cbor")),
            "link exports into output",
        );
        let definitions = [LawpackDependencyBundle {
            manifest: PathBuf::from("dependencies/manifest.cbor"),
            exports: PathBuf::from("dependencies/exports.cbor"),
        }];
        let canonical_root = test_ok(fs::canonicalize(&root), "canonicalize root");
        let canonical_output = test_ok(fs::canonicalize(&output), "canonicalize output");

        assert_eq!(
            test_err(
                load_dependencies(&canonical_root, &definitions, &canonical_output),
                "resolved dependency inside output rejects",
            )
            .kind,
            "InvalidLawpackConfig"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_ownership_index_cannot_authorize_replacement() {
        use std::os::unix::fs::symlink;

        let root = temp_tree("symlink-index");
        let output = root.join("generated");
        let external_index = root.join("external-index.json");
        test_ok(fs::create_dir_all(&output), "create output");
        test_ok(
            fs::write(&external_index, valid_index()),
            "write external index",
        );
        test_ok(
            symlink(&external_index, output.join("edict.lawpack-output.json")),
            "link ownership index",
        );
        test_ok(
            fs::write(output.join("unrelated"), b"keep"),
            "write unrelated file",
        );
        let expected = files(&[
            ("edict.lawpack-output.json", valid_index()),
            ("manifest.cbor", b"expected"),
        ]);

        assert_eq!(
            test_err(
                publish_output(&output, &expected),
                "symlinked ownership index rejects",
            )
            .kind,
            "LawpackOutputOwnershipFailed"
        );
        assert_eq!(
            test_ok(fs::read(output.join("unrelated")), "read unrelated file"),
            b"keep"
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    #[cfg(unix)]
    #[test]
    fn ownership_index_substituted_with_a_symlink_after_inspection_rejects() {
        use std::os::unix::fs::symlink;

        let root = temp_tree("ownership-index-inspection-race");
        let output = root.join("generated");
        let index = output.join("edict.lawpack-output.json");
        let displaced_index = root.join("displaced-index.json");
        let external_index = output.join("alternate-index.json");
        test_ok(fs::create_dir(&output), "create output");
        test_ok(fs::write(&index, valid_index()), "write admitted index");
        test_ok(
            fs::write(&external_index, valid_index()),
            "write external index",
        );
        let output_dir = test_ok(
            cap_std::fs::Dir::open_ambient_dir(&output, cap_std::ambient_authority()),
            "open output directory",
        );
        let expected_owner = ("test".to_owned(), "1".to_owned());

        let failure = test_err(
            validate_owned_output_dir_with_hook(&output_dir, &output, &expected_owner, || {
                test_ok(fs::rename(&index, &displaced_index), "displace index");
                test_ok(
                    symlink("alternate-index.json", &index),
                    "substitute index symlink",
                );
                assert_eq!(
                    test_ok(fs::read(&index), "read substituted index by pathname"),
                    valid_index()
                );
            }),
            "post-inspection ownership symlink rejects",
        );

        assert_eq!(failure.kind, "LawpackOutputOwnershipFailed");
        assert_eq!(
            test_ok(fs::read(&displaced_index), "read displaced index"),
            valid_index()
        );
        test_ok(fs::remove_dir_all(root), "remove test tree");
    }

    fn files(entries: &[(&str, &[u8])]) -> BTreeMap<PathBuf, Vec<u8>> {
        entries
            .iter()
            .map(|(path, bytes)| (PathBuf::from(path), bytes.to_vec()))
            .collect()
    }

    fn directory_set(root: &std::path::Path) -> BTreeSet<PathBuf> {
        let mut directories = BTreeSet::new();
        let mut pending = vec![root.to_path_buf()];
        while let Some(directory) = pending.pop() {
            for entry in test_ok(fs::read_dir(&directory), "read snapshot directory") {
                let entry = test_ok(entry, "read snapshot entry");
                let file_type = test_ok(entry.file_type(), "read snapshot entry type");
                if file_type.is_dir() {
                    let path = entry.path();
                    let relative =
                        test_ok(path.strip_prefix(root), "relativize snapshot directory");
                    directories.insert(relative.to_path_buf());
                    pending.push(path);
                }
            }
        }
        directories
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
