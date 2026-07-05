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

This note now formalizes the broader cross-project taxonomy after the source,
Core, Target IR, and Echo receipt-bridge slices have landed. The taxonomy is a
language and evidence boundary. It does not implement runtime scheduling,
counterfactual exploration, editor projection, Continuum settlement, or XYPH
settlement.

## Outcome Taxonomy

The obstruction-strand corridor uses three outcome families. They must not be
collapsed into a generic failure bucket.

### Not-Admitted Scheduler Counterfactual

A not-admitted scheduler counterfactual is a candidate that was available to a
scheduler or runtime-selection surface but did not run. The expected early
reason is a footprint conflict or scheduler choice, but the important property
is prior to execution:

```text
not admitted = did not run
```

The scheduler/runtime authority owns whether a candidate was considered,
selected, or left unselected. Edict source syntax, Core lowering, and Target IR
artifact emission do not create not-admitted counterfactuals. Graft and jedit
may display such records only after an authority emits them.

### Admitted Obstructed Strand

An admitted obstructed strand is an accepted/evaluated attempt whose guard or
runtime condition continued into an obstruction outcome instead of producing a
success-path write. In the first corridor, the Edict source/Core/Target IR
contract makes the obstruction disposition explicit, and Echo can emit an
in-memory receipt that binds the Target IR digest.

```text
obstructed strand = ran into an obstruction outcome
```

The obstruction record is support for a blocked or repairable attempt. It is
not a success, not a hidden retry, and not a scheduler counterfactual. The
receipt authority owns the fact that an accepted artifact was evaluated and
obstructed.

### Hard Rejection

A hard rejection is a refusal before the candidate can be treated as an admitted
execution attempt. Examples include malformed artifacts, unsupported target
profiles, invalid digests, validation failures, unsupported runtime features,
and runtime refusal before execution.

```text
hard rejection = refused
```

Hard rejection is not an obstructed strand, even when both mention the same
domain reason. It is also not a not-admitted scheduler counterfactual, because
the candidate never reached scheduler selection as a valid execution candidate.

### Authority Split

| Term | Short definition | Owning authority |
| --- | --- | --- |
| Not-admitted scheduler counterfactual | Candidate did not run after scheduler/runtime selection. | Scheduler/runtime receipt authority |
| Admitted obstructed strand | Accepted/evaluated attempt continued into obstruction outcome. | Runtime receipt authority, grounded in Edict-produced artifact semantics |
| Hard rejection | Artifact, input, profile, validation, or runtime refused before admitted execution. | Validator, target/runtime acceptor, or receipt authority |

The source, Core, Target IR, receipt, projection, and editor layers may carry
evidence for these terms, but they must not claim another layer's authority.
Target IR emission is not Echo acceptance. Echo acceptance is not execution.
Execution receipt display is not semantic interpretation by the editor.

The future form must lower differently from terminal obstruction. Target IR
needs an explicit disposition so Echo/Jim can record an obstruction witness,
Continuum can transport the support, and XYPH can decide whether the supported
native consequence is blocked, repairable, diagnostic completion, or something
else.

The Target IR v0 shape models source/Core `require` guards as explicit
requirements on the intent, separate from target effect steps:

```text
TargetIrIntent {
  requirements: [
    {
      id,
      predicate,
      onFailure:
        | terminal { reason }
        | continueObstructed { reason }
    }
  ],
  steps: [...]
}
```

`steps` remain target effect operations. `requirements` are target-owned
precondition or guard obligations. This keeps hard rejection and preserved
obstruction digest-distinct without pretending that an obstructed strand is a
success-path write.

Intent-level Target IR requirements are pre-step guards. A source/Core `require`
after an emitted target step, including a guard that depends on a prior effect
result, needs an ordered or step-attached Target IR shape before it can be
represented honestly.

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

Already verified in the Core slice for issue #129:

- terminal `require ... else <obstruction>` lowers to
  `CoreRequireFailureArm::Terminal`;
  `terminal_require_obstruction_lowers_to_core_failure_arm`;
- `require ... else continue obstructed { ... }` lowers to
  `CoreRequireFailureArm::ContinueObstructed`;
  `continue_obstructed_require_lowers_to_core_failure_arm`;
- terminal and preserved-obstruction require arms remain Core-distinct and
  Core-digest-distinct:
  `terminal_and_continue_obstructed_require_arms_are_core_distinct`;
- reason kind and payload value changes move the Core digest, while payload
  field order and non-semantic formatting do not:
  `obstruction_reason_mutations_move_core_digest`;
- duplicate reason payload fields reject before Core digesting:
  `duplicate_obstruction_reason_payload_fields_reject_before_core_digest`.

Already verified in the Target IR slice for issue #131:

- Echo Target IR carries `require` guards as explicit requirements with
  `terminal` and `continueObstructed` dispositions:
  `echo_target_ir_contains_obstruction_requirement_payload` and
  `terminal_and_preserved_requirements_are_target_ir_distinct`;
- requirement predicate, reason kind, reason payload value, and disposition
  mutations move the Target IR digest:
  `target_ir_requirement_mutations_move_digest`;
- targets without requirement support reject with a stable target-feature
  failure before artifact emission:
  `targets_without_obstruction_requirement_support_reject_with_stable_feature_kind`;
- requirements after emitted target steps reject with a stable target-feature
  failure before artifact emission, with a more specific detail when they read a
  step output:
  `requirement_after_target_step_rejects_with_stable_feature_kind` and
  `requirement_that_reads_step_output_rejects_with_stable_feature_kind`.

Already verified in the Echo receipt bridge for flyingrobots/echo#641:

- Echo accepts the Edict Echo Target IR obstruction fixture shape as an
  acceptance phase separate from execution;
- invalid domain, digest, requirement-count, requirement-disposition, and
  requirement-predicate shapes reject with stable acceptance failures before an
  execution receipt exists;
- stale-basis execution produces an obstructed attempt receipt that binds the
  Target IR digest, while a fresh basis produces a committed-success receipt.

Future implementation work should add RED/GREEN evidence for:

- receipt fixtures proving success, terminal obstruction, and preserved
  repairable obstruction are distinct runtime outcomes.

Invalid Target IR fixtures should be derived by mutation tooling from valid
Edict-produced artifacts. The compiler must not produce malformed Target IR as a
normal output path.
