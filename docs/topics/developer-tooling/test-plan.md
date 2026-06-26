# Developer Tooling Test Plan

## Scope

In scope:

- editor-facing source highlighting roles;
- comment visibility for editor integrations;
- deterministic source spans for highlighted tokens;
- Tree-sitter grammar source for the current accepted fixture subset;
- Tree-sitter highlight query captures aligned with the editor roles;
- Tree-sitter corpus examples that remain accepted by the reference parser.
- TextMate grammar artifacts for `.edict` lexical highlighting.

Out of scope:

- generated Tree-sitter parser packages;
- editor extension packaging;
- parse, resolution, type-check, Core lowering, or admission behavior.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| DEVTOOLS-REQ-001 | implemented | `highlight_source` emits deterministic editor roles for lexically meaningful Edict source spans, including comments that the parser otherwise treats as trivia. | issue #7 |
| DEVTOOLS-REQ-002 | implemented | The Tree-sitter grammar source and highlight query expose the current editor-facing Edict surface without claiming generated parser packages or editor extension support. | issue #7 |
| DEVTOOLS-REQ-003 | implemented | Tree-sitter corpus examples stay aligned with Edict's reference parser for the current accepted source subset. | issue #7 |
| DEVTOOLS-REQ-004 | implemented | The TextMate grammar exposes `.edict` lexical scopes aligned with the public editor-facing highlight roles without claiming packaged editor extension support. | issue #7 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/tooling/highlight-smoke.edict | Positive highlighting fixture covering comments, declarations, identifiers, type identifiers, strings, numbers, operators, and punctuation. | `highlight_source` emits the expected roles and no whitespace-only tokens. |
| grammars/tree-sitter-edict/grammar.js | Tree-sitter grammar source for editor syntax trees over the current accepted fixture subset. | The grammar declares the current editor-facing declaration, type, statement, expression, lexical, and keyword surface. |
| grammars/tree-sitter-edict/queries/highlights.scm | Tree-sitter highlight query for the grammar source. | The query emits captures for comments, keywords, strings, numbers, operators, punctuation, type identifiers, and callable identifiers. |
| grammars/tree-sitter-edict/test/corpus/current-subset.txt | Tree-sitter corpus examples covering the accepted fixture families. | Each example parses through `edict_syntax::parse_module`, and each corpus case has an expected syntax tree. |
| grammars/textmate/edict.tmLanguage.json | TextMate grammar artifact for `.edict` lexical scopes. | The grammar is valid JSON, registers `source.edict` for `.edict`, and exposes lexical scopes aligned with public highlight roles. |

## Cases

| ID | Status | Kind | Requirement | Scenario | Evidence | Fixtures | Oracle |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DEVTOOLS-TP-001 | implemented | Golden path | DEVTOOLS-REQ-001 | Highlighting a representative source fixture emits stable roles and spans for editor adapters. | highlight_source_emits_editor_roles_for_fixture | fixtures/lang/tooling/highlight-smoke.edict | Comments, keywords, identifiers, type identifiers, strings, numbers, operators, and punctuation are classified distinctly; whitespace-only spans are not emitted. |
| DEVTOOLS-TP-002 | implemented | Contract artifact | DEVTOOLS-REQ-002 | Tree-sitter grammar artifacts expose the current editor syntax surface and highlight roles. | tree_sitter_grammar_declares_current_editor_contract | grammars/tree-sitter-edict/grammar.js, grammars/tree-sitter-edict/queries/highlights.scm | Grammar rules cover package, imports, declarations, intent clauses, blocks, statements, match/call/type-call/record expressions, comments, strings, and numbers; highlight captures cover the public editor roles. |
| DEVTOOLS-TP-003 | implemented | Corpus alignment | DEVTOOLS-REQ-003 | Tree-sitter corpus examples remain accepted Edict source under the reference parser. | tree_sitter_corpus_examples_match_reference_parser | grammars/tree-sitter-edict/test/corpus/current-subset.txt | Corpus cases are nonempty, unique, include expected trees, cover the accepted fixture families, and parse through `parse_module`. |
| DEVTOOLS-TP-004 | implemented | Contract artifact | DEVTOOLS-REQ-002 | Tree-sitter keyword captures stay aligned with the public lexical highlighter keyword role. | tree_sitter_query_covers_public_keyword_roles | grammars/tree-sitter-edict/queries/highlights.scm | Every keyword lexeme emitted by `highlight_source` for current and reserved public keyword tokens is covered by the Tree-sitter keyword capture contract. |
| DEVTOOLS-TP-005 | implemented | Corpus alignment | DEVTOOLS-REQ-003 | Tree-sitter accepts formatted type-call suffixes that the reference parser accepts. | tree_sitter_corpus_examples_match_reference_parser | grammars/tree-sitter-edict/test/corpus/current-subset.txt | A typed call with whitespace before `<...>` parses as a call expression with type arguments and remains accepted by `parse_module`; `tree-sitter test` verifies the checked-in syntax tree. |
| DEVTOOLS-TP-006 | implemented | Corpus alignment | DEVTOOLS-REQ-003 | Tree-sitter accepts uppercase bare identifiers in positions where the reference parser accepts them. | tree_sitter_corpus_examples_match_reference_parser | grammars/tree-sitter-edict/test/corpus/current-subset.txt | Uppercase import aliases, intent names, parameters, local binders, and record shorthands parse without error and remain accepted by `parse_module`; `tree-sitter test` verifies the checked-in syntax tree. |
| DEVTOOLS-TP-007 | implemented | Contract artifact | DEVTOOLS-REQ-002 | Tree-sitter operator and punctuation captures stay aligned with the public lexical highlighter roles. | tree_sitter_query_operator_and_punctuation_roles_match_public_highlighter | grammars/tree-sitter-edict/queries/highlights.scm | Query captures do not assign a lexeme to both operator and punctuation, and captured operator or punctuation lexemes match the role emitted by `highlight_source`. |
| DEVTOOLS-TP-008 | implemented | Contract artifact | DEVTOOLS-REQ-004 | TextMate grammar declares the current editor contract for `.edict` lexical scopes. | textmate_grammar_declares_current_editor_contract | grammars/textmate/edict.tmLanguage.json | The grammar is valid JSON, names `source.edict`, registers `.edict`, includes comments, strings, numbers, keywords, types, operators, punctuation, and identifiers, and scopes line/block comments distinctly. |
| DEVTOOLS-TP-009 | implemented | Contract artifact | DEVTOOLS-REQ-004 | TextMate grammar keyword, operator, punctuation, type, and identifier patterns stay aligned with public highlighter roles. | textmate_grammar_covers_public_highlight_roles | grammars/textmate/edict.tmLanguage.json | Lexemes emitted by `highlight_source` as keywords, operators including `->`, punctuation, type identifiers, and identifiers are covered by the corresponding TextMate scope patterns. |
| DEVTOOLS-TP-010 | implemented | Contract artifact | DEVTOOLS-REQ-004 | TextMate grammar number patterns stay aligned with public highlighter spans for package version labels. | textmate_grammar_scopes_public_number_spans_in_version_labels | grammars/textmate/edict.tmLanguage.json | In a valid version label such as `@1_beta`, a number scope starts at and covers the public `Number` span emitted by `highlight_source`. |

## Known Gaps

- Generated Tree-sitter parser packages are not shipped yet.
- No editor extension package is shipped yet.
