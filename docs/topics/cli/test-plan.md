# CLI Test Plan

Status: current verification design for the first public command-line surface.

## Scope

In scope:

- JSONL-only stdin request records;
- JSONL-only stdout and stderr records;
- compiler settings as a stable JSON Schema artifact;
- `check` over inline source, file paths, directories, path lists, and glob
  patterns;
- structured parser and CLI diagnostics;
- a checked-in golden fixture corpus replayed end-to-end through the binary.

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
| CLI-REQ-004 | implemented | Compiler input records are declared as `edict.compiler.input/v1` and have a checked-in JSON Schema contract covering every supported input kind. | docs/schemas/edict.compiler-input.v1.schema.json |
| CLI-REQ-005 | implemented | Success results are declared as `edict.cli.check-result/v1` and have a checked-in JSON Schema contract. | docs/schemas/edict.cli-check-result.v1.schema.json |
| CLI-REQ-006 | implemented | Diagnostics are declared as `edict.cli.diagnostic/v1` and have a checked-in JSON Schema contract spanning parse, semantic, and CLI stages. | docs/schemas/edict.cli-diagnostic.v1.schema.json |
| CLI-REQ-007 | implemented | Terminal status events are declared as `edict.cli.event/v1` and have a checked-in JSON Schema contract. | docs/schemas/edict.cli-event.v1.schema.json |
| CLI-REQ-008 | implemented | A checked-in golden fixture corpus replays end-to-end through the binary and matches stdout, stderr, and exit code byte-for-byte for success, compiler rejection, CLI-input rejection, and deterministic input expansion. | crates/edict-cli/tests/golden_cli.rs |
| CLI-REQ-009 | implemented | The binary supports `--help`/`-h` and `--version`/`-V`, which emit a single `edict.cli.info/v1` record (declared by a checked-in JSON Schema) on stdout and exit 0; any other argument is rejected with an actionable `InvalidArguments` diagnostic and exit 2. | crates/edict-cli/tests/jsonl_cli.rs, docs/schemas/edict.cli-info.v1.schema.json |
| CLI-REQ-010 | implemented | The binary rejects stdin that exceeds its configured byte limit before request parsing, emits a stable `InputTooLarge` CLI diagnostic, and exits 2. | crates/edict-cli/tests/jsonl_cli.rs, fixtures/cli/12-input-too-large/request.jsonl |
| CLI-REQ-011 | implemented | Compiler settings may set `inputRoot` to confine path, path-list, directory, and glob inputs to a caller-selected filesystem root; resolved inputs outside that root fail with `InputPathOutsideRoot` and exit 2. | crates/edict-cli/tests/jsonl_cli.rs, fixtures/cli/13-input-root-outside/request.jsonl |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| crates/edict-cli/tests/jsonl_cli.rs | Runtime-created JSONL requests and source files for CLI behavior. | Tests parse every stdout and stderr line as JSON objects and assert stable fields. |
| docs/schemas/edict.compiler-settings.v1.schema.json | Stable JSON Schema for compiler settings records. | The schema contract test validates the schema identifier, required fields, supported operation values, deterministic input-expansion settings, and optional root confinement. |
| docs/schemas/edict.compiler-input.v1.schema.json | Stable JSON Schema for compiler input records. | The schema contract test validates the identifier, required fields, supported input kinds, and per-kind variant fields. |
| docs/schemas/edict.cli-check-result.v1.schema.json | Stable JSON Schema for success result records. | The schema contract test validates the identifier, required fields, and pinned `command`, `type`, and `status` values. |
| docs/schemas/edict.cli-diagnostic.v1.schema.json | Stable JSON Schema for diagnostic records. | The schema contract test validates the identifier, required fields, supported stages, and optional span, line, and message fields. |
| docs/schemas/edict.cli-event.v1.schema.json | Stable JSON Schema for terminal status records. | The schema contract test validates the identifier, required fields, terminal status values, and supported exit codes. |
| docs/schemas/edict.cli-info.v1.schema.json | Stable JSON Schema for `--help`/`--version` informational records. | The schema contract test validates the identifier, required fields, supported topics, and the help-topic conditional fields. |
| crates/edict-cli/tests/golden_cli.rs | Golden replay harness for the `fixtures/cli/` corpus. | Replays each case through the binary and matches stdout, stderr, and exit code byte-for-byte against checked-in goldens. |
| fixtures/cli/01-source-ok/request.jsonl | Representative golden CLI request record. | Replayed by the golden harness; its goldens pin the success-path stdout and status records. |
| fixtures/cli/12-input-too-large/request.jsonl | Golden CLI request replayed with a tiny stdin cap. | Replayed by the golden harness; stderr pins the `InputTooLarge` diagnostic and exit 2. |
| fixtures/cli/13-input-root-outside/request.jsonl | Golden CLI request with `inputRoot` set to a sibling directory of the requested path. | Replayed by the golden harness; stderr pins the `InputPathOutsideRoot` diagnostic and exit 2. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CLI-TP-001 | implemented | Golden path | CLI-REQ-001, CLI-REQ-002 | A valid inline source request exits 0, writes only JSONL objects to stdout, writes nothing to stderr, and emits an `ok` status record. | check_accepts_inline_source_jsonl_and_emits_jsonl_stdout | crates/edict-cli/tests/jsonl_cli.rs | Tests parse the stream as JSONL, not prose. |
| CLI-TP-002 | implemented | Error handling | CLI-REQ-001 | Invalid Edict source exits 1, writes only JSONL diagnostics and status to stderr, and writes nothing to stdout. | check_rejects_invalid_source_with_jsonl_stderr_only | crates/edict-cli/tests/jsonl_cli.rs | Asserts stable stage and kind fields. |
| CLI-TP-003 | implemented | Error handling | CLI-REQ-001 | Non-JSONL stdin exits 2 and writes a structured CLI diagnostic plus status to stderr. | non_jsonl_input_rejects_with_jsonl_cli_diagnostic | crates/edict-cli/tests/jsonl_cli.rs | Prevents ad hoc text input modes. |
| CLI-TP-004 | implemented | Golden path | CLI-REQ-002 | Path, directory, path-list, glob, and source input records all feed the `check` operation and produce deterministic JSONL result records. | check_accepts_path_directory_path_list_glob_and_source_records | crates/edict-cli/tests/jsonl_cli.rs | Directory and glob expansion order is sorted. |
| CLI-TP-005 | implemented | Schema guard | CLI-REQ-003 | The compiler settings JSON Schema declares `edict.compiler.settings/v1`, `compilerSettings`, the `check` operation, and supported deterministic input-expansion fields. | compiler_settings_schema_declares_jsonl_contract | docs/schemas/edict.compiler-settings.v1.schema.json | Contract-artifact test, not prose matching. |
| CLI-TP-006 | implemented | Schema guard | CLI-REQ-004 | The compiler input JSON Schema declares `edict.compiler.input/v1`, `compilerInput`, every supported input kind, and the per-kind variant fields. | compiler_input_schema_declares_jsonl_contract | docs/schemas/edict.compiler-input.v1.schema.json | Contract-artifact test, not prose matching. |
| CLI-TP-007 | implemented | Schema guard | CLI-REQ-005 | The check result JSON Schema declares `edict.cli.check-result/v1`, `checkResult`, the `check` command, the `ok` status, and the input descriptor. | check_result_schema_declares_jsonl_contract | docs/schemas/edict.cli-check-result.v1.schema.json | Contract-artifact test, not prose matching. |
| CLI-TP-008 | implemented | Schema guard | CLI-REQ-006 | The diagnostic JSON Schema declares `edict.cli.diagnostic/v1`, `diagnostic`, the parse, semantic, and cli stages, and optional span, line, and message fields. | diagnostic_schema_declares_jsonl_contract | docs/schemas/edict.cli-diagnostic.v1.schema.json | Contract-artifact test, not prose matching. |
| CLI-TP-009 | implemented | Schema guard | CLI-REQ-007 | The event JSON Schema declares `edict.cli.event/v1`, `status`, the `ok` and `error` terminal statuses, and exit codes 0, 1, and 2. | event_schema_declares_jsonl_contract | docs/schemas/edict.cli-event.v1.schema.json | Contract-artifact test, not prose matching. |
| CLI-TP-010 | implemented | Golden path | CLI-REQ-008 | Success and deterministic input-expansion cases (inline source, GraphQL-shape-importing source, file path, recursive directory, ordered path list, glob) replay through the binary and match their stdout, stderr, and exit-code goldens exactly. | golden_cli_fixtures_replay_exactly | crates/edict-cli/tests/golden_cli.rs | Byte-for-byte golden replay, not prose matching. |
| CLI-TP-011 | implemented | Error handling | CLI-REQ-008 | Compiler parse rejection, compiler semantic rejection, and CLI-input rejection cases replay through the binary and match their stderr diagnostics, terminal status, and exit-code goldens exactly. | golden_cli_fixtures_replay_exactly | crates/edict-cli/tests/golden_cli.rs | Covers exit codes 1 and 2 with stable stage and kind fields. |
| CLI-TP-012 | implemented | Golden path | CLI-REQ-009 | `--version`/`-V` and `--help`/`-h` each emit exactly one `edict.cli.info/v1` record on stdout, write nothing to stderr, and exit 0; the help record carries usage, request schemas, and exit codes. | version_flag_emits_info_record, help_flag_emits_info_record | crates/edict-cli/tests/jsonl_cli.rs | Flags emit JSONL, not plain text. |
| CLI-TP-013 | implemented | Error handling | CLI-REQ-009 | An unrecognized argument exits 2 with an `InvalidArguments` diagnostic whose message points at `--help` and the CLI docs. | unknown_argument_rejected_with_actionable_diagnostic | crates/edict-cli/tests/jsonl_cli.rs | Actionable error, not just a rejection. |
| CLI-TP-014 | implemented | Schema guard | CLI-REQ-009 | The info JSON Schema declares `edict.cli.info/v1`, `info`, the `help` and `version` topics, and the help-topic conditional fields. | info_schema_declares_jsonl_contract | docs/schemas/edict.cli-info.v1.schema.json | Contract-artifact test, not prose matching. |
| CLI-TP-015 | implemented | Error handling | CLI-REQ-004 | The binary rejects input records the `edict.compiler.input/v1` schema rejects: an unrecognized field and a record mixing fields from two input kinds both fail with `InvalidInputRecord` and exit 2. | golden_cli_fixtures_replay_exactly | crates/edict-cli/tests/golden_cli.rs | Parser accepts exactly what the published schema accepts. |
| CLI-TP-016 | implemented | Error handling | CLI-REQ-010 | Stdin larger than the configured byte limit fails before request parsing with `InputTooLarge`, writes no stdout, and exits 2. | oversized_stdin_rejects_with_input_too_large_diagnostic, golden_cli_fixtures_replay_exactly | crates/edict-cli/tests/jsonl_cli.rs, fixtures/cli/12-input-too-large/request.jsonl | Prevents unbounded stdin buffering from a hostile or runaway producer. |
| CLI-TP-017 | implemented | Error handling | CLI-REQ-011 | A path or glob input that resolves outside configured `inputRoot` fails before source checking with `InputPathOutsideRoot`, writes no stdout, and exits 2. | input_root_rejects_path_outside_configured_root, input_root_rejects_glob_outside_configured_root, golden_cli_fixtures_replay_exactly | crates/edict-cli/tests/jsonl_cli.rs, fixtures/cli/13-input-root-outside/request.jsonl | Documents the trusted-input default and gives automation callers an opt-in root-confinement guard. |

## Determinism Obligations

- Directory and glob expansion must be sorted before checking files.
- Tests parse stdout and stderr as JSONL; they do not inspect diagnostic prose.
- Raw source input is carried inside a JSON string field.
- CLI input errors must not panic or fall back to non-JSON output.
- Stdin must be bounded before request parsing; over-limit input must fail as
  `InputTooLarge` rather than flowing into JSONL parsing.
- When `inputRoot` is configured, filesystem-backed inputs must resolve inside
  that root before source checking; outside paths fail as `InputPathOutsideRoot`.

## Open Gaps

- No CLI surface for compile, lower, explain, bundle, or admission workflows yet.
- No JSON Schema validation engine is embedded in the CLI; the schema is the
  stable contract artifact for callers.
