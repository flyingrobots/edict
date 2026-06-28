# Target IR Test Plan

Status: current verification design for first target IR generation.

## Scope

In scope:

- typed target-lowering API over `CoreModule`;
- explicit selected target profile facts;
- deterministic `echo.span-ir/v1` review artifact for `echo.dpo@1`;
- stable target-lowering failure kinds for unsupported target obligations;
- explicit non-claim that Target IR generation executes Echo, admits a bundle,
  or implements git-warp.

Out of scope:

- Echo runtime execution;
- full Echo DPO verifier completeness;
- bundle/admission validation;
- general plugin dispatch through `edict-target-lowerer.wit`;
- git-warp Target IR;
- v2 adapter composition.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| TIR-REQ-001 | implemented | `edict_syntax` exposes a typed target-lowering API that consumes Core and explicit target-profile facts without reading ambient environment or runtime state. | issue #66 |
| TIR-REQ-002 | implemented | The first target lowerer supports `echo.dpo@1` and emits a deterministic `echo.span-ir/v1` review artifact for the supported effectful Core shape. | issue #66, docs/SPEC_edict-target-profile-abi-v1.md |
| TIR-REQ-003 | implemented | Target lowering rejects unsupported target profiles with stable structured failure kinds before producing an artifact. | issue #66 |
| TIR-REQ-004 | implemented | Target lowering rejects unsupported Core or operation-profile obligations with stable structured failure kinds before producing an artifact. | issue #66 |
| TIR-REQ-005 | implemented | Target IR lowering facts can be derived from selected native lowerability results, so lowerability evidence and Target IR generation select the same target profile and effect support. | issue #66, ROADMAP.md |
| TIR-REQ-006 | policy | The Echo Target IR slice does not execute Echo, run admission, implement git-warp, or claim general target-lowering plugin dispatch. | ROADMAP.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| crates/edict-syntax/tests/target_ir.rs | In-test Core fixtures and explicit Echo target-lowering facts for the first Target IR slice. | The supported Core effect shape emits Echo Span IR, while unsupported target profiles and Core nodes reject with stable failure kinds. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TIR-TP-001 | implemented | Golden path | TIR-REQ-001, TIR-REQ-002 | The supported effectful Core shape lowers to an `echo.span-ir/v1` artifact whose selected profile is `echo.dpo@1` and whose steps preserve effect coordinates, effect result bindings, inputs, obstruction arms, intent results, and deterministic order. | supported_effectful_core_lowers_to_echo_span_ir, obstruction_arm_values_are_preserved_in_echo_span_ir, intent_result_is_preserved_in_echo_span_ir, effect_result_bindings_are_preserved_in_echo_span_ir | crates/edict-syntax/tests/target_ir.rs | Echo first; git-warp follows later. |
| TIR-TP-002 | implemented | Golden path | TIR-REQ-001, TIR-REQ-005 | Native lowerability support for `echo.dpo@1` feeds Target IR lowering facts and produces the same Echo intrinsic selection in the emitted artifact. | lowerability_native_support_feeds_echo_target_lowering, lowerability_bridge_carries_only_selected_native_effect | crates/edict-syntax/tests/target_ir.rs | Bridges lowerability evidence to Target IR generation without adapter search or carrying unselected native supports. |
| TIR-TP-003 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-003 | Selecting a non-Echo target profile returns `TargetLoweringFailureKind::UnsupportedTargetProfile` with no artifact. | non_echo_target_profile_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Keeps the first slice explicitly Echo-only. |
| TIR-TP-004 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core module with an unsupported node returns `TargetLoweringFailureKind::UnsupportedCoreNode` with no artifact. | unsupported_core_nodes_reject_without_artifact | crates/edict-syntax/tests/target_ir.rs | No silent fallback or partial artifact. |
| TIR-TP-005 | policy | Release boundary | TIR-REQ-006 | Roadmap and release scope keep runtime execution, admission, git-warp lowering, and general plugin dispatch outside the first Echo slice. | - | - | Non-goal boundary; not a substitute for behavior tests. |
| TIR-TP-006 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core intent whose required operation profile is absent from the selected target-lowering facts returns `TargetLoweringFailureKind::MissingOperationProfile` with no artifact. | unsupported_operation_profile_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Prevents effect-only support from bypassing lowerability profile selection. |

## Determinism Obligations

- Tests inspect structured Rust values only.
- Target facts are in-memory constants.
- Output ordering is derived from Core order and sorted maps, not hash maps.
- No test reads stdout, stderr, logs, wall-clock time, random values, network
  state, or filesystem ordering.

## Open Gaps

- Target IR canonical-CBOR encoding and reviewed byte fixtures.
- Echo verifier reports.
- Bundle integration from generated Target IR.
- CLI exposure.
- Source-to-target fixture through `fixtures/lang/effects/read-greeting.edict`
  once the compiler spine supports its non-`basis none` Echo source shape.
- git-warp Target IR.
