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
- selected source/Core shape: the first supported effectful Core effect node;
- selected outcome: a deterministic in-memory target-owned review artifact;
- selected failure mode: stable structured target-lowering errors before any
  target artifact is emitted.

The `edict_syntax` crate exposes `lower_to_target_ir`,
`TargetIrLoweringFacts`, `TargetLoweringReport`, `TargetIrArtifact`, and stable
`TargetLoweringFailureKind` values. The lowerer consumes an already-built
`CoreModule` and explicit target-lowering facts supplied by the caller. It does
not read target facts from ambient environment, discover runtimes, or fetch
registries.

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
- the selected Echo target intrinsic;
- the structured Core input expression;
- sorted obstruction failure keys and their structured obstruction arm values.

Each Target IR intent also preserves the Core input constraints, Core evaluation
budget, and structured Core result expression for the supported slice. This
records preconditions, evaluation limits, and success-output semantics without
executing Echo or admitting a bundle.

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
outside the first supported effect shape rejects with
`TargetLoweringFailureKind::UnsupportedCoreNode`. Missing or ambiguous effect
lowering facts, non-Echo target intrinsics, missing operation-profile support,
and obstruction keys absent from the selected target facts also reject before any
artifact is emitted. A Core intent with no target-owned steps, or a Core module
with no intents, rejects with `TargetLoweringFailureKind::NoTargetSteps`.
Duplicate target-lowering facts are ambiguous only when they match an effect
used by the Core module being lowered; unrelated duplicate facts do not block
the supported artifact.

## Deferred

The following are not implemented by this slice:

- Echo runtime execution;
- Echo verifier completeness;
- bundle or admission generation;
- general target-lowering plugin dispatch;
- git-warp runtime execution, commit object creation, and CRDT reducer
  verification;
- additional target profiles beyond Echo and git-warp;
- canonical Target IR bytes, digests, or reviewed golden artifacts;
- v2 chained or composite adapter resolution.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
