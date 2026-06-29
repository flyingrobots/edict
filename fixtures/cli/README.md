# CLI Golden Fixtures

Each subdirectory is one end-to-end golden case for the public `edict` CLI. The
`golden_cli_fixtures_replay_exactly` test (in
`crates/edict-cli/tests/golden_cli.rs`) replays every case through the compiled
binary and matches stdout, stderr, and the exit code byte-for-byte against the
checked-in goldens.

## Case layout

| File | Required | Meaning |
| --- | --- | --- |
| `request.jsonl` | yes | JSONL request written to the binary's stdin. |
| `expected.stdout.jsonl` | no | Exact expected stdout. Absent means empty. |
| `expected.stderr.jsonl` | no | Exact expected stderr. Absent means empty. |
| `exit` | no | Expected process exit code. Absent means `0`. |
| `inputs/…` | no | Source files referenced by `path`, `directory`, `pathList`, or `glob` requests. |

The binary runs with the case directory as its working directory, so any path a
request names resolves to a stable relative path in the emitted records.

## Coverage

- `01-source-ok` — inline source success.
- `02-source-parse-rejection` — parse-stage diagnostic, exit `1`.
- `03-source-semantic-rejection` — semantic-stage diagnostic, exit `1`.
- `04-cli-missing-settings` — CLI-input rejection before compilation, exit `2`.
- `05-path-input-ok` — single file path input.
- `06-directory-expansion-ok` — recursive directory expansion, sorted, non-`.edict` files ignored.
- `07-path-list-ok` — ordered path list, request order preserved.
- `08-glob-expansion-ok` — glob expansion, sorted.

## Regenerating a case

After an intentional contract change, regenerate a case from inside its
directory and review the diff before committing:

```sh
cd fixtures/cli/<case>
edict < request.jsonl > expected.stdout.jsonl 2> expected.stderr.jsonl
```

Delete `expected.stdout.jsonl` or `expected.stderr.jsonl` if the corresponding
stream is empty, and keep the `exit` file in sync with the binary's exit code.
