# Core IR Test Plan

Status: current verification design for the Core semantic model and CDDL schema.

## Scope

In scope:

- `edict.core/v1` semantic model boundaries;
- normative CDDL declarations in `docs/abi/edict-core.cddl`;
- local-reference normalization shape;
- input-constraint and predicate-tree shape;
- explicit non-claim that `v0.2` freezes no Core bytes or Core digest fields;
- deterministic contract metadata linking this shelf to executable tests.

Out of scope:

- source-to-Core lowering;
- canonical encoder implementation;
- golden Core bytes;
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
| COREIR-REQ-007 | implemented | The `v0.2` Core contract does not freeze canonical encoders, golden bytes, exact Core digests, target IR, or admission bundles. | ROADMAP.md, docs/abi/edict-core.cddl |
| COREIR-REQ-008 | implemented | `docs/abi/edict-core.cddl` is the normative machine-readable schema for `edict.core/v1`. | docs/abi/edict-core.cddl, issue #19 |
| COREIR-REQ-009 | implemented | Core states the required operation profile; verified operation mode is external verifier/admission evidence, not a Core field. | docs/abi/edict-core.cddl, EDICT-CORE-VERIFIED-EXTERNAL-001 |
| COREIR-REQ-010 | implemented | Accepted and rejected Core schema-shape fixtures are checked against the CDDL declarations. | docs/abi/edict-core.cddl, issue #19 |
| COREIR-REQ-011 | implemented | Edict-authored lawpack pure helper bodies use a pure `core-fn-body` shape, not the effect-capable `core-block` node algebra. | docs/abi/edict-core.cddl, docs/abi/edict-lawpack.cddl |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/abi/edict-core.cddl | Normative Core semantic schema. | Required semantic declarations exist and forbidden byte/hash freeze fields are absent. |
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
| COREIR-TP-002 | implemented | Boundary guard | COREIR-REQ-007 | `edict-core.cddl` contains no fields that freeze Core byte or digest artifacts in `v0.2`. | core_cddl_has_no_digest_freeze_fields | docs/abi/edict-core.cddl | Encoding/hash work is owned by the compiler-spine milestone. |
| COREIR-TP-003 | implemented | Contract graph | COREIR-REQ-001, COREIR-REQ-002, COREIR-REQ-003, COREIR-REQ-004, COREIR-REQ-005, COREIR-REQ-006, COREIR-REQ-007, COREIR-REQ-008, COREIR-REQ-009, COREIR-REQ-010, COREIR-REQ-011 | The topic shelf resolves requirement IDs, case IDs, sources, evidence test names, fixtures, and local links. | contract_graph_is_valid | docs/abi/edict-core.cddl | Executed by `cargo xtask contract-check` and `cargo xtask verify`. |
| COREIR-TP-004 | implemented | Schema validation | COREIR-REQ-008, COREIR-REQ-010, COREIR-REQ-011 | Accepted and rejected Core field-shape fixtures validate against the required and allowed fields extracted from `edict-core.cddl`. | core_schema_shape_fixtures_match_cddl | fixtures/core/schema/accepted/core-module-minimal.fields, fixtures/core/schema/accepted/core-intent-minimal.fields, fixtures/core/schema/accepted/core-fn-body-minimal.fields, fixtures/core/schema/rejected/core-module-missing-intents.fields, fixtures/core/schema/rejected/local-ref-missing-alpha-name.fields, fixtures/core/schema/rejected/core-intent-unknown-verified-mode.fields, fixtures/core/schema/rejected/core-fn-body-effect-node-field.fields | Lightweight schema-shape validation, not full CDDL instance validation. |
| COREIR-TP-005 | planned | Schema validation | COREIR-REQ-008 | Accepted and rejected Core instance fixtures validate against the CDDL through a complete CDDL validator. | - | - | Deferred until the validator harness exists. |
| COREIR-TP-006 | planned | Golden artifact | COREIR-REQ-007 | Executable canonical encoder produces reviewed golden bytes and exact digest fixtures. | - | - | Owned by the compiler-spine milestone after an encoder exists. |

## Determinism Obligations

- Static schema tests read checked-in files only.
- Tests assert structured schema declarations and forbidden field names, not
  rendered prose.
- The contract graph checker verifies requirement/case/evidence/fixture links.
- Schema-shape fixtures are checked by extracting required and allowed fields
  from the checked-in CDDL, not by duplicating a prose field list.
- No test computes, records, or compares Core digest values in this milestone.

## Open Gaps

- Full CDDL instance validation fixtures still require a validation harness.
- Source-to-Core lowering is not implemented.
- Canonical encoding, golden bytes, exact digest fixtures, and platform
  independence tests belong to the compiler-spine milestone.
