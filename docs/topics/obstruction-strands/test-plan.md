# Obstruction Strands Test Plan

Status: planned verification design for future resumable obstruction syntax.

## Scope

In scope:

- current terminal `require ... else` obstruction semantics;
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
| OBSTRAND-REQ-001 | implemented | Current `require ... else <obstruction>` remains terminal obstruction syntax. | docs/topics/syntax/test-plan.md |
| OBSTRAND-REQ-002 | planned | A future resumable obstruction form uses first-class syntax, not an ordinary helper call hidden inside `else`. | issue #116 |
| OBSTRAND-REQ-003 | planned | Future Core/Target IR exposes a distinct preserved-obstruction disposition such as `continue_obstructed_strand`. | issue #116 |
| OBSTRAND-REQ-004 | planned | Future preserved obstruction records no success-path write and carries repair metadata only as causal support. | issue #116 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/topics/obstruction-strands/README.md | Planned semantics shelf. | Human review confirms terminal obstruction and preserved obstruction remain separate. |
| docs/topics/syntax/test-plan.md | Syntax ledger for future parser coverage. | `SYNTAX-REQ-013` and `SYNTAX-TP-023` preserve the planned parser guard. |
| docs/topics/target-ir/README.md | Target IR deferred-boundary note. | Target IR explicitly does not yet claim `continue_obstructed_strand`. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| OBSTRAND-TP-001 | implemented | Current contract | OBSTRAND-REQ-001 | Current `require ... else` parses as terminal obstruction syntax and Target IR preserves obstruction arms without a preserved-strand disposition. | read_greeting_parses, supported_effectful_core_lowers_to_echo_span_ir | docs/topics/syntax/test-plan.md, docs/topics/target-ir/test-plan.md | Existing parser/lowering tests cover current obstruction syntax. |
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
- No Target IR model exists for `continue_obstructed_strand`.
- No Echo/Jim receipt fixture exists for preserved repairable obstruction.
