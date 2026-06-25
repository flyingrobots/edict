# Contract Bundles Test Plan

Status: current verification design for v1 contract bundle and assurance
manifest checks.

## Scope

In scope:

- typed `ContractBundleManifest` values;
- `validate_contract_bundle_manifest` validation behavior;
- runtime-neutral acceptance of Echo and non-Echo target-profile bundle shapes;
- digest-locked source, Core, target-profile, target-IR, lawpack, generated
  artifact, toolchain, compile-option, build-provenance, verifier-report, and
  explanation references;
- logical source path validation;
- optional HOLMES, Watson, and Moriarty evidence binding, when present, to the
  selected bundle subject, target profile digest, and target IR digest.

Out of scope:

- canonical-CBOR encode/decode helpers for `ContractBundleManifest`;
- recomputing semantic or release bundle digests from canonical preimages;
- file-backed bundle loading;
- full CDDL instance validation;
- target lowering;
- verifier execution;
- admission request, receipt, policy, catalog, participant descriptor, or
  signature validation.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| BUNDLE-REQ-001 | implemented | `ContractBundleManifest` is a typed contract for `edict.contract-bundle/v1`, including semantic and release bundle digests plus source, Core, target-profile, target-IR, lawpack, generated artifact, toolchain, semantic and nonsemantic compile-option, build-provenance, verifier-report, compile-explanation, and assurance evidence references. | issue #1, docs/SPEC_continuum-contract-bundle-v1.md |
| BUNDLE-REQ-002 | implemented | Contract bundle validation is runtime-neutral: Echo-shaped and KV-shaped bundles are checked by the same obligations without requiring graph/runtime-specific nouns. | issue #1 |
| BUNDLE-REQ-003 | implemented | Every hash-significant artifact reference carried by the typed bundle must be digest-locked by non-empty coordinate and valid `sha256:<64 lowercase hex>` digest review rendering. | docs/SPEC_continuum-contract-bundle-v1.md |
| BUNDLE-REQ-004 | implemented | Source artifact provenance paths must be logical package-relative paths, not absolute, drive-letter, current-directory, parent-directory, backslash, or empty machine-local paths. | docs/SPEC_continuum-contract-bundle-v1.md |
| BUNDLE-REQ-005 | implemented | HOLMES, Watson, and Moriarty evidence entries are optional in the typed bundle; when present, each entry must bind to the manifest's selected bundle subject digest, target profile digest, and target IR digest. | issue #1, docs/GUIDE_edict-assurance-transparency.md |
| BUNDLE-REQ-006 | implemented | Admission artifacts remain out of the participant-neutral contract bundle manifest; non-empty admission references are rejected. | docs/SPEC_continuum-contract-bundle-v1.md |
| BUNDLE-REQ-007 | implemented | The typed bundle pins `canonicalization_profile.coordinate` to `edict.canonical-cbor/v1`. | docs/SPEC_continuum-contract-bundle-v1.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| crates/edict-syntax/tests/contract_bundle.rs | In-memory typed Echo and KV contract bundle manifests plus negative manifest mutations. | The same checker accepts both runtime-neutral positive shapes and rejects only the mutated obligations with stable failure kinds. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| BUNDLE-TP-001 | implemented | Golden path | BUNDLE-REQ-001, BUNDLE-REQ-002 | Echo-shaped and KV-shaped manifests both return `ContractBundleValidationStatus::Valid` with no failures. | echo_and_kv_bundles_validate_with_the_same_runtime_neutral_contract | crates/edict-syntax/tests/contract_bundle.rs | Proves the checker is not Echo-specific. |
| BUNDLE-TP-002 | implemented | Boundary guard | BUNDLE-REQ-003 | Removing, malforming, or using uppercase review rendering for digests returns `InvalidArtifactReference` on artifact fields or `InvalidBundleDigest` on bundle digest fields. | bundle_artifact_references_must_be_digest_locked, malformed_bundle_artifact_digest_is_rejected, uppercase_bundle_digest_rendering_is_rejected, uppercase_artifact_digest_rendering_is_rejected | crates/edict-syntax/tests/contract_bundle.rs | Stable failure kind and field, not prose. |
| BUNDLE-TP-003 | implemented | Boundary guard | BUNDLE-REQ-004 | Absolute, parent-directory, current-directory, drive-letter, backslash, and empty source paths return `InvalidSourcePath`. | source_artifact_paths_must_be_logical_package_relative_paths | crates/edict-syntax/tests/contract_bundle.rs | Asserts stable failure kind only. |
| BUNDLE-TP-004 | implemented | Boundary guard | BUNDLE-REQ-005 | Assurance evidence whose bundle subject, target profile digest, or target IR digest does not match the manifest returns the matching structured failure kind. | assurance_evidence_must_match_bundle_subject, assurance_evidence_must_match_target_profile_digest, assurance_evidence_must_match_target_ir_digest | crates/edict-syntax/tests/contract_bundle.rs | Proves HOLMES/Watson/Moriarty evidence cannot float across bundles or profiles. |
| BUNDLE-TP-005 | implemented | Golden path | BUNDLE-REQ-005 | Omitting external assurance evidence returns `Valid`; evidence entries are checked only when present. | external_assurance_evidence_is_optional | crates/edict-syntax/tests/contract_bundle.rs | Keeps participant-neutral bundle identity independent from optional external assurance artifacts. |
| BUNDLE-TP-006 | implemented | Boundary guard | BUNDLE-REQ-006 | Non-empty admission references return `AdmissionArtifactUnsupported`. | admission_artifacts_are_rejected_from_contract_bundles | crates/edict-syntax/tests/contract_bundle.rs | Admission remains external to the participant-neutral bundle. |
| BUNDLE-TP-007 | implemented | Boundary guard | BUNDLE-REQ-001 | Removing the release-only build-provenance digest returns `InvalidArtifactReference` on `build_provenance`. | release_bundle_inputs_must_be_digest_locked | crates/edict-syntax/tests/contract_bundle.rs | Proves release digest preimage inputs are represented by the typed manifest. |
| BUNDLE-TP-008 | implemented | Boundary guard | BUNDLE-REQ-007 | Changing `canonicalization_profile.coordinate` returns `UnsupportedCanonicalizationProfile`. | canonicalization_profile_must_be_the_v1_cbor_profile | crates/edict-syntax/tests/contract_bundle.rs | Pins the v1 bundle to the canonical CBOR profile. |
| BUNDLE-TP-009 | implemented | Golden path | BUNDLE-REQ-001 | Empty lawpack, generated-artifact, and conformance-corpus lists remain valid because optional lists bind what is present without creating non-empty obligations. | optional_artifact_lists_may_be_empty | crates/edict-syntax/tests/contract_bundle.rs | Source artifacts remain the required artifact set. |

## Determinism Obligations

- Manifests are built from in-memory constants.
- Assertions use structured statuses, failure kinds, and stable fields.
- No test inspects stdout, stderr, diagnostic prose, serialized bytes,
  filesystem ordering, network state, or wall-clock time.
- The contract graph is checked by `cargo xtask contract-check`.

## Open Gaps

- Canonical-CBOR encode/decode helpers for `edict.contract-bundle/v1`.
- Digest recomputation from canonical semantic and release preimages.
- File-backed bundle loading.
- Full CDDL instance validation.
- Target lowerer and verifier execution.
- Admission request, receipt, policy, catalog, participant descriptor, and
  signature validation.
