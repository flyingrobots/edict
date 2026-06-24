# Testing Workflow Test Plan

Status: current verification design for the repo testing workflow.

This plan verifies that the RED/GREEN workflow is discoverable, test-plan
driven, and connected to executable contract checks.

## Scope

In scope:

- contributor and agent entry-point references to this testing workflow shelf;
- RED/GREEN sequencing for nontrivial contract changes;
- topic test-plan rows as the planning ledger for requirements, cases, oracles,
  evidence, and fixtures;
- stable structured assertions for tests;
- fixture reuse across parser, validation, compiler-spine, canonicalization, and
  golden-artifact stages;
- local verification through `cargo xtask contract-check` and
  `cargo xtask verify`.

Out of scope:

- enforcing every RED/GREEN observation automatically;
- prescribing a single test file layout for every future crate;
- replacing domain-specific topic test plans.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| TESTS-REQ-001 | policy | Agent and contributor entry points reference the testing workflow shelf. | AGENTS.md, CONTRIBUTING.md |
| TESTS-REQ-002 | policy | Nontrivial contract changes use a visible RED/GREEN cycle before implementation is claimed complete. | AGENTS.md, CONTRIBUTING.md |
| TESTS-REQ-003 | policy | Topic test plans are the ledger for implemented, planned, gap, and policy requirements, cases, oracles, evidence, and fixtures. | AGENTS.md, docs/topics/README.md |
| TESTS-REQ-004 | policy | Tests assert software behavior and stable artifacts, not implementation details, documentation details, repository structure, diagnostic prose, or incidental output. | AGENTS.md |
| TESTS-REQ-005 | policy | Fixtures are reused across compatible stages, while executable encoder behavior and reviewed golden bytes remain separate steps. | fixtures/README.md, ROADMAP.md |
| TESTS-REQ-006 | policy | Local verification includes topic contract checking and the full `cargo xtask verify` gate. | AGENTS.md, xtask/src/main.rs |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| AGENTS.md | Agent-facing workflow entry point. | References the testing workflow topic and RED/GREEN rule. |
| CONTRIBUTING.md | Human-facing contribution entry point. | References the same testing workflow topic and local verification gate. |
| docs/topics/tests/README.md | Canonical testing workflow contract. | Describes the RED/GREEN contract, fixture reuse, and verification commands. |
| docs/topics/README.md | Topic index. | Lists the testing workflow topic. |
| fixtures/README.md | Fixture corpus contract. | Defines positive, negative, and golden fixture roles. |
| xtask/src/main.rs | Local verification implementation. | Provides `contract-check` and `verify`; tests cover tool behavior, not policy prose. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| TESTS-TP-001 | policy | Policy discovery | TESTS-REQ-001 | Review confirms the workflow policy is discoverable from contributor and agent entry points. | - | AGENTS.md, CONTRIBUTING.md, docs/topics/README.md | Documentation detail; do not encode as a Rust test. |
| TESTS-TP-002 | policy | Workflow policy | TESTS-REQ-002, TESTS-REQ-003, TESTS-REQ-004 | Review confirms the workflow policy requires RED/GREEN and behavior-level tests only. | - | docs/topics/tests/README.md, docs/topics/tests/test-plan.md | Policy detail; do not encode as a Rust test. |
| TESTS-TP-003 | policy | Fixture policy | TESTS-REQ-005 | Review confirms the workflow policy keeps executable encoder behavior separate from reviewed golden bytes and exact digests. | - | docs/topics/tests/README.md, ROADMAP.md, fixtures/README.md | Policy detail; do not encode as a Rust test. |
| TESTS-TP-004 | policy | Local gate policy | TESTS-REQ-006 | Review confirms the local gate is documented; executable tests remain focused on validator behavior. | - | xtask/src/main.rs | Tool behavior is covered by existing `contract_graph_*` tests. |

## Determinism Obligations

- Workflow policy is reviewed as documentation, not asserted through prose
  string tests.
- Executable tests assert software behavior, such as validator acceptance or
  rejection of structured input.
- Link and evidence resolution behavior is covered by existing `contract_graph_*`
  tests.

## Open Gaps

- No automated tool can prove that a contributor actually observed RED before
  implementation. The report or pull request body remains the evidence for that
  sequencing.
