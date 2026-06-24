# Target Profiles Topic

Status: current HEAD contract.

This chapter describes the v1 target-profile manifest conformance contract that
exists today. It validates typed manifest values for runtime-neutral profile
metadata and atomic application doctrine before any target lowerer runs. It does
not load manifest files, lower Core to Target IR, run a verifier, or make an
admission claim.

## Public Surface

The `edict_syntax` crate exposes `validate_target_profile_manifest` and typed
target-profile data structures for:

- `TargetProfileManifest`, the typed `edict.target-profile/v1` manifest value;
- `TargetProfileConformanceReport`, including `Conformant` and `NonConformant`
  classifications;
- stable `TargetProfileConformanceFailureKind` categories. [TPROF-REQ-001]

The canonical artifact shape for `edict.target-profile/v1` is named in
[`docs/abi/edict-target-profile.cddl`](../../abi/edict-target-profile.cddl).
[TPROF-REQ-001]

## Current Contract

- `TargetProfileManifest` records profile identity, accepted Core ABI,
  intrinsic namespace, every required digest-locked manifest component,
  canonical encoding rules, deferred lawpack-adapter ABI entries, diagnostics,
  v1 application doctrine, deterministic execution, and conformance fixture
  corpus references. [TPROF-REQ-001]
- Conformance is runtime-neutral. `echo.dpo@1` and `kv.transactional@1` shaped
  profiles are checked by the same manifest obligations; the checker does not
  require Echo, graph, database, event-log, repository, or storage runtime
  nouns. [TPROF-REQ-002]
- Every normative manifest component reference must carry a non-empty
  coordinate and digest. [TPROF-REQ-003]
- A conforming profile must accept `edict.core/v1`. [TPROF-REQ-004]
- `acceptedLawpackAdapterAbi` remains empty in v1 until the byte-level
  `edict.lawpack-adapter/v1` ABI is specified. [TPROF-REQ-005]
- The v1 application doctrine accepted by the checker is atomic application,
  application-snapshot reads, precommit-atomic guard evaluation, and
  no-visible-effects obstruction rollback. [TPROF-REQ-006]

## Deferred

The following are not implemented by this target-profile slice:

- canonical-CBOR encode/decode helpers for `TargetProfileManifest`;
- file-backed manifest loading;
- CDDL instance validation;
- intrinsic and operation-profile corpus parsing;
- target lowerers;
- verifier reports;
- bundle/admission tooling;
- multi-target composite profile validation.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
