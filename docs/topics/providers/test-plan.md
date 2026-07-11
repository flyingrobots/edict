# Providers Test Plan

Status: current verification design for provider manifest validation.

## Scope

In scope:

- typed `TargetProviderManifest` values;
- provider artifact role classification;
- generated artifact provenance;
- component provenance;
- digest-locked provider and artifact references;
- stable provider manifest validation failures.

Out of scope:

- provider file discovery or package loading;
- WIT component loading;
- target lowering through providers;
- provider verifier execution;
- lawpack generation through Wesley;
- Echo-specific provider semantics;
- runtime execution;
- admission or registration.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| PROVIDERS-REQ-001 | implemented | Provider manifests describe provider packages as assembled generated artifacts plus provider-owned components under API version `edict.provider-manifest/v1`. | issue #139, docs/design/provider-artifact-pipeline-alpha.md |
| PROVIDERS-REQ-002 | implemented | Provider, artifact, semantic-source, generator, and component references must be digest-locked with lowercase `sha256:<64 hex>` review strings. | issue #139 |
| PROVIDERS-REQ-003 | implemented | Generated metadata artifact roles must carry generated provenance; lowerer and verifier component roles must carry component provenance. | issue #139 |
| PROVIDERS-REQ-004 | implemented | Artifact role slots must be non-empty and unique so provider package routing is deterministic. | issue #139 |
| PROVIDERS-REQ-005 | policy | Provider validation does not interpret runtime semantics, execute lowerers, run verifiers, admit bundles, or execute runtimes. | docs/design/provider-artifact-pipeline-alpha.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/providers/echo-generated/provider-manifest.json | Representative provider manifest envelope using Echo-shaped coordinates. | Deserializes as `TargetProviderManifest` and validates through `validate_target_provider_manifest` without interpreting Echo semantics. |

## Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| PROVIDERS-TP-001 | implemented | Golden path | PROVIDERS-REQ-001, PROVIDERS-REQ-002, PROVIDERS-REQ-003, PROVIDERS-REQ-004 | The checked-in provider manifest fixture validates with status `Valid` and no failures. | generated_provider_manifest_fixture_validates | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Echo-shaped coordinates are envelope data only. |
| PROVIDERS-TP-002 | implemented | Boundary guard | PROVIDERS-REQ-002 | Removing an artifact digest rejects with `NonDigestLockedArtifact`; uppercase generated source digests reject with `NonDigestLockedGeneratedSource`; removing a generator digest rejects with `NonDigestLockedGenerator`. | provider_manifest_rejects_unlocked_generated_artifact, provider_manifest_rejects_unlocked_generated_provenance, provider_manifest_rejects_unlocked_generator_provenance | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Provider artifact references use lowercase-strict digest locks. |
| PROVIDERS-TP-003 | implemented | Boundary guard | PROVIDERS-REQ-003 | A lowerer role with generated provenance rejects with `ComponentRoleRequiresComponentSource`; a lawpack role with component provenance rejects with `GeneratedRoleRequiresGeneratedSource`. | provider_manifest_rejects_generated_component_roles, provider_manifest_rejects_component_metadata_roles | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Prevents generated metadata and executable components from collapsing. |
| PROVIDERS-TP-004 | implemented | Boundary guard | PROVIDERS-REQ-004 | Reusing a role slot rejects with `DuplicateArtifactRole`. | provider_manifest_rejects_duplicate_artifact_roles | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Role uniqueness is package-routing evidence, not runtime semantics. |
| PROVIDERS-TP-006 | implemented | Boundary guard | PROVIDERS-REQ-001 | Removing every artifact from the provider manifest rejects with `MissingArtifact`. | provider_manifest_rejects_missing_artifacts | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Empty provider manifests cannot stand in for an assembled provider package. |
| PROVIDERS-TP-005 | policy | Authority boundary | PROVIDERS-REQ-005 | Review confirms provider validation is limited to package envelopes and provenance, with runtime semantics, verifier execution, admission, and runtime execution explicitly out of scope. | - | docs/design/provider-artifact-pipeline-alpha.md, docs/topics/providers/README.md | Policy row; behavior changes still require executable tests. |

## Determinism Obligations

- Provider validation tests use checked-in JSON fixtures and typed Rust values.
- Assertions use structured status and failure kinds.
- Tests do not inspect stdout, stderr, diagnostic prose, filesystem ordering,
  network state, wall-clock time, random values, or runtime behavior.

## Open Gaps

- No provider package loader exists.
- No provider-shaped target lowerer invocation exists.
- No WIT provider host exists.
- No Echo-owned provider implementation exists.
