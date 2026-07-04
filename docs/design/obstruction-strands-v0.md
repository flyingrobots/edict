# Obstruction Strands Design Note

Status: staged design note for issue #116. This is not a topic shelf contract
for landed behavior. Each implementation PR updates the owning topic shelf only
after executable evidence exists.

This note records the planned design direction for preserving repairable
obstruction attempts. The current executable contract remains in the syntax,
compiler-spine, Target IR, and obstruction-strands topic shelves.

## Problem

Current Edict `require ... else <obstruction>` syntax parses as a source
statement with a predicate and obstruction target. The planned terminal meaning
is that, if the predicate is false, the success path stops and returns a typed
domain obstruction. That remains the correct default for optimistic concurrency
checks such as stale bases: the requested write must not happen.

Some runtimes and editor workflows also need to preserve the blocked attempt as
repairable causal material. A stale-base edit, for example, may be useful as a
draft to rebase, compare, preserve in a conflict lane, or inspect in a
time-travel/debugging surface.

That preservation must be an explicit language/runtime contract. It must not be
hidden inside an ordinary obstruction constructor or helper-shaped expression.

## Source Shape

The selected first-class source form is:

```edict
require jim.basisFresh(input.basis)
  else continue obstructed {
    reason: jim.EditObstruction.StaleBase,
    providedBasis: input.basis,
    draftDigest: hash("Draft", input.content),
    repair: jim.RepairHint.RebaseRequired,
  };
```

For the first parser slice, the required field is `reason`. Additional fields
are source payload entries carried for later semantic work. Duplicate `reason`
fields are rejected before Core digesting.

`continue obstructed { ... }` is contextual syntax recognized only as a
`require ... else` arm. It is not a normal expression. It cannot be imported,
shadowed, called, assigned to a value, or used outside the require-else arm.

Legacy `require predicate else SomeObstruction;` remains terminal obstruction
syntax. A helper-shaped obstruction constructor such as
`continueInObstructedStrand({ reason: ... })` remains an ordinary obstruction
target and does not gain hidden continuation semantics.

The important property is that the parser, Core, Target IR, and runtime can
distinguish terminal obstruction from repairable obstruction preservation.

## Implementation Ladder

The implementation is deliberately layered:

```text
source syntax
  -> Core meaning
    -> Target IR lowering
      -> Echo acceptance/execution receipt
        -> projection/editor display
```

Each PR may claim only its own layer.

| Layer | First artifact | Authority |
| --- | --- | --- |
| Source syntax | PR 1 | Edict parser |
| Core meaning | PR 2 | Edict Core |
| Target IR lowering | PR 3 | Edict target lowerer |
| Echo receipt | PR 4 | Echo runtime |
| Projection display | PR 5B/5C | Graft/jedit display only |

PR 1 parses the source form and preserves it in the source AST. It does not
claim Core meaning, Target IR lowering, Echo acceptance, Echo execution, or
editor projection behavior.

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

Minimal terminology used before implementation:

```text
not admitted      = did not run, usually scheduler conflict
obstructed strand = admitted/evaluated and continued into obstruction outcome
hard rejection    = artifact, input, profile, validation, or runtime refused
```

This note defines only the minimal taxonomy needed to prevent term collapse.
The broader cross-project taxonomy should be formalized after Echo receipt
artifacts exist.

The future form must lower differently from terminal obstruction. Target IR
needs an explicit disposition so Echo/Jim can record an obstruction witness,
Continuum can transport the support, and XYPH can decide whether the supported
native consequence is blocked, repairable, diagnostic completion, or something
else.

The v0 reason envelope should use a stable reason kind plus an opaque canonical
payload value. The outer envelope is closed; the payload policy can evolve only
through explicit versioned semantics.

Receipts must bind the Target IR digest. Receipt concepts must distinguish
input basis facts from observed/current basis facts, for example
`input_basis_digest` and `observed_basis_digest`. A single ambiguous
`basis_digest` is not enough for stale-basis evidence.

Echo receipt determinism must exclude wall-clock timestamps, random IDs, host
paths, pointer addresses, map iteration order, and environment-dependent values
unless they are explicitly supplied as deterministic test inputs.

## Invariants

- Terminal obstruction and resumable obstruction are different semantics.
- Obstructed-strand preservation must be first-class in source, Core, Target IR,
  and participant receipts before it can carry control-flow meaning.
- A helper such as `continueInObstructedStrand(...)` must not be accepted as
  ordinary expression evaluation with hidden control-flow meaning.
- Any preserved obstruction attempt must disclose that no success-path write
  occurred.
- Source spans and comments may appear in diagnostics or review metadata, but
  they must not affect semantic Core or Target IR digests unless explicitly
  marked semantic.
- Proof-carrying execution is not a prerequisite for the syntax; support tier is
  a consuming-authority policy.
- No canonical Echo receipt digest may be claimed until canonical Echo receipt
  bytes exist.

## Verification

Already verified in PR #128:

- a distinct source AST node for repairable obstruction preservation;
  `continue_obstructed_source_arm_parses` and
  `stale_basis_obstruction_strand_fixture_parses`;
- a negative guard proving helper-shaped obstruction constructors do not gain
  hidden control-flow semantics:
  `helper_shaped_continue_in_obstructed_strand_is_terminal`;
- parser rejection for missing or duplicate `reason` fields;
  `continue_obstructed_requires_reason_field` and
  `continue_obstructed_rejects_duplicate_reason_field`;
- parser rejection when `continue obstructed { ... }` appears outside a
  require-else arm: `continue_obstructed_is_contextual_to_require_else`.

Future implementation work should add RED/GREEN evidence for:

- Core and Target IR fields that distinguish terminal obstruction from preserved
  obstruction;
- digest or mutation evidence if the new disposition becomes part of canonical
  Core or Target IR bytes;
- receipt fixtures proving success, terminal obstruction, and preserved
  repairable obstruction are distinct runtime outcomes.

Invalid Target IR fixtures should be derived by mutation tooling from valid
Edict-produced artifacts. The compiler must not produce malformed Target IR as a
normal output path.
