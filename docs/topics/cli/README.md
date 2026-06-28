# CLI Topic

Status: current HEAD contract for the first public command-line surface.

This chapter describes the Edict CLI stream contract. The CLI is a compiler and
validation boundary, not an interactive pretty-printer. Every input record and
every output record is JSON Lines.

## Public Surface

The `edict` binary reads compiler requests from stdin as JSONL records. It emits
only JSONL records on stdout and stderr. [CLI-REQ-001]

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
`edict.compiler.settings/v1`. The schema for those settings is checked in as a
stable contract artifact at
[`docs/schemas/edict.compiler-settings.v1.schema.json`](../../schemas/edict.compiler-settings.v1.schema.json).
[CLI-REQ-003]

Successful compiler results are emitted to stdout. Compiler diagnostics, CLI
input errors, and failure status records are emitted to stderr. Both streams use
one JSON object per line with no banners, spinners, blank lines, or direct human
prose outside JSON string fields. [CLI-REQ-001]

## Exit Codes

- `0`: request completed successfully.
- `1`: compiler or validation diagnostics were produced for at least one
  source input.
- `2`: CLI input or usage was invalid before compiler validation could run.

## Deferred

The following are not implemented by this first CLI slice:

- Core artifact emission;
- Target IR artifact emission;
- bundle assembly;
- admission workflow execution;
- human-pretty output mode;
- language-server transport.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
