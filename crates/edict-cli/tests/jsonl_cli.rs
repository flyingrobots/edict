use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{json, Value};

const VALID_SOURCE: &str = r#"package examples.hello@1;

use lawpack hello.optics@1 digest "sha256:0000000000000000000000000000000000000000000000000000000000000000" as hello;

type HelloInput = {
  name: String<max=256>,
};

intent sayHello(input: HelloInput)
  returns HelloInput
  profile hello.readOnly
  basis none
  budget <= hello.tinyBudget
{
  return { name: input.name };
}
"#;

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn check_accepts_inline_source_jsonl_and_emits_jsonl_stdout() {
    let output = run_edict(&jsonl([
        compiler_settings(),
        json!({
            "schema": "edict.compiler.input/v1",
            "type": "compilerInput",
            "kind": "source",
            "name": "inline.edict",
            "source": VALID_SOURCE,
        }),
    ]));

    assert!(
        output.status.success(),
        "check should accept valid inline source"
    );
    assert_jsonl_stream(&output.stderr, "stderr");
    assert!(
        output.stderr.is_empty(),
        "successful check must not write stderr"
    );
    let stdout = assert_jsonl_stream(&output.stdout, "stdout");
    assert_eq!(check_result_count(&stdout), 1);
    assert_status(&stdout, "ok", 0);
}

#[test]
fn check_rejects_invalid_source_with_jsonl_stderr_only() {
    let output = run_edict(&jsonl([
        compiler_settings(),
        json!({
            "schema": "edict.compiler.input/v1",
            "type": "compilerInput",
            "kind": "source",
            "name": "broken.edict",
            "source": "package examples.broken@1\n",
        }),
    ]));

    assert_eq!(output.status.code(), Some(1));
    assert!(
        output.stdout.is_empty(),
        "failed compiler checks must not write stdout"
    );
    let stderr = assert_jsonl_stream(&output.stderr, "stderr");
    assert!(stderr.iter().any(|line| {
        line.get("type").and_then(Value::as_str) == Some("diagnostic")
            && line.get("stage").and_then(Value::as_str) == Some("parse")
            && line.get("kind").and_then(Value::as_str) == Some("ExpectedToken")
    }));
    assert_status(&stderr, "error", 1);
}

#[test]
fn non_jsonl_input_rejects_with_jsonl_cli_diagnostic() {
    let output = run_edict("not json\n");

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "CLI input failures must not write stdout"
    );
    let stderr = assert_jsonl_stream(&output.stderr, "stderr");
    assert!(stderr.iter().any(|line| {
        line.get("type").and_then(Value::as_str) == Some("diagnostic")
            && line.get("stage").and_then(Value::as_str) == Some("cli")
            && line.get("kind").and_then(Value::as_str) == Some("InvalidJsonl")
    }));
    assert_status(&stderr, "error", 2);
}

#[test]
fn oversized_stdin_rejects_with_input_too_large_diagnostic() {
    let output = run_edict_with_env(&"x".repeat(65), &[(edict_cli::MAX_STDIN_BYTES_ENV, "64")]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "oversized stdin must not write stdout"
    );
    let stderr = assert_jsonl_stream(&output.stderr, "stderr");
    let diagnostic = stderr
        .iter()
        .find(|line| line.get("kind").and_then(Value::as_str) == Some("InputTooLarge"))
        .expect("stderr must contain an InputTooLarge diagnostic");
    assert_eq!(diagnostic.get("stage").and_then(Value::as_str), Some("cli"));
    assert_eq!(
        diagnostic
            .get("input")
            .and_then(|input| input.get("kind"))
            .and_then(Value::as_str),
        Some("stdin")
    );
    let message = diagnostic
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("64 bytes"),
        "InputTooLarge should report the configured byte limit"
    );
    assert_status(&stderr, "error", 2);
}

