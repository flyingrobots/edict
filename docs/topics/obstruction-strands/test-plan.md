# Obstruction Strands Test Plan

Status: current verification ledger for `require` obstruction source syntax and
planned verification design for future resumable obstruction semantics.

## Scope

In scope:

- current parser support for `require ... else` obstruction source shape;
- current compiler boundary that rejects `Stmt::Require` before lowering;
- first-class source syntax for preserved repairable obstruction attempts;
- future Core and Target IR disposition for obstruction preservation;
- negative guard against hidden control flow in ordinary obstruction-target
  constructors.

Out of scope:

- Core semantics for the first-class source syntax;
- Target IR lowering for the first-class source syntax;
- Echo or Jim runtime execution;
- Continuum support-ledger verification;
- XYPH Quest settlement policy.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| OBSTRAND-REQ-001 | implemented | Current `require ... else <obstruction>` parses as `Stmt::Require` source syntax and does not imply hidden continuation semantics. | docs/topics/syntax/test-plan.md |
| OBSTRAND-REQ-002 | implemented | The `require ... else continue obstructed { reason: ... }` form parses as first-class source syntax, not an ordinary helper call hidden inside `else`. | issue #118, docs/design/obstruction-strands-v0.md |
| OBSTRAND-REQ-003 | implemented | The `continue obstructed { ... }` form is contextual to `require ... else`, requires exactly one `reason` field, and rejects duplicate `reason` fields before Core digesting. | issue #118, docs/design/obstruction-strands-v0.md |
| OBSTRAND-REQ-004 | implemented | Helper-shaped obstruction constructors such as `continueInObstructedStrand(...)` do not acquire hidden continuation semantics. | issue #118, docs/design/obstruction-strands-v0.md |
| OBSTRAND-REQ-005 | planned | Future Core/Target IR exposes a distinct preserved-obstruction disposition. | issue #116, docs/design/obstruction-strands-v0.md |
| OBSTRAND-REQ-006 | planned | Future preserved obstruction records no success-path write and carries repair metadata only as causal support. | issue #116 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/topics/obstruction-strands/README.md | Current boundary shelf. | Human review confirms HEAD claims parser support only for current `require` obstruction syntax. |
| docs/design/obstruction-strands-v0.md | Future design note. | Human review confirms planned syntax/lowering material is outside the topic README contract. |
| docs/topics/syntax/test-plan.md | Syntax ledger for future parser coverage. | `SYNTAX-REQ-013` and `SYNTAX-TP-023` preserve the planned parser guard. |
| docs/topics/target-ir/README.md | Target IR deferred-boundary note. | Target IR explicitly does not yet claim first-class resumable obstruction strands. |
| fixtures/obstruction-strands/v0/stale-basis/README.md | Golden corridor manifest. | Human review confirms future artifacts appear only after the owning layer lands. |
| fixtures/obstruction-strands/v0/stale-basis/source.edict | Stale-basis source corridor fixture. | Parsed by `stale_basis_obstruction_strand_fixture_parses`. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| OBSTRAND-TP-001 | implemented | Current parser contract | OBSTRAND-REQ-001 | Current `require ... else` parses as `Stmt::Require` with a predicate and obstruction target. | require_statement_parses_terminal_obstruction_source_shape | crates/edict-syntax/tests/parse_review_regressions.rs | Parser evidence only; compiler lowering for `Stmt::Require` remains outside the landed boundary. |
| OBSTRAND-TP-002 | implemented | Syntax guard | OBSTRAND-REQ-002 | Parser introduces a distinct source AST arm for first-class preserved obstruction syntax. | continue_obstructed_source_arm_parses, stale_basis_obstruction_strand_fixture_parses | crates/edict-syntax/tests/parse_review_regressions.rs, fixtures/obstruction-strands/v0/stale-basis/source.edict | Parser evidence only. |
| OBSTRAND-TP-003 | implemented | Source validation | OBSTRAND-REQ-003 | `continue obstructed { ... }` requires a `reason` field and rejects duplicate `reason` fields. | continue_obstructed_requires_reason_field, continue_obstructed_rejects_duplicate_reason_field | crates/edict-syntax/tests/parse_review_regressions.rs | Rejection happens before Core digesting. |
| OBSTRAND-TP-004 | implemented | Contextual syntax guard | OBSTRAND-REQ-003 | `continue obstructed { ... }` is not a normal expression outside a require-else arm. | continue_obstructed_is_contextual_to_require_else | crates/edict-syntax/tests/parse_review_regressions.rs | Prevents function-like or value-like use. |
| OBSTRAND-TP-005 | implemented | Negative guard | OBSTRAND-REQ-004 | `else continueInObstructedStrand(...)` is not interpreted as hidden control flow. | helper_shaped_continue_in_obstructed_strand_is_terminal | crates/edict-syntax/tests/parse_review_regressions.rs | It remains an ordinary terminal obstruction target. |
| OBSTRAND-TP-006 | planned | Future Target IR guard | OBSTRAND-REQ-005, OBSTRAND-REQ-006 | Lowering emits a distinct preserved-obstruction disposition with no success-path write. | - | - | Must remain separate from terminal obstruction and success. |

## Determinism Obligations

- Future tests must assert structured AST/Core/Target IR values, not diagnostic
  prose.
- Future fixtures must be checked in under `fixtures/` or explicit topic test
  sources.
- No future test may depend on wall-clock time, runtime execution, network
  state, or live Continuum/XYPH services.

## Open Gaps

- No Core model exists for preserved obstruction disposition.
- No Target IR model exists for preserved obstruction disposition.
- No Core lowering exists for `Stmt::Require`.
- No Echo/Jim receipt fixture exists for preserved repairable obstruction.
