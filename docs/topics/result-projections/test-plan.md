# Result Projections Test Plan

## Scope

This shelf owns the compiler-authored, runtime-neutral representation of an
application operation result. It covers derivation from exact Core and Target
IR, canonical bytes and digest identity, source authority, boundedness, and
independent reverse reconstruction. Runtime evaluation and durable result
binding belong to Echo.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| RESULT-PROJ-REQ-001 | planned | The compiler derives one operation result projection from exact Core and the matching Target IR result without host-authored application semantics. | issue #173 |
| RESULT-PROJ-REQ-002 | planned | Projection leaves may name only the declared application input or a declared capability-step result, with explicit bounded field paths. | issue #173 |
| RESULT-PROJ-REQ-003 | planned | `edict.result-projection/v1` has canonical CBOR bytes, a published CDDL root, and a domain-framed `edict.result-projection.artifact/v1` identity. | issue #173 |
| RESULT-PROJ-REQ-004 | planned | Projection node count, path depth, text size, artifact bytes, and output bytes are bounded before the artifact can be admitted. | issue #173 |
| RESULT-PROJ-REQ-005 | planned | Independent verification decodes the claimed bytes, recomputes their identity, reconstructs the authored Core result from declared sources, and requires exact Core and Target IR agreement. | issue #173 |
| RESULT-PROJ-REQ-006 | planned | Mutated, incomplete, unsupported, undeclared-source, digest-substituted, malformed, and over-bound projections fail with stable error kinds. | issue #173 |
| RESULT-PROJ-REQ-007 | planned | The representation remains target-neutral and contains no runtime callback, external effect, model call, or application-specific production behavior. | issue #173 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| `fixtures/lawpack/hello-echo/create-greeting.edict` | Real application source with one application-input field and one capability-result field in its result. | Compilation and Target IR lowering derive the reviewed projection without native Hello Echo code. |
| `fixtures/lawpack/hello-echo/create-greeting.result-projection.cbor` | Planned reviewed canonical projection bytes. | `cargo xtask lawpack-goldens --check` reproduces exact bytes through the compiler-owned emitter. |
| `fixtures/lawpack/hello-echo/create-greeting.result-projection.sha256` | Planned reviewed domain-framed projection identity. | The emitter and independent verifier reproduce the same identity. |
| `docs/abi/edict-result-projection.cddl` | Edict-owned canonical schema. | The provider contract pack validates a reference projection and rejects structural mutation. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| RESULT-PROJ-TP-001 | planned | Golden path | RESULT-PROJ-REQ-001, RESULT-PROJ-REQ-002 | Exact Hello Echo Core and Target IR emit the expected typed record projection. | `exact_core_and_target_ir_emit_the_typed_hello_echo_projection` | The test enters through real source and lawpack compilation. |
| RESULT-PROJ-TP-002 | planned | Independent verification | RESULT-PROJ-REQ-005 | The verifier reconstructs the exact authored Core result from projection sources and accepts the emitted identity. | `independent_verifier_reconstructs_the_authored_core_result` | Verification runs in the reverse direction from projection to Core expression. |
| RESULT-PROJ-TP-003 | planned | Mutation rejection | RESULT-PROJ-REQ-005, RESULT-PROJ-REQ-006 | A Target IR result that differs from its source Core result fails closed. | `mutated_target_result_fails_closed` | Prevents target-owned result authorship. |
| RESULT-PROJ-TP-004 | planned | Authority rejection | RESULT-PROJ-REQ-002, RESULT-PROJ-REQ-006 | Undeclared locals and call expressions fail with stable error kinds. | `undeclared_locals_and_unsupported_calls_fail_closed` | No callback or general expression VM enters the artifact. |
| RESULT-PROJ-TP-005 | planned | Shape and bound rejection | RESULT-PROJ-REQ-004, RESULT-PROJ-REQ-006 | Incomplete output records and a zero output-byte ceiling fail closed. | `incomplete_output_and_zero_output_bound_fail_closed` | Output shape is checked against declared Core types. |
| RESULT-PROJ-TP-006 | planned | Canonical identity | RESULT-PROJ-REQ-003, RESULT-PROJ-REQ-006 | Trailing bytes and a substituted digest fail independently. | `canonical_bytes_and_digest_are_independently_enforced` | Canonicality and identity are separate gates. |
| RESULT-PROJ-TP-007 | planned | Boundary | RESULT-PROJ-REQ-004 | The exact node limit encodes and the next node rejects. | `projection_node_limit_accepts_the_boundary_and_rejects_the_next_node` | The root record counts as one node. |
| RESULT-PROJ-TP-008 | planned | Property | RESULT-PROJ-REQ-003 | Sixty-four fixed-seed insertion orders produce identical canonical bytes. | `canonical_encoding_is_insertion_order_independent_for_fixed_seed_cases` | Fixed seed: `0x17305eedcafebabe`. |
| RESULT-PROJ-TP-009 | planned | Stress | RESULT-PROJ-REQ-001, RESULT-PROJ-REQ-005 | 128 bounded emit/verify repetitions reproduce identical artifacts. | `repeated_emit_and_verify_is_stable_under_bounded_stress` | Bounded deterministic stress, not a benchmark. |
| RESULT-PROJ-TP-010 | planned | Schema fidelity | RESULT-PROJ-REQ-003 | The provider contract pack publishes the result-projection root and domain. | `result_projection_root_matches_reference_encoder` | Echo #698 can consume only this admitted representation. |

## Determinism Obligations

- Projection derivation performs no I/O and consumes only supplied Core and
  Target IR values.
- Record fields are canonicalized by key.
- Step references use Target IR step identities, not source variable names.
- Verification never trusts a claimed digest or a host-authored result.
- Exact replay of the same Core and Target IR produces identical bytes and
  identity.

## Non-Goals

- Runtime projection evaluation.
- Echo package or verification-report changes.
- General expression evaluation.
- External effects or model execution.
- Application-specific callbacks.
