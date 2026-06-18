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

A prose ABI is not yet an ABI. It becomes an ABI when two independent
implementations can exchange bytes and agree. This specification is therefore
split into three layers, and **all three are normative**:

1. The canonical manifest schema, defined once in
   [`abi/edict-target-profile.cddl`](./abi/edict-target-profile.cddl).
2. The executable plugin boundary, defined once in
   [`abi/edict-target-lowerer.wit`](./abi/edict-target-lowerer.wit).
3. The exchange types and doctrine in this document.

Every JSON block below is **generated from the CDDL schema and is illustrative
only**. The schema is the single source of truth; duplicate normative JSON is
forbidden (`EDICT-ABI-NODUP-001`).

## Profile Manifest

A target profile manifest is hash-significant data conforming to
[`abi/edict-target-profile.cddl`](./abi/edict-target-profile.cddl). It references
each normative subcomponent by identity plus digest (`resource-ref`). It must not
embed its own self-digest in its preimage (`EDICT-CORE-SELFHASH-001`). The
illustrative shape:

```json
{
  "apiVersion": "edict.target-profile/v1",
  "id": "echo.dpo",
  "version": "1",
  "acceptedCoreAbi": ["edict.core/v1"],
  "intrinsicNamespace": "echo.dpo@1",
  "intrinsics": { "id": "echo.dpo.intrinsics/v1", "digest": "sha256:..." },
  "footprintAlgebra": { "id": "echo.dpo.footprint/v1", "digest": "sha256:..." },
  "costAlgebra": { "id": "echo.dpo.cost/v1", "digest": "sha256:..." },
  "targetIr": { "id": "echo.span-ir/v1", "digest": "sha256:..." },
  "obstructionTaxonomy": {
    "id": "echo.dpo.obstructions/v1", "digest": "sha256:..."
  },
  "verifier": { "id": "echo.dpo.verifier/v1", "digest": "sha256:..." },
  "lowerer": { "id": "echo.dpo.lowerer/v1", "digest": "sha256:..." },
  "sandbox": { "id": "edict.wasm-component/v1", "digest": "sha256:..." },
  "fuelModel": { "id": "edict.fuel/v1", "digest": "sha256:..." },

  "bundleProfile": { "id": "echo.dpo.bundle/v1", "digest": "sha256:..." },
  "generatedArtifactProfiles": [
    { "id": "echo.dpo.registration/v1", "digest": "sha256:..." }
  ],
  "canonicalEncodingRules": {
    "id": "edict.canonical-cbor/v1", "digest": "sha256:..."
  },
  "acceptedLawpackAdapterAbi": [],
  "diagnosticAbi": { "id": "edict.diagnostics/v1", "digest": "sha256:..." },

  "applicationModel": "atomic",
  "readConsistency": "application-snapshot",
  "guardEvaluation": "precommit-atomic",
  "obstructionRollback": "no-visible-effects",
  "multiTarget": false,
  "postconditionSupport": true,
  "deterministicExecution": {
    "id": "edict.determinism/v1", "digest": "sha256:..."
  },
  "conformanceFixtureCorpus": {
    "id": "echo.dpo.fixtures/v1", "digest": "sha256:..."
  }
}
```

The manifest carries every field the Language spec requires of a profile,
including `bundleProfile`, `generatedArtifactProfiles`, `canonicalEncodingRules`,
and `diagnosticAbi`. This is the **single** authoritative manifest; the Language
spec must not duplicate it (`EDICT-ABI-NODUP-001`).

`acceptedLawpackAdapterAbi` is **reserved and deferred**: it will list accepted
lawpack-adapter ABI ids once the byte-level `edict.lawpack-adapter/v1` ABI is
specified. Until that schema exists it is optional and empty, and a target must
not be expected to validate adapter compatibility from it
(`EDICT-ABI-LAWPACK-ADAPTER-DEFER-001`). Lawpack adapters are still digest-locked
resource references in the lawpack manifest; only the cross-ABI compatibility
field is deferred.

Display metadata is not part of this manifest. Human-facing names, codenames,
and marketing copy live in sidecar documents keyed by the target profile digest.

## Exchange Types And Plugin Boundary

The component boundary is defined in
[`abi/edict-target-lowerer.wit`](./abi/edict-target-lowerer.wit). It pins the
byte-level exchange so two independent lowerers/verifiers can interoperate:

```text
Core IR (canonical-cbor)        --> lowerer.lower             --> Target IR
target intrinsic coordinate     --> lowerer.effect-signature  --> signature
computed footprint + declared ceiling --> lowerer.footprint-compare --> ok | rejected
computed cost + declared target ceiling --> lowerer.cost-compare    --> ok | rejected
effect node + guard             --> lowerer.attach-guard      --> node'
Target IR (canonical-cbor)      --> verifier.verify           --> verifier report
any stage                       --> diagnostic[]              --> Watson input
```

