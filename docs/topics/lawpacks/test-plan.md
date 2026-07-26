# Lawpacks Test Plan

Status: current verification design for the lawpack boundary.

## Scope

In scope:

- source-level lawpack import parsing and digest literal validation;
- canonical `edict.lawpack/v1` manifest and export-surface loading;
- digest binding between a manifest, its export surface, and source imports;
- closed validation of manifest dependencies, verifier classes, executable
  component bounds, target-adapter descriptors, exports, semantic-effect
  failures, obstruction schemas, and operation-profile optic templates;
- acyclic validation of a complete digest-locked lawpack dependency set;
- v1 target-profile rejection of deferred lawpack adapter ABI declarations;
- lowerability behavior for one-hop digest-locked direct adapters;
- contract-bundle handling of lawpack artifact references as external,
  participant-neutral resources.
- authority-facts documents whose source kind is `lawpack` for first compiler
  budget and effect write-class facts.
- provider manifests that describe lawpacks as generated, digest-locked
  provider artifacts with explicit provenance.

Out of scope:

- execution or semantic verification of target-adapter component bytes;
- a general target-adapter byte ABI beyond the digest-locked descriptor
  envelope owned by `edict.lawpack/v1`;
- lawpack conformance fixtures and differential lowerer trials.
- generating lawpacks from Wesley or runtime-owned semantic sources.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| LAWPACKS-REQ-001 | implemented | Source lawpack imports preserve kind, coordinate, version label, alias, and digest review string through the public parser. | docs/SPEC_edict-language-v1.md, crates/edict-syntax/src/parser.rs |
| LAWPACKS-REQ-002 | implemented | v1 lowerability supports at most one digest-locked direct adapter per semantic effect and rejects floating, chained, or ambiguous adapter claims. | crates/edict-syntax/src/lowerability.rs |
| LAWPACKS-REQ-003 | implemented | v1 target-profile validation rejects non-empty `accepted_lawpack_adapter_abi` declarations until the adapter ABI is supported. | crates/edict-syntax/src/target_profile.rs |
| LAWPACKS-REQ-004 | implemented | Contract-bundle validation treats lawpacks as external participant-neutral artifact references, not loaded or executed manifests. | crates/edict-syntax/src/contract_bundle.rs |
| LAWPACKS-REQ-005 | implemented | Edict loads canonical `edict.lawpack/v1` manifests and export surfaces into typed values, rejects every value outside the closed CDDL shape with stable failure kinds, corroborates the export digest, and validates a complete supplied dependency set as digest-locked and acyclic before exposing any exports to compilation. | issue #169, crates/edict-syntax/src/lawpack.rs, docs/abi/edict-lawpack.cddl, docs/abi/edict-common.cddl, docs/abi/edict-core.cddl |
| LAWPACKS-REQ-006 | implemented | Authority-facts loading accepts digest-locked `lawpack` source identity for first compiler budget and effect write-class facts without claiming full manifest validation. | docs/topics/authority-facts/test-plan.md |
| LAWPACKS-REQ-007 | implemented | Provider manifests model lawpacks as generated provider artifacts with digest-locked semantic source and generator provenance; Edict validates the reference/provenance envelope without owning runtime lawpack semantics. | issue #139, docs/topics/providers/test-plan.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/bounds/bounded-hello.edict | Lawpack import source fixture. | Parser preserves the `hello.optics@1` lawpack import and digest review string. |
| fixtures/lang/effects/read-greeting.edict | Multi-import source fixture. | Parser preserves shape, lawpack, and target imports for effect-call syntax. |
| fixtures/lawpack/hello-echo/README.md | Standalone capability fixture for the first real Edict-to-Echo crossing. | Canonical manifest and exports load with exact digests; `createGreeting` exposes a bounded create effect and typed `AlreadyExists` failure without GraphQL or a handwritten Echo package. |

## Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LAWPACKS-TP-001 | implemented | Source import | LAWPACKS-REQ-001 | Lawpack imports preserve version labels and valid digest strings, and invalid digest strings reject with a stable parser error kind. | bounded_hello_parses, read_greeting_parses, import_versions_preserve_underscore_labels, import_digest_literals_are_validated | fixtures/lang/bounds/bounded-hello.edict, fixtures/lang/effects/read-greeting.edict | Tests use the public parser and AST/error contract. |
| LAWPACKS-TP-002 | implemented | Lowerability | LAWPACKS-REQ-002 | Exactly one digest-locked direct adapter can satisfy a v1 semantic effect; floating, chained, and ambiguous adapters reject. | one_direct_adapter_satisfies_v1_lowering_requirements, v1_rejects_floating_direct_adapter_claims, v1_rejects_chained_adapter_claims, v1_rejects_ambiguous_direct_adapters | - | Tests assert lowerability classification and stable failure kinds. |
| LAWPACKS-TP-003 | implemented | Target profile | LAWPACKS-REQ-003 | A non-empty lawpack adapter ABI declaration rejects from v1 target-profile conformance. | deferred_lawpack_adapter_abi_must_stay_empty_in_v1 | - | Keeps the future adapter slot from becoming an implicit claim. |
| LAWPACKS-TP-004 | implemented | Contract bundle | LAWPACKS-REQ-004 | Runtime-neutral bundles can carry lawpack artifact references, and lawpacks remain optional artifact-list entries. | echo_and_kv_bundles_validate_with_the_same_runtime_neutral_contract, optional_artifact_lists_may_be_empty | - | Contract-bundle validation does not load lawpack manifests. |
| LAWPACKS-TP-005 | implemented | Manifest validation | LAWPACKS-REQ-005 | The Hello Echo bundle loads from canonical bytes; non-canonical or malformed values, unknown or missing fields, digest substitution, invalid identifiers or discriminants, unbounded executable components, runtime effects without target adapters, duplicate identities, missing dependencies, digest conflicts, and dependency cycles reject with stable failure kinds before exports become compiler facts. | hello_echo_lawpack_bundle_loads_from_exact_canonical_resources, all_hash_bound_helper_and_verifier_variants_load, noncanonical_manifest_bytes_reject_before_shape_validation, export_digest_substitution_rejects, runtime_effect_requires_at_least_one_target_adapter, effect_failure_names_must_be_source_mappable_identifiers, dependency_graph_requires_the_complete_exact_set_independent_of_input_order, dependency_graph_rejects_missing_and_substituted_manifests, dependency_graph_rejects_cycles_before_digest_corroboration | fixtures/lawpack/hello-echo/README.md, crates/edict-syntax/tests/lawpack.rs, xtask/src/lawpack_goldens.rs | `cargo xtask lawpack-goldens --check` reproduces exact manifest/export bytes and digests without rewriting them. |
| LAWPACKS-TP-006 | implemented | Authority facts | LAWPACKS-REQ-006 | A lawpack-sourced authority-facts file can provide budget and effect write-class facts consumed by the compiler. | file_backed_authority_facts_compile_bounded_hello, file_backed_authority_facts_reject_write_effect_profile_mismatch | crates/edict-syntax/tests/authority_facts.rs | Asserts compiler behavior, not manifest prose. |
| LAWPACKS-TP-007 | implemented | Provider provenance | LAWPACKS-REQ-007 | A provider manifest fixture can carry a generated lawpack artifact with digest-locked semantic source and generator provenance, while unlocked artifact/provenance references reject with stable provider validation failures. | generated_provider_manifest_fixture_validates, provider_manifest_rejects_unlocked_generated_artifact, provider_manifest_rejects_unlocked_generated_provenance, provider_manifest_rejects_unlocked_generator_provenance | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Provider validation is envelope/provenance validation only; no Echo semantics are interpreted. |

## Determinism Obligations

- Lawpack parser tests must assert AST fields or stable parser error kinds.
- Lowerability tests must assert public classification and failure kinds, not
  internal branch choices.
- Contract-bundle tests must assert validation behavior, not the text of the
  lawpack ABI specification.
- Manifest tests must enter through exact canonical bytes and assert typed
  exports or stable failure kinds. Constructing a typed manifest directly does
  not prove the loader boundary.
- Dependency validation must be input-order invariant and must bind each edge
  to the exact supplied manifest digest.

## Open Gaps

- No lawpack target adapter ABI is accepted in v1 target-profile manifests.
- No target-adapter component is executed or semantically verified by Edict.
