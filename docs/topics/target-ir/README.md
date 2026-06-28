# Target IR Topic

Status: current HEAD contract.

This chapter describes Edict's target IR generation boundary. Target IR is the
first target-owned artifact after typed Core. It is selected by an explicit
target profile and remains participant-neutral: producing Target IR does not
execute a runtime, admit a bundle, or mutate participant state.

## Current Contract

The current target IR implementation is deliberately narrow:

- selected target profile: `echo.dpo@1`;
- selected Target IR artifact domain: `echo.span-ir/v1`;
- selected source/Core shape: the first supported effectful Core effect node;
- selected outcome: a deterministic in-memory Echo Span IR review artifact;
- selected failure mode: stable structured target-lowering errors before any
  target artifact is emitted.

The `edict_syntax` crate exposes `lower_to_target_ir`,
`TargetIrLoweringFacts`, `TargetLoweringReport`, `TargetIrArtifact`, and stable
`TargetLoweringFailureKind` values. The lowerer consumes an already-built
`CoreModule` and explicit target-lowering facts supplied by the caller. It does
not read target facts from ambient environment, discover runtimes, or fetch
registries.

`TargetIrLoweringFacts::from_lowerability_report` derives the first Echo
effect-to-intrinsic lowering table from the selected native effects reported by
the lowerability checker. The v0.9 bridge is native-only: it consumes selected
native effect support and does not perform adapter-chain search or general
target plugin dispatch.

Target-lowering facts also carry the operation profiles selected by
lowerability. A Core intent whose `required_operation_profile` is absent from
that explicit set rejects before Target IR is emitted.

For the supported Echo slice, each supported Core effect node becomes a
deterministic Target IR step that records:

- the source Core effect coordinate;
- the effect result binding;
- the selected Echo target intrinsic;
- the structured Core input expression;
- sorted obstruction failure keys and their structured obstruction arm values.

Each Target IR intent also preserves the structured Core result expression for
the supported slice. This records success-output semantics without executing
Echo or admitting a bundle.

Selecting a non-Echo target profile rejects with
`TargetLoweringFailureKind::UnsupportedTargetProfile`. Supplying Core nodes
outside the first supported effect shape rejects with
`TargetLoweringFailureKind::UnsupportedCoreNode`. Missing or ambiguous effect
lowering facts and missing operation-profile support also reject before any
artifact is emitted.

`gitwarp.ref_crdt@1` is the next target after Echo. It is not part of the first
Echo Target IR slice.

## Deferred

The following are not implemented by this slice:

- Echo runtime execution;
- Echo verifier completeness;
- bundle or admission generation;
- general target-lowering plugin dispatch;
- git-warp target lowering;
- canonical Target IR bytes, digests, or reviewed golden artifacts;
- v2 chained or composite adapter resolution.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
