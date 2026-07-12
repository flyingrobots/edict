# Providers Test Plan

Status: current verification design for provider manifest validation and
built-in lowerer compatibility.

## Scope

In scope:

- typed `TargetProviderManifest` values;
- provider artifact role classification;
- generated artifact provenance;
- component provenance;
- digest-locked provider and artifact references;
- stable provider manifest validation failures;
- explicit in-process compatibility invocation of the existing Echo and
  git-warp lowerers.

Out of scope:

- provider file discovery or package loading;
- WIT component loading;
- manifest-backed provider resolution or external provider dispatch;
- provider verifier execution;
- lawpack generation through Wesley;
- Echo-specific provider semantics;
- runtime execution;
- admission or registration.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| PROVIDERS-REQ-001 | implemented | Provider manifests describe an explicit nonempty set of generated-artifact and/or provider-owned component entries under API version `edict.provider-manifest/v1`. | issue #139, docs/design/provider-artifact-pipeline-alpha.md |
| PROVIDERS-REQ-002 | implemented | Provider, artifact, semantic-source, generator, and component references must be digest-locked with lowercase `sha256:<64 hex>` review strings. | issue #139 |
| PROVIDERS-REQ-003 | implemented | Generated metadata artifact roles must carry generated provenance; lowerer and verifier component roles must carry component provenance. | issue #139 |
| PROVIDERS-REQ-004 | implemented | Artifact role slots must be non-empty and unique so lookup by role is unambiguous within one manifest. Deterministic provider and lowerer selection remains a resolver obligation. | issue #139 |
| PROVIDERS-REQ-005 | policy | Provider manifest validation does not interpret runtime semantics, and the built-in compatibility adapter adds no semantic interpretation or reclassification beyond the existing target lowerers. Neither surface loads external components, runs verifiers, admits bundles, or executes runtimes. | docs/design/provider-artifact-pipeline-alpha.md |
| PROVIDERS-REQ-006 | implemented | `edict_syntax` exposes a borrowed built-in-lowerer request and explicit Echo/git-warp lowerer selection as an in-process migration adapter over existing Core and target-lowering facts. It does not define a public provider trait, manifest-backed resolution, or WIT dispatch. | issue #140, docs/design/provider-artifact-pipeline-alpha.md |
| PROVIDERS-REQ-007 | implemented | Each built-in lowerer is bound to its declared target-profile coordinate. A selection mismatch returns a stable lowerer-compatibility failure, while matched-profile target-lowering failures pass through without reclassification. | issue #140 |
| PROVIDERS-REQ-008 | implemented | For identical Core and lowering facts, the built-in lowerer compatibility adapter preserves the direct target-lowering report, Target IR artifact, canonical bytes, and digest for Echo and git-warp. | issue #140 |
| PROVIDERS-REQ-009 | implemented | A built-in-lowerer Target IR artifact preserves direct-path semantic and release bundle identity when every explicit assembly input, including lowerer identity, is unchanged; changing only lowerer identity changes release identity, not semantic identity. | issue #140, docs/topics/contract-bundles/README.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/providers/echo-generated/provider-manifest.json | Representative provider manifest envelope using Echo-shaped coordinates. | Deserializes as `TargetProviderManifest` and validates through `validate_target_provider_manifest` without interpreting Echo semantics. |
| crates/edict-syntax/tests/provider_lowering.rs | In-memory Core and target-lowering facts exercised through direct and explicit built-in lowerer paths. | Reports, artifacts, canonical Target IR bytes, digests, structured failures, and bundle identities remain equal when their semantic and release inputs remain equal. |

## Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PROVIDERS-TP-001 | implemented | Golden path | PROVIDERS-REQ-001, PROVIDERS-REQ-002, PROVIDERS-REQ-003, PROVIDERS-REQ-004 | The checked-in provider manifest fixture validates with status `Valid` and no failures. | generated_provider_manifest_fixture_validates | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Echo-shaped coordinates are envelope data only. |
| PROVIDERS-TP-002 | implemented | Boundary guard | PROVIDERS-REQ-002 | Removing provider, artifact, generator, or component digests rejects with the matching structured digest-lock failure; uppercase generated source digests reject with `NonDigestLockedGeneratedSource`. | provider_manifest_rejects_unlocked_provider, provider_manifest_rejects_unlocked_generated_artifact, provider_manifest_rejects_unlocked_generated_provenance, provider_manifest_rejects_unlocked_generator_provenance, provider_manifest_rejects_unlocked_component | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Provider artifact references use lowercase-strict digest locks. |
| PROVIDERS-TP-003 | implemented | Boundary guard | PROVIDERS-REQ-003 | A lowerer role with generated provenance rejects with `ComponentRoleRequiresComponentSource`; a lawpack role with component provenance rejects with `GeneratedRoleRequiresGeneratedSource`. | provider_manifest_rejects_generated_component_roles, provider_manifest_rejects_component_metadata_roles | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Prevents generated metadata and executable components from collapsing. |
| PROVIDERS-TP-004 | implemented | Boundary guard | PROVIDERS-REQ-004 | Empty role slots reject with `MissingRole`; reusing a role slot rejects with `DuplicateArtifactRole`. | provider_manifest_rejects_empty_artifact_role, provider_manifest_rejects_duplicate_artifact_roles | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Role identity is package-routing evidence, not runtime semantics. |
| PROVIDERS-TP-005 | policy | Authority boundary | PROVIDERS-REQ-005 | Review confirms the provider surface is limited to manifest envelopes, provenance, and delegation to existing in-tree lowerers; the adapter adds no semantic interpretation or reclassification beyond those lowerers. External component loading, verifier execution, admission, and runtime execution remain out of scope. | - | docs/design/provider-artifact-pipeline-alpha.md, docs/topics/providers/README.md | Policy row; behavior changes still require executable tests. |
| PROVIDERS-TP-006 | implemented | Boundary guard | PROVIDERS-REQ-001 | Removing every artifact from the provider manifest rejects with `MissingArtifact`. | provider_manifest_rejects_missing_artifacts | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Empty provider manifests cannot stand in for an explicit provider artifact set. |
| PROVIDERS-TP-007 | implemented | Boundary guard | PROVIDERS-REQ-001 | An unsupported provider-manifest API version rejects with `InvalidApiVersion`. | provider_manifest_rejects_unknown_api_version | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Provider ABI changes require explicit version support. |
| PROVIDERS-TP-008 | implemented | Integration | PROVIDERS-REQ-006, PROVIDERS-REQ-008 | Explicit built-in Echo and git-warp lowerers return reports and artifacts identical to direct lowering, including canonical Target IR bytes and digests. | builtin_echo_lowerer_matches_direct_target_ir, builtin_gitwarp_lowerer_matches_direct_target_ir | crates/edict-syntax/tests/provider_lowering.rs | The compatibility API is an in-process migration seam, not the external provider ABI. |
| PROVIDERS-TP-009 | implemented | Boundary guard | PROVIDERS-REQ-007 | Cross-profile built-in lowerer selection returns `TargetProfileMismatch`; matched-profile invalid facts and target-profile digests preserve the direct structured target-lowering report exactly. | builtin_lowerers_reject_mismatched_target_profiles, builtin_lowerers_preserve_structured_lowering_failures, builtin_lowerers_preserve_target_profile_digest_failures | crates/edict-syntax/tests/provider_lowering.rs | Lowerer selection failure remains distinct from a lowerer's typed refusal, and coordinate matching does not bypass digest validation. |
| PROVIDERS-TP-010 | implemented | Integration | PROVIDERS-REQ-009 | Direct and built-in-lowerer artifacts produce identical semantic and release bundle identities under identical explicit inputs; changing only lowerer identity preserves semantic identity and changes release identity. | builtin_lowerer_bundles_preserve_semantic_and_release_identity, changing_builtin_lowerer_identity_changes_only_release_identity | crates/edict-syntax/tests/provider_lowering.rs | Bundle assembly consumes artifacts and explicit identities; it does not invoke providers. |

## Determinism Obligations

- Provider validation tests use checked-in JSON fixtures and typed Rust values.
- Built-in lowerer compatibility tests use in-memory Core and explicit target
  facts, then compare structured reports and canonical Target IR artifacts.
- Assertions use structured status and failure kinds.
- Tests do not inspect stdout, stderr, diagnostic prose, filesystem ordering,
  network state, wall-clock time, random values, or runtime behavior.

## Open Gaps

- No provider package loader exists.
- No manifest-backed provider resolver or external provider dispatch exists.
- No WIT provider host exists.
- No Echo-owned provider implementation exists.
