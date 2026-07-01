use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use edict_cli::{
    CHECK_RESULT_SCHEMA, COMPILER_INPUT_SCHEMA as INPUT_SCHEMA, DEFAULT_MAX_STDIN_BYTES,
    DIAGNOSTIC_SCHEMA, EVENT_SCHEMA, INFO_SCHEMA, MAX_STDIN_BYTES_ENV,
};
use edict_syntax::{CheckOutcome, ParseError, SemanticError, Span};
use serde::Deserialize;
use serde_json::{json, Value};

const COMMAND_CHECK: &str = "check";
const EXIT_OK: i32 = 0;
const EXIT_CHECK_FAILED: i32 = 1;
const EXIT_CLI_FAILED: i32 = 2;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompilerSettings {
    schema: String,
    #[serde(rename = "type")]
    record_type: String,
    operation: Operation,
    #[serde(default = "default_directory_extensions")]
    directory_extensions: Vec<String>,
    #[serde(default)]
    follow_symlinks: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Operation {
    Check,
}

#[derive(Debug, Clone)]
struct Request {
    settings: CompilerSettings,
    inputs: Vec<CompilerInput>,
}

#[derive(Debug, Clone)]
enum CompilerInput {
    Source { name: String, source: String },
    Path { path: PathBuf },
    PathList { paths: Vec<PathBuf> },
    Directory { path: PathBuf },
    Glob { pattern: String },
}

#[derive(Debug, Clone)]
struct SourceDocument {
    input: Value,
    source: String,
}

#[derive(Debug, Clone)]
struct CliFailure {
    kind: &'static str,
    line: Option<usize>,
    message: String,
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut args = std::env::args_os().skip(1);
    if let Some(first) = args.next() {
        let only_arg = args.next().is_none();
        match first.to_str() {
            Some("--help" | "-h") if only_arg => {
                write_info(&help_record());
                return EXIT_OK;
            }
            Some("--version" | "-V") if only_arg => {
                write_info(&version_record());
                return EXIT_OK;
            }
            _ => {
                let failure = CliFailure {
                    kind: "InvalidArguments",
                    line: None,
                    message: "edict reads JSONL request records on stdin and takes no positional \
                              arguments; run `edict --help` for the request schema, or see \
                              docs/topics/cli/README.md"
                        .to_owned(),
                };
                write_cli_failure(&failure);
                return EXIT_CLI_FAILED;
            }
        }
    }

    let input = match read_stdin_bounded() {
        Ok(input) => input,
        Err(failure) => {
            write_cli_failure(&failure);
            return EXIT_CLI_FAILED;
        }
    };

    match parse_request(&input) {
        Ok(request) => match run_request(&request) {
            Ok(exit_code) => exit_code,
            Err(failure) => {
                write_cli_failure(&failure);
                EXIT_CLI_FAILED
            }
        },
        Err(failure) => {
            write_cli_failure(&failure);
            EXIT_CLI_FAILED
        }
    }
}

fn read_stdin_bounded() -> Result<String, CliFailure> {
    let limit = configured_stdin_limit()?;
    let max_read = limit.checked_add(1).ok_or_else(|| CliFailure {
        kind: "InvalidStdinLimit",
        line: None,
        message: format!("{MAX_STDIN_BYTES_ENV} must be below usize::MAX"),
    })?;
    let max_read = u64::try_from(max_read).map_err(|_| CliFailure {
        kind: "InvalidStdinLimit",
        line: None,
        message: format!("{MAX_STDIN_BYTES_ENV} exceeds the supported byte limit"),
    })?;
    let mut bytes = Vec::new();
    io::stdin()
        .take(max_read)
        .read_to_end(&mut bytes)
        .map_err(|err| CliFailure {
            kind: "StdinRead",
            line: None,
            message: err.to_string(),
        })?;
    if bytes.len() > limit {
        return Err(CliFailure {
            kind: "InputTooLarge",
            line: None,
            message: format!("stdin exceeds the configured maximum of {limit} bytes"),
        });
    }
    String::from_utf8(bytes).map_err(|err| CliFailure {
        kind: "StdinRead",
        line: None,
        message: err.to_string(),
    })
}

fn configured_stdin_limit() -> Result<usize, CliFailure> {
    match std::env::var(MAX_STDIN_BYTES_ENV) {
        Ok(raw) => {
            let limit = raw.parse::<usize>().map_err(|_| CliFailure {
                kind: "InvalidStdinLimit",
                line: None,
                message: format!("{MAX_STDIN_BYTES_ENV} must be a positive byte count"),
            })?;
            if limit == 0 {
                return Err(CliFailure {
                    kind: "InvalidStdinLimit",
                    line: None,
                    message: format!("{MAX_STDIN_BYTES_ENV} must be a positive byte count"),
                });
            }
            Ok(limit)
        }
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_MAX_STDIN_BYTES),
        Err(std::env::VarError::NotUnicode(_)) => Err(CliFailure {
            kind: "InvalidStdinLimit",
            line: None,
            message: format!("{MAX_STDIN_BYTES_ENV} must be valid UTF-8"),
        }),
    }
}

