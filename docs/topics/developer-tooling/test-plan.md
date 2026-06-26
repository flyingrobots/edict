# Developer Tooling Test Plan

## Scope

In scope:

- editor-facing source highlighting roles;
- comment visibility for editor integrations;
- deterministic source spans for highlighted tokens.

Out of scope:

- Tree-sitter grammar generation;
- TextMate grammar generation;
- editor extension packaging;
- parse, resolution, type-check, Core lowering, or admission behavior.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| DEVTOOLS-REQ-001 | implemented | `highlight_source` emits deterministic editor roles for lexically meaningful Edict source spans, including comments that the parser otherwise treats as trivia. | issue #7 |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/lang/tooling/highlight-smoke.edict | Positive highlighting fixture covering comments, declarations, identifiers, type identifiers, strings, numbers, operators, and punctuation. | `highlight_source` emits the expected roles and no whitespace-only tokens. |

## Cases

| ID | Status | Kind | Requirement | Scenario | Evidence | Fixtures | Oracle |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DEVTOOLS-TP-001 | implemented | Golden path | DEVTOOLS-REQ-001 | Highlighting a representative source fixture emits stable roles and spans for editor adapters. | highlight_source_emits_editor_roles_for_fixture | fixtures/lang/tooling/highlight-smoke.edict | Comments, keywords, identifiers, type identifiers, strings, numbers, operators, and punctuation are classified distinctly; whitespace-only spans are not emitted. |

## Known Gaps

- Tree-sitter and TextMate grammar artifacts are not implemented yet.
- No editor extension package is shipped yet.
