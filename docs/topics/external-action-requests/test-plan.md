# External-Action Requests Test Plan

Status: planned contract for issue #172.

## Scope

In scope:

- digest-locked `capability` imports that name requestable external operation
  families;
- source-authored external-action request declarations;
- deterministic Core and Target IR representation of request data;
- package-closure binding for operation, schema, and reconciliation resources;
- explicit awaiting-settlement and schema-admission posture;
- compile-time denial of undeclared or directly callable operation families;
- omission of filesystem, process, network, Git, GitHub, model, and shell
  authority from compiler and provider interfaces.

Out of scope:

- performing external actions;
- adapter selection or authorization;
- live path, ref, basis, budget, or settlement admission;
- stack suspension, continuations, or `async` host frames;
- general-purpose effects or ambient I/O.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| EXTREQ-REQ-001 | planned | Source syntax binds each requestable operation through one digest-locked `capability` import and carries exact input-schema, settlement-schema, authority-scope, basis, budget, input, and reconciliation-law values. | issue #172 |
| EXTREQ-REQ-002 | planned | Core represents external-action requests as data with compiler-owned binding identity, typed input and settlement coordinates, explicit `awaitingSettlement` state, and required schema admission. | issue #172 |
| EXTREQ-REQ-003 | planned | Target IR preserves external-action requests separately from callable target steps and target intrinsics. | issue #172 |
| EXTREQ-REQ-004 | planned | Core and Target IR semantic closure bind the exact capability resource; undeclared operations and floating capability imports fail closed. | issue #172 |
| EXTREQ-REQ-005 | planned | Equivalent admitted source and compiler facts produce byte-identical Core and Target IR; every request authority or meaning mutation moves the corresponding digest. | issue #172 |
| EXTREQ-REQ-006 | planned | Runtime-valued authority scope, basis, and budget expressions survive compilation for Echo admission; Edict performs no external action while compiling or lowering them. | issue #172 |
| EXTREQ-REQ-007 | planned | Raw filesystem, process, network, Git, GitHub, model, and shell operation families are outside the requestable capability vocabulary. | issue #172 |
| EXTREQ-REQ-008 | planned | Request construction remains bounded under a fixed-seed mutation corpus and a 64-request stress module. | issue #172 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| In-test `workspace.snapshot.observe@1` source | First bounded read-only external request. | Public parser, compiler, canonical encoders, and Target IR lowerer preserve the exact request contract. |
| Fixed seed `0x4558_5452_4551_0001` | Determinism and mutation corpus. | Repeated compilation is byte-identical and distinct capability identities produce distinct Core identities. |
| 64-request generated module | Bounded stress case. | All 64 requests survive Core and Target IR without becoming callable steps. |

## Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| EXTREQ-TP-001 | planned | Golden path | EXTREQ-REQ-001, EXTREQ-REQ-002, EXTREQ-REQ-003 | One workspace observation request parses, compiles, and lowers with exact structured fields, awaiting posture, and zero callable target steps. | `workspace_observation_request_compiles_as_non_callable_data` | The target owns later admission and execution. |
| EXTREQ-TP-002 | planned | Closure guard | EXTREQ-REQ-004 | Missing operation import, floating capability digest, and operation-alias substitution reject with stable compiler or canonical error kinds. | `undeclared_or_floating_operation_families_fail_closed` | A source alias is not authority by itself. |
| EXTREQ-TP-003 | planned | Authority guard | EXTREQ-REQ-003, EXTREQ-REQ-007 | Direct invocation of a capability import fails, and raw ambient operation families are unrepresentable. | `capability_import_cannot_be_called_as_an_effect`, `ambient_operation_families_are_not_requestable` | Request authority and performance authority remain distinct. |
| EXTREQ-TP-004 | planned | Dynamic boundary | EXTREQ-REQ-006 | Scope, basis, maximum settlement bytes, and attempt count remain Core expressions sourced from admitted input. | `dynamic_admission_values_survive_without_compile_time_execution` | Echo owns value-instance admission. |
| EXTREQ-TP-005 | planned | Determinism | EXTREQ-REQ-005 | Repeated compilation and lowering produce identical canonical bytes and digests. | `request_artifacts_are_reproducible` | No clock, randomness, filesystem, or network input enters the compiler. |
| EXTREQ-TP-006 | planned | Mutation sensitivity | EXTREQ-REQ-005 | Operation, input schema, settlement schema, authority scope, basis, budget, input, and reconciliation mutations move Core identity. | `every_request_authority_field_moves_core_identity` | Each field is request-authoritative. |
| EXTREQ-TP-007 | planned | Property | EXTREQ-REQ-005, EXTREQ-REQ-008 | A fixed-seed 32-case capability-digest corpus is reproducible and collision-free within the corpus. | `fixed_seed_request_identity_corpus_is_deterministic` | Seed is recorded in the fixture table. |
| EXTREQ-TP-008 | planned | Stress | EXTREQ-REQ-008 | A generated module with 64 request declarations emits 64 Core requests and 64 Target IR requests with zero callable steps. | `sixty_four_requests_remain_bounded_non_callable_data` | Bound is fixed for CI. |

## Determinism Obligations

- Tests use no filesystem discovery, network access, clock, environment, or
  randomness.
- The property corpus uses the recorded fixed seed and a local deterministic
  generator.
- Canonical comparisons assert decoded structured fields and exact bytes, not
  diagnostics or log text.
- Stress cardinality is fixed at 64.

## Open Gaps

- Echo admission of compiler-emitted request values.
- A concrete workspace-observation adapter and settlement witness.
- Basis-bound validated patch application.
