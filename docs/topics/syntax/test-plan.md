# Syntax Test Plan

Status: current verification design for the Phase 1 syntax front end.

This plan is the verification side of the syntax topic shelf. It records how the
current contract is proven, what fixtures are authoritative, and which gaps must
be closed before later phases claim stronger coverage.

## Scope

In scope:

- lexical tokenization exposed through `edict_syntax::lex`;
- module parsing exposed through `edict_syntax::parse_module`;
- source AST shape for the minimal-v1 syntax subset;
- stable parser error identities through `ParseErrorKind`;
- deterministic contract metadata linking this plan to tests and fixtures.

Out of scope:

- semantic validation;
- Core IR lowering and canonical encodings;
- type resolution across imports;
- target/lawpack ABI validation.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| SYNTAX-REQ-001 | implemented | `parse_module` returns source AST only; later phases own semantic validation and lowering. | crates/edict-syntax/src/lib.rs |
| SYNTAX-REQ-002 | implemented | Module/package/import coordinates parse and preserve source-significant version spelling. | docs/SPEC_edict-language-v1.md |
| SYNTAX-REQ-003 | implemented | Imports parse supported kinds, validate digest literals, and reject minimal-v1 unsupported `capability`. | docs/SPEC_edict-language-v1.md |
| SYNTAX-REQ-004 | implemented | Type declarations parse Phase 1 type surface and reject empty enum declarations. | docs/SPEC_edict-language-v1.md |
| SYNTAX-REQ-005 | implemented | Integer suffixes remain source-significant in expression literals and static bounds. | docs/SPEC_edict-language-v1.md |
| SYNTAX-REQ-006 | implemented | Intent declaration syntax parses parameters, return type, clauses, and blocks; semantic requiredness is deferred. | docs/SPEC_edict-language-v1.md |
| SYNTAX-REQ-007 | implemented | Statement syntax includes effect call positions, guards, control flow, and bounded loops. | docs/SPEC_edict-language-v1.md |
| SYNTAX-REQ-008 | implemented | Expression syntax includes precedence, records, literals, conditionals, variants, and match. | docs/SPEC_edict-language-v1.md |
| SYNTAX-REQ-009 | implemented | Reserved keywords reject in bare-name positions while remaining legal after `.`. | docs/SPEC_edict-language-v1.md |
| SYNTAX-REQ-010 | implemented | Negative tests assert stable error kinds, not diagnostic prose or incidental output. | crates/edict-syntax/src/parser.rs |
| SYNTAX-REQ-011 | implemented | Source/surface validation rejects context-free semantic errors while deferring resolution, contextual typing, bound proof, and target/lawpack-dependent checks. | issue #10 |
| SYNTAX-REQ-012 | planned | Core lowering emits canonical Core IR with byte-stable golden artifacts. | issue #21, issue #22 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/bounds/bounded-hello.edict | Positive bounded scalar and record fixture. | Parsed AST structural equality in `bounded_hello_parses`. |
| fixtures/lang/effects/read-greeting.edict | Positive imports, effect calls, obstruction map, and record result fixture. | Parsed AST structural equality in `read_greeting_parses`. |
| fixtures/lang/effects/conditional-blob.edict | Positive branch-yield conditional effect fixture. | Parsed AST structural equality in `conditional_blob_fixture_parses`. |
| fixtures/lang/types/color-match.edict | Positive enum, variant, variant literal, and match fixture. | Parsed AST structural equality in `palette_fixture_parses`. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SYNTAX-TP-001 | implemented | Golden path | SYNTAX-REQ-001 | Public API returns a `Module` with expected package/declaration shape. | bounded_hello_parses | fixtures/lang/bounds/bounded-hello.edict | Source AST, not Core IR. |
| SYNTAX-TP-002 | implemented | Golden path | SYNTAX-REQ-002 | Exact version strings match source spelling. | multi_part_package_version, package_versions_preserve_underscores, import_versions_preserve_underscore_labels | - | Covers `_beta` labels. |
| SYNTAX-TP-003 | implemented | Error handling | SYNTAX-REQ-003 | Invalid digest and unsupported import syntax produce stable error kinds. | import_digest_literals_are_validated, capability_imports_are_rejected_in_v1 | - | Digest oracle is fixed `sha256:` length/hex rule. |
| SYNTAX-TP-004 | implemented | Golden path | SYNTAX-REQ-004 | Type AST nodes match expected records, bytes, variants, enums, and bounds. | bounded_hello_parses, bytes_accept_coordinate_bounds, enum_decl_parses, variant_type_with_and_without_payloads_parses | fixtures/lang/bounds/bounded-hello.edict | Structural AST equality. |
| SYNTAX-TP-005 | implemented | Error handling | SYNTAX-REQ-004 | Empty enums reject with `ParseErrorKind::EmptyEnum`. | empty_enum_and_empty_obstruction_maps_reject | - | Parser-level syntactic emptiness. |
| SYNTAX-TP-006 | implemented | Edge case | SYNTAX-REQ-005 | Integer suffix is preserved in `Expr::Int` and `BoundRef::Int`. | typed_integer_suffix, bound_integer_suffixes_are_preserved | - | Source-significant suffix oracle. |
| SYNTAX-TP-007 | implemented | Golden path | SYNTAX-REQ-006 | Intent clauses and bodies parse into expected AST. | bounded_hello_parses, read_greeting_parses | fixtures/lang/bounds/bounded-hello.edict, fixtures/lang/effects/read-greeting.edict | Requiredness deferred. |
| SYNTAX-TP-008 | implemented | Golden path | SYNTAX-REQ-007 | Bounded loops parse literal and coordinate bounds. | for_with_integer_bound_parses, for_with_coordinate_bound_parses | - | Loop semantic proof deferred. |
| SYNTAX-TP-009 | implemented | Known failure | SYNTAX-REQ-007 | Missing `bounded` rejects. | for_without_bounded_is_rejected | - | Syntactic mandatory bound. |
| SYNTAX-TP-010 | implemented | Error handling | SYNTAX-REQ-007 | Effect positions reject non-call expressions. | effect_positions_must_be_calls | - | Stable `NonCallEffect` kind. |
| SYNTAX-TP-011 | implemented | Golden path | SYNTAX-REQ-008 | Conditional expression and branch-yield AST nodes match expected structure. | pure_ternary_parses_as_let_value, ternary_is_usable_in_nested_expression_position, branch_yield_parses_only_as_let_rhs, conditional_blob_fixture_parses | fixtures/lang/effects/conditional-blob.edict | Branch-yield only in `let` RHS. |
| SYNTAX-TP-012 | implemented | Known failure | SYNTAX-REQ-008 | Branch-yield without `yield` rejects. | branch_yield_without_yield_is_rejected | - | Stable parser rejection. |
| SYNTAX-TP-013 | implemented | Golden path | SYNTAX-REQ-008 | Record shorthand, explicit fields, and spread parse. | bounded_hello_parses, read_greeting_parses | fixtures/lang/bounds/bounded-hello.edict, fixtures/lang/effects/read-greeting.edict | Shorthand semantics checked as AST entry. |
| SYNTAX-TP-014 | implemented | Golden path | SYNTAX-REQ-008 | Variants and match parse with payload binders and empty match rejects. | variant_literal_with_payload_parses, variant_literal_without_payload_parses, match_expr_with_binders_parses, empty_match_is_rejected, palette_fixture_parses | fixtures/lang/types/color-match.edict | Empty match has stable error kind. |
| SYNTAX-TP-015 | implemented | Edge case | SYNTAX-REQ-008 | Type-call parsing requires adjacency and does not swallow comparisons. | generic_call_vs_comparison_disambiguation, type_call_suffix_requires_adjacency_to_call_paren | - | Oracle is AST operator/call shape. |
| SYNTAX-TP-016 | implemented | Golden path | SYNTAX-REQ-008 | Boolean and digest value literals parse as dedicated AST nodes. | bool_and_digest_literals_are_real_literals | - | Digest oracle is fixed string payload. |
| SYNTAX-TP-017 | implemented | Error handling | SYNTAX-REQ-009 | Bare reserved keywords reject in value and binder positions. | keywords_are_rejected_as_bare_values, keywords_are_rejected_as_let_binder_names, reserved_words_are_rejected_in_all_binder_positions | - | Includes booleans as unusable binder names. |
| SYNTAX-TP-018 | implemented | Error handling | SYNTAX-REQ-009 | Import aliases, coordinate roots, and record shorthand reject reserved keywords. | reserved_keywords_are_rejected_as_import_aliases, reserved_words_are_rejected_as_coordinate_roots, reserved_words_are_rejected_as_record_shorthand | - | Dotted members remain legal. |
| SYNTAX-TP-019 | implemented | Golden path | SYNTAX-REQ-009 | Keywords after `.` and prelude constructors remain legal. | keywords_are_legal_as_member_names, prelude_constructors_are_not_reserved | - | Contextual keyword oracle. |
| SYNTAX-TP-020 | implemented | Error handling | SYNTAX-REQ-010 | Negative cases assert stable `ParseErrorKind` identities. | reserved_future_decls_are_rejected_at_top_level, missing_package_is_rejected, bytes_rejects_canonical_policy, unterminated_string_is_rejected | - | No stdout/stderr scraping. |
| SYNTAX-TP-021 | implemented | Semantic validation | SYNTAX-REQ-011 | The source/surface validator returns stable diagnostic kinds for context-free source errors and accepts unresolved downstream facts. | validate_module_remains_surface_stage_compatibility_alias, surface_validation_defers_import_and_name_resolution, surface_validation_defers_contextual_typing_and_loop_bound_proof, surface_validation_defers_obstruction_exhaustiveness | - | Owned by the semantic-validation shelf. |
| SYNTAX-TP-022 | planned | Golden artifact | SYNTAX-REQ-012 | Core lowering emits byte-stable canonical artifacts. | - | - | Owned by issues #21 and #22. |

## Determinism Obligations

- Tests use checked-in source strings and fixtures only.
- Negative tests assert structured `ParseErrorKind` values.
- Parser tests inspect returned AST state; they do not inspect stdout, stderr,
  logs, or human-readable diagnostic prose.
- Fixture paths are repository-relative and checked by the contract checker.
- Topic metadata is checked by `cargo xtask contract-check` and by the `xtask`
  test target in `cargo test --workspace`.

## Fuzz And Stress Plan

No fuzz target exists yet. The first useful fuzz target should generate bounded
token streams for the lexer/parser and minimize failures into permanent
deterministic regression cases in `crates/edict-syntax/tests/`.

## Open Gaps

- `SYNTAX-REQ-011`: semantic validation is planned for issue #10.
- `SYNTAX-REQ-012`: Core IR lowering and golden artifacts are planned for issues
  #21 and #22.
- Fixture coverage is not exhaustive across every `EDICT-LANG-*` row in
  `docs/REQUIREMENTS.md`; this shelf covers the landed Phase 1 syntax parser
  only.