fn parse_request(input: &str) -> Result<Request, CliFailure> {
    if input.trim().is_empty() {
        return Err(CliFailure {
            kind: "EmptyInput",
            line: None,
            message: "stdin must contain at least one JSONL record".to_owned(),
        });
    }

    let mut settings = None;
    let mut inputs = Vec::new();
    for (index, line) in input.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() {
            return Err(CliFailure {
                kind: "BlankLine",
                line: Some(line_number),
                message: "JSONL input must not contain blank lines".to_owned(),
            });
        }
        let value = serde_json::from_str::<Value>(line).map_err(|err| CliFailure {
            kind: "InvalidJsonl",
            line: Some(line_number),
            message: err.to_string(),
        })?;
        let object = value.as_object().ok_or_else(|| CliFailure {
            kind: "InvalidRecord",
            line: Some(line_number),
            message: "each JSONL record must be a JSON object".to_owned(),
        })?;
        let schema = object
            .get("schema")
            .and_then(Value::as_str)
            .ok_or_else(|| CliFailure {
                kind: "InvalidRecord",
                line: Some(line_number),
                message: "JSONL record missing string field `schema`".to_owned(),
            })?;

        match schema {
            edict_cli::COMPILER_SETTINGS_SCHEMA => {
                if settings.is_some() {
                    return Err(CliFailure {
                        kind: "DuplicateSettings",
                        line: Some(line_number),
                        message: "request may contain only one compiler settings record".to_owned(),
                    });
                }
                settings = Some(parse_settings(value, line_number)?);
            }
            INPUT_SCHEMA => inputs.push(parse_compiler_input(object, line_number)?),
            _ => {
                return Err(CliFailure {
                    kind: "InvalidRecord",
                    line: Some(line_number),
                    message: format!("unsupported JSONL schema `{schema}`"),
                });
            }
        }
    }

    let settings = settings.ok_or_else(|| CliFailure {
        kind: "MissingSettings",
        line: None,
        message: "request missing compiler settings record".to_owned(),
    })?;
    if inputs.is_empty() {
        return Err(CliFailure {
            kind: "MissingInput",
            line: None,
            message: "request must contain at least one compiler input record".to_owned(),
        });
    }

    Ok(Request { settings, inputs })
}

fn parse_settings(value: Value, line: usize) -> Result<CompilerSettings, CliFailure> {
    let settings = serde_json::from_value::<CompilerSettings>(value).map_err(|err| CliFailure {
        kind: "InvalidSettings",
        line: Some(line),
        message: err.to_string(),
    })?;
    if settings.schema != edict_cli::COMPILER_SETTINGS_SCHEMA {
        return Err(CliFailure {
            kind: "InvalidSettings",
            line: Some(line),
            message: "compiler settings schema field does not match the settings schema".to_owned(),
        });
    }
    if settings.record_type != "compilerSettings" {
        return Err(CliFailure {
            kind: "InvalidSettings",
            line: Some(line),
            message: "compiler settings record type must be `compilerSettings`".to_owned(),
        });
    }
    if settings.operation != Operation::Check {
        return Err(CliFailure {
            kind: "UnsupportedOperation",
            line: Some(line),
            message: "only the `check` operation is supported".to_owned(),
        });
    }
    if settings.directory_extensions.iter().any(|ext| {
        ext.len() < 2
            || !ext.starts_with('.')
            || !ext
                .chars()
                .skip(1)
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    }) {
        return Err(CliFailure {
            kind: "InvalidSettings",
            line: Some(line),
            message: "directoryExtensions entries must be dotted ASCII extensions".to_owned(),
        });
    }
    Ok(settings)
}

