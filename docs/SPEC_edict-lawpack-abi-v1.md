---
title: "SPEC - Edict Lawpack ABI v1"
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

# SPEC - Edict Lawpack ABI v1

<!-- markdownlint-enable MD025 -->

## Purpose

A lawpack is a digest-locked unit of portable, authority-free Edict semantics.
It exports pure helper functions, typed constants, semantic effects, typed
obstructions, and the target lowerings that interpret its semantic effects into
concrete target profiles. Portable semantic Edict depends entirely on lawpacks:
`history.entry.record`, `rope.replaceRangePlan`, and
`history.textHistorySnapshot.read` are lawpack exports, not Core built-ins
(see [SPEC - Edict Language v1](./SPEC_edict-language-v1.md) I-017, I-021).

This specification owns the lawpack manifest, the export surface, the pure
versus effect distinction, the proof-only versus runtime-materialized
classification, obstruction payload schemas, footprint and cost obligations,
target-adapter requirements, and the conformance posture. Without it, every
lawpack import is a well-dressed mystery.

This ABI is data plus digest-locked executable components. The manifest is the
contract; sandboxed lowerer/verifier/helper components satisfy it. Like the
[Target Profile ABI](./SPEC_edict-target-profile-abi-v1.md), the canonical
manifest is defined once as a machine schema (`abi/edict-lawpack.cddl`) and all
prose JSON in this document is generated from that schema. Duplicate normative
JSON is forbidden (see `EDICT-ABI-NODUP-001`).

## Pure Versus Effect Exports

The single most important lawpack rule: a lawpack must declare, per export,
whether it is a pure function or an effect. Pure constructors such as id
builders and plan builders must not be described in terms of an effect kind.

```text
exports:
  types:        [ ... exported type definitions ... ]
  constants:    [ ... typed constant definitions ... ]
  pureFunctions:[ ... deterministic, authority-free helpers ... ]
  effects:      [ ... semantic effects that emit facts ... ]
  obstructions: [ ... typed obstruction definitions ... ]
```

- A `pureFunction` is deterministic, total over valid input, cost-bounded,
  authority-free, and observes no runtime state. It may be called from Core pure
  expression position. It may never read host state, allocate without bound, or
  reach a target through a side door (`EDICT-LAWPACK-PURE-001`).
- An `effect` emits a semantic fact. It is callable only in A-normal effect
  position inside an intent body, exactly like a target intrinsic.

A pure helper that touches runtime state, even transitively, is a relapse and
must reject locked-bundle production.

## Lawpack Manifest

The canonical schema is `abi/edict-lawpack.cddl`. The following is generated
from that schema and is illustrative only.

```json
{
  "apiVersion": "edict.lawpack/v1",
  "id": "jedit.structural_history",
  "version": "1",
  "acceptedCoreAbi": ["edict.core/v1"],
  "dependencies": [
    { "id": "jedit.rope", "version": "1", "digest": "sha256:..." }
  ],
  "exports": {
    "id": "jedit.structural_history.exports/v1",
    "digest": "sha256:..."
  },
  "targetAdapters": [
    {
      "targetProfile": "echo.dpo",
      "targetProfileVersion": "1",
      "adapter": { "id": "jsh.echo-dpo.adapter/v1", "digest": "sha256:..." }
    },
    {
      "targetProfile": "kv.transactional",
      "targetProfileVersion": "1",
      "adapter": { "id": "jsh.kv.adapter/v1", "digest": "sha256:..." }
    }
  ],
  "helperComponent": { "id": "jsh.helpers/v1", "digest": "sha256:..." },
  "verifier": { "id": "jsh.verifier/v1", "digest": "sha256:..." },
  "sandbox": { "id": "edict.wasm-component/v1", "digest": "sha256:..." },
  "fuelModel": { "id": "edict.fuel/v1", "digest": "sha256:..." },
  "compatibility": {
    "id": "jsh.compatibility/v1",
    "digest": "sha256:..."
  },
  "conformanceFixtureCorpus": {
    "id": "jsh.fixtures/v1",
    "digest": "sha256:..."
  }
}
```

The lawpack manifest is hash-significant. Display metadata, codenames, and
marketing copy are not part of the manifest; they live in sidecars keyed by the
lawpack digest (`EDICT-ABI-DISPLAY-001`). The dependency graph must be acyclic
and fully digest-locked before a locked bundle is produced
(`EDICT-LAWPACK-DAG-001`).

## Exported Types And Constants

Every exported type resolves to the Edict Core type universe (scalars, records,
enums, variants, `Option`, bounded `List`, bounded `Map`, refined scalars).
Exported `String`/`Bytes` types must carry maxima and, for `String`, a
canonicalization policy, exactly as required of all checked-lane values
(`EDICT-LANG-BOUNDS-001`). Recursive exported types are rejected unless every
recursive path carries a digest-locked maximum depth.

Exported constants are typed canonical values. Budget obligations such as
`recordBatchBudget` are exported constants; see Footprint And Cost Obligations.

