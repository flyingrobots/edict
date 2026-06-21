# Semantic Validation Test Plan

Status: current verification design for source-AST semantic validation.

## Scope

In scope:

- `edict_syntax::validate_module`;
- stable `SemanticErrorKind` identities;
- source-AST checks independent of import resolution and Core IR;
- deterministic fixture and oracle mapping.

Out of scope for this first slice:

- resolved type checking;
- import/lawpack/target profile validation;
- Core IR lowering and relapse-zoo golden artifacts;
- participant/runtime admission checks.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| SEMVAL-REQ-001 | implemented | Validation returns structured semantic errors with stable kinds and source span payloads. | crates/edict-syntax/src/semantic.rs |
| SEMVAL-REQ-002 | implemented | Runtime `String` and `Bytes` type references must be explicitly bounded. | EDICT-LANG-BOUNDS-001 |
| SEMVAL-REQ-003 | implemented | Each intent must declare at least one operation mode: `profile` or `implements`. | EDICT-LANG-INTENT-CLAUSES-001 |
| SEMVAL-REQ-004 | implemented | Each intent must declare a `budget` clause. | EDICT-LANG-INTENT-CLAUSES-001 |
| SEMVAL-REQ-005 | implemented | Each intent must declare a `basis` clause until template resolution exists. | EDICT-LANG-INTENT-CLAUSES-001 |
| SEMVAL-REQ-006 | implemented | Singleton intent clauses reject duplicates. | docs/SPEC_edict-language-v1.md |
| SEMVAL-REQ-007 | implemented | Source binders must not shadow visible module/prelude names, parameters, or earlier locals; module-scope import/declaration names cannot collide. | EDICT-LANG-NOSHADOW-001 |
| SEMVAL-REQ-008 | planned | Integer literal suffixes must agree with contextual integer types. | EDICT-LANG-INTLIT-002 |
| SEMVAL-REQ-009 | planned | Loop bounds must be provable against list cardinality. | EDICT-LANG-LOOP-001 |
| SEMVAL-REQ-010 | planned | Obstruction maps must be exhaustive over domain-mappable failure coordinates. | EDICT-LANG-OBSTRUCT-EXHAUST-001 |
| SEMVAL-REQ-011 | planned | Core/assurance relapse-zoo fixtures reject graph nouns, ambient clocks, randomness, host callbacks, unbounded closures, hidden appends, and related non-lawful constructs. | issue #10 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/bounds/bounded-hello.edict | Positive bounded scalar and required-clause fixture. | `validate_module` returns `Ok(())`. |
| fixtures/lang/effects/read-greeting.edict | Positive effect body and obstruction fixture. | `validate_module` returns `Ok(())`. |
| fixtures/lang/effects/conditional-blob.edict | Positive branch-yield fixture. | `validate_module` returns `Ok(())`. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SEMVAL-TP-001 | implemented | Golden path | SEMVAL-REQ-001 | Valid Phase 1 fixtures return `Ok(())`. | phase1_fixtures_validate_semantically | fixtures/lang/bounds/bounded-hello.edict, fixtures/lang/effects/read-greeting.edict, fixtures/lang/effects/conditional-blob.edict | Source-AST validator only. |
| SEMVAL-TP-002 | implemented | Error handling | SEMVAL-REQ-002 | Nested unbounded scalars produce `UnboundedScalar` for each occurrence. | unbounded_runtime_scalars_are_rejected_recursively, unbounded_runtime_scalars_are_rejected_in_declaration_type_surfaces, unbounded_runtime_scalars_are_rejected_in_intent_and_expression_surfaces | - | Covers `Option`, `List`, `Map`, `CapabilityRef`, variant payloads, intent params/returns, typed `let`, and expression type args. |
| SEMVAL-TP-003 | implemented | Error handling | SEMVAL-REQ-003, SEMVAL-REQ-004, SEMVAL-REQ-005 | Missing required intent clauses produce `MissingOperationMode`, `MissingBudget`, and `MissingBasis`. | intent_required_clauses_are_validated | - | Does not require import resolution. |
| SEMVAL-TP-004 | implemented | Golden path | SEMVAL-REQ-003 | Either `profile` or `implements` satisfies operation mode. | profile_or_implements_satisfies_operation_mode | - | Both remain legal. |
| SEMVAL-TP-005 | implemented | Error handling | SEMVAL-REQ-006 | Duplicate singleton clauses produce `DuplicateIntentClause`. | duplicate_singleton_intent_clauses_are_rejected, duplicate_implements_and_footprint_clauses_are_rejected | - | Covers `profile`, `implements`, `basis`, `footprint`, and `budget` duplicates. |
| SEMVAL-TP-006 | implemented | Error handling | SEMVAL-REQ-007 | Module-scope declaration and import aliases cannot collide in the same namespace. | module_namespace_collisions_are_rejected | - | Source-AST environment only. |
| SEMVAL-TP-007 | implemented | Error handling | SEMVAL-REQ-007 | Intent parameters and local binders cannot shadow module names, parameters, or earlier locals. | local_binders_cannot_shadow_visible_names | - | Source-AST environment only. |
| SEMVAL-TP-008 | implemented | Edge/scope | SEMVAL-REQ-007 | Branch, loop, match, obstruction-map, and branch-yield binders are scoped deterministically and do not leak into sibling or outer scopes. | branch_and_loop_binders_are_scoped, branch_yield_binders_are_scoped, clause_expression_binders_see_parameters, expression_binders_cannot_shadow_visible_names, obstruction_map_binders_cannot_shadow_visible_names | - | Source-AST environment only. |
| SEMVAL-TP-009 | planned | Error handling | SEMVAL-REQ-008 | Integer suffix/context mismatch produces stable semantic error kind. | - | - | Requires contextual typing. |
| SEMVAL-TP-010 | planned | Error handling | SEMVAL-REQ-009 | Unprovable loop bound produces stable semantic error kind. | - | - | Requires cardinality reasoning. |
| SEMVAL-TP-011 | planned | Error handling | SEMVAL-REQ-010 | Missing obstruction arm produces stable semantic error kind. | - | - | Requires target/lawpack failure facts. |
| SEMVAL-TP-012 | planned | Relapse zoo | SEMVAL-REQ-011 | Relapse fixtures reject non-lawful Core/assurance constructs with stable kinds or golden negative artifacts. | - | - | Requires Core IR and issue #3 artifacts. |

## Determinism Obligations

- Semantic tests inspect returned `SemanticErrorKind` values and AST validation
  state.
- Exact error ordering is not a Phase 2 contract; tests may normalize errors when
  a case is about identity rather than traversal order.
- Tests do not inspect stdout, stderr, logs, or diagnostic prose.
- Source inputs are inline strings or checked-in fixtures.
- The contract graph is checked by `cargo xtask contract-check`.

## Open Gaps

- Contextual typing for integer suffix checks.
- Cardinality proof machinery for loop bounds.
- Target/lawpack facts for obstruction exhaustiveness.
- Core relapse-zoo fixtures after Core IR exists.
- Clause-level diagnostic spans; duplicate singleton diagnostics currently report
  at the enclosing intent span because intent clauses do not retain spans.
