# Review Process Test Plan

Status: current verification design for pull request review policy.

## Scope

In scope:

- primary CodeRabbit review requirement when CodeRabbit is actively reviewing;
- alternate bot review request when CodeRabbit is rate limited or out of
  credits;
- waiting for the alternate review response before merge readiness;
- blocking merge when no reviewer has responded unless a maintainer explicitly
  overrides the gate.

Out of scope:

- live GitHub review state;
- branch-protection administration;
- requiring a specific paid review-bot plan;
- treating policy rows as a substitute for behavior tests.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| REVIEW-REQ-001 | implemented | When CodeRabbit is actively reviewing, CodeRabbit approval is required before merge. | docs/topics/review-process/policy.toml |
| REVIEW-REQ-002 | implemented | When CodeRabbit is rate limited, out of credits, or reports insufficient usage credits, the reviewer requests alternate bot review with `@codex review please`. | docs/topics/review-process/policy.toml |
| REVIEW-REQ-003 | implemented | Alternate bot fallback requires waiting for the alternate review response before merge readiness. | docs/topics/review-process/policy.toml |
| REVIEW-REQ-004 | implemented | A pull request without CodeRabbit approval or fallback review response remains blocked unless a maintainer explicitly overrides the review-bot gate. | docs/topics/review-process/policy.toml |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/topics/review-process/policy.toml | Structured review-bot fallback policy. | The xtask regression checks bot, fallback, response, and blocked-merge fields. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| REVIEW-TP-001 | implemented | Policy guard | REVIEW-REQ-001, REVIEW-REQ-002, REVIEW-REQ-003, REVIEW-REQ-004 | Structured review policy names CodeRabbit as primary, enumerates unavailable states, requires `@codex review please`, requires fallback response before merge, and blocks merge without review. | review_bot_fallback_policy_is_structured | docs/topics/review-process/policy.toml | Tests structured policy, not PR prose or live GitHub state. |

## Determinism Obligations

- The implemented check reads a checked-in structured policy file.
- The check does not query live GitHub review state or bot availability.
- Human operators still verify the actual PR state at merge time.

## Open Gaps

- No local tool automates the fallback comment and wait cycle yet.
