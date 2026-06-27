# Release Process Test Plan

Status: current verification design for Edict release automation.

## Scope

In scope:

- tag-triggered GitHub Release publication;
- main-branch reachability checks for release tag targets;
- release notes lookup by full tag name;
- prerelease classification for SemVer prerelease versions;
- no crates.io publication in the current release workflow;
- operator runbook phases and required release checks;
- release-prep PR auto-tagging after successful `main` CI;
- manual recovery dispatch for verified release-prep merge commits;
- milestone closure after successful release publication;
- docs/topics coverage and accuracy audit thresholds before release;
- release thesis, previous-tag diff reconciliation, no-crates verification, and
  release-report evidence;
- structured release metadata for alpha scope and non-goal boundaries;
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
| RELEASE-REQ-007 | implemented | Structured release policy captures the `v0.2.0-alpha.1` Core schema scope and explicit non-goals. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-008 | implemented | Scheduled alpha release changelog dates match their structured release policy target dates. | CHANGELOG.md, docs/topics/release-process/policy.toml |
| RELEASE-REQ-009 | implemented | Release preparation follows a documented runbook with branch prep, local verification, PR merge gate, tag publication, workflow watch, evidence capture, and non-mutating recovery phases. | docs/topics/release-process/runbook.md, docs/topics/release-process/policy.toml |
| RELEASE-REQ-010 | implemented | Structured release policy captures the `v0.3.0-alpha.1` compiler-spine, canonical encoder, reviewed golden fixture, exact digest, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-011 | implemented | Structured release policy captures the `v0.4.0-alpha.1` target-profile, lowerability, contract-bundle validation, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-012 | implemented | Successful `main` CI on a merged `release/vX.Y.Z-alpha.N-prep` pull request creates an immutable `vX.Y.Z-alpha.N` tag and dispatches release publication. | .github/workflows/auto-release-tag.yml, docs/topics/release-process/policy.toml |
| RELEASE-REQ-013 | implemented | Release publication closes the matching GitHub milestone only after the release exists and the milestone has zero open issues. | .github/workflows/release.yml, docs/topics/release-process/policy.toml |
| RELEASE-REQ-014 | implemented | Structured release policy captures the `v0.5.0-alpha.1` Gate C admission-boundary scope, release automation, and explicit Continuum-owned non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-015 | implemented | Manual auto-release recovery must only tag a requested `v*` release when the provided SHA is reachable from `origin/main`, has successful `main` CI, came from exactly one merged `release/*-prep` pull request, and derives the requested tag. | .github/workflows/auto-release-tag.yml, docs/topics/release-process/policy.toml |
| RELEASE-REQ-016 | implemented | Release preparation must audit `docs/topics/` coverage and accuracy, and releases are blocked unless both metrics are at least 90%. | docs/topics/release-process/policy.toml, docs/topics/release-process/runbook.md |
| RELEASE-REQ-017 | implemented | Structured release policy captures the `v0.6.0-alpha.1` developer-tooling scope, supported editor integration boundary, topic-shelf audit, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-018 | implemented | Release preparation must record a release thesis, previous-tag diff reconciliation, zero-open milestone evidence before tag creation, no-crates publication evidence, and a release report with plan-versus-actual, fallout, and next-thesis sections. | docs/topics/release-process/policy.toml, docs/topics/release-process/runbook.md |
| RELEASE-REQ-019 | implemented | Structured release policy captures the `v0.7.0-alpha.1` file-backed authority-facts scope, governance-design boundary, policy hardening, review fallback, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/releases/v0.1.0-alpha.1.md | Published release notes for the first front-end alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.2.0-alpha.1.md | Published release notes for the Core semantic model and schema alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.3.0-alpha.1.md | Published release notes for the compiler-spine and canonical Core alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.4.0-alpha.1.md | Published release notes for the target-profile, lowerability, and contract-bundle validation alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.5.0-alpha.1.md | Published release notes for the Gate C admission-boundary alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.6.0-alpha.1.md | Published release notes for the developer-tooling alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.7.0-alpha.1.md | Publish-ready release notes for the file-backed authority-facts alpha. | The release workflow looks up this file by full tag name after the release-prep PR merges. |
| .github/workflows/auto-release-tag.yml | Successful main-CI release-prep merges create immutable release tags and dispatch publication. | The workflow derives tags only from merged `release/*-prep` branches and refuses tag mutation. |
| CHANGELOG.md | Release history for published and publish-ready alpha releases. | Scheduled alpha release sections use the matching release target date. |
| docs/topics/release-process/policy.toml | Structured release-tag, runbook, and alpha boundary policy. | Tag mutation is forbidden, runbook phases are named, and release scope/non-goals are structured. |
| docs/topics/release-process/runbook.md | Operator steps for preparing, tagging, publishing, and recovering releases. | The structured policy names the phases and checks the runbook must cover. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| RELEASE-TP-001 | implemented | Golden path | RELEASE-REQ-001, RELEASE-REQ-002, RELEASE-REQ-003, RELEASE-REQ-004, RELEASE-REQ-005 | The workflow contains the tag trigger, main reachability guard, full-tag release-notes path, verified GitHub Release creation, prerelease flag, and no package publish command. | release_workflow_publishes_only_main_reachable_tags | docs/releases/v0.1.0-alpha.1.md | Static workflow contract regression. |
| RELEASE-TP-002 | implemented | Policy guard | RELEASE-REQ-006 | Structured policy forbids tag mutation and names existing-valid-tag publication as recovery. | release_tag_recovery_policy_is_structured | docs/topics/release-process/policy.toml | Policy evidence is structured, not prose. |
| RELEASE-TP-003 | implemented | Boundary guard | RELEASE-REQ-007 | Structured policy captures the v0.2 Core schema scope and non-goals for lowering, encoder, bytes, digests, targets, and admission. | release_policy_tracks_v0_2_boundary | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the Core milestone. |
| RELEASE-TP-004 | implemented | Consistency guard | RELEASE-REQ-008 | Scheduled alpha changelog section dates equal their target dates in structured release policy. | alpha_changelog_dates_match_release_policy | CHANGELOG.md, docs/topics/release-process/policy.toml | Prevents release chronology drift across release prep and publication. |
| RELEASE-TP-005 | implemented | Runbook guard | RELEASE-REQ-009 | Structured policy names the release-prep phases and required checks for local verification, PR checks, and release existence. | release_runbook_policy_is_structured | docs/topics/release-process/policy.toml, docs/topics/release-process/runbook.md | Keeps the human runbook tied to a stable release contract. |
| RELEASE-TP-006 | implemented | Boundary guard | RELEASE-REQ-010 | Structured policy captures the v0.3 compiler-spine, canonical encoder, reviewed golden fixture, exact digest, target-lowering, and admission boundaries. | release_policy_tracks_v0_3_boundary | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the compiler-spine milestone. |
| RELEASE-TP-007 | implemented | Boundary guard | RELEASE-REQ-011 | Structured policy captures the v0.4 target-profile, lowerability, contract-bundle validation, target-lowering, admission, and publication boundaries. | release_policy_tracks_v0_4_boundary | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the target-profile and lowerability milestone. |
| RELEASE-TP-008 | implemented | Automation guard | RELEASE-REQ-012, RELEASE-REQ-013 | The auto-release workflow watches successful `main` CI, derives tags from merged release-prep PRs, refuses tag moves, dispatches release publication, and the Release workflow closes zero-open milestones after publication. | release_automation_policy_is_structured, auto_release_tag_workflow_is_guarded, release_workflow_supports_dispatch_and_milestone_closure | .github/workflows/auto-release-tag.yml, .github/workflows/release.yml, docs/topics/release-process/policy.toml | Keeps release automation deterministic and non-mutating. |
| RELEASE-TP-009 | implemented | Boundary guard | RELEASE-REQ-014 | Structured policy captures the v0.5 admission-boundary scope and explicit non-goals for Continuum-owned policy, identity, delegation, revocation, ledger persistence, signature verification, target lowering, and crates.io publication. | release_policy_tracks_v0_5_boundary | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the Gate C admission milestone. |
| RELEASE-TP-010 | implemented | Recovery guard | RELEASE-REQ-015 | The manual auto-release recovery path requires a successful main-CI release-prep merge, derives the tag from the merged release-prep PR, and rejects mismatched operator tag input before writing release outputs. | auto_release_tag_manual_dispatch_checks_verified_main_sha, auto_release_tag_workflow_is_guarded | .github/workflows/auto-release-tag.yml, docs/topics/release-process/policy.toml | Keeps manual recovery idempotent without allowing arbitrary tag/SHA pairing. |
| RELEASE-TP-011 | implemented | Audit guard | RELEASE-REQ-016 | Structured policy defines the `docs/topics/` coverage and accuracy formulas, requires issue-or-PR evidence before merge, records release-blocking evidence fields, requires stale current-truth claims to be corrected or removed before counting as accurate, and sets both floors to at least 90%. | release_topic_audit_policy_sets_minimums | docs/topics/release-process/policy.toml | Keeps release preparation from shipping stale or under-reviewed topic shelves. |
| RELEASE-TP-012 | implemented | Boundary guard | RELEASE-REQ-017 | Structured policy captures the v0.6 developer-tooling scope, supported VS Code/Cursor integration, topic-shelf audit, and explicit non-goals for compiler CLI, language-server diagnostics, marketplace publication, target lowering, and admission tooling. | release_policy_tracks_v0_6_boundary | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the developer-tooling milestone. |
| RELEASE-TP-013 | implemented | Runbook guard | RELEASE-REQ-018 | Structured policy requires release thesis, previous-tag diff reconciliation, milestone-zero evidence at tag time, no-crates verification, release-report sections, and next-release thesis evidence. | release_runbook_policy_is_structured | docs/topics/release-process/policy.toml | Makes release claim integrity durable before and after publication. |
| RELEASE-TP-014 | implemented | Boundary guard | RELEASE-REQ-019 | Structured policy captures the v0.7 file-backed authority-facts scope, first compiler fact classes, governance design note, policy hardening, review fallback, and explicit non-goals for trusted authorship, full manifests, broader fact corpora, target IR, admission execution, and crates.io publication. | release_policy_tracks_v0_7_boundary | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the authority-facts milestone. |

## Determinism Obligations

- Release workflow contract tests inspect checked-in workflow text, not live
  GitHub state.
- Release tags must be explicit automation or operator actions; no test creates
  or pushes live tags.
- Recovery documentation must distinguish workflow fixes from tag mutation.
- Tests do not scrape human diagnostic prose from Actions logs.
- Release metadata tests assert structured policy artifacts rather than rendered
  prose or live GitHub state.
- Automation tests inspect checked-in workflows and policy, not live GitHub PR,
  tag, release, or milestone state.
- Topic audit tests assert numeric release-policy thresholds and required audit
  evidence fields; the human audit records branch-specific accuracy findings.
- Release report tests assert structured policy requirements, not chat summaries
  or live GitHub state.

## Open Gaps

- The release workflow does not yet have a local end-to-end dry run harness.
- The current checker proves workflow contract structure, not GitHub API
  availability.
- No crates.io policy exists; package publication remains intentionally absent.
