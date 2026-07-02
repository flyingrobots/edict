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

The implemented operations are `check` and `project`.

A `check` request accepts:

- inline source code;
- one file path;
- one directory path;
- an ordered list of file paths;
- one glob pattern.

Each input is represented by a JSON object whose `schema` is
`edict.compiler.input/v1`. Raw source code is not a separate text mode; it is
the `source` field of a JSONL input record. [CLI-REQ-002]

A `project` request uses the same input records, including inline `source`
records for dirty editor buffers whose contents do not have to exist on disk.
It emits editor-facing projection records for requested slots:

- lexical syntax spans;
- diagnostics, always as a projection record when requested, even when empty,
  and also when a requested syntax projection cannot be produced because lexing
  fails;
- Core review JSON plus the canonical Core digest;
- Target IR review JSON plus the canonical Target IR digest.

Projection review JSON is display data for tools. The digest fields are
computed from the existing canonical Core and Target IR encoders; the projection
JSON is not a canonical hash contract. Compiler-level source or lowering
failures remain projection data on stdout, so editor adapters can distinguish a
bad buffer from a broken CLI transport. [CLI-REQ-013, CLI-REQ-014]

Compiler settings are represented by a JSON object whose `schema` is
`edict.compiler.settings/v1`. The request is trusted local input by default:
path, directory, path-list, and glob records read files with the caller's
filesystem privileges. Callers that accept untrusted request records can set
`inputRoot` in compiler settings; then every resolved filesystem input must stay
within that root, or the CLI rejects the request with `InputPathOutsideRoot` and
exit `2`. Inline source records are not filesystem reads. [CLI-REQ-003,
CLI-REQ-011]

Successful `check` results are emitted to stdout. `check` compiler diagnostics,
CLI input errors, and failure status records are emitted to stderr. `project`
projection records, including compiler diagnostics and lowering failures, are
emitted to stdout when the request itself is valid. Both streams use one JSON
object per line with no banners, spinners, blank lines, or direct human prose
outside JSON string fields. [CLI-REQ-001, CLI-REQ-014]

When a CLI-input failure happens after the requested operation is known, the
diagnostic and terminal status records carry that command. Invalid `project`
settings therefore report `command: "project"` rather than falling back to the
`check` command. [CLI-REQ-006, CLI-REQ-007]

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
| `edict.projection.syntax/v1` | stdout | [`projection-syntax`](../../schemas/edict.projection-syntax.v1.schema.json) [CLI-REQ-013] |
| `edict.projection.diagnostics/v1` | stdout | [`projection-diagnostics`](../../schemas/edict.projection-diagnostics.v1.schema.json) [CLI-REQ-013, CLI-REQ-014] |
| `edict.projection.core/v1` | stdout | [`projection-core`](../../schemas/edict.projection-core.v1.schema.json) [CLI-REQ-013, CLI-REQ-014] |
| `edict.projection.target-ir/v1` | stdout | [`projection-target-ir`](../../schemas/edict.projection-target-ir.v1.schema.json) [CLI-REQ-013, CLI-REQ-014] |
| `edict.cli.diagnostic/v1` | stderr | [`cli-diagnostic`](../../schemas/edict.cli-diagnostic.v1.schema.json) [CLI-REQ-006] |
| `edict.cli.event/v1` | stdout/stderr | [`cli-event`](../../schemas/edict.cli-event.v1.schema.json) [CLI-REQ-007] |
| `edict.cli.info/v1` | stdout | [`cli-info`](../../schemas/edict.cli-info.v1.schema.json) [CLI-REQ-009] |

## Exit Codes

- `0`: request completed successfully. For `project`, this can include
  compiler diagnostics or Target IR lowering failures emitted as projection
  records.
- `1`: compiler or validation diagnostics were produced for at least one
  source input in the `check` operation.
- `2`: CLI input or usage was invalid before compiler validation could run.

## Golden Fixtures

The CLI contract is pinned by a checked-in golden corpus under
[`fixtures/cli/`](../../../fixtures/cli/). Each case is replayed end-to-end
through the binary and its stdout, stderr, and exit code are matched
byte-for-byte. The corpus covers success, parse and semantic rejection,
CLI-input rejection, and the deterministic path, directory, path-list, and glob
expansion paths, including optional root-confinement rejection. [CLI-REQ-008]

## Deferred

The following are not implemented by this first CLI slice:

- bundle assembly;
- admission workflow execution;
- human-pretty output mode;
- Echo execution;
- language-server transport.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