fn parse_compiler_input(
    object: &serde_json::Map<String, Value>,
    line: usize,
) -> Result<CompilerInput, CliFailure> {
    require_string_field(object, "type", line).and_then(|record_type| {
        if record_type == "compilerInput" {
            Ok(())
        } else {
            Err(CliFailure {
                kind: "InvalidInputRecord",
                line: Some(line),
                message: "compiler input record type must be `compilerInput`".to_owned(),
            })
        }
    })?;

    let kind = require_string_field(object, "kind", line)?;
    let allowed: &[&str] = match kind {
        "source" => &["name", "source"],
        "path" | "directory" => &["path"],
        "pathList" => &["paths"],
        "glob" => &["pattern"],
        _ => {
            return Err(CliFailure {
                kind: "InvalidInputRecord",
                line: Some(line),
                message: format!("unsupported compiler input kind `{kind}`"),
            });
        }
    };
    // Match the published `edict.compiler.input/v1` schema, which pins
    // `additionalProperties: false` and mutually exclusive input kinds: reject
    // any field outside the envelope and this kind's own variant fields so the
    // binary accepts exactly what the schema accepts.
    reject_foreign_input_fields(object, kind, allowed, line)?;
    match kind {
        "source" => Ok(CompilerInput::Source {
            name: optional_string_field(object, "name")
                .unwrap_or_else(|| "inline.edict".to_owned()),
            source: require_string_field(object, "source", line)?.to_owned(),
        }),
        "path" => Ok(CompilerInput::Path {
            path: PathBuf::from(require_string_field(object, "path", line)?),
        }),
        "pathList" => {
            let paths = object
                .get("paths")
                .and_then(Value::as_array)
                .ok_or_else(|| CliFailure {
                    kind: "InvalidInputRecord",
                    line: Some(line),
                    message: "pathList input records require an array field `paths`".to_owned(),
                })?
                .iter()
                .map(|value| {
                    value.as_str().map(PathBuf::from).ok_or_else(|| CliFailure {
                        kind: "InvalidInputRecord",
                        line: Some(line),
                        message: "pathList `paths` entries must be strings".to_owned(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(CompilerInput::PathList { paths })
        }
        "directory" => Ok(CompilerInput::Directory {
            path: PathBuf::from(require_string_field(object, "path", line)?),
        }),
        "glob" => Ok(CompilerInput::Glob {
            pattern: require_string_field(object, "pattern", line)?.to_owned(),
        }),
        _ => Err(CliFailure {
            kind: "InvalidInputRecord",
            line: Some(line),
            message: format!("unsupported compiler input kind `{kind}`"),
        }),
    }
}

fn reject_foreign_input_fields(
    object: &serde_json::Map<String, Value>,
    kind: &str,
    allowed: &[&str],
    line: usize,
) -> Result<(), CliFailure> {
    for key in object.keys() {
        let key = key.as_str();
        if matches!(key, "schema" | "type" | "kind") || allowed.contains(&key) {
            continue;
        }
        return Err(CliFailure {
            kind: "InvalidInputRecord",
            line: Some(line),
            message: format!("`{kind}` compiler input record has unexpected field `{key}`"),
        });
    }
    Ok(())
}

fn require_string_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    line: usize,
) -> Result<&'a str, CliFailure> {
    object
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| CliFailure {
            kind: "InvalidInputRecord",
            line: Some(line),
            message: format!("record missing string field `{key}`"),
        })
}

fn optional_string_field(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_owned)
}

fn run_request(request: &Request) -> Result<i32, CliFailure> {
    let sources = expand_inputs(&request.settings, &request.inputs)?;
    if sources.is_empty() {
        return Err(CliFailure {
            kind: "MissingInput",
            line: None,
            message: "request did not expand to any source inputs".to_owned(),
        });
    }
    let report = check_sources(&sources);
    if report.diagnostics.is_empty() {
        let mut stdout = io::stdout().lock();
        write_records(&mut stdout, &report.results);
        let status = status_record("ok", sources.len(), 0, EXIT_OK);
        write_record(&mut stdout, &status);
        Ok(EXIT_OK)
    } else {
        let mut stderr = io::stderr().lock();
        write_records(&mut stderr, &report.diagnostics);
        let status = status_record(
            "error",
            report.results.len(),
            report.diagnostics.len(),
            EXIT_CHECK_FAILED,
        );
        write_record(&mut stderr, &status);
        Ok(EXIT_CHECK_FAILED)
    }
}

#[derive(Debug)]
struct CheckReport {
    results: Vec<Value>,
    diagnostics: Vec<Value>,
}

fn check_sources(sources: &[SourceDocument]) -> CheckReport {
    let mut results = Vec::new();
    let mut diagnostics = Vec::new();
    for document in sources {
        match edict_syntax::check(&document.source) {
            CheckOutcome::Valid => results.push(check_result_record(&document.input)),
            CheckOutcome::ParseFailed(error) => {
                diagnostics.push(parse_diagnostic(&document.input, &error));
            }
            CheckOutcome::SemanticFailed(errors) => diagnostics.extend(
                errors
                    .iter()
                    .map(|error| semantic_diagnostic(&document.input, error)),
            ),
        }
    }
    CheckReport {
        results,
        diagnostics,
    }
}

fn expand_inputs(
    settings: &CompilerSettings,
    inputs: &[CompilerInput],
) -> Result<Vec<SourceDocument>, CliFailure> {
    let mut sources = Vec::new();
    for input in inputs {
        match input {
            CompilerInput::Source { name, source } => sources.push(SourceDocument {
                input: json!({
                    "kind": "source",
                    "name": name,
                }),
                source: source.clone(),
            }),
            CompilerInput::Path { path } => expand_path(settings, path, "path", &mut sources)?,
            CompilerInput::PathList { paths } => {
                for path in paths {
                    expand_path(settings, path, "pathList", &mut sources)?;
                }
            }
            CompilerInput::Directory { path } => {
                expand_directory(settings, path, "directory", &mut sources)?;
            }
            CompilerInput::Glob { pattern } => expand_glob(pattern, &mut sources)?,
        }
    }
    Ok(sources)
}

fn expand_path(
    settings: &CompilerSettings,
    path: &Path,
    origin: &str,
    sources: &mut Vec<SourceDocument>,
) -> Result<(), CliFailure> {
    let metadata = fs::metadata(path).map_err(|err| path_failure("PathRead", path, &err))?;
    if metadata.is_dir() {
        expand_directory(settings, path, origin, sources)
    } else {
        read_source_file(path, origin).map(|source| sources.push(source))
    }
}

fn expand_directory(
    settings: &CompilerSettings,
    path: &Path,
    origin: &str,
    sources: &mut Vec<SourceDocument>,
) -> Result<(), CliFailure> {
    let mut files = Vec::new();
    collect_directory_files(settings, path, &mut files)?;
    files.sort();
    for file in files {
        sources.push(read_source_file(&file, origin)?);
    }
    Ok(())
}

fn collect_directory_files(
    settings: &CompilerSettings,
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), CliFailure> {
    let mut entries = fs::read_dir(path)
        .map_err(|err| path_failure("DirectoryRead", path, &err))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| CliFailure {
            kind: "DirectoryRead",
            line: None,
            message: err.to_string(),
        })?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        let metadata = if settings.follow_symlinks {
            fs::metadata(&path)
        } else {
            fs::symlink_metadata(&path)
        }
        .map_err(|err| path_failure("DirectoryRead", &path, &err))?;
        if metadata.file_type().is_symlink() && !settings.follow_symlinks {
            continue;
        }
        if metadata.is_dir() {
            collect_directory_files(settings, &path, files)?;
        } else if metadata.is_file() && directory_extension_matches(settings, &path) {
            files.push(path);
        }
    }
    Ok(())
}

