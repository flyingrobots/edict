# Obstruction Strands

Status: planned topic shelf.

This topic records the Edict side of Supported Outcome Settlement. Edict's
current `require ... else <obstruction>` form is terminal: when the predicate is
false, the success path stops and the target returns a typed obstruction with no
visible success-path write.

Future Edict may add first-class resumable obstruction syntax. That must be a
language/runtime contract, not an ordinary function call hidden inside `else`.

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

## Planned Contract

A future first-class form could look like:

```edict
require jim.basisFresh(input.basis)
  else continue obstructed jim.EditObstruction.StaleBase {
    providedBasis: input.basis,
    draftDigest: hash("Draft", input.content),
    repair: jim.RepairHint.RebaseRequired,
  };
```

Meaning:

```text
The success path cannot proceed.
No write occurs.
Preserve the blocked attempt as repairable causal material.
```

This should lower differently from terminal obstruction. Target IR must expose
the disposition so Echo/Jim can record an obstruction witness, Continuum can
transport the support, and XYPH can decide whether the supported native
consequence is blocked, repairable, diagnostic completion, or something else.

## Invariants

- Terminal obstruction and resumable obstruction are different semantics.
- Obstructed-strand preservation must be first-class in source, Core, Target IR,
  and participant receipts.
- A fake helper such as `continueInObstructedStrand(...)` must not be accepted as
  ordinary expression evaluation with hidden control-flow meaning.
- Any preserved obstruction attempt must disclose that no success-path write
  occurred.
- Proof-carrying execution is not a prerequisite for the syntax; support tier is
  a consuming-authority policy.

## Related Work

- Syntax should remain terminal until a dedicated grammar and lowering contract
  exist.
- Target IR needs an explicit disposition for `continue_obstructed_strand`.
- Echo/Jim receipts need to distinguish success, terminal obstruction, and
  preserved repairable obstruction.
- XYPH support obligations decide what an obstruction means for a Quest.
