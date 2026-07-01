# Target IR Test Plan

Status: current verification design for first target IR generation.

## Scope

In scope:

- typed target-lowering API over `CoreModule`;
- explicit selected target profile facts;
- deterministic `echo.span-ir/v1` review artifact for `echo.dpo@1`;
- deterministic `gitwarp.commit-reducer-ir/v1` review artifact for
  `gitwarp.ref_crdt@1`;
- canonical Target IR artifact bytes and digests for the current Echo and
  git-warp review artifacts;
- stable target-lowering failure kinds for unsupported target obligations;
- explicit non-claim that Target IR generation executes Echo, admits a bundle,
  executes git-warp, or implements general target dispatch.

Out of scope:

- Echo runtime execution;
- full Echo DPO verifier completeness;
- git-warp runtime execution, commit object creation, and CRDT reducer
  verification;
- bundle/admission validation;
- general plugin dispatch through `edict-target-lowerer.wit`;
- additional target profiles beyond Echo and git-warp;
- canonical `ContractBundleManifest` bytes;
- v2 adapter composition.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| TIR-REQ-001 | implemented | `edict_syntax` exposes a typed target-lowering API that consumes Core and explicit target-profile facts without reading ambient environment or runtime state. | issue #66 |
| TIR-REQ-002 | implemented | The first target lowerer supports `echo.dpo@1` and emits a deterministic `echo.span-ir/v1` review artifact for the supported effectful Core shape. | issue #66, docs/SPEC_edict-target-profile-abi-v1.md |
| TIR-REQ-003 | implemented | Target lowering rejects unsupported target profiles with stable structured failure kinds before producing an artifact. | issue #66 |
| TIR-REQ-004 | implemented | Target lowering rejects unsupported Core, empty-step, or operation-profile obligations with stable structured failure kinds before producing an artifact. | issue #66 |
| TIR-REQ-005 | implemented | Target IR lowering facts can be derived from selected native lowerability results, so lowerability evidence and Target IR generation select the same target profile and effect support. | issue #66, ROADMAP.md |
| TIR-REQ-006 | policy | The Target IR slices do not execute runtimes, run admission, or claim general target-lowering plugin dispatch. | ROADMAP.md |
| TIR-REQ-007 | implemented | The next target lowerer supports `gitwarp.ref_crdt@1` and emits a deterministic `gitwarp.commit-reducer-ir/v1` review artifact for the supported effectful Core shape without adding general plugin dispatch. | issue #68 |
| TIR-REQ-008 | implemented | Target IR artifacts have an intentional canonical value model, canonical CBOR bytes, and an `edict.target-ir.artifact/v1` digest frame over the current Echo and git-warp artifact envelope. | issue #105, docs/design/canonical-target-ir-v0.11.md |
| TIR-REQ-009 | implemented | Reviewed Echo and git-warp Target IR byte/digest golden fixtures are regenerated and checked by `xtask target-ir-goldens`. | issue #105, docs/design/canonical-target-ir-v0.11.md |
| TIR-REQ-010 | implemented | Bundle assembly can use a computed Target IR artifact digest as the single source of truth for `targetIrDigest` after canonical Target IR bytes exist. | issue #105, docs/design/canonical-target-ir-v0.11.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| crates/edict-syntax/tests/target_ir.rs | In-test Core fixtures and explicit Echo/git-warp target-lowering facts for the first Target IR slices. | The supported Core effect shape emits target-owned IR, while unsupported target profiles and Core nodes reject with stable failure kinds. |
| fixtures/target-ir/canonical/echo-effectful.target-ir.cbor | Reviewed Echo Target IR canonical byte golden. | `cargo xtask target-ir-goldens --check` compares the checked-in bytes to executable regeneration. |
| fixtures/target-ir/canonical/echo-effectful.target-ir.sha256 | Reviewed Echo Target IR digest golden. | `cargo xtask target-ir-goldens --check` compares the checked-in review digest to executable regeneration. |
| fixtures/target-ir/canonical/gitwarp-append.target-ir.cbor | Reviewed git-warp Target IR canonical byte golden. | `cargo xtask target-ir-goldens --check` compares the checked-in bytes to executable regeneration. |
| fixtures/target-ir/canonical/gitwarp-append.target-ir.sha256 | Reviewed git-warp Target IR digest golden. | `cargo xtask target-ir-goldens --check` compares the checked-in review digest to executable regeneration. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TIR-TP-001 | implemented | Golden path | TIR-REQ-001, TIR-REQ-002 | The supported effectful Core shape lowers to an `echo.span-ir/v1` artifact whose selected profile is `echo.dpo@1` and whose steps preserve effect coordinates, effect result bindings, inputs, obstruction arms, intent input constraints, evaluation budgets, intent results, and deterministic order. | supported_effectful_core_lowers_to_echo_span_ir, obstruction_arm_values_are_preserved_in_echo_span_ir, intent_constraints_and_budget_are_preserved_in_echo_span_ir, intent_result_is_preserved_in_echo_span_ir, effect_result_bindings_are_preserved_in_echo_span_ir | crates/edict-syntax/tests/target_ir.rs | Echo remains the first supported target slice. |
| TIR-TP-002 | implemented | Golden path | TIR-REQ-001, TIR-REQ-005 | Native lowerability support for `echo.dpo@1` feeds Target IR lowering facts and produces the same Echo intrinsic selection in the emitted artifact; rejected lowerability reports cannot build Target IR lowering facts; derived facts keep the lowerability report's target-profile coordinate, operation profile, and obstruction coordinates; prevalidated target-profile references must match the lowerability report; repeated identical native selections are coalesced. | lowerability_native_support_feeds_echo_target_lowering, lowerability_bridge_carries_only_selected_native_effect, lowerability_bridge_deduplicates_identical_native_effect_selection, unsupported_lowerability_report_does_not_build_target_ir_facts, lowerability_bridge_uses_report_target_profile_identity, lowerability_bridge_uses_report_operation_profile_identity, lowerability_bridge_requires_matching_target_profile_reference | crates/edict-syntax/tests/target_ir.rs | Bridges lowerability evidence to Target IR generation without adapter search or carrying unselected native supports. |
| TIR-TP-003 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-003 | Selecting an unsupported target profile returns `TargetLoweringFailureKind::UnsupportedTargetProfile` with no artifact; selecting an unsupported Target IR domain returns `TargetLoweringFailureKind::UnsupportedTargetIrDomain` with no artifact. | unsupported_target_profile_rejects_without_artifact, unsupported_target_ir_domain_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Keeps supported targets explicit. |
| TIR-TP-004 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core module with an unsupported node returns `TargetLoweringFailureKind::UnsupportedCoreNode` with no artifact. | unsupported_core_nodes_reject_without_artifact | crates/edict-syntax/tests/target_ir.rs | No silent fallback or partial artifact. |
| TIR-TP-005 | policy | Release boundary | TIR-REQ-006 | Roadmap and release scope keep runtime execution, admission, and general plugin dispatch outside the first target IR slices. | - | - | Non-goal boundary; not a substitute for behavior tests. |
| TIR-TP-006 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core intent whose required operation profile is absent from the selected target-lowering facts returns `TargetLoweringFailureKind::MissingOperationProfile` with no artifact; missing or ambiguous selected effect lowerings return `MissingEffectLowering` or `AmbiguousEffectLowering` with no artifact. | unsupported_operation_profile_rejects_without_artifact, missing_effect_lowering_rejects_without_artifact, ambiguous_effect_lowering_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Prevents effect-only support from bypassing lowerability profile selection and keeps effect matching deterministic. |
| TIR-TP-007 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core intent with no target-owned steps, or a Core module with no intents, returns `TargetLoweringFailureKind::NoTargetSteps` with no artifact. | empty_target_step_intents_reject_without_artifact, empty_core_modules_reject_without_artifact | crates/edict-syntax/tests/target_ir.rs | Prevents empty Echo artifacts from standing in for unsupported work. |
| TIR-TP-008 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-003 | A matched effect lowering whose target intrinsic belongs to a non-Echo profile returns `TargetLoweringFailureKind::UnsupportedTargetIntrinsic` with no artifact. | foreign_target_intrinsic_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Keeps nominal Echo artifacts from carrying another target's intrinsic. |
| TIR-TP-009 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core module whose ABI is not `edict.core/v1` returns `TargetLoweringFailureKind::UnsupportedCoreAbi` with no artifact. | unsupported_core_abi_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Prevents Target IR from silently reinterpreting future or stale Core shapes. |
| TIR-TP-010 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core module with unsupported required Core capability flags returns `TargetLoweringFailureKind::UnsupportedCoreCapability` with no artifact. | unsupported_core_capability_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Prevents Target IR from omitting hash-significant Core obligations. |
| TIR-TP-011 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-003 | Selecting Echo with a missing or invalid target-profile digest returns `TargetLoweringFailureKind::UndigestedTargetProfile` with no artifact. | undigested_target_profile_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Keeps emitted Target IR bound to a reproducible selected target profile. |
| TIR-TP-012 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | Duplicate lowerings for an unused effect do not reject an otherwise supported Core module. | unused_duplicate_effect_lowerings_do_not_reject_supported_effect | crates/edict-syntax/tests/target_ir.rs | Ambiguity is scoped to effects the Core module actually uses. |
| TIR-TP-013 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core effect obstruction key absent from the selected target facts returns `TargetLoweringFailureKind::MissingObstruction` with no artifact. | unsupported_obstruction_key_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Prevents lowerability evidence for one obstruction set from being reused for another. |
| TIR-TP-014 | implemented | Boundary guard | TIR-REQ-001, TIR-REQ-004 | A Core module with a floating import returns `TargetLoweringFailureKind::UndigestedCoreImport` with no artifact. | undigested_core_import_rejects_without_artifact | crates/edict-syntax/tests/target_ir.rs | Keeps Target IR generation aligned with canonical Core reproducibility. |
| TIR-TP-015 | implemented | Golden path | TIR-REQ-001, TIR-REQ-007 | The supported git-warp effectful Core shape lowers to a `gitwarp.commit-reducer-ir/v1` artifact for `gitwarp.ref_crdt@1`, preserving selected intrinsic, input, obstruction arms, input constraints, evaluation budget, result expression, and deterministic order. | supported_gitwarp_core_lowers_to_commit_reducer_ir | crates/edict-syntax/tests/target_ir.rs | Proves Target IR is not Echo-shaped. |
| TIR-TP-016 | implemented | Golden path | TIR-REQ-001, TIR-REQ-005, TIR-REQ-007 | Native lowerability support for `gitwarp.ref_crdt@1` feeds Target IR lowering facts and produces the same git-warp intrinsic selection in the emitted artifact. | lowerability_native_support_feeds_gitwarp_target_lowering | crates/edict-syntax/tests/target_ir.rs | Bridge remains native-only and explicit. |
| TIR-TP-017 | implemented | Golden path | TIR-REQ-008 | Echo and git-warp Target IR artifacts encode to deterministic canonical CBOR, decode as canonical CBOR, and produce stable `edict.target-ir.artifact/v1` digest review strings without colliding with each other. | target_ir_artifact_bytes_and_digests_are_deterministic | crates/edict-syntax/tests/target_ir.rs | The digest domain is the Edict envelope; the artifact domain remains inside the value. |
| TIR-TP-018 | implemented | Determinism guard | TIR-REQ-008 | Equivalent Target IR construction order for maps, obstruction failures, and input constraints does not change canonical bytes or digest, while semantic list order for steps remains preserved. | target_ir_artifact_canonicalization_ignores_equivalent_construction_order, target_ir_step_order_changes_digest | crates/edict-syntax/tests/target_ir.rs | Prevents Rust construction order from becoming a cryptographic contract. |
| TIR-TP-019 | implemented | Mutation sensitivity | TIR-REQ-008 | Target profile digest, source Core coordinate, intent name, effect coordinate, selected intrinsic, input expression, obstruction failure/arm, input constraint, budget, and result mutations each move the Target IR digest. | target_ir_digest_moves_for_artifact_semantic_mutations, target_ir_obstruction_arm_value_mutation_moves_digest | crates/edict-syntax/tests/target_ir.rs | Freezes the reviewed value shape without re-litigating lowering semantics. |
| TIR-TP-020 | implemented | Boundary guard | TIR-REQ-008 | Canonical Target IR encoding rejects missing target-profile digests and non-lowercase digest review strings before hashing. | target_ir_encoder_rejects_unlocked_or_uppercase_target_profile_digest | crates/edict-syntax/tests/target_ir.rs | Target IR artifact references use the strict bundle-artifact digest policy. |
| TIR-TP-021 | implemented | Golden path | TIR-REQ-009 | `xtask target-ir-goldens --check` fails on drift and `--write` regenerates Echo and git-warp byte/digest golden fixtures from executable assembly. | target_ir_goldens_match_executable_encoder | xtask/src/goldens.rs, xtask/src/main.rs, xtask/src/tests.rs, fixtures/target-ir/canonical/echo-effectful.target-ir.cbor, fixtures/target-ir/canonical/echo-effectful.target-ir.sha256, fixtures/target-ir/canonical/gitwarp-append.target-ir.cbor, fixtures/target-ir/canonical/gitwarp-append.target-ir.sha256 | Golden scope is Target IR artifact bytes and digest review strings only. |
| TIR-TP-022 | implemented | Integration | TIR-REQ-010 | Bundle assembly from a real `TargetIrArtifact` computes `targetIrDigest`, writes that same digest into the manifest, and changes bundle digests when the Target IR artifact changes. | assembled_bundle_from_real_target_ir_computes_target_ir_digest | crates/edict-syntax/tests/contract_bundle.rs | Keeps generated Target IR and manifest `target_ir.digest` as one source of truth. |

## Determinism Obligations

- Tests inspect structured Rust values and canonical Target IR bytes only.
- Target facts are in-memory constants.
- Output ordering is derived from Core order and sorted maps, not hash maps.
- Target IR golden bytes and digest review strings are regenerated by
  `cargo xtask target-ir-goldens --write` and checked by
  `cargo xtask target-ir-goldens --check`.
- No test reads stdout, stderr, logs, wall-clock time, random values, network
  state, or filesystem ordering.

## Open Gaps

- Echo verifier reports.
- git-warp verifier reports.
- CLI exposure.
- Source-to-target fixture through `fixtures/lang/effects/read-greeting.edict`
  once the compiler spine supports its non-`basis none` Echo source shape.
- Additional target profiles beyond Echo and git-warp.
