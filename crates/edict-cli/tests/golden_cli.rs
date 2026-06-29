//! Golden CLI fixture corpus.
//!
//! Each case under `fixtures/cli/<name>/` is replayed end-to-end through the
//! `edict` binary and its stdout, stderr, and exit code are matched byte-for-byte
//! against checked-in goldens. The binary runs with the case directory as its
//! working directory, so any `inputs/` paths a request names resolve to stable
//! relative paths in the emitted records.
//!
//! Case layout:
//! - `request.jsonl`        — stdin sent to the binary (required).
//! - `expected.stdout.jsonl`— exact expected stdout (absent ⇒ empty).
//! - `expected.stderr.jsonl`— exact expected stderr (absent ⇒ empty).
//! - `exit`                 — expected process exit code (absent ⇒ `0`).
//! - `inputs/…`             — source files referenced by path/dir/glob requests.
//!
//! To regenerate a case's goldens after an intentional contract change, build
//! the binary and replay it from inside the case directory, then review the diff
//! before committing:
//! `cargo build -p edict-cli`
//! `cd fixtures/cli/<name>`
//! `../../../target/debug/edict < request.jsonl > expected.stdout.jsonl 2> expected.stderr.jsonl`

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[test]
fn golden_cli_fixtures_replay_exactly() {
    // `fixtures/cli/` lives at the workspace root, two directories above this
    // crate (`crates/edict-cli`); keep that invariant if the crate ever moves.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cli");
    let mut cases: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap_or_else(|err| panic!("read {}: {err}", root.display()))
        .map(|entry| entry.expect("read fixtures/cli entry").path())
        .filter(|path| path.is_dir())
        .collect();
    cases.sort();

    assert!(
        cases.len() >= 9,
        "expected at least 9 golden CLI cases, found {}",
        cases.len()
    );

    for case in &cases {
        replay_case(case);
    }
}

fn replay_case(dir: &Path) {
    let name = dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_else(|| panic!("case dir {} has no name", dir.display()));
    let request = fs::read_to_string(dir.join("request.jsonl"))
        .unwrap_or_else(|err| panic!("[{name}] read request.jsonl: {err}"));
    let expected_stdout = read_optional(&dir.join("expected.stdout.jsonl"));
    let expected_stderr = read_optional(&dir.join("expected.stderr.jsonl"));
    let expected_exit: i32 = read_optional(&dir.join("exit")).trim().parse().unwrap_or(0);

    let bin = env!("CARGO_BIN_EXE_edict");
    let mut child = Command::new(bin)
        .current_dir(dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap_or_else(|err| panic!("[{name}] spawn edict: {err}"));
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(request.as_bytes())
        .unwrap_or_else(|err| panic!("[{name}] write stdin: {err}"));
    let output = child
        .wait_with_output()
        .unwrap_or_else(|err| panic!("[{name}] collect output: {err}"));

    let stdout = String::from_utf8(output.stdout)
        .unwrap_or_else(|_| panic!("[{name}] stdout must be utf-8"));
    let stderr = String::from_utf8(output.stderr)
        .unwrap_or_else(|_| panic!("[{name}] stderr must be utf-8"));

    assert_eq!(stdout, expected_stdout, "[{name}] stdout golden mismatch");
    assert_eq!(stderr, expected_stderr, "[{name}] stderr golden mismatch");
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "[{name}] exit code golden mismatch"
    );
}

fn read_optional(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}
