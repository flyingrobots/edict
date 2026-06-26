# Developer Tooling Test Plan

## Scope

In scope:

- editor-facing source highlighting roles;
- comment visibility for editor integrations;
- deterministic source spans for highlighted tokens;
- Tree-sitter grammar source for the current accepted fixture subset;
- Tree-sitter highlight query captures aligned with the editor roles;
- Tree-sitter corpus examples that remain accepted by the reference parser.

Out of scope:

- generated Tree-sitter parser packages;
- TextMate grammar generation;
- editor extension packaging;
- parse, resolution, type-check, Core lowering, or admission behavior.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| DEVTOOLS-REQ-001 | implemented | `highlight_source` emits deterministic editor roles for lexically meaningful Edict source spans, including comments that the parser otherwise treats as trivia. | issue #7 |
| DEVTOOLS-REQ-002 | implemented | The Tree-sitter grammar source and highlight query expose the current editor-facing Edict surface without claiming generated parser packages or editor extension support. | issue #7 |
| DEVTOOLS-REQ-003 | implemented | Tree-sitter corpus examples stay aligned with Edict's reference parser for the current accepted source subset. | issue #7 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/tooling/highlight-smoke.edict | Positive highlighting fixture covering comments, declarations, identifiers, type identifiers, strings, numbers, operators, and punctuation. | `highlight_source` emits the expected roles and no whitespace-only tokens. |
| grammars/tree-sitter-edict/grammar.js | Tree-sitter grammar source for editor syntax trees over the current accepted fixture subset. | The grammar declares the current editor-facing declaration, type, statement, expression, lexical, and keyword surface. |
| grammars/tree-sitter-edict/queries/highlights.scm | Tree-sitter highlight query for the grammar source. | The query emits captures for comments, keywords, strings, numbers, operators, punctuation, type identifiers, and callable identifiers. |
| grammars/tree-sitter-edict/test/corpus/current-subset.txt | Tree-sitter corpus examples covering the accepted fixture families. | Each example parses through `edict_syntax::parse_module`, and each corpus case has an expected syntax tree. |

## Cases

| ID | Status | Kind | Requirement | Scenario | Evidence | Fixtures | Oracle |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DEVTOOLS-TP-001 | implemented | Golden path | DEVTOOLS-REQ-001 | Highlighting a representative source fixture emits stable roles and spans for editor adapters. | highlight_source_emits_editor_roles_for_fixture | fixtures/lang/tooling/highlight-smoke.edict | Comments, keywords, identifiers, type identifiers, strings, numbers, operators, and punctuation are classified distinctly; whitespace-only spans are not emitted. |
| DEVTOOLS-TP-002 | implemented | Contract artifact | DEVTOOLS-REQ-002 | Tree-sitter grammar artifacts expose the current editor syntax surface and highlight roles. | tree_sitter_grammar_declares_current_editor_contract | grammars/tree-sitter-edict/grammar.js, grammars/tree-sitter-edict/queries/highlights.scm | Grammar rules cover package, imports, declarations, intent clauses, blocks, statements, match/call/type-call/record expressions, comments, strings, and numbers; highlight captures cover the public editor roles. |
| DEVTOOLS-TP-003 | implemented | Corpus alignment | DEVTOOLS-REQ-003 | Tree-sitter corpus examples remain accepted Edict source under the reference parser. | tree_sitter_corpus_examples_match_reference_parser | grammars/tree-sitter-edict/test/corpus/current-subset.txt | Corpus cases are nonempty, unique, include expected trees, cover the accepted fixture families, and parse through `parse_module`. |
| DEVTOOLS-TP-004 | implemented | Contract artifact | DEVTOOLS-REQ-002 | Tree-sitter keyword captures stay aligned with the public lexical highlighter keyword role. | tree_sitter_query_covers_public_keyword_roles | grammars/tree-sitter-edict/queries/highlights.scm | Every keyword lexeme emitted by `highlight_source` for current and reserved public keyword tokens is covered by the Tree-sitter keyword capture contract. |
| DEVTOOLS-TP-005 | implemented | Corpus alignment | DEVTOOLS-REQ-003 | Tree-sitter accepts formatted type-call suffixes that the reference parser accepts. | tree_sitter_corpus_examples_match_reference_parser | grammars/tree-sitter-edict/test/corpus/current-subset.txt | A typed call with whitespace before `<...>` parses as a call expression with type arguments and remains accepted by `parse_module`; `tree-sitter test` verifies the checked-in syntax tree. |
| DEVTOOLS-TP-006 | implemented | Corpus alignment | DEVTOOLS-REQ-003 | Tree-sitter accepts uppercase bare identifiers in positions where the reference parser accepts them. | tree_sitter_corpus_examples_match_reference_parser | grammars/tree-sitter-edict/test/corpus/current-subset.txt | Uppercase import aliases, intent names, parameters, local binders, and record shorthands parse without error and remain accepted by `parse_module`; `tree-sitter test` verifies the checked-in syntax tree. |

## Known Gaps

- Generated Tree-sitter parser packages are not shipped yet.
- TextMate grammar artifacts are not implemented yet.
- No editor extension package is shipped yet.
