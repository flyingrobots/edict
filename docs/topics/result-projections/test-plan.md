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
| RESULT-PROJ-REQ-001 | implemented | The compiler derives one operation result projection from exact Core and the matching Target IR result without host-authored application semantics. | issue #173 |
| RESULT-PROJ-REQ-002 | implemented | Projection leaves may name only the declared application input or a declared capability-step result, with explicit bounded field paths. | issue #173 |
| RESULT-PROJ-REQ-003 | implemented | `edict.result-projection/v1` has canonical CBOR bytes, a published CDDL root, and a domain-framed `edict.result-projection.artifact/v1` identity. | issue #173 |
| RESULT-PROJ-REQ-004 | implemented | Projection node count, path depth, text size, artifact bytes, and output bytes are bounded before the artifact can be admitted. | issue #173 |
| RESULT-PROJ-REQ-005 | implemented | Independent verification decodes the claimed bytes, recomputes their identity, reconstructs the authored Core result from declared sources, and requires exact Core and Target IR agreement. | issue #173 |
| RESULT-PROJ-REQ-006 | implemented | Mutated, incomplete, unsupported, undeclared-source, digest-substituted, malformed, and over-bound projections fail with stable error kinds. | issue #173 |
| RESULT-PROJ-REQ-007 | implemented | The representation remains target-neutral and contains no runtime callback, external effect, model call, or application-specific production behavior. | issue #173 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| `fixtures/lawpack/hello-echo/create-greeting.edict` | Real application source with one application-input field and one capability-result field in its result. | Compilation and Target IR lowering derive the reviewed projection without native Hello Echo code. |
| `fixtures/lawpack/hello-echo/create-greeting.result-projection.cbor` | Reviewed canonical projection bytes. | `cargo xtask lawpack-goldens --check` reproduces exact bytes through the compiler-owned emitter. |
| `fixtures/lawpack/hello-echo/create-greeting.result-projection.sha256` | Reviewed domain-framed projection identity. | The emitter and independent verifier reproduce the same identity. |
| `docs/abi/edict-result-projection.cddl` | Edict-owned canonical schema. | The provider contract pack validates a reference projection and rejects structural mutation. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| RESULT-PROJ-TP-001 | implemented | Golden path | RESULT-PROJ-REQ-001, RESULT-PROJ-REQ-002 | Exact Hello Echo Core and Target IR emit the expected typed record projection. | `exact_core_and_target_ir_emit_the_typed_hello_echo_projection` | fixtures/lawpack/hello-echo/create-greeting.edict, fixtures/lawpack/hello-echo/create-greeting.result-projection.cbor | The test enters through real source and lawpack compilation. |
| RESULT-PROJ-TP-002 | implemented | Independent verification | RESULT-PROJ-REQ-005 | The verifier reconstructs the exact authored Core result from projection sources and accepts the emitted identity. | `independent_verifier_reconstructs_the_authored_core_result` | crates/edict-syntax/tests/result_projection.rs | Verification runs in the reverse direction from projection to Core expression. |
| RESULT-PROJ-TP-003 | implemented | Mutation rejection | RESULT-PROJ-REQ-005, RESULT-PROJ-REQ-006 | A Target IR result that differs from its source Core result fails closed. | `mutated_target_result_fails_closed` | crates/edict-syntax/tests/result_projection.rs | Prevents target-owned result authorship. |
| RESULT-PROJ-TP-004 | implemented | Authority rejection | RESULT-PROJ-REQ-002, RESULT-PROJ-REQ-006, RESULT-PROJ-REQ-007 | Undeclared locals and call expressions fail with stable error kinds. | `undeclared_locals_and_unsupported_calls_fail_closed` | crates/edict-syntax/tests/result_projection.rs | No callback or general expression VM enters the artifact. |
| RESULT-PROJ-TP-005 | implemented | Shape and bound rejection | RESULT-PROJ-REQ-004, RESULT-PROJ-REQ-006 | Incomplete output records and a zero output-byte ceiling fail closed. | `incomplete_output_and_zero_output_bound_fail_closed` | crates/edict-syntax/tests/result_projection.rs | Output shape is checked against declared Core types. |
| RESULT-PROJ-TP-006 | implemented | Canonical identity | RESULT-PROJ-REQ-003, RESULT-PROJ-REQ-006 | Trailing bytes and a substituted digest fail independently. | `canonical_bytes_and_digest_are_independently_enforced` | fixtures/lawpack/hello-echo/create-greeting.result-projection.cbor, fixtures/lawpack/hello-echo/create-greeting.result-projection.sha256 | Canonicality and identity are separate gates. |
| RESULT-PROJ-TP-007 | implemented | Boundary | RESULT-PROJ-REQ-004 | The exact node limit encodes and the next node rejects. | `projection_node_limit_accepts_the_boundary_and_rejects_the_next_node` | crates/edict-syntax/tests/result_projection.rs | The root record counts as one node. |
| RESULT-PROJ-TP-008 | implemented | Property | RESULT-PROJ-REQ-003 | Sixty-four fixed-seed insertion orders produce identical canonical bytes. | `canonical_encoding_is_insertion_order_independent_for_fixed_seed_cases` | crates/edict-syntax/tests/result_projection.rs | Fixed seed: `0x17305eedcafebabe`. |
| RESULT-PROJ-TP-009 | implemented | Stress | RESULT-PROJ-REQ-001, RESULT-PROJ-REQ-005 | 128 bounded emit/verify repetitions reproduce identical artifacts. | `repeated_emit_and_verify_is_stable_under_bounded_stress` | crates/edict-syntax/tests/result_projection.rs | Bounded deterministic stress, not a benchmark. |
| RESULT-PROJ-TP-010 | implemented | Schema fidelity | RESULT-PROJ-REQ-003 | The provider contract pack publishes the result-projection root and domain. | `result_projection_root_matches_reference_encoder` | docs/abi/edict-result-projection.cddl, fixtures/provider-contracts/v1/edict-provider-contracts.cddl | Echo #698 can consume only this admitted representation. |
| RESULT-PROJ-TP-011 | implemented | Closure binding | RESULT-PROJ-REQ-001, RESULT-PROJ-REQ-005, RESULT-PROJ-REQ-006 | Substituting the Target IR semantic-closure Core identity rejects before projection emission. | `mutated_target_core_closure_fails_closed` | crates/edict-syntax/tests/result_projection.rs | Matching result syntax cannot detach the projection from the exact compiler input. |
| RESULT-PROJ-TP-012 | implemented | Bound matrix | RESULT-PROJ-REQ-004, RESULT-PROJ-REQ-006 | One-over-limit path depth, UTF-8 text, artifact bytes, and structurally incompatible output types reject with stable kinds. | `path_text_artifact_and_structure_bounds_fail_closed` | crates/edict-syntax/tests/result_projection.rs | Complements the exact node-limit boundary. |
| RESULT-PROJ-TP-013 | implemented | Target emission | RESULT-PROJ-REQ-001, RESULT-PROJ-REQ-005 | Echo target lowering returns the compiler-produced projection and the independent verifier reconstructs that exact output. | `echo_target_lowering_emits_the_verified_result_projection` | crates/edict-syntax/tests/result_projection.rs | Prevents callers from reauthoring or dropping the projection between Core and the provider boundary. |
| RESULT-PROJ-TP-014 | implemented | Provider closure | RESULT-PROJ-REQ-001, RESULT-PROJ-REQ-003 | The application build binds the compiler-emitted canonical projection and digest into the provider semantic-input closure. | `compiler_result_projection_is_bound_into_the_provider_closure` | crates/edict-cli/src/application_build.rs | Echo #698 consumes this Edict-owned representation rather than inventing one inside the runtime. |
| RESULT-PROJ-TP-015 | implemented | Target refusal | RESULT-PROJ-REQ-006, RESULT-PROJ-REQ-007 | A result outside the closed projection language produces no projection claim and records the stable per-intent projection failure while preserving general Target IR. | `target_lowering_exposes_an_unsupported_result_projection_without_claiming_one` | crates/edict-syntax/tests/result_projection.rs | The application build treats any recorded projection failure as terminal; non-application Target IR consumers remain compatible. |

## Determinism Obligations

- Projection derivation performs no I/O and consumes only supplied Core and
  Target IR values.
- Record fields are canonicalized by key.
- Step references use Target IR step identities, not source variable names.
- Verification never trusts a claimed digest or a host-authored result.
- Exact replay of the same Core and Target IR produces identical bytes and
  identity.
- The lowerer and verifier receive byte-identical projection inputs bound to
  the compiler-authored identity.

## Non-Goals

- Runtime projection evaluation.
- Echo package or verification-report changes.
- General expression evaluation.
- External effects or model execution.
- Application-specific callbacks.
