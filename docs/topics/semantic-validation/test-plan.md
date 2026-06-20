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
| SEMVAL-REQ-001 | implemented | Validation returns structured semantic errors with stable kinds and source spans. | crates/edict-syntax/src/semantic.rs |
| SEMVAL-REQ-002 | implemented | Runtime `String` and `Bytes` type references must be explicitly bounded. | EDICT-LANG-BOUNDS-001 |
| SEMVAL-REQ-003 | implemented | Each intent must declare at least one operation mode: `profile` or `implements`. | EDICT-LANG-INTENT-CLAUSES-001 |
| SEMVAL-REQ-004 | implemented | Each intent must declare a `budget` clause. | EDICT-LANG-INTENT-CLAUSES-001 |
| SEMVAL-REQ-005 | implemented | Each intent must declare a `basis` clause until template resolution exists. | EDICT-LANG-INTENT-CLAUSES-001 |
| SEMVAL-REQ-006 | implemented | Singleton intent clauses reject duplicates. | docs/SPEC_edict-language-v1.md |
| SEMVAL-REQ-007 | planned | Locals must not shadow import, package, type, or prelude names. | EDICT-LANG-NOSHADOW-001 |
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
| SEMVAL-TP-002 | implemented | Error handling | SEMVAL-REQ-002 | Nested unbounded scalars produce `UnboundedScalar` for each occurrence. | unbounded_runtime_scalars_are_rejected_recursively | - | Recurses through `Option` and `List`. |
| SEMVAL-TP-003 | implemented | Error handling | SEMVAL-REQ-003, SEMVAL-REQ-004, SEMVAL-REQ-005 | Missing required intent clauses produce `MissingOperationMode`, `MissingBudget`, and `MissingBasis`. | intent_required_clauses_are_validated | - | Does not require import resolution. |
| SEMVAL-TP-004 | implemented | Golden path | SEMVAL-REQ-003 | Either `profile` or `implements` satisfies operation mode. | profile_or_implements_satisfies_operation_mode | - | Both remain legal. |
| SEMVAL-TP-005 | implemented | Error handling | SEMVAL-REQ-006 | Duplicate singleton clauses produce `DuplicateIntentClause`. | duplicate_singleton_intent_clauses_are_rejected | - | Covers profile, basis, and budget duplicates. |
| SEMVAL-TP-006 | planned | Error handling | SEMVAL-REQ-007 | Shadowing produces stable semantic error kinds. | - | - | Requires symbol table. |
| SEMVAL-TP-007 | planned | Error handling | SEMVAL-REQ-008 | Integer suffix/context mismatch produces stable semantic error kind. | - | - | Requires contextual typing. |
| SEMVAL-TP-008 | planned | Error handling | SEMVAL-REQ-009 | Unprovable loop bound produces stable semantic error kind. | - | - | Requires cardinality reasoning. |
| SEMVAL-TP-009 | planned | Error handling | SEMVAL-REQ-010 | Missing obstruction arm produces stable semantic error kind. | - | - | Requires target/lawpack failure facts. |
| SEMVAL-TP-010 | planned | Relapse zoo | SEMVAL-REQ-011 | Relapse fixtures reject non-lawful Core/assurance constructs with stable kinds or golden negative artifacts. | - | - | Requires Core IR and issue #3 artifacts. |

## Determinism Obligations

- Semantic tests inspect returned `SemanticErrorKind` values and AST validation
  state.
- Tests do not inspect stdout, stderr, logs, or diagnostic prose.
- Source inputs are inline strings or checked-in fixtures.
- The contract graph is checked by `cargo xtask contract-check`.

## Open Gaps

- Symbol-table validation for shadowing and namespace collisions.
- Contextual typing for integer suffix checks.
- Cardinality proof machinery for loop bounds.
- Target/lawpack facts for obstruction exhaustiveness.
- Core relapse-zoo fixtures after Core IR exists.
