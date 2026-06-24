# Documentation Standards Test Plan

Status: current verification design for Edict documentation workflow.

This plan verifies that Edict's local documentation standard is discoverable,
reader-task oriented, and connected to executable contract checks.

## Scope

In scope:

- repo entry-point links to the documentation standards shelf;
- page type separation for tutorials, how-tos, reference, explanation,
  troubleshooting, and contributor guidance;
- Edict-specific documentation coverage by capability;
- example honesty, command/output separation, and placeholder rules;
- documentation impact declarations for contract-bearing changes;
- deterministic checks plus human reader-task review.

Out of scope:

- mass-converting existing documentation into a new directory layout;
- automatically proving prose quality;
- generating a full documentation catalog;
- building a public documentation site.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| DOCS-REQ-001 | policy | Documentation pages have one primary reader job and use the local page type vocabulary. | docs/topics/documentation/README.md |
| DOCS-REQ-002 | policy | Topic shelves remain contributor/evidence material and do not replace user-facing tutorials or task guides. | AGENTS.md, docs/topics/README.md |
| DOCS-REQ-003 | policy | Edict tracks documentation coverage by capability and reader need. | docs/topics/documentation/README.md |
| DOCS-REQ-004 | policy | Examples distinguish runnable, illustrative, and abridged use; copyable shell commands omit prompts. | docs/topics/documentation/README.md, fixtures/README.md |
| DOCS-REQ-005 | policy | Contract-bearing changes update affected docs or declare `docs-impact: none`; changed documentation preserves page type. | AGENTS.md, CONTRIBUTING.md, docs/topics/documentation/README.md |
| DOCS-REQ-006 | policy | Documentation quality uses deterministic checks for software facts and human review for reader-task success. | docs/topics/documentation/README.md, xtask/src/main.rs |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| AGENTS.md | Agent-facing documentation workflow entry point. | Links to the documentation standards topic and states impact rules. |
| CONTRIBUTING.md | Human-facing contributor entry point. | Links to the same documentation standards topic and summarizes reader-task practice. |
| docs/topics/documentation/README.md | Canonical local documentation policy. | Defines page types, coverage matrix, example rules, and impact rules. |
| docs/topics/README.md | Topic index. | Lists the documentation standards topic. |
| docs/README.md | Documentation router. | Links to the documentation standards topic. |
| fixtures/README.md | Fixture corpus contract. | Defines placeholder digest handling and golden fixture expectations. |
| xtask/src/main.rs | Local verification implementation. | Provides contract-check behavior tests; documentation policy prose is reviewed, not string-tested. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DOCS-TP-001 | policy | Contract discovery | DOCS-REQ-001, DOCS-REQ-002 | Review confirms the documentation standards policy is discoverable from contributor, agent, and docs entry points. | - | AGENTS.md, CONTRIBUTING.md, docs/README.md, docs/topics/README.md | Documentation detail; do not encode as a Rust test. |
| DOCS-TP-002 | policy | Page type policy | DOCS-REQ-001, DOCS-REQ-002 | Review confirms the documentation shelf names the reader-task model and local page types. | - | docs/topics/documentation/README.md | Policy detail; do not encode as a Rust test. |
| DOCS-TP-003 | policy | Coverage policy | DOCS-REQ-003 | Review confirms the documentation shelf contains an Edict coverage matrix. | - | docs/topics/documentation/README.md | Policy detail; do not encode as a Rust test. |
| DOCS-TP-004 | policy | Example and impact policy | DOCS-REQ-004, DOCS-REQ-005 | Review confirms the documentation shelf states runnable example rules, copyable shell command rules, `docs-impact: none`, and page-type preservation. | - | docs/topics/documentation/README.md, docs/topics/documentation/test-plan.md | Policy detail; do not encode as a Rust test. |
| DOCS-TP-005 | policy | Local gate policy | DOCS-REQ-006 | Review confirms deterministic checks are described as fact checks and behavior tests, with prose quality left to human review. | - | docs/topics/documentation/README.md, docs/topics/documentation/test-plan.md | Tool behavior is covered by existing `contract_graph_*` tests. |

## Determinism Obligations

- Documentation policy is reviewed as documentation, not asserted through prose
  string tests.
- Executable tests assert software behavior, such as validator acceptance or
  rejection of structured input.
- Contract graph validation behavior checks local links, requirement sources,
  evidence test names, and fixture paths.

## Open Gaps

- No `docs/catalog.yaml` exists yet.
- Automated coverage checking does not yet map public surfaces to page types.
- Executable tutorial harnesses are deferred until Edict has a user-facing CLI
  surface.