fn expand_glob(pattern: &str, sources: &mut Vec<SourceDocument>) -> Result<(), CliFailure> {
    let mut paths = glob::glob(pattern)
        .map_err(|err| CliFailure {
            kind: "InvalidGlob",
            line: None,
            message: err.to_string(),
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| CliFailure {
            kind: "GlobRead",
            line: None,
            message: err.to_string(),
        })?;
    paths.retain(|path| path.is_file());
    paths.sort();
    for path in paths {
        sources.push(read_source_file(&path, "glob")?);
    }
    Ok(())
}

fn read_source_file(path: &Path, origin: &str) -> Result<SourceDocument, CliFailure> {
    let source = fs::read_to_string(path).map_err(|err| path_failure("PathRead", path, &err))?;
    Ok(SourceDocument {
        input: json!({
            "kind": origin,
            "path": path.display().to_string(),
        }),
        source,
    })
}

fn directory_extension_matches(settings: &CompilerSettings, path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| format!(".{extension}"))
        .is_some_and(|extension| settings.directory_extensions.contains(&extension))
}

fn path_failure(kind: &'static str, path: &Path, err: &io::Error) -> CliFailure {
    CliFailure {
        kind,
        line: None,
        message: format!("{}: {err}", path.display()),
    }
}

fn check_result_record(input: &Value) -> Value {
    json!({
        "schema": CHECK_RESULT_SCHEMA,
        "type": "checkResult",
        "command": COMMAND_CHECK,
        "input": input,
        "status": "ok",
    })
}

