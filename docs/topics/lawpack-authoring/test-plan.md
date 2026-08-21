# Lawpack Authoring Test Plan

Status: current contract for Edict issue #195.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| LAUTH-REQ-001 | implemented | A public, typed authoring boundary accepts application-owned lawpack semantics without requiring callers to construct `CanonicalValue`, canonical CBOR, or derived local-artifact digests. | crates/edict-syntax/src/lawpack_authoring.rs |
| LAUTH-REQ-002 | implemented | Identical semantic inputs and exact dependency bundles produce byte-identical canonical artifacts and lowercase digest sidecars independent of caller working directory. | crates/edict-syntax/tests/lawpack_authoring.rs, crates/edict-cli/tests/jsonl_cli.rs |
| LAUTH-REQ-003 | implemented | Emitted manifest, exports, and adapter bytes pass the same public decoders and complete dependency-closure checks used by application builds before publication. | crates/edict-syntax/src/lawpack_authoring.rs |
| LAUTH-REQ-004 | implemented | Local resources receive Edict-derived identities; external resources and dependency edges require exact caller-authored digest pins that are corroborated against supplied bytes. | crates/edict-syntax/src/lawpack_authoring.rs |
| LAUTH-REQ-005 | implemented | Duplicate coordinates or output paths, namespace escapes, malformed pure Core, invalid adapters, incomplete or disconnected dependency closures, and digest substitution fail with stable structured authoring failures. | crates/edict-syntax/tests/lawpack_authoring.rs |
| LAUTH-REQ-006 | implemented | The public CLI supports write and check-only lawpack builds with confined relative paths, bounded reads, stale-output detection, and failure-atomic publication. | crates/edict-cli/src/lawpack_build.rs |
| LAUTH-REQ-007 | implemented | A consumer invoked from outside the Edict checkout can author, publish, check, and feed its exact lawpack closure into the public application-build boundary without invoking `xtask`. | crates/edict-cli/tests/lawpack_authoring_cli.rs |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LAUTH-TP-001 | implemented | Deterministic identity | LAUTH-REQ-001, LAUTH-REQ-002 | A typed minimal definition emits canonical manifest/exports pairs twice with identical bytes and digests; changing semantic surfaces moves only their owning identities. | minimal_authoring_emits_a_deterministic_valid_bundle, semantic_surface_mutations_move_their_owning_identities | crates/edict-syntax/tests/lawpack_authoring.rs | Covers the public typed boundary and semantic mutation matrix. |
| LAUTH-TP-002 | implemented | Complete surface | LAUTH-REQ-003 | Types, constants, component pure helpers, effects, obstructions, profiles, verifier metadata, adapter mappings, budgets, and local target configuration all round-trip through the existing decoders. | full_surface_round_trips_through_existing_decoders | crates/edict-syntax/tests/lawpack_authoring.rs | Edict-authored pure bodies are also exercised by the malformed-body refusal. |
| LAUTH-TP-003 | implemented | Dependency closure | LAUTH-REQ-003, LAUTH-REQ-004, LAUTH-REQ-005 | Root dependencies are corroborated against supplied bundles; missing, substituted, cyclic, and disconnected bundles reject before artifacts are returned. | exact_dependency_closure_is_required_and_corroborated | crates/edict-syntax/tests/lawpack_authoring.rs | The authoring test covers missing, substituted, and disconnected inputs; the shared graph validator retains its cycle case. |
| LAUTH-TP-004 | implemented | Error handling | LAUTH-REQ-004, LAUTH-REQ-005 | Invalid digests, non-integral JSON numbers, byte sentinels, duplicate paths, unresolved local resources, path escape, malformed pure Core, and incomplete adapters return stable failure kinds. | malformed_inputs_fail_with_stable_categories | crates/edict-syntax/tests/lawpack_authoring.rs | Assertions target structured failure kinds rather than diagnostic prose. |
| LAUTH-TP-005 | implemented | Transactional publication | LAUTH-REQ-006 | Publication replaces one owned output set, removes stale owned artifacts, preserves the previous set on injected failure, and check-only mode reports drift without writes. | publication_replaces_the_owned_tree_and_removes_stale_files, injected_pre_activation_failure_restores_the_previous_tree, check_only_detects_exact_tree_drift | crates/edict-cli/src/lawpack_build.rs | The persistent sibling lock is outside the replaceable output directory. |
| LAUTH-TP-006 | implemented | External consumer | LAUTH-REQ-002, LAUTH-REQ-006, LAUTH-REQ-007 | The built `edict` binary runs with temporary external working directories; repeated write/check runs are deterministic, and a standalone application feeds the exact generated closure into `edict application build`. | lawpack_build_writes_checks_repairs_and_is_cwd_independent, external_application_authors_vendors_and_builds_its_own_lawpack | crates/edict-cli/tests/jsonl_cli.rs, crates/edict-cli/tests/lawpack_authoring_cli.rs | Proves authoring and application compilation, not runtime execution. |

## Oracles

- Canonical bytes are decoded by `decode_lawpack_bundle` and `decode_lawpack_adapter`; a second authoring run must match them byte for byte.
- Digest sidecars must equal `digest_canonical_artifact` under the artifact's owning domain and end in one newline.
- Negative tests assert `LawpackAuthoringFailureKind` or the public CLI diagnostic kind, not prose.
- Publication tests compare the complete previous output tree after failure rather than inspecting transaction internals.
