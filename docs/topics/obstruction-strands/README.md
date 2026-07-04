# Obstruction Strands

Status: current HEAD boundary. No first-class obstruction-strand syntax is
implemented in HEAD.

This topic records the current Edict boundary for obstruction-strand planning.
Edict's current `require ... else <obstruction>` form is terminal: when the
predicate is false, the success path stops and the target returns a typed
obstruction with no visible success-path write.

No source syntax, Core model, Target IR disposition, participant receipt, or
runtime behavior exists yet for preserving a blocked attempt as a repairable
obstruction strand. Future design work is tracked in issue #116 and the
non-topic design note
[`docs/design/obstruction-strands-v0.md`](../../design/obstruction-strands-v0.md).
That document is planning material, not a topic README contract for landed
behavior.

## Current Contract

Current syntax:

```edict
require jim.basisFresh(input.basis)
  else jim.EditObstruction.StaleBase;
```

Meaning:

```text
The success path cannot proceed.
No write occurs.
Return a typed domain obstruction.
```

## Invariants

- Terminal obstruction and resumable obstruction are different semantics.
- Resumable obstruction is not currently implemented.
- Current `else <obstruction>` parses as an obstruction constructor, not as a
  hidden continuation or recovery workflow.
- A helper-like obstruction constructor such as
  `continueInObstructedStrand(...)` must not acquire hidden control-flow
  semantics without a first-class language/runtime contract.

## Related Work

- Syntax should remain terminal until a dedicated grammar and lowering contract
  exist.
- Planned parser, Core, Target IR, and receipt work is tracked in
  [test-plan.md](./test-plan.md).
- Future design material lives in
  [`docs/design/obstruction-strands-v0.md`](../../design/obstruction-strands-v0.md).
