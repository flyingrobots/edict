# Target IR Topic

Status: current HEAD contract.

This chapter describes Edict's target IR generation boundary. Target IR is the
first target-owned artifact after typed Core. It is selected by an explicit
target profile and remains participant-neutral: producing Target IR does not
execute a runtime, admit a bundle, or mutate participant state.

## Current Contract

The current target IR implementation is deliberately narrow:

- selected target profile: `echo.dpo@1` or `gitwarp.ref_crdt@1`;
- selected Target IR artifact domain: `echo.span-ir/v1` or
  `gitwarp.commit-reducer-ir/v1`;
- selected source/Core shape: the first supported effectful Core effect node
  and Echo `require` guard requirements;
- selected outcome: a deterministic target-owned review artifact with canonical
  `edict.canonical-cbor/v1` bytes and a reviewed
  `edict.target-ir.artifact/v1` digest;
- selected failure mode: stable structured target-lowering errors before any
  target artifact is emitted.

The `edict_syntax` crate exposes `lower_to_target_ir`,
`TargetIrLoweringFacts`, `TargetLoweringReport`, `TargetIrArtifact`,
`TargetIrSemanticClosure`,
`encode_target_ir_artifact`, `digest_target_ir_artifact`, and stable
`TargetLoweringFailureKind` values. The lowerer consumes an already-built
`CoreModule` and explicit target-lowering facts supplied by the caller. It does
not read target facts from ambient environment, discover runtimes, or fetch
registries.

The crate also exposes `BuiltinTargetLowerer`, `BuiltinLowererRequest`, and
`lower_with_builtin_lowerer` as an
in-process migration seam for the existing Echo and git-warp lowerers. Selection
is explicit and bound to the lowerer's target-profile coordinate. A mismatch
rejects before invocation; once matched, the complete direct
`TargetLoweringReport` passes through unchanged. Tests prove the direct and
compatibility paths produce identical Target IR artifacts, canonical bytes, and
digests. This adapter does not resolve provider manifests, load components, or
define general target plugin dispatch. [TIR-REQ-013]

`TargetIrLoweringFacts::from_lowerability_report` derives the effect-to-intrinsic
lowering table from accepted native lowerability reports.
The derived facts use the target-profile coordinate and operation profile from
the lowerability report, along with the obstruction coordinates proven by that
report. The caller supplies a prevalidated target-profile reference, and the
bridge rejects references whose coordinate does not match the lowerability
report or whose digest is missing or malformed. Repeated identical native effect
selections are coalesced. Rejected lowerability reports cannot build
target-lowering facts. The v0.9 bridge is native-only: it consumes selected
native effect support for the explicitly supported Echo and git-warp target
profiles and does not perform adapter-chain search or general target plugin
dispatch.

Target-lowering facts also carry the operation profiles selected by
lowerability. A Core intent whose `required_operation_profile` is absent from
that explicit set rejects before Target IR is emitted.

For the supported Echo and git-warp slices, each supported Core effect node
becomes a deterministic Target IR step that records:

- the source Core effect coordinate;
- the effect result binding;
- the selected target intrinsic;
- the structured Core input expression;
- sorted obstruction failure keys and their structured obstruction arm values.

For the supported Echo slice, each supported Core `require` node before any
target step becomes a deterministic Target IR requirement that records:

- the requirement id;
- the structured Core predicate;
- a terminal or `continueObstructed` failure disposition;
- the stable obstruction reason kind and canonical reason payload fields.

git-warp does not currently claim Target IR requirement support. A Core module
with `require` nodes selected for git-warp rejects before artifact emission with
`TargetLoweringFailureKind::UnsupportedTargetFeature`.

Intent-level Target IR requirements are pre-step guards. A Core `require` after
an emitted target step rejects with
`TargetLoweringFailureKind::UnsupportedTargetFeature` before artifact emission.
If the requirement predicate or reason payload references a local produced by an
earlier target step, lowering uses the same stable failure kind with a more
specific step-output-dependency detail. Ordered or step-attached guards remain a
future artifact-model change.

Each Target IR intent also preserves an explicit Core basis expression when
present, the Core input constraints, Core evaluation budget, source-ordered
requirements, source-ordered effect steps, and structured Core result
expression for the supported slice. This records authored basis, preconditions,
evaluation limits, guard dispositions, and success-output semantics without
resolving a runtime basis, executing Echo, or admitting a bundle.