#[test]
fn check_accepts_path_directory_path_list_glob_and_source_records() {
    let root = temp_tree("inputs");
    let nested = root.join("nested");
    fs::create_dir_all(&nested).expect("create nested fixture dir");
    let first = root.join("first.edict");
    let second = nested.join("second.edict");
    fs::write(&first, VALID_SOURCE).expect("write first source");
    fs::write(&second, VALID_SOURCE).expect("write second source");
    fs::write(root.join("ignored.txt"), VALID_SOURCE).expect("write ignored source");

    let output = run_edict(&jsonl([
        json!({
            "schema": "edict.compiler.settings/v1",
            "type": "compilerSettings",
            "operation": "check",
            "directoryExtensions": [".edict"],
            "followSymlinks": false,
        }),
        json!({
            "schema": "edict.compiler.input/v1",
            "type": "compilerInput",
            "kind": "path",
            "path": first,
        }),
        json!({
            "schema": "edict.compiler.input/v1",
            "type": "compilerInput",
            "kind": "pathList",
            "paths": [first, second],
        }),
        json!({
            "schema": "edict.compiler.input/v1",
            "type": "compilerInput",
            "kind": "directory",
            "path": root,
        }),
        json!({
            "schema": "edict.compiler.input/v1",
            "type": "compilerInput",
            "kind": "glob",
            "pattern": format!("{}/**/*.edict", root.display()),
        }),
        json!({
            "schema": "edict.compiler.input/v1",
            "type": "compilerInput",
            "kind": "source",
            "name": "inline.edict",
            "source": VALID_SOURCE,
        }),
    ]));

    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "all explicit input record kinds should check"
    );
    assert!(
        output.stderr.is_empty(),
        "successful check must not write stderr"
    );
    let stdout = assert_jsonl_stream(&output.stdout, "stdout");
    assert_eq!(check_result_count(&stdout), 8);
    assert_status(&stdout, "ok", 0);
}

#[test]
fn cli_schema_constants_match_checked_in_artifacts() {
    // Every schema identifier the binary emits or accepts must equal the
    // `properties.schema.const` of its checked-in JSON Schema artifact, so the
    // contract files cannot silently drift from the runtime constants.
    let schemas = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/schemas");
    let cases = [
        (
            edict_cli::COMPILER_SETTINGS_SCHEMA,
            "edict.compiler-settings.v1.schema.json",
        ),
        (
            edict_cli::COMPILER_INPUT_SCHEMA,
            "edict.compiler-input.v1.schema.json",
        ),
        (
            edict_cli::CHECK_RESULT_SCHEMA,
            "edict.cli-check-result.v1.schema.json",
        ),
        (
            edict_cli::DIAGNOSTIC_SCHEMA,
            "edict.cli-diagnostic.v1.schema.json",
        ),
        (edict_cli::EVENT_SCHEMA, "edict.cli-event.v1.schema.json"),
        (edict_cli::INFO_SCHEMA, "edict.cli-info.v1.schema.json"),
    ];
    for (constant, file) in cases {
        let text = fs::read_to_string(schemas.join(file))
            .unwrap_or_else(|err| panic!("read schema artifact `{file}`: {err}"));
        let schema: Value = serde_json::from_str(&text)
            .unwrap_or_else(|err| panic!("parse schema artifact `{file}`: {err}"));
        let declared = schema["properties"]["schema"]["const"]
            .as_str()
            .unwrap_or_else(|| panic!("`{file}` missing `properties.schema.const`"));
        assert_eq!(
            declared, constant,
            "`{file}` const must match the runtime schema constant"
        );
    }
}

#[test]
fn version_flag_emits_info_record() {
    for flag in ["--version", "-V"] {
        let output = run_edict_args(&[flag]);
        assert_eq!(output.status.code(), Some(0), "{flag} should exit 0");
        assert!(output.stderr.is_empty(), "{flag} must not write stderr");
        let stdout = assert_jsonl_stream(&output.stdout, "stdout");
        assert_eq!(stdout.len(), 1, "{flag} emits exactly one record");
        assert_eq!(
            stdout[0].get("schema").and_then(Value::as_str),
            Some("edict.cli.info/v1")
        );
        assert_eq!(
            stdout[0].get("topic").and_then(Value::as_str),
            Some("version")
        );
        assert!(stdout[0].get("version").and_then(Value::as_str).is_some());
    }
}