Each artifact crossing the boundary is `{ domain, bytes }` where `bytes` is
`edict.canonical-cbor/v1` of the named artifact, so digests remain stable across
hosts. Diagnostics are structured (`code`, `severity`, `message`, optional
`repair`, optional non-hash `span`); they are never hashed into semantic
artifacts. The verifier is evidence, not authority: it never mutates runtime
state and never overrides participant admission (I-014).

The lowerer compares cost and footprint against the operation's **declared**
target ceiling only. It never receives a participant-admitted budget; admission
is external by design, so target IR and verifier artifacts stay
participant-neutral bundle inputs (`EDICT-TARGET-NEUTRAL-LOWERING-001`).

## Application Model

A lawful-autonomous v1 operation lowers to one target-owned atomic application
unit.

Target profiles must declare:

- read consistency model;
- precommit guard evaluation model;
- `postconditionSupport`: whether precommit postcondition (`guarantee`) checks
  can be evaluated inside the atomic application unit;
- obstruction rollback behavior;
- visibility rules for successful writes;
- resource failure behavior;
- whether the profile is single-target or a composite target profile.

A precommit `guarantee` lowers only to a profile whose `postconditionSupport` is
true. If an operation requires a precommit postcondition and the selected target
profile does not support it, lowering fails with an unsupported-lowering
compiler error (`EDICT-TARGET-POSTCOND-001`); it is never silently dropped or
downgraded to a non-atomic check.

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

Each target intrinsic declares an `intrinsicClass` of `pure` or `effect`. Pure
symbolic constructors (`echo.ref<T>(id)`, `echo.edge<...>(...)`, id constructors,
plan constructors) are `pure`: they produce inert, canonical, hashable plan
terms and carry no effect kind. Only an `effect` intrinsic carries an effect
kind and failure classes (`EDICT-TARGET-INTRINSIC-CLASS-001`).

Each target intrinsic must declare:

- canonical coordinate;
- `intrinsicClass`: `pure` or `effect`;
- type parameters;
- argument types;
- return type;
- guard support;
- footprint template;
- cost template;
- `writeClass`: `read`, `create`, `ensure`, `append`, `replace`, `delete`,
  `none` (for pure constructors), or `custom`.

An `effect` intrinsic additionally declares:

- effect kind;
- `effectFailures`: the **named, typed** low-level failures it can raise. Each
  has a `coordinate` (e.g. `mismatch`, `boundExceeded`), an `authorityClass`
  (`domainMappable`/`participantOwned`/...), and a bounded `payloadType`
  (`EDICT-ABI-FAILURE-NAMED-001`);
- whether the effect can participate in a target-atomic guard.

Broad authority classes alone are insufficient: a source obstruction arm
`mismatch(fault) => Domain.X(...)` binds the named failure `mismatch` declared
here and constructs its typed domain obstruction from `fault`'s payload. An arm
naming a failure the effect does not declare is a compile error.

A `pure` intrinsic must declare `effectKind` absent and `writeClass: none`. A
pure constructor that reaches runtime state is a relapse and rejects.

## Operation Mode Predicates

Operation mode is a verifier predicate over inferred effects:

- `readOnly`: effects that are **either** proof-only semantic facts **or** have
  authoritative `writeClass` `read` (including runtime semantic reads); no
  mutating `writeClass`.
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

Each effect declares **named** low-level failures (`effectFailures`), and each
named failure carries an `authorityClass`. The authority class governs whether
source may map the failure:

- `domainMappable`: may be mapped by source to a typed domain obstruction.
- `participantOwned`: remains a participant or admission obstruction.
- `integrityFault`: remains an integrity failure.
- `resourceFault`: remains a resource or budget obstruction.
- `internalFault`: remains a platform/internal failure.

The obstruction map is keyed by **failure coordinate**, not by authority class,
so two `domainMappable` failures on the same effect (e.g. `mismatch` and
`boundExceeded`) are distinct, separately-mapped arms.

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
  "obstruction": {
    "coordinate": "jedit.rope@1.StaleBaseHead",
    "failureBinder": null,
    "payload": {
      "expected": { "local": "%baseHead.headId" },
      "observed": { "field": "current.canonicalHeadId" }
    }
  }
}
```

`localPrecheck` is never authoritative for mutable target state. It may be used
for diagnostics or optimization, but target-owned precommit guard evaluation is
the enforcement boundary for mutable state assumptions.

## Conformance

The profile digest pins the normative fixture corpus. Third-party conformance
execution results are external attestations that reference the profile digest.
Adding a new independent test lab must not change the target profile identity.
