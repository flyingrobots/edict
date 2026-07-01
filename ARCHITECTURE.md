# Edict Architecture

Status: current workspace map for `v0.11.0-alpha.1` plus the audit-remediation
branch. This page describes what exists in HEAD; it is not a future package
plan.

## Workspace Shape

The Rust workspace has three members:

```text
edict-cli  ->  edict-syntax
xtask      ->  edict-syntax
```

`edict-syntax` has no dependency on the CLI or `xtask`. The CLI owns the public
JSONL process boundary and delegates language work to `edict-syntax`. `xtask`
owns repository maintenance checks, reviewed golden regeneration, release
process guards, and topic-shelf contract checks.

## Crates

### `edict-syntax`

`edict-syntax` is the implementation crate for more than syntax. Its public name
is historical and currently too narrow for its responsibilities. The crate
exports:

- lexical analysis and parsing;
- source/surface semantic validation;
- compiler context facts and authority-fact loading;
- source-to-Core compiler spine for the current supported subset;
- Core IR data structures;
- canonical Core, Target IR, bundle-layer encoders and digest helpers;
- target-profile conformance checks;
- lowerability checks;
- Echo and git-warp Target IR artifact lowering;
- contract-bundle assembly and validation;
- Gate C admission-boundary request/receipt validation;
- editor/highlighting support.

That breadth is deliberate as current branch truth, not an endorsement of the
name. The crate-scope decision is recorded in
[`docs/design/crate-scope-v0.11.md`](./docs/design/crate-scope-v0.11.md):
eventual layered split behind an umbrella crate, not a simple rename. Until that
split lands, new code should preserve the existing dependency direction and keep
layer boundaries explicit inside the crate.

Module map:

| Module | Responsibility |
| --- | --- |
| `token` | Lexer tokens, spans, integer suffixes, and lexing failures. |
| `parser` | Source grammar parser producing AST modules. |
| `ast` | Source-level syntax tree types. |
| `semantic` | Surface validation that does not require import resolution or target facts. |
| `authority_facts` | File-backed compiler context facts for profiles, budgets, write classes, and source identity. |
| `compiler` | Resolve, type-check, and lower the supported source subset to Core IR. |
| `core_ir` | Runtime-neutral Core module, intent, expression, budget, import, and obstruction data. |
| `canonical` | Canonical value model, canonical CBOR encoder/decoder, digest frames, and reviewed golden digest helpers. |
| `target_profile` | Runtime-neutral target-profile manifest conformance. |
| `lowerability` | Checks whether Core requirements can be satisfied natively, by a direct adapter, or not at all. |
| `target_ir` | Current Echo and git-warp Target IR artifact construction from Core plus lowering facts. |
| `contract_bundle` | Participant-neutral bundle assembly, bundle digest preimages, validation, and assurance evidence binding. |
| `admission` | Edict-owned Gate C request/receipt shape and binding validation without participant policy execution. |
| `highlight` | Lexical highlight roles consumed by editor tooling. |
| `lib` | Public API facade and re-exports for the current alpha surface. |

### `edict-cli`

`edict-cli` provides the `edict` binary and its small support library. The binary
is JSONL-only today:

- it accepts compiler settings and compiler input records on stdin;
- it supports `check`;
- it emits typed JSONL result, diagnostic, status, and info records;
- it enforces a bounded stdin size;
- it can confine path, path-list, directory, and glob inputs to an optional
  `inputRoot`;
- it delegates parse and surface validation to `edict_syntax::check`.

The CLI does not compile to Core, lower to Target IR, assemble bundles, admit
bundles, or execute runtime behavior. It should remain a stream-contract and
local input-boundary owner unless a future topic shelf expands its public
surface.

### `xtask`

`xtask` is the repository contract harness. Current commands include:

- `cargo xtask verify`;
- `cargo xtask contract-check`;
- `cargo xtask core-goldens --check/--write`;
- `cargo xtask target-ir-goldens --check/--write`;
- `cargo xtask bundle-goldens --check/--write`;
- `cargo xtask cli-goldens --check/--write`;
- `cargo xtask release-prep <version>`.

`verify` runs formatting, clippy, workspace tests, doctests, golden checks, topic
contract checks, and diff hygiene. `contract-check` validates topic-shelf tables,
evidence references, and local links. Golden commands regenerate or check the
reviewed byte/digest/stream fixtures. `release-prep` writes mechanical release
scaffolding but does not decide scope or create GitHub state.

The `xtask` implementation is split by responsibility:

- `main.rs` owns command dispatch and the `verify` sequence;
- `contract_check.rs` owns topic-shelf, evidence, and local-link validation;
- `goldens.rs` owns Core, Target IR, bundle, and CLI golden check/write paths;
- `release_prep.rs` owns mechanical release-prep scaffolding;
- `util.rs` owns repository walking, command execution, and git-base helpers;
- `tests.rs` keeps the contract-harness regression tests out of dispatch code.

This split is structural only; command behavior remains guarded by
`cargo test -p xtask` and `cargo xtask verify`.

## Layer Flow

The language and artifact flow is:

```text
source text
  -> token / parser / ast
  -> semantic surface validation
  -> compiler context facts
  -> compiler spine
  -> Core IR
  -> canonical Core bytes and digest
  -> lowerability + target facts
  -> Target IR artifact
  -> canonical Target IR bytes and digest
  -> contract-bundle assembly and validation
  -> admission-boundary request/receipt validation
```

The CLI currently exercises only the front end through surface validation. Tests
and `xtask` exercise deeper layers directly.

## Dependency Rules

Use these rules when placing new code:

- Language model, validation, compiler, canonicalization, target, bundle, and
  admission-boundary behavior belongs in `edict-syntax` until the crate-scope
  decision changes the package layout.
- CLI stream parsing, process exit codes, stdin/path trust boundaries, and JSONL
  record production belong in `edict-cli`.
- Repository checks, golden regeneration, release scaffolding, workflow guards,
  and topic-shelf validation belong in `xtask`.
- `edict-syntax` must not depend on `edict-cli` or `xtask`.
- `edict-cli` may depend on `edict-syntax`; it should not duplicate language
  semantics that the library already owns.
- `xtask` may depend on `edict-syntax` and may inspect repository files, but its
  checks must remain deterministic and avoid live GitHub state unless a workflow
  explicitly owns that boundary.

## Current Non-Claims

This workspace does not yet implement:

- target runtime execution;
- participant admission execution;
- participant policy evaluation;
- trusted lawpack or target-profile authorship;
- general target plugin dispatch;
- canonical `ContractBundleManifest` bytes;
- crates.io publication.

Those boundaries are intentionally not hidden in the crate map. They are current
non-claims and should stay explicit in release notes, topic shelves, and pull
request bodies until the owning behavior lands.
