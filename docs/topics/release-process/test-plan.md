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
| RELEASE-REQ-006 | implemented | Pushed release tags are durable; recovery must not move, delete, or recreate release tags. | AGENTS.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/releases/v0.1.0-alpha.1.md | Published release notes for the first front-end alpha. | The release workflow looks up this file by full tag name. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| RELEASE-TP-001 | implemented | Golden path | RELEASE-REQ-001, RELEASE-REQ-002, RELEASE-REQ-003, RELEASE-REQ-004, RELEASE-REQ-005 | The workflow contains the tag trigger, main reachability guard, full-tag release-notes path, verified GitHub Release creation, prerelease flag, and no package publish command. | release_workflow_publishes_only_main_reachable_tags | docs/releases/v0.1.0-alpha.1.md | Static workflow contract regression. |
| RELEASE-TP-002 | implemented | Policy guard | RELEASE-REQ-006 | Repository instructions forbid force operations, so release recovery cannot rely on moving or recreating pushed tags. | agents_topic_shelf_policy_is_present | - | Policy evidence in `AGENTS.md`. |

## Determinism Obligations

- Release workflow contract tests inspect checked-in workflow text, not live
  GitHub state.
- Release tags must be explicit operator actions; no test creates or pushes live
  tags.
- Recovery documentation must distinguish workflow fixes from tag mutation.
- Tests do not scrape human diagnostic prose from Actions logs.

## Open Gaps

- The release workflow does not yet have a local end-to-end dry run harness.
- The current checker proves workflow contract structure, not GitHub API
  availability.
- No crates.io policy exists; package publication remains intentionally absent.