When any intent has an explicit basis or the Core module imports a lawpack, the
artifact carries a `TargetIrSemanticClosure`. The closure binds the exact
canonical Core coordinate/digest and a coordinate-keyed, lowercase
digest-locked lawpack set. Equivalent lawpack order and duplicate identical
references canonicalize to the same Target IR identity; conflicting resources,
an empty source Core coordinate, an unidentifiable Core, or a basis-bearing
artifact without its closure rejects before Target IR identity exists.
[TIR-REQ-015]

Canonical Target IR uses an intentional artifact-envelope value model rather
than Rust struct serialization. The reviewed digest is SHA-256 over canonical
CBOR for:

```text
["edict.digest/v1", "edict.target-ir.artifact/v1", <canonical Target IR value>]
```

The canonical value includes the artifact's own domain, digest-locked target
profile resource, source Core coordinate, optional semantic closure, sorted
intent map, optional explicit basis expressions, input constraints, Core
evaluation budget, source-ordered requirements, requirement predicates and
failure dispositions, source-ordered target steps, sorted obstruction failure
keys and arms, and structured Core result expression. Target profile and
semantic-closure digests are strict artifact references: missing digests and non-lowercase
`sha256:<64 hex>` review strings reject before hashing.

Reviewed Echo and git-warp Target IR byte/digest goldens live under
`fixtures/target-ir/canonical/`. `cargo xtask target-ir-goldens --check`
regenerates them from executable lowering and canonical encoding, and
`cargo xtask verify` includes that check.

[`edict-target-ir.cddl`](../../abi/edict-target-ir.cddl) defines the
`target-ir-artifact` root from that canonical value shape. The provider contract
pack assembles it with the common and Core rules it references, verifies the
complete rule closure, and checks both reviewed Target IR artifacts through the
compiled root. This is structural wire-schema evidence for valid
lowering-produced artifacts; the lowerer and encoder continue to own semantic
identifier validity and canonical ordering or deduplication of set-like
fields. The schema has separate closed and legacy artifact variants, so an
external artifact carrying an explicit basis cannot validate after its semantic
closure is removed. [TIR-REQ-014] [TIR-REQ-015]

Bundle assembly can now consume a real `TargetIrArtifact` through
`assemble_contract_bundle_from_target_ir`. That path computes
`targetIrDigest` from canonical Target IR bytes and writes the same digest into
the manifest, recomputes the expected semantic closure from the exact supplied
Core, and requires both the artifact closure and bundle lawpack set to match.
Standalone canonical Target IR encoding provides deterministic structural
self-validation; it cannot reconstruct dependencies that a caller erased from
an in-memory artifact. The computed assembly path is the separate
cross-artifact corroboration boundary. The supplied-reference assembly path
remains available for already-digested external artifact graphs, but the
computed Target IR path has no caller-supplied target IR digest field.

Selecting a target profile outside the explicit supported set rejects with
`TargetLoweringFailureKind::UnsupportedTargetProfile`. Selecting an unsupported
Target IR domain rejects with
`TargetLoweringFailureKind::UnsupportedTargetIrDomain`. Selecting Echo without a
digest-locked target-profile reference, or selecting git-warp without a
digest-locked target-profile reference, rejects with
`TargetLoweringFailureKind::UndigestedTargetProfile`. Supplying a Core module
with an unsupported ABI rejects with
`TargetLoweringFailureKind::UnsupportedCoreAbi`. Supplying a Core module with
floating imports rejects with `TargetLoweringFailureKind::UndigestedCoreImport`.
Supplying unsupported Core capability flags rejects with
`TargetLoweringFailureKind::UnsupportedCoreCapability`. Supplying Core nodes
outside the supported effect and Echo requirement shapes rejects with
`TargetLoweringFailureKind::UnsupportedCoreNode`. Supplying a target-specific
Core feature that the selected target does not support rejects with
`TargetLoweringFailureKind::UnsupportedTargetFeature`. Missing or ambiguous
effect lowering facts, non-Echo target intrinsics, missing operation-profile
support, and obstruction keys absent from the selected target facts also reject
before any artifact is emitted. A Core intent with no target-owned requirements
or steps, or a Core module with no intents, rejects with
`TargetLoweringFailureKind::NoTargetSteps`. Duplicate target-lowering facts are
ambiguous only when they match an effect used by the Core module being lowered;
unrelated duplicate facts do not block the supported artifact.

## Deferred

The following are not implemented by this slice:

- Echo runtime execution;
- Echo verifier completeness;
- admission generation;
- general target-lowering plugin dispatch;
- git-warp runtime execution, commit object creation, and CRDT reducer
  verification;
- Echo runtime receipts for first-class resumable obstruction strands;
- additional target profiles beyond Echo and git-warp;
- v2 chained or composite adapter resolution.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
