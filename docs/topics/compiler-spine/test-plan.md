# Compiler Spine Test Plan

Status: current verification design for the executable source-to-Core spine.

## Scope

In scope:

- explicit stage APIs: resolve, type-check, lower to in-memory Core;
- deterministic compiler context facts for profile and budget resolution;
- deterministic compiler context facts for profile write permissions and effect
  write classes;
- file-backed authority facts for the first compiler context fact set;
- typed representation boundary distinct from source AST;
- source-to-Core lowering for the initial pure local-record subset;
- source-to-Core lowering for lowerable `require ... else` obstruction arms;
- structured compiler error identity.

Out of scope:

- canonical Core bytes embedded in lowerer output;
- Core digest computation owned by the Core IR shelf;
- target-profile lowering;
- admission bundles;
- full source language coverage.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| CSPINE-REQ-001 | implemented | `resolve_module` resolves module/import coordinates and explicit context facts without collapsing into type checking or lowering. | issue #20 |
| CSPINE-REQ-002 | implemented | `type_check` produces a typed module boundary distinct from source AST and rejects unresolved or incompatible types. | issue #20 |
| CSPINE-REQ-003 | implemented | `lower_core` lowers the typed initial subset into structured in-memory Core IR. | issue #20, docs/abi/edict-core.cddl |
| CSPINE-REQ-004 | implemented | `compile_to_core` executes `validate_surface -> resolve_module -> type_check -> lower_core` in order. | issue #20 |
| CSPINE-REQ-005 | implemented | Profile and budget source coordinates require explicit deterministic context facts; missing facts reject instead of producing placeholder Core. | issue #20 |
| CSPINE-REQ-006 | implemented | The first lowerable subset covers `bounded-hello` style pure local-record intents and rejects out-of-subset constructs structurally. | fixtures/lang/bounds/bounded-hello.edict, issue #20 |
| CSPINE-REQ-007 | implemented | Compiler-spine errors expose stable stage and kind identities. | crates/edict-syntax/src/compiler.rs |
| CSPINE-REQ-008 | implemented | The compiler-spine lowerer embeds no canonical bytes, exact digest, target lowering, or admission artifacts in Core modules. | ROADMAP.md |
| CSPINE-REQ-009 | implemented | The compiler spine rejects source effect bodies whose effect write class is not allowed by the resolved operation profile. | issue #54 |
| CSPINE-REQ-010 | implemented | The first compiler context fact set can be loaded from explicit authority-facts files instead of caller-built in-memory context. | ROADMAP.md, docs/topics/authority-facts/test-plan.md |
| CSPINE-REQ-011 | implemented | The compiler spine lowers one annotated effectful `let ... else` shape into typed Core using file-backed profile, budget, and effect write-class facts. | issue #62 |
| CSPINE-REQ-012 | implemented | Effectful source shapes outside the first supported subset reject with stable compiler stage and kind identities before Core lowering. | issue #62 |
| CSPINE-REQ-013 | implemented | Duplicate failure keys in effect obstruction maps reject with a stable compiler error instead of silently dropping effects. | issue #62 |
| CSPINE-REQ-014 | implemented | Chained effect-call shapes reject instead of lowering as plain one-argument effects. | issue #62 |
| CSPINE-REQ-015 | implemented | Typed effect-call shapes reject instead of dropping unsupported type arguments. | issue #62 |
| CSPINE-REQ-016 | implemented | Obstruction binder identities are stable under source obstruction-arm reordering. | issue #62 |
| CSPINE-REQ-017 | implemented | The compiler spine lowers terminal and preserved-obstruction `require ... else` arms into distinct Core require-failure arms. | issue #129 |
| CSPINE-REQ-018 | implemented | Duplicate preserved-obstruction payload fields reject with a stable compiler error before Core digesting. | issue #129 |
| CSPINE-REQ-019 | implemented | The compiler spine recognizes the source `I32`, `I64`, `U32`, and `U64` scalar types, preserves an explicitly suffixed literal's exact width and signedness, propagates an unambiguous expected integer width through supported comparisons, annotations, and record returns, and rejects unconstrained, out-of-range, or cross-width values before Core lowering. | docs/SPEC_edict-language-v1.md |
| CSPINE-REQ-020 | implemented | An explicit source basis expression is type-checked only in the pure pre-body input environment and is preserved structurally in the lowered Core intent; `basis none` remains the explicit no-basis posture. | docs/SPEC_edict-language-v1.md, EDICT-LANG-BASIS-PURE-001 |
| CSPINE-REQ-021 | implemented | The compiler spine accepts a statically bounded `Bytes<max=N>` field in an operation input and preserves its exact bound in Core; unbounded bytes remain a surface error. | docs/SPEC_edict-language-v1.md |
| CSPINE-REQ-022 | implemented | A validated lawpack plus its exact direct target adapter derive the compiler profile, write-class, effect, and budget facts under the source module's import alias; application compilation must not require a caller-built or handwritten authority-facts substitute. | issue #169, docs/topics/lawpacks/test-plan.md |
| CSPINE-REQ-023 | implemented | Pure conditional expressions lower to canonical Core only when their predicate is valid and both branches have one compatible bounded type. | issue #192, docs/abi/edict-core.cddl |
| CSPINE-REQ-024 | implemented | Calls to pure helpers resolve only from facts owned by an exact digest-bound lawpack import, lower under their canonical exported coordinate, resolve non-primitive signatures from the same lawpack's bounded exported type closure, preserve declared type parameters in compiler facts, and reject generic declarations, missing signatures, or type-incompatible signatures before Core exists. | issue #192, docs/SPEC_edict-lawpack-abi-v1.md |
| CSPINE-REQ-025 | implemented | A `for` statement lowers only over a statically bounded list, carries its authored bound into Core, and rejects a bound below the iterable maximum or cumulative sequential/nested loop work above the operation step budget. | issue #192, docs/abi/edict-core.cddl |
| CSPINE-REQ-026 | implemented | Statement conditionals lower to isolated Core branch blocks; branch-local bindings never leak into the enclosing environment, and returns inside a branch reject until explicit result-join semantics exist. | issue #192, docs/abi/edict-core.cddl |
| CSPINE-REQ-027 | implemented | Coordinate loop bounds resolve only from compiler facts owned by an exact digest-bound lawpack import, lower under their canonical exported coordinate, and use the bound value for iterable-soundness and operation-budget checks. | issue #192, docs/SPEC_edict-lawpack-abi-v1.md |
| CSPINE-REQ-028 | implemented | An effectful branch-yield `let` lowers to one Core branch with an explicit result binding only when both isolated branches produce compatible bounded values; branch effects and locals remain scoped to their selected block, branch-local loop work retains the enclosing step factor and joins by worst-case path, and bare integer yields inherit an unambiguous width from either branch. | issue #192, docs/SPEC_edict-language-v1.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/bounds/bounded-hello.edict | Initial pure local-record source-to-Core fixture. | `compile_to_core` returns a structured `CoreModule` with expected records, profile, budget, predicate, nodes, and result. |
| fixtures/lang/operations/explicit-basis-u64.edict | Application-neutral executable-operation prerequisite fixture with exact `U64` coordinates, an explicit input-derived basis, and a digest-locked lawpack import. | `compile_to_core` preserves scalar width, basis expression, and lawpack identity without introducing target or runtime authority. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| CSPINE-TP-001 | implemented | Golden path | CSPINE-REQ-001, CSPINE-REQ-002, CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-006 | `bounded-hello` compiles through all stages to a `CoreModule` with expected structured fields. | bounded_hello_compiles_to_initial_core | fixtures/lang/bounds/bounded-hello.edict | In-memory Core only; no bytes or digests. |
| CSPINE-TP-002 | implemented | Stage boundary | CSPINE-REQ-001, CSPINE-REQ-002, CSPINE-REQ-003 | Explicit `resolve_module`, `type_check`, and `lower_core` calls expose distinct stage outputs. | compiler_spine_exposes_distinct_stage_boundaries | fixtures/lang/bounds/bounded-hello.edict | Prevents a hidden monolithic semantic pass. |
| CSPINE-TP-003 | implemented | Error handling | CSPINE-REQ-005, CSPINE-REQ-007 | Missing profile or budget facts return `CompilerStage::Resolve` plus `MissingContextFact`. | missing_context_facts_reject_in_resolve_stage | fixtures/lang/bounds/bounded-hello.edict | No placeholder Core budgets. |
| CSPINE-TP-004 | implemented | Error handling | CSPINE-REQ-002, CSPINE-REQ-007 | Unknown local named types return `CompilerStage::TypeCheck` plus `UnresolvedType`. | unresolved_local_types_reject_in_type_check_stage, unresolved_record_field_types_reject_in_type_check_stage | - | Surface validation still accepts the source. |
| CSPINE-TP-005 | implemented | Error handling | CSPINE-REQ-002, CSPINE-REQ-007 | Returning a record with the wrong field shape, or failing to return, returns `CompilerStage::TypeCheck` plus `TypeMismatch`. | record_return_shape_mismatch_rejects_in_type_check_stage, missing_return_rejects_in_type_check_stage | - | Asserts type identity, not diagnostic prose. |
| CSPINE-TP-006 | implemented | Boundary guard | CSPINE-REQ-008 | The lowered Core module carries no canonical bytes, digest, target IR, or admission fields. | initial_core_lowering_makes_no_canonical_or_target_claim | fixtures/lang/bounds/bounded-hello.edict | Keeps #21/#22 boundaries honest. |
| CSPINE-TP-007 | implemented | Boundary guard | CSPINE-REQ-007, CSPINE-REQ-009 | A write-class effect body under a read-only operation profile rejects in `CompilerStage::TypeCheck` with `ProfileEffectMismatch`. | read_only_profile_rejects_write_effect_body | - | Uses caller-built in-memory context facts. |
| CSPINE-TP-008 | implemented | Boundary guard | CSPINE-REQ-007, CSPINE-REQ-009 | A write-class effect in a `let` initializer without an obstruction handler rejects in `CompilerStage::TypeCheck` with `ProfileEffectMismatch`. | read_only_profile_rejects_write_effect_let_without_else | - | The compatibility check is independent of source obstruction syntax. |
| CSPINE-TP-009 | implemented | Golden path | CSPINE-REQ-005, CSPINE-REQ-009, CSPINE-REQ-010 | File-backed authority facts produce a compiler context that compiles `bounded-hello` and rejects a read-only profile/write-effect mismatch. | file_backed_authority_facts_compile_bounded_hello, file_backed_authority_facts_reject_write_effect_profile_mismatch | crates/edict-syntax/tests/authority_facts.rs | Proves file-loaded facts enter the same compiler path as in-memory facts. |
| CSPINE-TP-010 | implemented | Golden path | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-010, CSPINE-REQ-011 | A minimal annotated effectful `let ... else` source shape compiles through typed Core with a semantic effect node and deterministic obstruction mapping. | effectful_write_intent_lowers_to_typed_core_from_file_backed_facts | - | Uses explicit authority-facts files for profile, budget, and effect write-class facts. |
| CSPINE-TP-011 | implemented | Boundary guard | CSPINE-REQ-007, CSPINE-REQ-012 | Unsupported chained and typed effect calls reject in `CompilerStage::TypeCheck` with `UnsupportedSourceShape` before Core lowering. | chained_effect_calls_reject_before_core_lowering, typed_effect_calls_reject_before_core_lowering | - | Supporting branch-yield does not widen the effect-call ABI. |
| CSPINE-TP-012 | implemented | Error handling | CSPINE-REQ-007, CSPINE-REQ-013 | Duplicate failure keys in a supported obstruction map reject in `CompilerStage::TypeCheck` with `DuplicateObstructionFailure`. | duplicate_obstruction_failures_reject_before_core_lowering | - | Prevents silent effect-node omission when map keys collide. |
| CSPINE-TP-013 | implemented | Boundary guard | CSPINE-REQ-007, CSPINE-REQ-012, CSPINE-REQ-014 | A chained effect-call RHS rejects in `CompilerStage::TypeCheck` with `UnsupportedSourceShape`. | chained_effect_calls_reject_before_core_lowering | - | Prevents the lowerer from discarding the inner call shape. |
| CSPINE-TP-014 | implemented | Boundary guard | CSPINE-REQ-007, CSPINE-REQ-012, CSPINE-REQ-015 | A typed effect-call RHS rejects in `CompilerStage::TypeCheck` with `UnsupportedSourceShape`. | typed_effect_calls_reject_before_core_lowering | - | Prevents unsupported effect type arguments from disappearing during Core lowering. |
| CSPINE-TP-015 | implemented | Determinism | CSPINE-REQ-011, CSPINE-REQ-016 | Equivalent obstruction maps with the same arms in different source orders lower to identical Core. | obstruction_binder_ids_are_stable_by_failure_key | - | Ensures binder identities are derived after failure-key normalization. |
| CSPINE-TP-016 | implemented | Golden path | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-017 | Terminal `require ... else <obstruction>` and `require ... else continue obstructed { ... }` source shapes lower to distinct Core require-failure arms. | terminal_require_obstruction_lowers_to_core_failure_arm, continue_obstructed_require_lowers_to_core_failure_arm, terminal_and_continue_obstructed_require_arms_are_core_distinct | - | Core evidence only; no Target IR or runtime claim. |
| CSPINE-TP-017 | implemented | Error handling | CSPINE-REQ-007, CSPINE-REQ-018 | Duplicate preserved-obstruction payload fields reject in `CompilerStage::TypeCheck` with `DuplicateObstructionPayloadField`. | duplicate_obstruction_reason_payload_fields_reject_before_core_digest | - | Prevents silent payload overwrite before canonical Core digesting. |
| CSPINE-TP-018 | implemented | Golden path | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-019 | The operation prerequisite fixture lowers `U64` input fields and explicitly suffixed values as `U64` Core types and values without `i32` or `I64` narrowing, the complete supported `I32`/`I64`/`U32`/`U64` source set preserves its exact domain, and unary-negative signed minima reach Core without overflow. | operation_prerequisite_fixture_preserves_fixed_width_basis_and_lawpack_closure, fixed_width_integer_types_and_suffixes_preserve_exact_domains, signed_fixed_width_minima_preserve_exact_domains | fixtures/lang/operations/explicit-basis-u64.edict | Application-neutral witness for the fixed-width coordinate domain required downstream. |
| CSPINE-TP-019 | implemented | Boundary guard | CSPINE-REQ-002, CSPINE-REQ-007, CSPINE-REQ-019 | Out-of-range or negative unsigned literals and cross-width integer assignments reject in type checking with stable structured failure kinds. | out_of_range_u64_and_cross_width_values_reject_before_core | - | Integer width and signedness are semantic identity, not coercion hints. |
| CSPINE-TP-020 | implemented | Golden path | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-020 | The operation prerequisite fixture lowers `basis input.basis` to the exact Core field-expression tree, while a basis that references a body local or a directly constructed resolved module with a missing or duplicate basis clause rejects before Core exists. | operation_prerequisite_fixture_preserves_fixed_width_basis_and_lawpack_closure, body_local_cannot_become_an_intent_basis, direct_type_check_refuses_a_missing_basis_without_panicking, direct_type_check_refuses_duplicate_basis_clauses | fixtures/lang/operations/explicit-basis-u64.edict | Basis authoring remains distinct from runtime basis resolution or admission. |
| CSPINE-TP-021 | implemented | Golden path | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-021 | The operation prerequisite fixture lowers its replacement field as bounded Core `Bytes` with the authored maximum unchanged. | operation_prerequisite_fixture_preserves_fixed_width_basis_and_lawpack_closure | fixtures/lang/operations/explicit-basis-u64.edict | Adds only the bounded byte payload required by the downstream operation seam. |
| CSPINE-TP-022 | implemented | Golden path and boundary guard | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-019 | Bare integer literals inherit `U64`/`U32` from a typed comparison operand, explicit annotation, or record return field, while an unconstrained bare integer rejects before Core identity exists. | bare_integer_literals_inherit_unambiguous_fixed_width_context, unconstrained_bare_integer_literal_refuses_before_core | fixtures/lang/operations/explicit-basis-u64.edict | Expected-type propagation is limited to contexts whose width is unambiguous; it never defaults an unconstrained literal to a host or convenience width. |
| CSPINE-TP-023 | implemented | Integration | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-022 | The exact Hello Echo source, manifest, exports, and selected adapter derive module-local compiler facts and produce canonical Core without a test-built `CompilerContext`. | hello_echo_source_compiles_to_echo_target_ir_from_exact_lawpack_adapter | fixtures/lawpack/hello-echo/README.md | The same witness continues into Target IR but makes no package or runtime claim. |
| CSPINE-TP-024 | implemented | Golden path, error handling, and mutation sensitivity | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-023 | A pure ternary with compatible bounded branches lowers to a Core `if` expression, swapping the branches moves canonical Core identity, and incompatible branches reject with stable `TypeMismatch` identity. | pure_conditional_expression_lowers_to_core, pure_conditional_branch_mutation_moves_core_digest, pure_conditional_expression_rejects_incompatible_branches | crates/edict-syntax/tests/compiler_spine.rs | This is value selection only; effectful branch-yield and statement control flow remain separate slices. |
| CSPINE-TP-025 | implemented | Golden path and authority boundary | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-024 | An explicitly supplied pure-helper signature lowers only when its exact owning lawpack is imported; an absent helper, substituted import digest, incompatible argument, or declared generic parameter rejects with stable structured identity. An exact loaded lawpack derives the helper plus bounded exported type closure without handwritten facts. | digest_bound_pure_helper_call_lowers_to_core, missing_pure_helper_rejects_before_core, pure_helper_fact_without_exact_owning_import_rejects_before_core, pure_helper_argument_type_mismatch_rejects_before_core, exact_lawpack_pure_helper_signature_enters_source_compilation, exact_lawpack_exported_type_enters_pure_helper_signature_closure, generic_lawpack_helper_rejects_before_core | crates/edict-syntax/tests/compiler_spine.rs, crates/edict-syntax/tests/lawpack.rs | Core imports bind executable semantics, declared genericity, and exported signature shapes to the same manifest digest. |
| CSPINE-TP-026 | implemented | Golden path, budget guard, and mutation sensitivity | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-025 | A literal-bounded list loop lowers to a Core `for` node with a typed binder and nested requirement; too-small, individually over-budget, cumulatively over-budget, and nested-over-budget bounds reject, and changing a safe bound moves Core identity. | bounded_list_loop_lowers_to_core, bounded_list_loop_rejects_unsound_or_over_budget_bounds, bounded_list_loops_reject_cumulative_and_nested_over_budget_work, bounded_list_loop_bound_mutation_moves_core_digest | crates/edict-syntax/tests/compiler_spine.rs | Static accounting charges sequential loop caps additively and nested caps multiplicatively before Core exists. |
| CSPINE-TP-027 | implemented | Golden path and scope boundary | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-026 | An `if`/`else` statement lowers to isolated Core branch blocks; a branch-local binding is unresolved after the branch and a branch return rejects with stable structured identity. | statement_conditional_lowers_to_isolated_core_branches, statement_conditional_does_not_leak_locals, statement_conditional_rejects_branch_return | crates/edict-syntax/tests/compiler_spine.rs | Statement branches have no result binding; branch-yield owns explicit result joins. |
| CSPINE-TP-028 | implemented | Golden path and authority boundary | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-027 | A coordinate-bounded loop lowers with the canonical bound coordinate only when an exact imported lawpack owns a fact whose numeric maximum is at least as large as the iterable and within budget; missing, unowned, too-small, and over-budget facts reject before Core. | digest_bound_coordinate_loop_cap_lowers_to_core, missing_coordinate_loop_cap_rejects_before_core, coordinate_bound_fact_without_exact_owning_import_rejects_before_core, coordinate_loop_cap_rejects_unsound_or_over_budget_fact_values, exact_lawpack_constant_enters_loop_bound_compilation, exact_lawpack_u32_constant_enters_loop_bound_compilation | crates/edict-syntax/tests/compiler_spine.rs, crates/edict-syntax/tests/lawpack.rs | The imported lawpack binds the constant's meaning; the numeric value is used only for static proof and budget admission. |
| CSPINE-TP-029 | implemented | Golden path, budget guard, scope boundary, and mutation sensitivity | CSPINE-REQ-003, CSPINE-REQ-004, CSPINE-REQ-028 | An effectful branch-yield produces one outer result binding and two isolated Core blocks; incompatible scalar or anonymous-record yields reject; branch-local loops retain enclosing cumulative budget accounting; a bare integer yield inherits a fixed width from either branch; swapping compatible yielded values moves Core identity; and removing only the result binding moves Core identity. | effectful_branch_yield_lowers_to_bound_core_branch, effectful_branch_yield_rejects_incompatible_results, branch_yield_rejects_incompatible_anonymous_record_shapes, branch_yield_inside_loop_preserves_cumulative_budget, branch_yield_bare_integer_inherits_width_from_either_branch, effectful_branch_yield_mutation_moves_core_digest | crates/edict-syntax/tests/compiler_spine.rs | Target IR execution remains a separate capability; this case proves source-to-Core meaning only. |

## Determinism Obligations

- Tests inspect structured Rust values only.
- Compiler context facts are in-memory constants, not environment reads.
- Maps use deterministic key ordering.
- No test reads stdout, stderr, logs, wall-clock time, random values, or
  filesystem ordering.
- Canonical encoder behavior, reviewed golden bytes, and digest determinism are
  verified in the Core IR shelf.

## Open Gaps

- Full target/lawpack/shape artifact loading beyond authority-facts documents
  belongs to later lowerability and lawpack work.
- Bare effect statements, matches, variants, and effect obstruction payloads
  remain outside the lowerable subset.