fn parse_diagnostic(input: &Value, error: &ParseError) -> Value {
    diagnostic_record("parse", error.kind.code(), input, Some(error.span), None)
}

fn semantic_diagnostic(input: &Value, error: &SemanticError) -> Value {
    diagnostic_record("semantic", error.kind.code(), input, Some(error.span), None)
}

fn cli_diagnostic(failure: &CliFailure) -> Value {
    let mut record = diagnostic_record(
        "cli",
        failure.kind,
        &json!({ "kind": "stdin" }),
        None,
        failure.line,
    );
    record["message"] = json!(failure.message);
    record
}

fn diagnostic_record(
    stage: &str,
    kind: &str,
    input: &Value,
    span: Option<Span>,
    line: Option<usize>,
) -> Value {
    let mut record = json!({
        "schema": DIAGNOSTIC_SCHEMA,
        "type": "diagnostic",
        "command": COMMAND_CHECK,
        "severity": "error",
        "stage": stage,
        "kind": kind,
        "input": input,
    });
    if let Some(span) = span {
        record["span"] = json!({
            "start": span.start,
            "end": span.end,
        });
    }
    if let Some(line) = line {
        record["line"] = json!(line);
    }
    record
}

fn status_record(status: &str, checked: usize, errors: usize, exit_code: i32) -> Value {
    json!({
        "schema": EVENT_SCHEMA,
        "type": "status",
        "command": COMMAND_CHECK,
        "status": status,
        "checked": checked,
        "errors": errors,
        "exitCode": exit_code,
    })
}

fn version_record() -> Value {
    json!({
        "schema": INFO_SCHEMA,
        "type": "info",
        "topic": "version",
        "version": env!("CARGO_PKG_VERSION"),
    })
}

fn help_record() -> Value {
    json!({
        "schema": INFO_SCHEMA,
        "type": "info",
        "topic": "help",
        "version": env!("CARGO_PKG_VERSION"),
        "usage": "edict reads JSONL request records on stdin and emits only JSONL records on \
                  stdout and stderr; it takes no positional arguments. A request is one compiler \
                  settings record followed by one or more compiler input records.",
        "requestSchemas": [
            edict_cli::COMPILER_SETTINGS_SCHEMA,
            INPUT_SCHEMA,
        ],
        "exitCodes": [
            { "code": EXIT_OK, "meaning": "request completed successfully" },
            { "code": EXIT_CHECK_FAILED, "meaning": "compiler or validation diagnostics were produced" },
            { "code": EXIT_CLI_FAILED, "meaning": "CLI input or usage was invalid" },
        ],
        "docs": "docs/topics/cli/README.md",
    })
}

fn write_info(record: &Value) {
    let mut stdout = io::stdout().lock();
    write_record(&mut stdout, record);
}

fn write_cli_failure(failure: &CliFailure) {
    let mut stderr = io::stderr().lock();
    let diagnostic = cli_diagnostic(failure);
    write_record(&mut stderr, &diagnostic);
    let status = status_record("error", 0, 1, EXIT_CLI_FAILED);
    write_record(&mut stderr, &status);
}

fn write_records(writer: &mut dyn Write, records: &[Value]) {
    for record in records {
        write_record(writer, record);
    }
}

fn write_record(writer: &mut dyn Write, record: &Value) {
    if serde_json::to_writer(&mut *writer, record).is_ok() {
        let _ = writer.write_all(b"\n");
    }
}

fn default_directory_extensions() -> Vec<String> {
    vec![".edict".to_owned()]
}
