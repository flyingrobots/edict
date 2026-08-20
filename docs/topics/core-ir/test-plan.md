# Core IR Test Plan

Status: current verification design for the Core semantic model, CDDL schema,
canonical encoder, and reviewed Core golden artifacts.

## Scope

In scope:

- `edict.core/v1` semantic model boundaries;
- normative CDDL declarations in `docs/abi/edict-core.cddl`;
- local-reference normalization shape;
- input-constraint and predicate-tree shape;
- reference canonical encoder and digest behavior for the current in-memory Core
  module model;
- require-failure arm encoding for terminal obstruction and preserved
  obstruction continuation;
- canonical byte validation through decode and re-encode stability, including a
  deterministic nesting limit for adversarial canonical values;
- reviewed Core golden bytes and exact digest fixtures produced from the
  executable encoder;
- explicit non-claim that the Core module schema freezes no self-hash, target
  IR, or admission bundles;
- deterministic contract metadata linking this shelf to executable tests.

Out of scope:

- full source-to-Core language coverage;
- target-profile lowering;
- bundle/admission validation.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| COREIR-REQ-001 | implemented | Core modules are semantic artifacts with `apiVersion: "edict.core/v1"`, imports, types, intents, and required Core capabilities. | docs/abi/edict-core.cddl, issue #3 |
| COREIR-REQ-002 | implemented | Core type schema covers bounded scalars, records, variants, options, lists, maps, and capability references. | docs/abi/edict-core.cddl, issue #3 |
| COREIR-REQ-003 | implemented | Core expressions and predicates are separate schema families. | docs/abi/edict-core.cddl, issue #3 |
| COREIR-REQ-004 | implemented | Core blocks/nodes represent ordered semantic execution with locals, effects, guards, branches, bounded loops, match blocks, proof obligations, and a result expression. | docs/abi/edict-core.cddl, issue #3 |
| COREIR-REQ-005 | implemented | Local references carry alpha-stable identity through `local-ref` with `alphaName`; source binder spelling is not identity. | docs/abi/edict-core.cddl, issue #3 |
| COREIR-REQ-006 | implemented | Input constraints carry predicate trees, not validator coordinate strings. | docs/abi/edict-core.cddl, EDICT-CORE-WHERE-HASH-001 |
| COREIR-REQ-007 | implemented | The Core module schema does not embed a self-hash, reviewed golden bytes, exact Core digests, target IR, or admission bundles. | ROADMAP.md, docs/abi/edict-core.cddl |
| COREIR-REQ-008 | implemented | `docs/abi/edict-core.cddl` is the normative machine-readable schema for `edict.core/v1`. | docs/abi/edict-core.cddl, issue #19 |
| COREIR-REQ-009 | implemented | Core states the required operation profile; verified operation mode is external verifier/admission evidence, not a Core field. | docs/abi/edict-core.cddl, EDICT-CORE-VERIFIED-EXTERNAL-001 |
| COREIR-REQ-010 | implemented | Accepted and rejected Core schema-shape fixtures are checked against the CDDL declarations. | docs/abi/edict-core.cddl, issue #19 |
| COREIR-REQ-011 | implemented | Edict-authored lawpack pure helper bodies use a pure `core-fn-body` shape, not the effect-capable `core-block` node algebra. | docs/abi/edict-core.cddl, docs/abi/edict-lawpack.cddl |
| COREIR-REQ-012 | implemented | Core modules have a reference `edict.canonical-cbor/v1` encoder that emits deterministic bytes from semantic Core values. | issue #21, crates/edict-syntax/src/canonical.rs |
| COREIR-REQ-013 | implemented | Canonical Core bytes can be decoded to a canonical value and re-encoded without byte changes; non-canonical encodings reject. | issue #21, crates/edict-syntax/src/canonical.rs |
| COREIR-REQ-014 | implemented | Reviewed Core golden byte fixtures and exact digest fixtures are produced from the executable canonical encoder and checked for stability. | issue #22, docs/SPEC_continuum-contract-bundle-v1.md |
| COREIR-REQ-015 | implemented | The Rust Core IR model and reference canonical encoder represent the first supported semantic effect-node shape. | issue #62, docs/abi/edict-core.cddl |
| COREIR-REQ-016 | implemented | The Rust Core IR model and reference canonical encoder represent `require` nodes with distinct terminal and preserved-obstruction failure arms. | issue #129, docs/abi/edict-core.cddl |
| COREIR-REQ-017 | implemented | Canonical encoding and decoding accept values through the public `MAX_CANONICAL_NESTING_DEPTH` of 128 nested containers and reject the next level with stable `CanonicalErrorKind::NestingLimitExceeded`, without changing canonical Core bytes or digests below the limit. | issue #146, crates/edict-syntax/src/canonical.rs |
| COREIR-REQ-018 | implemented | A Core intent may bind one optional typed basis expression. The expression participates in canonical bytes and digest when present, while the absent representation preserves existing `basis none` Core bytes. | docs/SPEC_edict-language-v1.md, docs/abi/edict-core.cddl |
| COREIR-REQ-019 | implemented | Core integer type and value identity preserves exact supported width and signedness, and canonical encoding rejects values outside the declared integer domain. | docs/SPEC_edict-language-v1.md, EDICT-LANG-INTLIT-001 |
| COREIR-REQ-020 | implemented | Core represents typed external-action requests as deterministic non-callable data and requires each request operation to remain in the exact digest-locked capability-import closure before canonical identity exists. | issue #172, docs/abi/edict-core.cddl |
| COREIR-REQ-021 | implemented | A Core branch may carry one optional result binding; when present, the selected block result becomes that local and the binding participates in canonical Core identity. | issue #192, docs/abi/edict-core.cddl |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/abi/edict-core.cddl | Normative Core semantic schema. | Required semantic declarations exist and forbidden byte/hash freeze fields are absent. |
| fixtures/lang/bounds/bounded-hello.edict | Initial pure local-record source-to-Core fixture. | Compiled Core module canonicalizes deterministically and produces the reviewed Core golden artifacts. |
| fixtures/core/canonical/statement-branch.core.hex | Test-owned reviewed canonical bytes for the inline `STATEMENT_BRANCH` source in `canonical_encoding.rs`, represented as lowercase hex for text review. | The decoded branch omits `binding`, and the executable encoder reproduces every byte; this fixture is not managed by `cargo xtask core-goldens`. |
| fixtures/core/canonical/statement-branch.core.sha256 | Test-owned reviewed digest for the inline `STATEMENT_BRANCH` source in `canonical_encoding.rs`. | The executable Core digest reproduces the exact digest committed by the reviewed bytes; this fixture is not managed by `cargo xtask core-goldens`. |
| fixtures/lang/external-actions/workspace-snapshot.edict | Typed external-request source-to-Core fixture. | Compiled Core preserves the complete request and exact capability closure in generated canonical bytes. |
| fixtures/core/schema/accepted/core-module-minimal.fields | Accepted Core module field-shape fixture. | Required `core-module` fields are present and no unknown fields appear. |
| fixtures/core/schema/accepted/core-intent-minimal.fields | Accepted Core intent field-shape fixture. | Required `core-intent` fields are present and no unknown fields appear. |
| fixtures/core/schema/accepted/core-fn-body-minimal.fields | Accepted pure Core function body field-shape fixture. | Required `core-fn-body` fields are present and no unknown fields appear. |
| fixtures/core/schema/rejected/core-module-missing-intents.fields | Rejected Core module field-shape fixture. | Missing required `intents` rejects. |
| fixtures/core/schema/rejected/local-ref-missing-alpha-name.fields | Rejected local reference field-shape fixture. | Missing required `alphaName` rejects. |
| fixtures/core/schema/rejected/core-intent-unknown-verified-mode.fields | Rejected Core intent field-shape fixture. | External verifier evidence field rejects as non-Core. |
| fixtures/core/schema/rejected/core-fn-body-effect-node-field.fields | Rejected pure Core function body field-shape fixture. | Effect-capable `nodes` field rejects as non-Core helper body shape. |
| fixtures/core/canonical/bounded-hello.core.cbor | Reviewed canonical Core byte fixture for the initial pure local-record source fixture. | Executable Core encoding exactly matches the checked-in byte fixture. |
| fixtures/core/canonical/bounded-hello.core.sha256 | Exact reviewed Core module digest for the initial pure local-record source fixture. | Domain-separated executable Core digest exactly matches the checked-in review rendering. |
| fixtures/core/canonical/workspace-snapshot.core.cbor | Reviewed canonical Core byte fixture for the typed workspace-snapshot request. | `cargo xtask core-goldens --check` compares the checked-in bytes to executable regeneration. |
| fixtures/core/canonical/workspace-snapshot.core.sha256 | Exact reviewed Core module digest for the typed workspace-snapshot request. | `cargo xtask core-goldens --check` compares the checked-in review digest to executable regeneration. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| COREIR-TP-001 | implemented | Golden path | COREIR-REQ-001, COREIR-REQ-002, COREIR-REQ-003, COREIR-REQ-004, COREIR-REQ-005, COREIR-REQ-006, COREIR-REQ-008, COREIR-REQ-009, COREIR-REQ-011 | `edict-core.cddl` contains the required module, type, intent, block, node, expression, predicate, function-body, input-constraint, local-ref, alpha-name, and operation-profile declarations. | core_cddl_declares_v1_semantic_model | docs/abi/edict-core.cddl | Static schema contract regression. |
| COREIR-TP-002 | implemented | Boundary guard | COREIR-REQ-007 | `edict-core.cddl` contains no fields that freeze self-hash, byte, digest, target, or admission artifacts. | core_cddl_has_no_digest_freeze_fields | docs/abi/edict-core.cddl | Golden byte and digest fixtures live outside the Core schema. |
| COREIR-TP-003 | implemented | Contract graph | COREIR-REQ-001, COREIR-REQ-002, COREIR-REQ-003, COREIR-REQ-004, COREIR-REQ-005, COREIR-REQ-006, COREIR-REQ-007, COREIR-REQ-008, COREIR-REQ-009, COREIR-REQ-010, COREIR-REQ-011 | The topic shelf resolves requirement IDs, case IDs, sources, evidence test names, fixtures, and local links. | contract_graph_is_valid | docs/abi/edict-core.cddl | Executed by `cargo xtask contract-check` and `cargo xtask verify`. |
| COREIR-TP-004 | implemented | Schema validation | COREIR-REQ-008, COREIR-REQ-010, COREIR-REQ-011 | Accepted and rejected Core field-shape fixtures validate against the required and allowed fields extracted from `edict-core.cddl`. | core_schema_shape_fixtures_match_cddl | fixtures/core/schema/accepted/core-module-minimal.fields, fixtures/core/schema/accepted/core-intent-minimal.fields, fixtures/core/schema/accepted/core-fn-body-minimal.fields, fixtures/core/schema/rejected/core-module-missing-intents.fields, fixtures/core/schema/rejected/local-ref-missing-alpha-name.fields, fixtures/core/schema/rejected/core-intent-unknown-verified-mode.fields, fixtures/core/schema/rejected/core-fn-body-effect-node-field.fields | Lightweight schema-shape validation, not full CDDL instance validation. |
| COREIR-TP-005 | planned | Schema validation | COREIR-REQ-008 | Accepted and rejected Core instance fixtures validate against the CDDL through a complete CDDL validator. | - | - | Deferred until the validator harness exists. |
| COREIR-TP-006 | implemented | Golden artifact | COREIR-REQ-014 | Reviewed golden bytes and exact digest fixtures are produced from the executable encoder. | reviewed_core_golden_bytes_match_executable_encoder, reviewed_core_digest_matches_exact_fixture | fixtures/core/canonical/bounded-hello.core.cbor, fixtures/core/canonical/bounded-hello.core.sha256 | Initial reviewed Core artifact check. |
| COREIR-TP-007 | implemented | Canonical encoding | COREIR-REQ-012 | Equivalent Core modules with maps constructed in different orders encode to the same bytes. | canonical_core_bytes_are_independent_of_map_construction_order | fixtures/lang/bounds/bounded-hello.edict | Tests map-order independence without freezing reviewed golden bytes. |
| COREIR-TP-008 | implemented | Canonical encoding | COREIR-REQ-012 | Mutating a semantic Core field changes canonical bytes. | canonical_core_bytes_change_when_core_meaning_changes | fixtures/lang/bounds/bounded-hello.edict | Tests mutation sensitivity without computing a digest. |
| COREIR-TP-009 | implemented | Canonical validation | COREIR-REQ-013 | Decoding canonical bytes and re-encoding them returns identical bytes; non-canonical bytes and duplicate map keys reject. | canonical_core_bytes_decode_and_reencode_stably, noncanonical_cbor_bytes_reject_with_stable_error_kind, canonical_cbor_rejects_duplicate_map_keys_on_encode | fixtures/lang/bounds/bounded-hello.edict | Encode and decode paths validate canonical byte shape. |
| COREIR-TP-010 | implemented | Alpha stability | COREIR-REQ-005, COREIR-REQ-012 | Source binder renaming that preserves Core local identity does not change canonical bytes. | canonical_core_bytes_are_source_alpha_rename_invariant | fixtures/lang/bounds/bounded-hello.edict | Tests source names do not enter Core identity. |
| COREIR-TP-011 | implemented | Platform independence | COREIR-REQ-012 | Canonical integer encodings use fixed CBOR width thresholds and big-endian multi-byte payloads. | canonical_cbor_integer_widths_are_platform_independent | - | Tests primitive encoder behavior, not a Core golden fixture. |
| COREIR-TP-012 | implemented | Canonical validation | COREIR-REQ-012, COREIR-REQ-013 | Core imports without resolved digests reject before canonical bytes are emitted. | canonical_core_rejects_unresolved_import_digest | fixtures/lang/bounds/bounded-hello.edict | Prevents floating imported semantics from entering the Core canonical preimage. |
| COREIR-TP-013 | implemented | Canonical encoding | COREIR-REQ-012 | Import alias spelling does not change canonical Core bytes when the resolved coordinate and digest are unchanged. | canonical_core_bytes_ignore_import_alias_spelling | fixtures/lang/bounds/bounded-hello.edict | Tests source-local import alias spelling is excluded from the Core canonical preimage. |
| COREIR-TP-014 | implemented | Canonical encoding | COREIR-REQ-012 | Reordering the same resolved Core imports does not change canonical bytes. | canonical_core_bytes_are_independent_of_import_order | fixtures/lang/bounds/bounded-hello.edict | Tests import declaration order is excluded from the Core canonical preimage. |
| COREIR-TP-015 | implemented | Alpha stability | COREIR-REQ-005, COREIR-REQ-012 | Source parameter renaming that preserves Core local identity does not change canonical bytes. | canonical_core_bytes_are_parameter_alpha_rename_invariant | fixtures/lang/bounds/bounded-hello.edict | Tests parameter source spelling does not enter Core identity. |
| COREIR-TP-016 | implemented | Canonical encoding | COREIR-REQ-012 | Reordering or duplicating the same required Core capability set does not change canonical bytes. | canonical_core_bytes_treat_required_capabilities_as_a_set | fixtures/lang/bounds/bounded-hello.edict | Tests capability flags are encoded as a canonical set. |
| COREIR-TP-017 | implemented | Canonical validation | COREIR-REQ-013 | Oversized declared CBOR collection lengths reject with a stable error instead of panicking or allocating from untrusted length. | oversized_cbor_array_length_returns_error_without_panicking | - | Tests public decode robustness for malformed canonical bytes. |
| COREIR-TP-018 | implemented | Canonical encoding | COREIR-REQ-012 | Uppercase and lowercase SHA-256 hex review forms encode to the same digest bytes. | canonical_core_bytes_normalize_digest_hex_case | fixtures/lang/bounds/bounded-hello.edict | Tests digest review rendering case does not affect canonical digest bytes. |
| COREIR-TP-019 | implemented | Canonical encoding | COREIR-REQ-012 | Reordering the same Core input constraints does not change canonical bytes. | canonical_core_bytes_are_independent_of_input_constraint_order | fixtures/lang/bounds/bounded-hello.edict | Tests coordinate-keyed constraint facts are sorted before encoding. |
| COREIR-TP-020 | implemented | Golden artifact | COREIR-REQ-014 | Core module digests are stable for equivalent canonical Core values and change when Core meaning changes. | core_module_digest_is_stable_for_equivalent_core_ordering, core_module_digest_changes_when_core_meaning_changes | fixtures/lang/bounds/bounded-hello.edict, fixtures/core/canonical/bounded-hello.core.sha256 | Tests digest stability and mutation sensitivity. |
| COREIR-TP-021 | implemented | Local identity | COREIR-REQ-005, COREIR-REQ-012, COREIR-REQ-014 | Changing a Core local identity changes canonical bytes and the Core module digest. | canonical_core_bytes_change_when_local_identity_changes, core_module_digest_changes_when_local_identity_changes | fixtures/lang/bounds/bounded-hello.edict, fixtures/core/canonical/bounded-hello.core.sha256 | Tests compiler-owned local `id` participates in the canonical preimage. |
| COREIR-TP-022 | implemented | Canonical encoding | COREIR-REQ-004, COREIR-REQ-012, COREIR-REQ-015 | Changing a semantic effect node coordinate changes canonical Core bytes. | canonical_core_bytes_change_when_effect_coordinate_changes | - | Tests effect-node meaning participates in the canonical preimage without adding a reviewed golden fixture. |
| COREIR-TP-023 | implemented | Canonical encoding | COREIR-REQ-004, COREIR-REQ-012, COREIR-REQ-016 | Terminal `require` obstruction and `continue obstructed` failure arms are distinct Core values and Core digests; reason kind and payload value mutations move the digest while payload field order and non-semantic formatting do not. | terminal_and_continue_obstructed_require_arms_are_core_distinct, obstruction_reason_mutations_move_core_digest | - | Tests require-failure arm meaning participates in the canonical preimage without adding a reviewed golden fixture. |
| COREIR-TP-024 | implemented | Canonical validation | COREIR-REQ-012, COREIR-REQ-013, COREIR-REQ-014, COREIR-REQ-017 | Encoding and decoding accept a scalar wrapped in exactly 128 arrays, domain-framed hashing preserves that full artifact nesting budget, both codec paths reject depth 129 with `NestingLimitExceeded`, and the reviewed Core bytes and digest remain unchanged. | canonical_nesting_limit_is_enforced_on_encode_and_decode, canonical_artifact_digest_accepts_the_public_nesting_boundary, reviewed_core_golden_bytes_match_executable_encoder, reviewed_core_digest_matches_exact_fixture | fixtures/core/canonical/bounded-hello.core.cbor, fixtures/core/canonical/bounded-hello.core.sha256 | The exact-boundary unit evidence lives in `canonical::canonical_artifact_tests`; digest framing does not become part of artifact nesting, and the already-validated-value helper remains crate-private. |
| COREIR-TP-025 | implemented | Canonical validation | COREIR-REQ-017 | Encoding and decoding accept exactly 128 nested empty arrays or maps and reject the 129th container even when it has no child value that would otherwise cross the depth guard. | canonical_nesting_limit_counts_empty_terminal_containers | crates/edict-syntax/src/canonical.rs | Code Lawyer regression for PR #150; prevents empty terminal containers from bypassing the public container-count bound. |
| COREIR-TP-026 | implemented | Canonical encoding | COREIR-REQ-012, COREIR-REQ-014, COREIR-REQ-018 | Adding or changing an explicit Core basis expression changes canonical Core bytes and digest; the reference-encoded operation Core satisfies the published schema, while omitting basis retains the reviewed `basis none` golden bytes and digest. | explicit_basis_and_semantic_input_mutations_move_target_identity, core_root_accepts_reference_encoded_operation_basis, reviewed_core_golden_bytes_match_executable_encoder, reviewed_core_digest_matches_exact_fixture | fixtures/lang/operations/explicit-basis-u64.edict, fixtures/core/canonical/bounded-hello.core.cbor | Proves basis is semantic while retaining provider-v1 compatibility. |
| COREIR-TP-027 | implemented | Canonical validation | COREIR-REQ-012, COREIR-REQ-019 | `U64::MAX` and the signed fixed-width minima canonicalize under their exact declared domains; an out-of-range value or a value incompatible with its declared width rejects before canonical identity exists, and the reference-encoded exact-width operation satisfies the published schema. | operation_prerequisite_fixture_preserves_fixed_width_basis_and_lawpack_closure, signed_fixed_width_minima_preserve_exact_domains, out_of_range_u64_and_cross_width_values_reject_before_core, canonical_core_rejects_values_outside_their_declared_integer_domain, core_root_accepts_reference_encoded_operation_basis | fixtures/lang/operations/explicit-basis-u64.edict | Prevents host-width and GraphQL-width narrowing from entering Core. |
| COREIR-TP-028 | implemented | External request | COREIR-REQ-012, COREIR-REQ-014, COREIR-REQ-017, COREIR-REQ-020 | Request fields survive canonical Core encoding, repeated compilation is byte-identical, authority mutations move Core and Target identity, removing the exact capability import makes the artifact unencodable, request traversal reaches closure inspection at the canonical nesting boundary, excessive nesting returns `NestingLimitExceeded`, and the owner-generated request golden reproduces exact bytes and digest. | workspace_observation_request_compiles_as_non_callable_data, request_artifacts_are_reproducible, every_request_authority_field_moves_core_and_target_identity, request_operation_must_remain_in_core_and_target_capability_closure, nested_request_collection_distinguishes_canonical_depth_boundary, core_goldens_match_executable_encoder | crates/edict-syntax/tests/external_action_requests.rs, fixtures/core/canonical/workspace-snapshot.core.cbor, fixtures/core/canonical/workspace-snapshot.core.sha256 | Request construction performs no external effect; the prepass shares and proves both sides of the public canonical depth boundary. |
| COREIR-TP-029 | implemented | Branch result identity | COREIR-REQ-004, COREIR-REQ-012, COREIR-REQ-021 | A compiler-produced branch-yield carries one result binding; removing only that binding changes the canonical Core digest. A statement-only branch omits the binding field and retains exact reviewed canonical bytes and digest. | effectful_branch_yield_lowers_to_bound_core_branch, effectful_branch_yield_mutation_moves_core_digest, statement_branch_omits_binding_and_preserves_exact_canonical_identity | crates/edict-syntax/tests/compiler_spine.rs, crates/edict-syntax/tests/canonical_encoding.rs | Statement-only branches omit the binding and retain null block results. |

## Determinism Obligations

- Static schema tests read checked-in files only.
- Tests assert structured schema declarations and forbidden field names, not
  rendered prose.
- The contract graph checker verifies requirement/case/evidence/fixture links.
- Schema-shape fixtures are checked by extracting required and allowed fields
  from the checked-in CDDL, not by duplicating a prose field list.
- Golden artifact tests compare behavior-derived bytes and digests against
  reviewed fixtures. Generator-owned Core fixtures are regenerated with
  `cargo xtask core-goldens --write`. The statement-branch hex and digest are
  test-owned: an intentional source or encoder change must regenerate both from
  the exact `canonical_encoding.rs` test path in one reviewed change, then run
  that test and `cargo xtask verify`; `core-goldens` does not manage them.

## Open Gaps

- Full CDDL instance validation fixtures still require a validation harness.
- Initial source-to-in-memory-Core lowering exists in the compiler-spine shelf;
  full source language coverage is still open.
- Additional reviewed Core golden fixtures remain open as Core language coverage
  expands beyond the initial pure local-record fixture.
