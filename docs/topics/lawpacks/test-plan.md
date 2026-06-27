# Lawpacks Test Plan

Status: current verification design for the lawpack boundary.

## Scope

In scope:

- source-level lawpack import parsing and digest literal validation;
- v1 target-profile rejection of deferred lawpack adapter ABI declarations;
- lowerability behavior for one-hop digest-locked direct adapters;
- contract-bundle handling of lawpack artifact references as external,
  participant-neutral resources.
- authority-facts documents whose source kind is `lawpack` for first compiler
  budget and effect write-class facts.

Out of scope:

- full `edict.lawpack/v1` manifest loading;
- lawpack export-surface validation;
- lawpack dependency DAG validation;
- target adapter ABI validation;
- lawpack conformance fixtures and differential lowerer trials.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| LAWPACKS-REQ-001 | implemented | Source lawpack imports preserve kind, coordinate, version label, alias, and digest review string through the public parser. | docs/SPEC_edict-language-v1.md, crates/edict-syntax/src/parser.rs |
| LAWPACKS-REQ-002 | implemented | v1 lowerability supports at most one digest-locked direct adapter per semantic effect and rejects floating, chained, or ambiguous adapter claims. | crates/edict-syntax/src/lowerability.rs |
| LAWPACKS-REQ-003 | implemented | v1 target-profile validation rejects non-empty `accepted_lawpack_adapter_abi` declarations until the adapter ABI is supported. | crates/edict-syntax/src/target_profile.rs |
| LAWPACKS-REQ-004 | implemented | Contract-bundle validation treats lawpacks as external participant-neutral artifact references, not loaded or executed manifests. | crates/edict-syntax/src/contract_bundle.rs |
| LAWPACKS-REQ-005 | gap | The `edict.lawpack/v1` CDDL manifest and export surface have no executable instance validator yet. | docs/abi/edict-lawpack.cddl, docs/SPEC_edict-lawpack-abi-v1.md |
| LAWPACKS-REQ-006 | implemented | Authority-facts loading accepts digest-locked `lawpack` source identity for first compiler budget and effect write-class facts without claiming full manifest validation. | docs/topics/authority-facts/test-plan.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/bounds/bounded-hello.edict | Lawpack import source fixture. | Parser preserves the `hello.optics@1` lawpack import and digest review string. |
| fixtures/lang/effects/read-greeting.edict | Multi-import source fixture. | Parser preserves shape, lawpack, and target imports for effect-call syntax. |

## Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LAWPACKS-TP-001 | implemented | Source import | LAWPACKS-REQ-001 | Lawpack imports preserve version labels and valid digest strings, and invalid digest strings reject with a stable parser error kind. | bounded_hello_parses, read_greeting_parses, import_versions_preserve_underscore_labels, import_digest_literals_are_validated | fixtures/lang/bounds/bounded-hello.edict, fixtures/lang/effects/read-greeting.edict | Tests use the public parser and AST/error contract. |
| LAWPACKS-TP-002 | implemented | Lowerability | LAWPACKS-REQ-002 | Exactly one digest-locked direct adapter can satisfy a v1 semantic effect; floating, chained, and ambiguous adapters reject. | one_direct_adapter_satisfies_v1_lowering_requirements, v1_rejects_floating_direct_adapter_claims, v1_rejects_chained_adapter_claims, v1_rejects_ambiguous_direct_adapters | - | Tests assert lowerability classification and stable failure kinds. |
| LAWPACKS-TP-003 | implemented | Target profile | LAWPACKS-REQ-003 | A non-empty lawpack adapter ABI declaration rejects from v1 target-profile conformance. | deferred_lawpack_adapter_abi_must_stay_empty_in_v1 | - | Keeps the future adapter slot from becoming an implicit claim. |
| LAWPACKS-TP-004 | implemented | Contract bundle | LAWPACKS-REQ-004 | Runtime-neutral bundles can carry lawpack artifact references, and lawpacks remain optional artifact-list entries. | echo_and_kv_bundles_validate_with_the_same_runtime_neutral_contract, optional_artifact_lists_may_be_empty | - | Contract-bundle validation does not load lawpack manifests. |
| LAWPACKS-TP-005 | gap | Manifest validation | LAWPACKS-REQ-005 | No executable lawpack manifest instance validator is claimed. | - | - | Add with lawpack loading or schema-validation work. |
| LAWPACKS-TP-006 | implemented | Authority facts | LAWPACKS-REQ-006 | A lawpack-sourced authority-facts file can provide budget and effect write-class facts consumed by the compiler. | file_backed_authority_facts_compile_bounded_hello, file_backed_authority_facts_reject_write_effect_profile_mismatch | crates/edict-syntax/tests/authority_facts.rs | Asserts compiler behavior, not manifest prose. |

## Determinism Obligations

- Lawpack parser tests must assert AST fields or stable parser error kinds.
- Lowerability tests must assert public classification and failure kinds, not
  internal branch choices.
- Contract-bundle tests must assert validation behavior, not the text of the
  lawpack ABI specification.

## Open Gaps

- No executable full lawpack manifest validator exists.
- No checked-in lawpack fixture corpus exists.
- No lawpack target adapter ABI is accepted in v1 target-profile manifests.