## Pure Helper Signatures

Each pure helper declares:

```text
pureFunction:
  coordinate          # e.g. jedit.rope@1.textBlobId
  typeParameters
  parameterTypes      # all bounded
  returnType          # bounded
  costTemplate        # bounded steps/allocated bytes/output bytes
  determinismClass    # total | total-with-typed-diagnostic
```

A helper whose return type is unbounded (`String` or `Bytes` without a maximum),
or whose cost template admits unbounded allocation or unbounded input scanning,
rejects (`EDICT-LAWPACK-PURE-002`). Helpers may only return typed diagnostics
through `Option`/typed result, never host exceptions.

## Semantic Effect Signatures

Each semantic effect declares:

```text
effect:
  coordinate          # e.g. jedit.structural_history@1.entry.record
  typeParameters
  inputType           # bounded
  outputType          # bounded
  materialization     # proofOnly | runtimeMaterialized
  effectKindHint      # read | append | create | ensure | replace | delete |
                      #   reduce | semantic.emit | custom
  footprintObligation # abstract obligation OR required target lowering
  costObligation      # abstract obligation OR required target lowering
  failureClasses      # see Obstruction Schemas
  guardSupport        # whether the effect can participate in a target guard
```

`materialization` is the load-bearing field for read-only inference
(`EDICT-LANG-READONLY-001`):

- `proofOnly` semantic facts may be compatible with `readOnly` operation mode.
- `runtimeMaterialized` effects (anything that lowers to a durable append,
  create, replace, delete, audit write, or projection mutation) are not
  read-only and must reject any `readOnly` claim that contains them.

A semantic effect's `effectKindHint` is advisory at the lawpack layer; the
authoritative effect kind for a given lowering is fixed by the target adapter.

## Obstruction Schemas

Lawpack obstructions are typed. Each obstruction declares a coordinate and a
bounded payload schema:

```text
obstruction:
  coordinate          # e.g. jedit.rope@1.TextBlobHashConflict
  failureClass        # domainMappable | participantOwned | integrityFault |
                      #   resourceFault | internalFault
  payloadSchema       # typed, bounded record (may be empty)
```

Only `domainMappable` classes may be author-mapped in source `else` clauses.
Payload fields must themselves be typed and bounded; an obstruction whose
payload contains a naked `String`/`Bytes` is rejected. Effect failure mapping in
source is exhaustive over the effect's declared domain-mappable failure classes
and reuses the language's exhaustive-match machinery
(`EDICT-LANG-OBSTRUCT-EXHAUST-001`).

## Footprint And Cost Obligations

A portable lawpack effect declares an abstract obligation; its target adapter
must discharge that obligation against the selected target's footprint algebra
and cost algebra. Footprint ("what state may this touch") and cost ("how much
work") remain separate (`EDICT-LANG-FOOTPRINT-COST-001`).

A lawpack may export an abstract `targetBudget` obligation. The target adapter
translates that obligation into the selected target cost algebra; participant
policy then applies an admitted ceiling. See the Core/target/admitted budget
split in [SPEC - Edict Language v1](./SPEC_edict-language-v1.md).

## Target Adapters

For each supported target profile, a lawpack supplies a digest-locked adapter
that lowers each semantic effect to that target's intrinsics, mapping:

```text
adapter:
  targetProfile + version + acceptedTargetIr
  perEffectLowering:
    semanticEffectCoordinate -> target intrinsic plan
    materialization confirmation
    footprint obligation discharge
    cost obligation discharge
    failureClass -> target obstruction class mapping
    guard attachment rules
```

A portable semantic intent compiles for a target **only** when the lawpack
supplies an adapter for that target profile. Absent an adapter, lowering fails
with a registration/admission-class error, never a silent fallback
(`EDICT-LAWPACK-ADAPTER-001`).

## Compatibility Matrix

The compatibility component declares, per export coordinate, which
`(targetProfile, version)` pairs and which Core ABI versions are supported, and
the compatibility class of each version transition (hash, source, Core, target
IR, verifier, admission). `edict profile diff` and `edict lawpack diff` classify
transitions against this matrix.

## Conformance

The lawpack digest pins the normative fixture corpus. Third-party conformance
execution results are external attestations referencing the lawpack digest;
adding a new test lab does not change lawpack identity. Per the Two-Lowerer
Trial, a lawpack target adapter is not considered stable until two independent
implementations produce byte-identical results for the normative corpus
(`EDICT-CONFORMANCE-DIFFERENTIAL-001`).

## Relapse Checks

- A lawpack "pure" helper must not observe runtime state through any side door.
- A semantic effect that lowers to a durable mutation must not be classified
  `proofOnly`.
- Lawpack obstruction payloads must be typed and bounded.
- A lawpack must not be admitted with a floating (digest-free) dependency,
  adapter, helper, or verifier reference.
- A lawpack must not export naked unbounded `String`/`Bytes` values into the
  checked lane.