#[test]
fn help_flag_emits_info_record() {
    for flag in ["--help", "-h"] {
        let output = run_edict_args(&[flag]);
        assert_eq!(output.status.code(), Some(0), "{flag} should exit 0");
        assert!(output.stderr.is_empty(), "{flag} must not write stderr");
        let stdout = assert_jsonl_stream(&output.stdout, "stdout");
        assert_eq!(stdout.len(), 1, "{flag} emits exactly one record");
        let record = &stdout[0];
        assert_eq!(record.get("topic").and_then(Value::as_str), Some("help"));
        assert!(record.get("usage").and_then(Value::as_str).is_some());
        // Pin the concrete public payload, not just field presence.
        assert_eq!(
            record.get("requestSchemas"),
            Some(&json!([
                "edict.compiler.settings/v1",
                "edict.compiler.input/v1"
            ])),
            "{flag} help must list the exact accepted request schemas"
        );
        let codes: Vec<i64> = record
            .get("exitCodes")
            .and_then(Value::as_array)
            .expect("help record carries exitCodes")
            .iter()
            .filter_map(|entry| entry.get("code").and_then(Value::as_i64))
            .collect();
        assert_eq!(
            codes,
            [0, 1, 2],
            "{flag} help must document exit codes 0, 1, 2 in order"
        );
        assert_eq!(
            record.get("docs").and_then(Value::as_str),
            Some("docs/topics/cli/README.md"),
            "{flag} help must point at the CLI docs"
        );
    }
}

#[test]
fn unknown_argument_rejected_with_actionable_diagnostic() {
    let output = run_edict_args(&["--nope"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "rejected args must not write stdout"
    );
    let stderr = assert_jsonl_stream(&output.stderr, "stderr");
    let diagnostic = stderr
        .iter()
        .find(|line| line.get("kind").and_then(Value::as_str) == Some("InvalidArguments"))
        .expect("stderr must contain an InvalidArguments diagnostic");
    let message = diagnostic
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    assert!(
        message.contains("--help") && message.contains("docs/topics/cli/README.md"),
        "InvalidArguments must point at --help and the docs"
    );
    assert_status(&stderr, "error", 2);
}

fn run_edict_args(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_edict"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("run edict with args")
}

fn compiler_settings() -> Value {
    json!({
        "schema": "edict.compiler.settings/v1",
        "type": "compilerSettings",
        "operation": "check",
    })
}

fn jsonl<const N: usize>(records: [Value; N]) -> String {
    let mut out = String::new();
    for record in records {
        out.push_str(&serde_json::to_string(&record).expect("serialize input record"));
        out.push('\n');
    }
    out
}

fn run_edict(input: &str) -> Output {
    run_edict_with_env(input, &[])
}

fn run_edict_with_env(input: &str, env: &[(&str, &str)]) -> Output {
    let bin = env!("CARGO_BIN_EXE_edict");
    let mut child = Command::new(bin)
        .env_remove(edict_cli::MAX_STDIN_BYTES_ENV)
        .envs(env.iter().copied())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn edict binary");
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(input.as_bytes())
        .expect("write jsonl stdin");
    child.wait_with_output().expect("collect output")
}

fn assert_jsonl_stream(bytes: &[u8], stream: &str) -> Vec<Value> {
    let text = std::str::from_utf8(bytes).unwrap_or_else(|_| panic!("{stream} must be utf-8"));
    if text.is_empty() {
        return Vec::new();
    }
    assert!(
        text.ends_with('\n'),
        "{stream} must end each JSONL record with a newline"
    );
    text.lines()
        .enumerate()
        .map(|(idx, line)| {
            assert!(
                !line.trim().is_empty(),
                "{stream} line {idx} must not be blank"
            );
            let value = serde_json::from_str::<Value>(line)
                .unwrap_or_else(|err| panic!("{stream} line {idx} must be JSON: {err}"));
            assert!(
                value.as_object().is_some(),
                "{stream} line {idx} must be a JSON object"
            );
            value
        })
        .collect()
}

fn check_result_count(lines: &[Value]) -> usize {
    lines
        .iter()
        .filter(|line| line.get("type").and_then(Value::as_str) == Some("checkResult"))
        .count()
}

fn assert_status(lines: &[Value], expected_status: &str, expected_exit: i32) {
    let status = lines
        .iter()
        .find(|line| line.get("type").and_then(Value::as_str) == Some("status"))
        .expect("stream must contain a status record");
    assert_eq!(
        status.get("schema").and_then(Value::as_str),
        Some("edict.cli.event/v1")
    );
    assert_eq!(
        status.get("status").and_then(Value::as_str),
        Some(expected_status)
    );
    assert_eq!(
        status.get("exitCode").and_then(Value::as_i64),
        Some(i64::from(expected_exit))
    );
}

fn temp_tree(name: &str) -> PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "edict-cli-jsonl-{}-{name}-{counter}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).expect("create temp fixture dir");
    path
}
