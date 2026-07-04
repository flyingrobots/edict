# Obstruction Strands Design Note

Status: future design note for issue #116. This is not a topic shelf contract
for landed behavior, and no first-class obstruction-strand syntax, Core model,
Target IR disposition, participant receipt, or runtime behavior is implemented
in HEAD.

This note records the planned design direction for preserving repairable
obstruction attempts. The current executable contract remains in the syntax,
compiler-spine, Target IR, and obstruction-strands topic shelves.

## Problem

Current Edict `require ... else <obstruction>` syntax is terminal. If the
predicate is false, the success path stops and returns a typed domain
obstruction. That is the correct default for optimistic concurrency checks such
as stale bases: the requested write did not happen.

Some runtimes and editor workflows also need to preserve the blocked attempt as
repairable causal material. A stale-base edit, for example, may be useful as a
draft to rebase, compare, preserve in a conflict lane, or inspect in a
time-travel/debugging surface.

That preservation must be an explicit language/runtime contract. It must not be
hidden inside an ordinary obstruction constructor or helper-shaped expression.

## Future Source Shape

A future first-class form could look like:

```edict
require jim.basisFresh(input.basis)
  else continue obstructed jim.EditObstruction.StaleBase {
    providedBasis: input.basis,
    draftDigest: hash("Draft", input.content),
    repair: jim.RepairHint.RebaseRequired,
  };
```

The exact syntax is not decided. The important property is that the parser,
Core, Target IR, and runtime can distinguish terminal obstruction from
repairable obstruction preservation.

## Planned Semantics

Terminal obstruction:

```text
The success path cannot proceed.
No write occurs.
Return a typed domain obstruction.
```

Repairable obstruction preservation:

```text
The success path cannot proceed.
No write occurs.
Preserve the blocked attempt as repairable causal material.
```

The future form must lower differently from terminal obstruction. Target IR
needs an explicit disposition so Echo/Jim can record an obstruction witness,
Continuum can transport the support, and XYPH can decide whether the supported
native consequence is blocked, repairable, diagnostic completion, or something
else.

## Invariants

- Terminal obstruction and resumable obstruction are different semantics.
- Obstructed-strand preservation must be first-class in source, Core, Target IR,
  and participant receipts before it can carry control-flow meaning.
- A helper such as `continueInObstructedStrand(...)` must not be accepted as
  ordinary expression evaluation with hidden control-flow meaning.
- Any preserved obstruction attempt must disclose that no success-path write
  occurred.
- Proof-carrying execution is not a prerequisite for the syntax; support tier is
  a consuming-authority policy.

## Future Verification

Future implementation work should add RED/GREEN evidence for:

- a distinct source AST node for repairable obstruction preservation;
- a negative guard proving helper-shaped obstruction constructors do not gain
  hidden control-flow semantics;
- Core and Target IR fields that distinguish terminal obstruction from preserved
  obstruction;
- digest or mutation evidence if the new disposition becomes part of canonical
  Core or Target IR bytes;
- receipt fixtures proving success, terminal obstruction, and preserved
  repairable obstruction are distinct runtime outcomes.
