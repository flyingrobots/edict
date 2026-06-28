# CLI Test Plan

Status: current verification design for the first public command-line surface.

## Scope

In scope:

- JSONL-only stdin request records;
- JSONL-only stdout and stderr records;
- compiler settings as a stable JSON Schema artifact;
- `check` over inline source, file paths, directories, path lists, and glob
  patterns;
- structured parser and CLI diagnostics.

Out of scope:

- human-pretty output;
- Core artifact emission;
- Target IR artifact emission;
- bundle assembly;
- admission workflows;
- language-server transport.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| CLI-REQ-001 | implemented | The `edict` CLI accepts JSONL request records on stdin and emits only JSONL records on stdout and stderr. | crates/edict-cli/tests/jsonl_cli.rs |
| CLI-REQ-002 | implemented | The `check` operation accepts source, path, directory, path-list, and glob input records without ad hoc text input modes. | crates/edict-cli/tests/jsonl_cli.rs |
| CLI-REQ-003 | implemented | Compiler settings are declared as `edict.compiler.settings/v1` and have a checked-in JSON Schema contract. | docs/schemas/edict.compiler-settings.v1.schema.json |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| crates/edict-cli/tests/jsonl_cli.rs | Runtime-created JSONL requests and source files for CLI behavior. | Tests parse every stdout and stderr line as JSON objects and assert stable fields. |
| docs/schemas/edict.compiler-settings.v1.schema.json | Stable JSON Schema for compiler settings records. | The schema contract test validates the schema identifier, required fields, and supported operation values. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-TP-001 | implemented | Golden path | CLI-REQ-001, CLI-REQ-002 | A valid inline source request exits 0, writes only JSONL objects to stdout, writes nothing to stderr, and emits an `ok` status record. | check_accepts_inline_source_jsonl_and_emits_jsonl_stdout | crates/edict-cli/tests/jsonl_cli.rs | Tests parse the stream as JSONL, not prose. |
| CLI-TP-002 | implemented | Error handling | CLI-REQ-001 | Invalid Edict source exits 1, writes only JSONL diagnostics and status to stderr, and writes nothing to stdout. | check_rejects_invalid_source_with_jsonl_stderr_only | crates/edict-cli/tests/jsonl_cli.rs | Asserts stable stage and kind fields. |
| CLI-TP-003 | implemented | Error handling | CLI-REQ-001 | Non-JSONL stdin exits 2 and writes a structured CLI diagnostic plus status to stderr. | non_jsonl_input_rejects_with_jsonl_cli_diagnostic | crates/edict-cli/tests/jsonl_cli.rs | Prevents ad hoc text input modes. |
| CLI-TP-004 | implemented | Golden path | CLI-REQ-002 | Path, directory, path-list, glob, and source input records all feed the `check` operation and produce deterministic JSONL result records. | check_accepts_path_directory_path_list_glob_and_source_records | crates/edict-cli/tests/jsonl_cli.rs | Directory and glob expansion order is sorted. |
| CLI-TP-005 | implemented | Schema guard | CLI-REQ-003 | The compiler settings JSON Schema declares `edict.compiler.settings/v1`, `compilerSettings`, the `check` operation, and supported deterministic input-expansion fields. | compiler_settings_schema_declares_jsonl_contract | docs/schemas/edict.compiler-settings.v1.schema.json | Contract-artifact test, not prose matching. |

## Determinism Obligations

- Directory and glob expansion must be sorted before checking files.
- Tests parse stdout and stderr as JSONL; they do not inspect diagnostic prose.
- Raw source input is carried inside a JSON string field.
- CLI input errors must not panic or fall back to non-JSON output.

## Open Gaps

- No CLI surface for compile, lower, explain, bundle, or admission workflows yet.
- No JSON Schema validation engine is embedded in the CLI; the schema is the
  stable contract artifact for callers.
