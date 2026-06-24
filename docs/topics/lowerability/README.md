# Lowerability Topic

Status: current HEAD contract.

This chapter describes the v1 lowerability contract that exists today. It asks
whether a typed lowering requirements artifact can be discharged by explicit
target-profile facts. It does not lower Core to Target IR and it does not make
an admission claim.

## Public Surface

The `edict_syntax` crate exposes `check_lowerability` and typed lowerability
data structures for:

- `LoweringRequirements`, the typed pre-lowering question;
- `TargetProfileFacts`, explicit target-profile support facts;
- `LowerabilityReport`, including `Native`, `Adapted`, and `Unsupported`
  classifications;
- stable `LowerabilityFailureKind` categories. [LOWER-REQ-001]
  [LOWER-REQ-002]

The canonical artifact shape for `edict.lowering-requirements/v1` is named in
[`docs/abi/edict-target-profile.cddl`](../../abi/edict-target-profile.cddl).
[LOWER-REQ-001]

## Current Contract

- `LoweringRequirements` records the required operation profile, semantic
  effects, write classes, guard kinds, atomicity, postcondition support,
  obstruction coordinates, footprint obligations, cost obligations, and optic
  contract. [LOWER-REQ-001]
- A target is `Native` only when explicit target-profile facts directly support
  every requirement and every semantic effect. [LOWER-REQ-002]
- A target is `Adapted` when every non-native semantic effect is discharged by
  exactly one digest-locked direct lawpack adapter and every other requirement
  is supported. [LOWER-REQ-003]
- A target is `Unsupported` when required profile facts, effect support, write
  classes, guards, atomicity, postcondition support, obstruction coordinates,
  footprint obligations, cost obligations, or optic contract facts are missing.
  [LOWER-REQ-004]
- Edict v1 rejects undigested adapter references, adapter chains, and ambiguous
  adapter choices. General composite / chained adapter legalization belongs to
  future v2 design work. [LOWER-REQ-005]
- Lowerability checks stop before Target IR, verifier reports, contract bundles,
  and admission receipts. [LOWER-REQ-006]

## Deferred

The following are not implemented by this lowerability slice:

- CLI commands such as `edict explain lowerability`;
- canonical-CBOR encode/decode helpers for `LoweringRequirements`;
- target-profile manifest loading from files;
- Target IR generation;
- verifier reports;
- bundle/admission tooling;
- v2 adapter-composition search.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
