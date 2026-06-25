# Target Profiles Test Plan

Status: current verification design for v1 target-profile manifest conformance.

## Scope

In scope:

- typed `TargetProfileManifest` values;
- `validate_target_profile_manifest` conformance behavior;
- runtime-neutral acceptance of Echo and non-Echo target-profile shapes;
- digest-locked manifest component references;
- accepted Core ABI requirements;
- deferred lawpack-adapter ABI emptiness;
- v1 atomic application doctrine.

Out of scope:

- canonical-CBOR encode/decode helpers for `TargetProfileManifest`;
- file-backed manifest loading;
- full CDDL instance validation;
- intrinsic and operation-profile corpus parsing;
- target lowering;
- verifier reports;
- file-backed integration with contract-bundle validation;
- admission validation;
- multi-target composite profile validation.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| TPROF-REQ-001 | implemented | `TargetProfileManifest` is a typed contract for `edict.target-profile/v1`, including profile identity, accepted Core ABI, intrinsic namespace, manifest component references, canonical encoding rules, diagnostics, v1 application doctrine, deterministic execution, and conformance fixture corpus. | issue #1, docs/abi/edict-target-profile.cddl |
| TPROF-REQ-002 | implemented | Target-profile conformance is runtime-neutral: Echo-shaped and KV-shaped profiles are checked by the same obligations without requiring graph/runtime-specific nouns. | issue #1 |
| TPROF-REQ-003 | implemented | Normative manifest component references must be digest-locked by non-empty coordinate and valid `sha256:<64 hex>` digest review renderings. | docs/abi/edict-target-profile.cddl |
| TPROF-REQ-004 | implemented | A conforming v1 target profile must accept `edict.core/v1`. | docs/abi/edict-target-profile.cddl |
| TPROF-REQ-005 | implemented | `acceptedLawpackAdapterAbi` is rejected when non-empty until the byte-level adapter ABI is specified. | EDICT-ABI-LAWPACK-ADAPTER-DEFER-001 |
| TPROF-REQ-006 | implemented | `multiTarget: true` is rejected until composite profile validation exists. | ROADMAP.md |
| TPROF-REQ-007 | implemented | V1 conformance requires atomic application, application-snapshot reads, precommit-atomic guard evaluation, and no-visible-effects obstruction rollback. | docs/SPEC_edict-target-profile-abi-v1.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| crates/edict-syntax/tests/target_profile.rs | In-memory typed Echo and KV target-profile manifests plus negative manifest mutations. | The same checker accepts both runtime-neutral positive shapes and rejects only the mutated obligations with stable failure kinds. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TPROF-TP-001 | implemented | Golden path | TPROF-REQ-001, TPROF-REQ-002 | Echo-shaped and KV-shaped manifests both return `TargetProfileConformanceStatus::Conformant` with no failures. | echo_and_kv_profiles_conform_to_the_same_runtime_neutral_manifest_contract | crates/edict-syntax/tests/target_profile.rs | Proves the checker is not Echo-specific. |
| TPROF-TP-002 | implemented | Boundary guard | TPROF-REQ-003 | Removing or malforming the verifier digest returns `NonConformant` with `NonDigestLockedResource` on the `verifier` field. | missing_digest_on_normative_manifest_slot_is_rejected, malformed_digest_on_normative_manifest_slot_is_rejected | crates/edict-syntax/tests/target_profile.rs | Stable failure kind and field, not prose. |
| TPROF-TP-003 | implemented | Boundary guard | TPROF-REQ-004 | Removing `edict.core/v1` from accepted Core ABI returns `MissingAcceptedCoreAbi`. | accepted_core_abi_must_include_v1_core | crates/edict-syntax/tests/target_profile.rs | Ensures target profiles declare the Core contract they accept. |
| TPROF-TP-004 | implemented | Boundary guard | TPROF-REQ-005 | Adding `edict.lawpack-adapter/v1` before that ABI is specified returns `DeferredLawpackAdapterAbiUnsupported`. | deferred_lawpack_adapter_abi_must_stay_empty_in_v1 | crates/edict-syntax/tests/target_profile.rs | Keeps adapter ABI claims out of this release. |
| TPROF-TP-005 | implemented | Boundary guard | TPROF-REQ-006 | Setting `multiTarget` true returns `UnsupportedCompositeProfile`. | multi_target_profiles_are_rejected_until_composite_validation_exists | crates/edict-syntax/tests/target_profile.rs | Prevents unvalidated composite profiles from passing v1 conformance. |
| TPROF-TP-006 | implemented | Boundary guard | TPROF-REQ-007 | Non-atomic application doctrine returns stable failure kinds for application model, read consistency, guard evaluation, and rollback. | atomic_application_semantics_are_required_for_v1_conformance | crates/edict-syntax/tests/target_profile.rs | Asserts structured behavior only. |

## Determinism Obligations

- Manifests are built from in-memory constants.
- Assertions use structured statuses, failure kinds, and stable fields.
- No test inspects stdout, stderr, diagnostic prose, serialized bytes,
  filesystem ordering, network state, or wall-clock time.
- The contract graph is checked by `cargo xtask contract-check`.

## Open Gaps

- Canonical-CBOR encode/decode helpers for `edict.target-profile/v1`.
- File-backed target-profile manifest loading and conformance fixtures.
- Full CDDL instance validation.
- Intrinsic and operation-profile corpus parsing.
- Target lowerers and verifier reports.
- File-backed integration with contract-bundle validation.
- Admission validation.
- Multi-target composite profile validation beyond explicit v1 rejection.
