# Obstruction Strands Test Plan

Status: current verification ledger for `require` obstruction source syntax and
planned verification design for future resumable obstruction semantics.

## Scope

In scope:

- current parser support for `require ... else` obstruction source shape;
- current compiler boundary that rejects `Stmt::Require` before lowering;
- future first-class syntax for preserved repairable obstruction attempts;
- future Core and Target IR disposition for obstruction preservation;
- negative guard against hidden control flow in ordinary obstruction-target
  constructors.

Out of scope:

- implementing the future syntax in this topic-only slice;
- Echo or Jim runtime execution;
- Continuum support-ledger verification;
- XYPH Quest settlement policy.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| OBSTRAND-REQ-001 | implemented | Current `require ... else <obstruction>` parses as `Stmt::Require` source syntax and does not imply hidden continuation semantics. | docs/topics/syntax/test-plan.md |
| OBSTRAND-REQ-002 | planned | A future resumable obstruction form uses first-class syntax, not an ordinary helper call hidden inside `else`. | issue #116, docs/design/obstruction-strands-v0.md |
| OBSTRAND-REQ-003 | planned | Future Core/Target IR exposes a distinct preserved-obstruction disposition. | issue #116, docs/design/obstruction-strands-v0.md |
| OBSTRAND-REQ-004 | planned | Future preserved obstruction records no success-path write and carries repair metadata only as causal support. | issue #116 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/topics/obstruction-strands/README.md | Current boundary shelf. | Human review confirms HEAD claims parser support only for current `require` obstruction syntax. |
| docs/design/obstruction-strands-v0.md | Future design note. | Human review confirms planned syntax/lowering material is outside the topic README contract. |
| docs/topics/syntax/test-plan.md | Syntax ledger for future parser coverage. | `SYNTAX-REQ-013` and `SYNTAX-TP-023` preserve the planned parser guard. |
| docs/topics/target-ir/README.md | Target IR deferred-boundary note. | Target IR explicitly does not yet claim first-class resumable obstruction strands. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| OBSTRAND-TP-001 | implemented | Current parser contract | OBSTRAND-REQ-001 | Current `require ... else` parses as `Stmt::Require` with a predicate and obstruction target. | require_statement_parses_terminal_obstruction_source_shape | crates/edict-syntax/tests/parse_review_regressions.rs | Parser evidence only; compiler lowering for `Stmt::Require` remains outside the landed boundary. |
| OBSTRAND-TP-002 | planned | Future syntax guard | OBSTRAND-REQ-002 | Parser introduces a distinct AST node for first-class preserved obstruction syntax. | - | - | Should be RED before grammar work lands. |
| OBSTRAND-TP-003 | planned | Future negative guard | OBSTRAND-REQ-002 | `else continueInObstructedStrand(...)` is not interpreted as hidden control flow. | - | - | It may remain an obstruction constructor or reject under the future grammar. |
| OBSTRAND-TP-004 | planned | Future Target IR guard | OBSTRAND-REQ-003, OBSTRAND-REQ-004 | Lowering emits a distinct preserved-obstruction disposition with no success-path write. | - | - | Must remain separate from terminal obstruction and success. |

## Determinism Obligations

- Future tests must assert structured AST/Core/Target IR values, not diagnostic
  prose.
- Future fixtures must be checked in under `fixtures/` or explicit topic test
  sources.
- No future test may depend on wall-clock time, runtime execution, network
  state, or live Continuum/XYPH services.

## Open Gaps

- No parser support exists for first-class preserved obstruction syntax.
- No Core model exists for preserved obstruction disposition.
- No Target IR model exists for preserved obstruction disposition.
- No Core lowering exists for `Stmt::Require`.
- No Echo/Jim receipt fixture exists for preserved repairable obstruction.
