# Public Rust API Test Plan

## Scope

In scope:

- a curated Rust facade with the library name `edict`;
- source checking and stable diagnostic-kind access;
- canonical Core, Target IR, and result-projection identity access;
- package inventory and clean external-consumer checks;
- an explicit non-publication boundary.

Out of scope:

- crates.io publication or crate-name reservation;
- a stable 1.0 API;
- exposing the implementation crate's full module tree;
- replacing the CLI application-build boundary;
- splitting every compiler subsystem into its final crate.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| PUBRUST-REQ-001 | implemented | One curated package exposes Edict source checking, stable diagnostic kinds, and canonical artifact identity operations without re-exporting the implementation module tree. | issue #189 |
| PUBRUST-REQ-002 | planned | The facade's package inventory is explicit, reproducible, and remains non-publishing until a separately approved publication policy exists. | issue #189 |
| PUBRUST-REQ-003 | planned | A clean external consumer can compile against the facade without an undocumented repository-relative dependency. | issue #189 |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PUBRUST-TP-001 | implemented | Public API | PUBRUST-REQ-001 | The consumer compiles and source checking returns `CheckOutcome::Valid`. | `curated_facade_exposes_check_diagnostics_and_artifact_identity` | crates/edict/tests/public_surface.rs | Imports check, diagnostic, and artifact-identity roles through `edict`. |
| PUBRUST-TP-002 | implemented | Negative compile | PUBRUST-REQ-001 | The implementation module tree is unavailable through `edict`. | implementation_modules_are_compile_fail_doctested | crates/edict/src/lib.rs, crates/edict/tests/public_surface.rs | The integration witness binds this row to the `compile_fail` doctest attempted by the workspace test pass. |
| PUBRUST-TP-003 | planned | Package boundary | PUBRUST-REQ-002 | Packaging succeeds with the reviewed inventory without publishing or mutating registry state. | release-engineering package check | crates/edict/Cargo.toml | The current package inventory dry run succeeds; the complete registry dependency closure remains unpublished. |
| PUBRUST-TP-004 | planned | External consumer | PUBRUST-REQ-003 | The project compiles and runs without a sibling Edict checkout. | release-engineering external-consumer check | - | Requires packaged implementation dependencies or a sealed local registry before publication. |

## Known Gaps

- The implementation dependency still needs a permanent registry package name
  and a completed dependency-closure dry run before publication can be
  considered.
- Registry names, ownership, credentials, and publication automation remain
  deliberately unconfigured.
