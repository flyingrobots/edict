---
title: "SPEC - Edict Target Profile ABI v1"
legend: "SPEC|TRANSMUTE|PLATFORM"
lane: "design"
packet: "0021-continuum-yolo-runtime-neutral-edict-sha-lock-assurance"
issue: "https://github.com/flyingrobots/wesley/issues/611"
status: "draft"
owners:
  - "@flyingrobots"
created: "2026-06-18"
updated: "2026-06-18"
---

<!-- markdownlint-disable MD025 -->

# SPEC - Edict Target Profile ABI v1

<!-- markdownlint-enable MD025 -->

## Purpose

This specification defines the runtime target profile ABI used by Edict Core
lowering. Edict Language v1 owns source syntax and Core IR. A target profile owns
runtime intrinsics, target IR, footprint algebra, cost algebra, obstruction
taxonomy, application semantics, lowering, and verification.

## Profile Manifest

A target profile manifest is hash-significant data. It must either embed each
normative subcomponent definition or reference it by identity plus digest:

```json
{
  "apiVersion": "edict.target-profile/v1",
  "id": "echo.dpo",
  "version": "1",
  "acceptedCoreAbi": ["edict.core/v1"],
  "intrinsics": {
    "id": "echo.dpo.intrinsics/v1",
    "digest": "sha256:..."
  },
  "footprintAlgebra": {
    "id": "echo.dpo.footprint/v1",
    "digest": "sha256:..."
  },
  "costAlgebra": {
    "id": "echo.dpo.cost/v1",
    "digest": "sha256:..."
  },
  "targetIr": {
    "id": "echo.span-ir/v1",
    "digest": "sha256:..."
  },
  "obstructionTaxonomy": {
    "id": "echo.dpo.obstructions/v1",
    "digest": "sha256:..."
  },
  "verifier": {
    "id": "echo.dpo.verifier/v1",
    "digest": "sha256:..."
  },
  "lowerer": {
    "id": "echo.dpo.lowerer/v1",
    "digest": "sha256:..."
  },
  "sandbox": {
    "id": "edict.wasm-component/v1",
    "digest": "sha256:..."
  },
  "fuelModel": {
    "id": "edict.fuel/v1",
    "digest": "sha256:..."
  },
  "applicationModel": "atomic",
  "readConsistency": "application-snapshot",
  "guardEvaluation": "precommit-atomic",
  "obstructionRollback": "no-visible-effects",
  "multiTarget": false,
  "conformanceFixtureCorpus": {
    "id": "echo.dpo.fixtures/v1",
    "digest": "sha256:..."
  }
}
```

Display metadata is not part of this manifest. Human-facing names, codenames,
and marketing copy live in sidecar documents keyed by the target profile digest.

## Application Model

A lawful-autonomous v1 operation lowers to one target-owned atomic application
unit.

Target profiles must declare:

- read consistency model;
- precommit guard evaluation model;
- obstruction rollback behavior;
- visibility rules for successful writes;
- resource failure behavior;
- whether the profile is single-target or a composite target profile.

The v1 default application model is:

```json
{
  "applicationModel": "atomic",
  "readConsistency": "application-snapshot",
  "guardEvaluation": "precommit-atomic",
  "obstructionRollback": "no-visible-effects",
  "multiTarget": false
}
```

All runtime reads observe one target-defined application snapshot. Mutable
requirements are checked in the same application unit. Writes become visible
atomically. Any obstruction, resource failure, or failed guarantee leaves
externally visible target state unchanged.

## Runtime Effect Domain

An Edict intent may lower to effects owned by at most one runtime target profile.
Lawpack semantic effects may lower into that same target. Cross-target
application requires a composite target profile that explicitly owns
coordination, obstruction classification, and atomicity semantics.

## Intrinsic Signatures

Each target intrinsic must declare:

- canonical coordinate;
- type parameters;
- argument types;
- return type;
- effect kind;
- failure classes;
- guard support;
- footprint template;
- cost template;
- whether the effect is read-only, create-only, append-only, replace-capable, or
  custom;
- whether the effect can participate in a target-atomic guard.

## Operation Mode Predicates

Operation mode is a verifier predicate over inferred effects:

- `readOnly`: read effects and proof-only semantic effects only.
- `createOnly`: read, create, and ensure effects; no replace, delete, append, or
  runtime-materialized semantic writes except effects explicitly classified as
  create by the resolved target profile.
- `append`: read and append effects; no replace or delete.
- `replace`: read, create, ensure, and replace effects as profile-defined; no
  delete unless the profile explicitly classifies the operation mode as custom.
- `custom`: target-profile-defined predicate with a digest-locked verifier rule.

`ensure` counts as a profile-declared effect. A target profile must state whether
it behaves as read plus conditional create, create-if-absent plus equality guard,
or a more restrictive target-native primitive.

## Obstruction Taxonomy

Effect failure classes are split by authority:

- `domainMappable`: may be mapped by source to a domain obstruction.
- `participantOwned`: remains a participant or admission obstruction.
- `integrityFault`: remains an integrity failure.
- `resourceFault`: remains a resource or budget obstruction.
- `internalFault`: remains a platform/internal failure.

Source `else` mappings may translate only profile-declared `domainMappable`
classes. Authors must not translate permission failure, signature failure,
verifier defects, participant policy rejection, or internal target failure into a
domain obstruction.

Obstructions must have typed payload schemas. For example, stale-basis
obstructions should be able to carry expected and observed basis values when the
target profile supports them.

## Guard Semantics

Target guards are attached to effect nodes or target application units:

```json
{
  "predicate": {
    "op": "==",
    "left": { "field": "current.canonicalHeadId" },
    "right": { "local": "%baseHead.headId" }
  },
  "enforcement": "targetAtomic",
  "obstruction": "jedit.rope@1.StaleBaseHead"
}
```

`localPrecheck` is never authoritative for mutable target state. It may be used
for diagnostics or optimization, but target-owned precommit guard evaluation is
the enforcement boundary for mutable state assumptions.

## Conformance

The profile digest pins the normative fixture corpus. Third-party conformance
execution results are external attestations that reference the profile digest.
Adding a new independent test lab must not change the target profile identity.
