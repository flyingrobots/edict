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
    let bin = env!("CARGO_BIN_EXE_edict");
    let mut child = Command::new(bin)
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
