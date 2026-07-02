# Fixtures Test Plan

Status: current verification design for shared fixtures and reviewed artifacts.

## Scope

In scope:

- checked-in source fixtures consumed by executable parser, validator, compiler,
  highlighter, grammar, and canonicalization behavior;
- reviewed Core golden bytes and exact digest artifacts;
- contract-graph validation of fixture references in topic test plans.

Out of scope:

- lawpack, target-profile, contract-bundle, admission, and conformance fixture
  families that do not have executable owning behavior yet;
- tests that assert documentation prose about fixtures.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| FIXTURES-REQ-001 | implemented | Source fixtures under `fixtures/lang/` are executable behavior inputs, not illustrative prose. | fixtures/README.md |
| FIXTURES-REQ-002 | implemented | Reviewed Core golden artifacts are generated from the executable compiler and canonical encoder, then checked for exact bytes and digest stability. | fixtures/core/canonical/README.md, xtask/src/goldens.rs |
| FIXTURES-REQ-003 | implemented | Topic-shelf fixture references resolve to checked-in artifacts through the local contract graph. | xtask/src/contract_check.rs |
| FIXTURES-REQ-004 | gap | Target, lawpack, contract-bundle, admission, and conformance fixture families remain unpopulated until owning behavior lands. | fixtures/README.md, ROADMAP.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| fixtures/README.md | Fixture constitution. | Defines executable source, negative, and golden fixture roles plus placeholder digest handling. |
| fixtures/lang/bounds/bounded-hello.edict | Bounded source fixture. | Parses, validates, compiles to initial Core, and produces reviewed Core artifacts. |
| fixtures/lang/effects/conditional-blob.edict | Control-flow source fixture. | Parses and validates for current surface checks. |
| fixtures/lang/effects/read-greeting.edict | Imported effect source fixture. | Parses and validates with shape, lawpack, target, obstruction, and effect-call syntax. |
| fixtures/lang/types/color-match.edict | Variant/match source fixture. | Parses variant and match syntax. |
| fixtures/lang/tooling/highlight-smoke.edict | Developer-tooling source fixture. | Emits public editor highlight roles for comments, keywords, identifiers, strings, numbers, operators, punctuation, and type identifiers. |
| fixtures/core/canonical/bounded-hello.core.cbor | Reviewed canonical Core byte fixture. | Matches executable `edict.canonical-cbor/v1` output for `bounded-hello`. |
| fixtures/core/canonical/bounded-hello.core.sha256 | Reviewed Core digest fixture. | Matches executable `edict.core.module/v1` digest output for `bounded-hello`. |

## Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| FIXTURES-TP-001 | implemented | Source behavior | FIXTURES-REQ-001 | Source fixtures are consumed through public APIs and produce stable parser, validator, compiler, and highlighter behavior. | bounded_hello_parses, read_greeting_parses, conditional_blob_fixture_parses, palette_fixture_parses, phase1_fixtures_validate_semantically, bounded_hello_compiles_to_initial_core, highlight_source_emits_editor_roles_for_fixture | fixtures/lang/bounds/bounded-hello.edict, fixtures/lang/effects/conditional-blob.edict, fixtures/lang/effects/read-greeting.edict, fixtures/lang/types/color-match.edict, fixtures/lang/tooling/highlight-smoke.edict | Fixtures are behavior inputs, not prose anchors. |
| FIXTURES-TP-002 | implemented | Golden artifact | FIXTURES-REQ-002 | Reviewed Core byte and digest fixtures exactly match executable compiler and encoder output. | reviewed_core_golden_bytes_match_executable_encoder, reviewed_core_digest_matches_exact_fixture | xtask/src/goldens.rs, fixtures/lang/bounds/bounded-hello.edict, fixtures/core/canonical/bounded-hello.core.cbor, fixtures/core/canonical/bounded-hello.core.sha256 | `cargo xtask core-goldens --check` covers the same artifact contract. |
| FIXTURES-TP-003 | implemented | Contract graph | FIXTURES-REQ-003 | Topic-shelf test plans cannot cite missing fixture paths. | contract_graph_is_valid | xtask/src/contract_check.rs, xtask/src/tests.rs, fixtures/README.md | The checker validates referenced artifacts rather than prose. |
| FIXTURES-TP-004 | gap | Future corpus | FIXTURES-REQ-004 | No target, lawpack, bundle, admission, or conformance fixture corpus is claimed before owning behavior lands. | - | - | Add these families with the implementation slice that first consumes them. |

## Determinism Obligations

- Source fixture tests must assert stable software behavior, such as AST shape,
  error kind, validation status, compiler output, highlighter role, or generated
  artifact equality.
- Golden tests compare behavior-derived bytes and digests against reviewed
  artifacts.
- Fixture coverage must not pass merely because a file exists or prose contains
  a phrase.

## Open Gaps

- Target, lawpack, bundle, admission, and conformance fixture families remain
  future work.
- Additional reviewed Core golden fixtures should be added as lowerable Core
  language coverage expands.
