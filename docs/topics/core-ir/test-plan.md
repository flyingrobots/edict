# Core IR Test Plan

Status: current verification design for the Core semantic model and CDDL schema.

## Scope

In scope:

- `edict.core/v1` semantic model boundaries;
- normative CDDL declarations in `docs/abi/edict-core.cddl`;
- local-reference normalization shape;
- input-constraint and predicate-tree shape;
- reference canonical encoder behavior for the current in-memory Core module
  model;
- canonical byte validation through decode and re-encode stability;
- explicit non-claim that the Core module schema freezes no self-hash, reviewed
  golden bytes, exact Core digests, target IR, or admission bundles;
- deterministic contract metadata linking this shelf to executable tests.

Out of scope:

- full source-to-Core language coverage;
- reviewed golden Core bytes;
- exact Core digests;
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
| COREIR-REQ-007 | implemented | The Core module schema does not freeze a self-hash, reviewed golden bytes, exact Core digests, target IR, or admission bundles. | ROADMAP.md, docs/abi/edict-core.cddl |
| COREIR-REQ-008 | implemented | `docs/abi/edict-core.cddl` is the normative machine-readable schema for `edict.core/v1`. | docs/abi/edict-core.cddl, issue #19 |
| COREIR-REQ-009 | implemented | Core states the required operation profile; verified operation mode is external verifier/admission evidence, not a Core field. | docs/abi/edict-core.cddl, EDICT-CORE-VERIFIED-EXTERNAL-001 |
| COREIR-REQ-010 | implemented | Accepted and rejected Core schema-shape fixtures are checked against the CDDL declarations. | docs/abi/edict-core.cddl, issue #19 |
| COREIR-REQ-011 | implemented | Edict-authored lawpack pure helper bodies use a pure `core-fn-body` shape, not the effect-capable `core-block` node algebra. | docs/abi/edict-core.cddl, docs/abi/edict-lawpack.cddl |
| COREIR-REQ-012 | implemented | Core modules have a reference `edict.canonical-cbor/v1` encoder that emits deterministic bytes from semantic Core values. | issue #21, crates/edict-syntax/src/canonical.rs |
| COREIR-REQ-013 | implemented | Canonical Core bytes can be decoded to a canonical value and re-encoded without byte changes; non-canonical encodings reject. | issue #21, crates/edict-syntax/src/canonical.rs |
| COREIR-REQ-014 | planned | The canonical encoder does not freeze reviewed golden byte fixtures or exact digest fixtures. | issue #21, issue #22 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/abi/edict-core.cddl | Normative Core semantic schema. | Required semantic declarations exist and forbidden byte/hash freeze fields are absent. |
| fixtures/lang/bounds/bounded-hello.edict | Initial pure local-record source-to-Core fixture. | Compiled Core module canonicalizes deterministically without reviewed golden bytes or digest fixtures. |
| fixtures/core/schema/accepted/core-module-minimal.fields | Accepted Core module field-shape fixture. | Required `core-module` fields are present and no unknown fields appear. |
| fixtures/core/schema/accepted/core-intent-minimal.fields | Accepted Core intent field-shape fixture. | Required `core-intent` fields are present and no unknown fields appear. |
| fixtures/core/schema/accepted/core-fn-body-minimal.fields | Accepted pure Core function body field-shape fixture. | Required `core-fn-body` fields are present and no unknown fields appear. |
| fixtures/core/schema/rejected/core-module-missing-intents.fields | Rejected Core module field-shape fixture. | Missing required `intents` rejects. |
| fixtures/core/schema/rejected/local-ref-missing-alpha-name.fields | Rejected local reference field-shape fixture. | Missing required `alphaName` rejects. |
| fixtures/core/schema/rejected/core-intent-unknown-verified-mode.fields | Rejected Core intent field-shape fixture. | External verifier evidence field rejects as non-Core. |
| fixtures/core/schema/rejected/core-fn-body-effect-node-field.fields | Rejected pure Core function body field-shape fixture. | Effect-capable `nodes` field rejects as non-Core helper body shape. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| COREIR-TP-001 | implemented | Golden path | COREIR-REQ-001, COREIR-REQ-002, COREIR-REQ-003, COREIR-REQ-004, COREIR-REQ-005, COREIR-REQ-006, COREIR-REQ-008, COREIR-REQ-009, COREIR-REQ-011 | `edict-core.cddl` contains the required module, type, intent, block, node, expression, predicate, function-body, input-constraint, local-ref, alpha-name, and operation-profile declarations. | core_cddl_declares_v1_semantic_model | docs/abi/edict-core.cddl | Static schema contract regression. |
| COREIR-TP-002 | implemented | Boundary guard | COREIR-REQ-007 | `edict-core.cddl` contains no fields that freeze self-hash, byte, digest, target, or admission artifacts. | core_cddl_has_no_digest_freeze_fields | docs/abi/edict-core.cddl | Golden byte and digest work is owned by #22. |
| COREIR-TP-003 | implemented | Contract graph | COREIR-REQ-001, COREIR-REQ-002, COREIR-REQ-003, COREIR-REQ-004, COREIR-REQ-005, COREIR-REQ-006, COREIR-REQ-007, COREIR-REQ-008, COREIR-REQ-009, COREIR-REQ-010, COREIR-REQ-011 | The topic shelf resolves requirement IDs, case IDs, sources, evidence test names, fixtures, and local links. | contract_graph_is_valid | docs/abi/edict-core.cddl | Executed by `cargo xtask contract-check` and `cargo xtask verify`. |
| COREIR-TP-004 | implemented | Schema validation | COREIR-REQ-008, COREIR-REQ-010, COREIR-REQ-011 | Accepted and rejected Core field-shape fixtures validate against the required and allowed fields extracted from `edict-core.cddl`. | core_schema_shape_fixtures_match_cddl | fixtures/core/schema/accepted/core-module-minimal.fields, fixtures/core/schema/accepted/core-intent-minimal.fields, fixtures/core/schema/accepted/core-fn-body-minimal.fields, fixtures/core/schema/rejected/core-module-missing-intents.fields, fixtures/core/schema/rejected/local-ref-missing-alpha-name.fields, fixtures/core/schema/rejected/core-intent-unknown-verified-mode.fields, fixtures/core/schema/rejected/core-fn-body-effect-node-field.fields | Lightweight schema-shape validation, not full CDDL instance validation. |
| COREIR-TP-005 | planned | Schema validation | COREIR-REQ-008 | Accepted and rejected Core instance fixtures validate against the CDDL through a complete CDDL validator. | - | - | Deferred until the validator harness exists. |
| COREIR-TP-006 | planned | Golden artifact | COREIR-REQ-014 | Reviewed golden bytes and exact digest fixtures are produced from the executable encoder. | - | - | Owned by #22 after #21 lands. |
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

## Determinism Obligations

- Static schema tests read checked-in files only.
- Tests assert structured schema declarations and forbidden field names, not
  rendered prose.
- The contract graph checker verifies requirement/case/evidence/fixture links.
- Schema-shape fixtures are checked by extracting required and allowed fields
  from the checked-in CDDL, not by duplicating a prose field list.
- Canonical encoder tests compare behavior-derived bytes only; they do not
  record reviewed golden fixtures or exact digest values in this milestone.

## Open Gaps

- Full CDDL instance validation fixtures still require a validation harness.
- Initial source-to-in-memory-Core lowering exists in the compiler-spine shelf;
  full source language coverage is still open.
- Reviewed golden bytes and exact digest fixtures remain open for #22.
