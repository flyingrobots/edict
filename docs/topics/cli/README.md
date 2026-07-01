# CLI Topic

Status: current HEAD contract for the first public command-line surface.

This chapter describes the Edict CLI stream contract. The CLI is a compiler and
validation boundary, not an interactive pretty-printer. Every input record and
every output record is JSON Lines.

## Public Surface

The `edict` binary reads compiler requests from stdin as JSONL records. It emits
only JSONL records on stdout and stderr. Stdin is bounded before request parsing:
the default cap is 8 MiB, and `EDICT_CLI_MAX_STDIN_BYTES` may override that cap
with a positive byte count. Over-limit input is rejected with an `InputTooLarge`
CLI diagnostic and exit `2`. [CLI-REQ-001, CLI-REQ-010]

The binary takes no positional arguments. The only accepted flags are
`--help`/`-h` and `--version`/`-V`, which emit a single `edict.cli.info/v1`
record on stdout and exit `0`; the `help` record carries the usage summary, the
accepted request schema identifiers, and the exit-code contract. Any other
argument is rejected with an `InvalidArguments` diagnostic and exit `2`.
[CLI-REQ-009]

The first implemented operation is `check`. A `check` request accepts:

- inline source code;
- one file path;
- one directory path;
- an ordered list of file paths;
- one glob pattern.

Each input is represented by a JSON object whose `schema` is
`edict.compiler.input/v1`. Raw source code is not a separate text mode; it is
the `source` field of a JSONL input record. [CLI-REQ-002]

Compiler settings are represented by a JSON object whose `schema` is
`edict.compiler.settings/v1`. [CLI-REQ-003]

Successful compiler results are emitted to stdout. Compiler diagnostics, CLI
input errors, and failure status records are emitted to stderr. Both streams use
one JSON object per line with no banners, spinners, blank lines, or direct human
prose outside JSON string fields. [CLI-REQ-001]

## Stream Contract Artifacts

Every record family on the CLI boundary has a checked-in JSON Schema. These
schemas are the stable contract for callers; the CLI does not embed a schema
validation engine. The binary rejects compiler input records with fields outside
the closed `edict.compiler.input/v1` schema variants, so callers should treat
the checked-in schemas as the accepted wire shape.

| Record `schema` | Direction | Artifact |
| --- | --- | --- |
| `edict.compiler.settings/v1` | stdin | [`compiler-settings`](../../schemas/edict.compiler-settings.v1.schema.json) [CLI-REQ-003] |
| `edict.compiler.input/v1` | stdin | [`compiler-input`](../../schemas/edict.compiler-input.v1.schema.json) [CLI-REQ-004] |
| `edict.cli.check-result/v1` | stdout | [`cli-check-result`](../../schemas/edict.cli-check-result.v1.schema.json) [CLI-REQ-005] |
| `edict.cli.diagnostic/v1` | stderr | [`cli-diagnostic`](../../schemas/edict.cli-diagnostic.v1.schema.json) [CLI-REQ-006] |
| `edict.cli.event/v1` | stdout/stderr | [`cli-event`](../../schemas/edict.cli-event.v1.schema.json) [CLI-REQ-007] |
| `edict.cli.info/v1` | stdout | [`cli-info`](../../schemas/edict.cli-info.v1.schema.json) [CLI-REQ-009] |

## Exit Codes

- `0`: request completed successfully.
- `1`: compiler or validation diagnostics were produced for at least one
  source input.
- `2`: CLI input or usage was invalid before compiler validation could run.

## Golden Fixtures

The CLI contract is pinned by a checked-in golden corpus under
[`fixtures/cli/`](../../../fixtures/cli/). Each case is replayed end-to-end
through the binary and its stdout, stderr, and exit code are matched
byte-for-byte. The corpus covers success, parse and semantic rejection,
CLI-input rejection, and the deterministic path, directory, path-list, and glob
expansion paths. [CLI-REQ-008]

## Deferred

The following are not implemented by this first CLI slice:

- Core artifact emission;
- Target IR artifact emission;
- bundle assembly;
- admission workflow execution;
- human-pretty output mode;
- language-server transport.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
