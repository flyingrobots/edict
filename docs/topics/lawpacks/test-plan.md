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
- v1 target-profile acceptance of the exact direct lawpack adapter ABI and
  rejection of unknown or duplicate declarations;
- canonical direct-adapter loading, digest corroboration, complete semantic
  closure, typed target-configuration references, and compiler/Target IR fact
  derivation;
- canonical compiler-produced Core and Target IR fixture bytes with their
  native domain-framed identities;
- lowerability behavior for one-hop digest-locked direct adapters;
- contract-bundle handling of lawpack artifact references as external,
  participant-neutral resources.
- authority-facts documents whose source kind is `lawpack` for first compiler
  budget and effect write-class facts.
- provider manifests that describe lawpacks as generated, digest-locked
  provider artifacts with explicit provenance.

Out of scope:

- executable target-adapter component loading;
- general or chained target-adapter composition beyond the direct declarative
  `edict.lawpack-adapter/v1` ABI;
- lawpack conformance fixtures and differential lowerer trials.
- generating lawpacks from Wesley or runtime-owned semantic sources.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| LAWPACKS-REQ-001 | implemented | Source lawpack imports preserve kind, coordinate, version label, alias, and digest review string through the public parser. | docs/SPEC_edict-language-v1.md, crates/edict-syntax/src/parser.rs |
| LAWPACKS-REQ-002 | implemented | v1 lowerability supports at most one digest-locked direct adapter per semantic effect and rejects floating, chained, or ambiguous adapter claims. | crates/edict-syntax/src/lowerability.rs |
| LAWPACKS-REQ-003 | implemented | v1 target-profile validation accepts an absent/empty adapter ABI list or exactly `["edict.lawpack-adapter/v1"]` and rejects unknown or duplicate declarations. | crates/edict-syntax/src/target_profile.rs |
| LAWPACKS-REQ-004 | implemented | Contract-bundle validation treats lawpacks as external participant-neutral artifact references, not loaded or executed manifests. | crates/edict-syntax/src/contract_bundle.rs |
| LAWPACKS-REQ-005 | implemented | Edict loads canonical `edict.lawpack/v1` manifests and export surfaces into typed values, rejects every value outside the closed CDDL shape or any callable coordinate shared by a pure helper and semantic effect with stable failure kinds, corroborates the export digest, and validates a complete supplied dependency set as digest-locked and acyclic before exposing any exports to compilation. | issue #169, crates/edict-syntax/src/lawpack.rs, docs/abi/edict-lawpack.cddl, docs/abi/edict-common.cddl, docs/abi/edict-core.cddl |
| LAWPACKS-REQ-006 | implemented | Authority-facts loading accepts digest-locked `lawpack` source identity for first compiler budget and effect write-class facts without claiming full manifest validation. | docs/topics/authority-facts/test-plan.md |
| LAWPACKS-REQ-007 | implemented | Provider manifests model lawpacks as generated provider artifacts with digest-locked semantic source and generator provenance; Edict validates the reference/provenance envelope without owning runtime lawpack semantics. | issue #139, docs/topics/providers/test-plan.md |
| LAWPACKS-REQ-008 | implemented | Edict validates one exact direct declarative `edict.lawpack-adapter/v1` resource selected by a loaded lawpack manifest. Callable profiles require complete effect/budget coverage and one typed target-configuration reference per runtime effect. Request-only profiles carry no semantic effects, bind their own exact budget obligation and target configuration, and reject source selecting another profile's budget. Edict preserves but does not interpret target-owned configuration semantics. | issue #169, issue #176, docs/abi/edict-lawpack-adapter.cddl |
| LAWPACKS-REQ-009 | implemented | The standalone Hello Echo fixture pins exact canonical Core and Target IR bytes produced from the digest-locked source/lawpack/adapter closure and computes each identity with the artifact's native domain. | issue #169, fixtures/lawpack/hello-echo/README.md, xtask/src/lawpack_goldens.rs |
| LAWPACKS-REQ-010 | implemented | The portable `causal.cell@1.createIfAbsent` capability closure is generated through the executable lawpack, adapter, compiler, and Target IR path, with exact canonical manifest, export, adapter, and target-configuration bytes and digests for external application builds. | fixtures/lawpack/causal-cell/README.md, xtask/src/lawpack_goldens.rs |
| LAWPACKS-REQ-011 | implemented | A request-only lawpack profile supplies an exact compiler budget and opaque target configuration without declaring a callable semantic effect or target intrinsic; the workspace-snapshot closure reproduces one request and zero Target IR steps. | issue #176, fixtures/lawpack/workspace-snapshot/README.md |
| LAWPACKS-REQ-012 | implemented | Exact lawpack preparation projects exported pure-helper signatures through the source import alias while preserving the canonical export coordinate in Core and the manifest digest as implementation identity. | issue #192, crates/edict-syntax/src/lawpack_adapter.rs |
| LAWPACKS-REQ-013 | implemented | Exact lawpack preparation projects exported `U32` and `U64` constants as numeric compiler-bound facts through the source import alias while preserving the canonical export coordinate in Core. | issue #192, crates/edict-syntax/src/lawpack_adapter.rs |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/bounds/bounded-hello.edict | Lawpack import source fixture. | Parser preserves the `hello.optics@1` lawpack import and digest review string. |
| fixtures/lang/effects/read-greeting.edict | Multi-import source fixture. | Parser preserves shape, lawpack, and target imports for effect-call syntax. |
| fixtures/lawpack/hello-echo/README.md | Standalone capability fixture for the first real Edict-to-Echo crossing. | Canonical manifest, exports, and adapter load with exact digests; exact source compiles to pinned canonical Core and Target IR; `createGreeting` exposes a bounded create effect and typed `AlreadyExists` failure without GraphQL or a handwritten Echo package. |
| fixtures/lawpack/causal-cell/README.md | Portable capability closure for external application builds. | `cargo xtask lawpack-goldens --check` reproduces the exact canonical closure after validating the bundle and adapter and compiling a source witness through Target IR. |
| fixtures/lawpack/workspace-snapshot/README.md | Request-only capability closure for bounded workspace observation. | `cargo xtask lawpack-goldens --check` reproduces the exact closure and requires one external request with zero callable target steps. |

## Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LAWPACKS-TP-001 | implemented | Source import | LAWPACKS-REQ-001 | Lawpack imports preserve version labels and valid digest strings, and invalid digest strings reject with a stable parser error kind. | bounded_hello_parses, read_greeting_parses, import_versions_preserve_underscore_labels, import_digest_literals_are_validated | fixtures/lang/bounds/bounded-hello.edict, fixtures/lang/effects/read-greeting.edict | Tests use the public parser and AST/error contract. |
| LAWPACKS-TP-002 | implemented | Lowerability | LAWPACKS-REQ-002 | Exactly one digest-locked direct adapter can satisfy a v1 semantic effect; floating, chained, and ambiguous adapters reject. | one_direct_adapter_satisfies_v1_lowering_requirements, v1_rejects_floating_direct_adapter_claims, v1_rejects_chained_adapter_claims, v1_rejects_ambiguous_direct_adapters | - | Tests assert lowerability classification and stable failure kinds. |
| LAWPACKS-TP-003 | implemented | Target profile | LAWPACKS-REQ-003 | The exact direct adapter ABI is accepted; unknown and duplicate declarations reject from v1 target-profile conformance. | direct_lawpack_adapter_abi_is_supported_in_v1, unknown_or_duplicate_lawpack_adapter_abis_are_rejected | - | Keeps target compatibility closed over the implemented ABI. |
| LAWPACKS-TP-004 | implemented | Contract bundle | LAWPACKS-REQ-004 | Runtime-neutral bundles can carry lawpack artifact references, and lawpacks remain optional artifact-list entries. | echo_and_kv_bundles_validate_with_the_same_runtime_neutral_contract, optional_artifact_lists_may_be_empty | - | Contract-bundle validation does not load lawpack manifests. |
| LAWPACKS-TP-005 | implemented | Manifest validation | LAWPACKS-REQ-005 | The Hello Echo bundle loads from canonical bytes; non-canonical or malformed values, unknown or missing fields, digest substitution, invalid identifiers or discriminants, unbounded executable components, runtime effects without target adapters, duplicate category identities, cross-category callable collisions, missing dependencies, digest conflicts, and dependency cycles reject with stable failure kinds before exports become compiler facts. | hello_echo_lawpack_bundle_loads_from_exact_canonical_resources, all_hash_bound_helper_and_verifier_variants_load, noncanonical_manifest_bytes_reject_before_shape_validation, export_digest_substitution_rejects, runtime_effect_requires_at_least_one_target_adapter, effect_failure_names_must_be_source_mappable_identifiers, duplicate_export_coordinates_reject_within_their_category, callable_export_coordinates_must_be_disjoint, dependency_graph_requires_the_complete_exact_set_independent_of_input_order, dependency_graph_rejects_missing_and_substituted_manifests, dependency_graph_rejects_cycles_before_digest_corroboration | fixtures/lawpack/hello-echo/README.md, crates/edict-syntax/tests/lawpack.rs, xtask/src/lawpack_goldens.rs | `cargo xtask lawpack-goldens --check` reproduces exact manifest/export bytes and digests without rewriting them. |
| LAWPACKS-TP-006 | implemented | Authority facts | LAWPACKS-REQ-006 | A lawpack-sourced authority-facts file can provide budget and effect write-class facts consumed by the compiler. | file_backed_authority_facts_compile_bounded_hello, file_backed_authority_facts_reject_write_effect_profile_mismatch | crates/edict-syntax/tests/authority_facts.rs | Asserts compiler behavior, not manifest prose. |
| LAWPACKS-TP-007 | implemented | Provider provenance | LAWPACKS-REQ-007 | A provider manifest fixture can carry a generated lawpack artifact with digest-locked semantic source and generator provenance, while unlocked artifact/provenance references reject with stable provider validation failures. | generated_provider_manifest_fixture_validates, provider_manifest_rejects_unlocked_generated_artifact, provider_manifest_rejects_unlocked_generated_provenance, provider_manifest_rejects_unlocked_generator_provenance | fixtures/providers/echo-generated/provider-manifest.json, crates/edict-syntax/tests/provider.rs | Provider validation is envelope/provenance validation only; no Echo semantics are interpreted. |
| LAWPACKS-TP-008 | implemented | Direct adapter | LAWPACKS-REQ-008 | The exact Hello Echo adapter selected by the manifest derives all compiler and Echo Target IR facts and exposes the exact target-configuration resource identity, while missing, substituted, non-canonical, incomplete, target-mismatched, import-mismatched, malformed-configuration, undeclared-write-class, or obligation-mismatched adapters fail closed before trusted compiler facts exist. | hello_echo_source_compiles_to_echo_target_ir_from_exact_lawpack_adapter, lawpack_adapter_bytes_must_be_canonical_and_digest_bound, lawpack_adapter_requires_a_typed_target_configuration_reference, lawpack_adapter_rejects_an_undeclared_write_class_at_the_effect_path, lawpack_adapter_selection_requires_one_exact_target_profile, lawpack_adapter_requires_complete_exported_effect_coverage, lawpack_adapter_corroborates_footprint_cost_and_failure_obligations, lawpack_compilation_requires_the_exact_digest_locked_source_import | fixtures/lawpack/hello-echo/README.md, crates/edict-syntax/tests/lawpack.rs | The positive test constructs no `CompilerContext` or `TargetIrLoweringFacts`; Echo-specific configuration interpretation remains outside Edict. |
| LAWPACKS-TP-009 | implemented | Compiler artifacts | LAWPACKS-REQ-009 | Compiling and lowering the exact Hello Echo closure reproduces the reviewed Core and Target IR bytes and their native domain-framed identities. | hello_echo_source_compiles_to_echo_target_ir_from_exact_lawpack_adapter | fixtures/lawpack/hello-echo/create-greeting.core.cbor, fixtures/lawpack/hello-echo/create-greeting.target-ir.cbor, crates/edict-syntax/tests/lawpack.rs, xtask/src/lawpack_goldens.rs | The fixtures are outputs of the real compiler pipeline, not handwritten substitutes; `cargo xtask lawpack-goldens --check` reproduces them. |
| LAWPACKS-TP-010 | implemented | Portable capability | LAWPACKS-REQ-010 | Generating the causal-cell closure validates its canonical lawpack and direct adapter, then compiles and lowers an Edict source witness that imports the exact generated manifest digest. | lawpack_goldens_match_executable_codec | fixtures/lawpack/causal-cell/README.md, xtask/src/lawpack_goldens.rs, xtask/src/tests.rs | The generator fails if the portable capability no longer reaches a compiler-produced Target IR artifact. |
| LAWPACKS-TP-011 | implemented | Request-only profile | LAWPACKS-REQ-008, LAWPACKS-REQ-011 | A profile with no semantic effects is accepted only when it binds an exact budget obligation and target configuration; it compiles one request without conferring target-call authority and rejects another profile's budget. | request_only_profile_supplies_budget_without_callable_effect_authority, request_only_profile_requires_an_exact_budget_obligation, request_only_profile_requires_an_exact_target_configuration, request_only_profile_rejects_another_profiles_budget | crates/edict-syntax/tests/lawpack.rs, fixtures/lawpack/workspace-snapshot/README.md | Empty semantic effects and adapter-wide budget availability are not authority escape hatches. |
| LAWPACKS-TP-012 | implemented | Pure-helper compilation | LAWPACKS-REQ-005, LAWPACKS-REQ-012 | A canonical lawpack with one hash-bound Edict helper derives a module-local compiler fact owned by the exact imported manifest; primitive, bounded, aliased, and nested exported-type signatures lower through that same lawpack closure without caller-authored facts; a missing nested type, an exported type outside the lawpack namespace, and a declared generic helper refuse before Core. | exact_lawpack_pure_helper_signature_enters_source_compilation, exact_lawpack_exported_type_enters_pure_helper_signature_closure, nested_exported_type_closure_enters_pure_helper_compilation, exported_type_outside_lawpack_namespace_rejects_before_compiler_facts, generic_lawpack_helper_rejects_before_core | crates/edict-syntax/tests/lawpack.rs | The nested closure test has both positive and missing-dependency arms. Signature, declared genericity, recursively bounded type closure, identity routing, and separately tested compiler cost charging are implemented; target execution remains separate. |
| LAWPACKS-TP-013 | implemented | Constant-bound compilation | LAWPACKS-REQ-005, LAWPACKS-REQ-013 | Canonical lawpacks with exported `U32` or `U64` constants derive module-local loop-bound facts; out-of-domain `U32` values and operation-budget violations reject before Core. | exact_lawpack_constant_enters_loop_bound_compilation, exact_lawpack_u32_constant_enters_loop_bound_compilation, exact_lawpack_numeric_constants_reject_type_and_budget_violations | crates/edict-syntax/tests/lawpack.rs | The constant value is used for static proof and budget checks; Core preserves the export coordinate as semantic identity. |

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

- No executable target-adapter component is loaded or semantically verified by
  Edict; v1 implements only the direct declarative adapter ABI.
- Runtime admission and execution of compiler-emitted packages remain outside
  Edict.
