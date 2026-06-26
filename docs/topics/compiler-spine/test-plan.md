# Compiler Spine Test Plan

Status: current verification design for the executable source-to-Core spine.

## Scope

In scope:

- explicit stage APIs: resolve, type-check, lower to in-memory Core;
- deterministic compiler context facts for profile and budget resolution;
- deterministic compiler context facts for profile write permissions and effect
  write classes;
- typed representation boundary distinct from source AST;
- source-to-Core lowering for the initial pure local-record subset;
- structured compiler error identity.

Out of scope:

- canonical Core bytes embedded in lowerer output;
- Core digest computation owned by the Core IR shelf;
- target-profile lowering;
- admission bundles;
- full source language coverage.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| CSPINE-REQ-001 | implemented | `resolve_module` resolves module/import coordinates and explicit context facts without collapsing into type checking or lowering. | issue #20 |
| CSPINE-REQ-002 | implemented | `type_check` produces a typed module boundary distinct from source AST and rejects unresolved or incompatible types. | issue #20 |
| CSPINE-REQ-003 | implemented | `lower_core` lowers the typed initial subset into structured in-memory Core IR. | issue #20, docs/abi/edict-core.cddl |
| CSPINE-REQ-004 | implemented | `compile_to_core` executes `validate_surface -> resolve_module -> type_check -> lower_core` in order. | issue #20 |
| CSPINE-REQ-005 | implemented | Profile and budget source coordinates require explicit deterministic context facts; missing facts reject instead of producing placeholder Core. | issue #20 |
| CSPINE-REQ-006 | implemented | The first lowerable subset covers `bounded-hello` style pure local-record intents and rejects out-of-subset constructs structurally. | fixtures/lang/bounds/bounded-hello.edict, issue #20 |
| CSPINE-REQ-007 | implemented | Compiler-spine errors expose stable stage and kind identities. | crates/edict-syntax/src/compiler.rs |
| CSPINE-REQ-008 | implemented | The compiler-spine lowerer embeds no canonical bytes, exact digest, target lowering, or admission artifacts in Core modules. | ROADMAP.md |
| CSPINE-REQ-009 | implemented | The compiler spine rejects source effect bodies whose effect write class is not allowed by the resolved operation profile. | issue #54 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/bounds/bounded-hello.edict | Initial pure local-record source-to-Core fixture. | `compile_to_core` returns a structured `CoreModule` with expected records, profile, budget, predicate, nodes, and result. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CSPINE-TP-001 | implemented | Golden path | CSPINE-REQ-001, CSPINE-REQ-002, CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-006 | `bounded-hello` compiles through all stages to a `CoreModule` with expected structured fields. | bounded_hello_compiles_to_initial_core | fixtures/lang/bounds/bounded-hello.edict | In-memory Core only; no bytes or digests. |
| CSPINE-TP-002 | implemented | Stage boundary | CSPINE-REQ-001, CSPINE-REQ-002, CSPINE-REQ-003 | Explicit `resolve_module`, `type_check`, and `lower_core` calls expose distinct stage outputs. | compiler_spine_exposes_distinct_stage_boundaries | fixtures/lang/bounds/bounded-hello.edict | Prevents a hidden monolithic semantic pass. |
| CSPINE-TP-003 | implemented | Error handling | CSPINE-REQ-005, CSPINE-REQ-007 | Missing profile or budget facts return `CompilerStage::Resolve` plus `MissingContextFact`. | missing_context_facts_reject_in_resolve_stage | fixtures/lang/bounds/bounded-hello.edict | No placeholder Core budgets. |
| CSPINE-TP-004 | implemented | Error handling | CSPINE-REQ-002, CSPINE-REQ-007 | Unknown local named types return `CompilerStage::TypeCheck` plus `UnresolvedType`. | unresolved_local_types_reject_in_type_check_stage, unresolved_record_field_types_reject_in_type_check_stage | - | Surface validation still accepts the source. |
| CSPINE-TP-005 | implemented | Error handling | CSPINE-REQ-002, CSPINE-REQ-007 | Returning a record with the wrong field shape, or failing to return, returns `CompilerStage::TypeCheck` plus `TypeMismatch`. | record_return_shape_mismatch_rejects_in_type_check_stage, missing_return_rejects_in_type_check_stage | - | Asserts type identity, not diagnostic prose. |
| CSPINE-TP-006 | implemented | Boundary guard | CSPINE-REQ-008 | The lowered Core module carries no canonical bytes, digest, target IR, or admission fields. | initial_core_lowering_makes_no_canonical_or_target_claim | fixtures/lang/bounds/bounded-hello.edict | Keeps #21/#22 boundaries honest. |
| CSPINE-TP-007 | implemented | Boundary guard | CSPINE-REQ-007, CSPINE-REQ-009 | A write-class effect body under a read-only operation profile rejects in `CompilerStage::TypeCheck` with `ProfileEffectMismatch`. | read_only_profile_rejects_write_effect_body | - | Uses in-memory context facts; no target/lawpack file loading. |

## Determinism Obligations

- Tests inspect structured Rust values only.
- Compiler context facts are in-memory constants, not environment reads.
- Maps use deterministic key ordering.
- No test reads stdout, stderr, logs, wall-clock time, random values, or
  filesystem ordering.
- Canonical encoder behavior, reviewed golden bytes, and digest determinism are
  verified in the Core IR shelf.

## Open Gaps

- Target/lawpack/shape artifact loading belongs to later lowerability work.
- Effectful branches, loops, matches, variants, and obstruction maps are present
  in the source AST and Core schema but outside the first lowerable subset.
