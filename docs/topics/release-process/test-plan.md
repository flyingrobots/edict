# Release Process Test Plan

Status: current verification design for Edict release automation.

## Scope

In scope:

- tag-triggered GitHub Release publication;
- main-branch reachability checks for release tag targets;
- release notes lookup by full tag name;
- prerelease classification for SemVer prerelease versions;
- no crates.io publication in the current release workflow;
- deterministic local checks for workflow contract drift.

Out of scope:

- crates.io publication;
- binary asset signing;
- artifact upload beyond GitHub Release notes;
- automatic retry of failed historical tag events.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| RELEASE-REQ-001 | implemented | Release publication is triggered by pushed `v*` tags. | .github/workflows/release.yml |
| RELEASE-REQ-002 | implemented | Release tags must target commits reachable from `origin/main`. | .github/workflows/release.yml |
| RELEASE-REQ-003 | implemented | Release notes are loaded by full tag name from `docs/releases/${TAG}.md`. | .github/workflows/release.yml |
| RELEASE-REQ-004 | implemented | SemVer prerelease tags publish as GitHub prereleases. | .github/workflows/release.yml |
| RELEASE-REQ-005 | implemented | The current release workflow does not publish crates or other package artifacts. | .github/workflows/release.yml |
| RELEASE-REQ-006 | implemented | Pushed release tags are durable; recovery must not move, delete, or recreate release tags. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-007 | implemented | `v0.2.0-alpha.1` release notes state the Core schema scope and explicit non-goals. | docs/releases/v0.2.0-alpha.1.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/releases/v0.1.0-alpha.1.md | Published release notes for the first front-end alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.2.0-alpha.1.md | Prepared release notes for the Core semantic model and schema alpha. | The notes state included scope and explicit non-goals. |
| docs/topics/release-process/policy.toml | Structured release-tag recovery policy. | Tag mutation is forbidden and recovery publishes the existing valid tag. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| RELEASE-TP-001 | implemented | Golden path | RELEASE-REQ-001, RELEASE-REQ-002, RELEASE-REQ-003, RELEASE-REQ-004, RELEASE-REQ-005 | The workflow contains the tag trigger, main reachability guard, full-tag release-notes path, verified GitHub Release creation, prerelease flag, and no package publish command. | release_workflow_publishes_only_main_reachable_tags | docs/releases/v0.1.0-alpha.1.md | Static workflow contract regression. |
| RELEASE-TP-002 | implemented | Policy guard | RELEASE-REQ-006 | Structured policy forbids tag mutation and names existing-valid-tag publication as recovery. | release_tag_recovery_policy_is_structured | docs/topics/release-process/policy.toml | Policy evidence is structured, not prose. |
| RELEASE-TP-003 | implemented | Boundary guard | RELEASE-REQ-007 | v0.2 notes include Core schema scope and all explicit non-goals for lowering, encoder, bytes, digests, targets, and admission. | v0_2_release_notes_state_core_boundary | docs/releases/v0.2.0-alpha.1.md | Prevents release notes from overclaiming the Core milestone. |

## Determinism Obligations

- Release workflow contract tests inspect checked-in workflow text, not live
  GitHub state.
- Release tags must be explicit operator actions; no test creates or pushes live
  tags.
- Recovery documentation must distinguish workflow fixes from tag mutation.
- Tests do not scrape human diagnostic prose from Actions logs.
- Release-note tests inspect checked-in Markdown as a stable release artifact,
  not terminal output or live GitHub state.

## Open Gaps

- The release workflow does not yet have a local end-to-end dry run harness.
- The current checker proves workflow contract structure, not GitHub API
  availability.
- No crates.io policy exists; package publication remains intentionally absent.
