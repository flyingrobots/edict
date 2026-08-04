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
| RELEASE-REQ-007 | policy | Structured release policy captures the `v0.2.0-alpha.1` Core schema scope and explicit non-goals. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-008 | implemented | Release dates recorded in the structured release policy, `CHANGELOG.md`, and release notes are reconciled against the annotated git tags that published them, rather than against each other. Missing tags, lightweight tags, and absent date-bearing surfaces fail; only allowlisted pre-policy omissions and the pre-publication `prep` window are advisory. The reconciliation runs in required CI. | CHANGELOG.md, docs/topics/release-process/policy.toml, xtask/src/release_dates.rs, .github/workflows/ci.yml |
| RELEASE-REQ-009 | implemented | Release preparation follows a documented runbook with branch prep, local verification, PR merge gate, tag publication, workflow watch, evidence capture, and non-mutating recovery phases. | docs/topics/release-process/runbook.md, docs/topics/release-process/policy.toml |
| RELEASE-REQ-010 | policy | Structured release policy captures the `v0.3.0-alpha.1` compiler-spine, canonical encoder, reviewed golden fixture, exact digest, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-011 | policy | Structured release policy captures the `v0.4.0-alpha.1` target-profile, lowerability, contract-bundle validation, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-012 | implemented | Successful `main` CI on a merged `release/vX.Y.Z-alpha.N-prep` pull request creates an immutable `vX.Y.Z-alpha.N` tag and dispatches release publication. | .github/workflows/auto-release-tag.yml, docs/topics/release-process/policy.toml |
| RELEASE-REQ-013 | implemented | Release publication closes the matching GitHub milestone only after the release exists and the milestone has zero open issues. | .github/workflows/release.yml, docs/topics/release-process/policy.toml |
| RELEASE-REQ-014 | policy | Structured release policy captures the `v0.5.0-alpha.1` Gate C admission-boundary scope, release automation, and explicit Continuum-owned non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-015 | implemented | Manual auto-release recovery must only tag a requested `v*` release when the provided SHA is reachable from `origin/main`, has successful `main` CI, came from exactly one merged `release/*-prep` pull request, and derives the requested tag. | .github/workflows/auto-release-tag.yml, docs/topics/release-process/policy.toml |
| RELEASE-REQ-016 | implemented | Release preparation must audit `docs/topics/` coverage and accuracy, and releases are blocked unless both metrics are at least 90%. | docs/topics/release-process/policy.toml, docs/topics/release-process/runbook.md |
| RELEASE-REQ-017 | policy | Structured release policy captures the `v0.6.0-alpha.1` developer-tooling scope, supported editor integration boundary, topic-shelf audit, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-018 | implemented | Release preparation must record a release thesis, previous-tag diff reconciliation, zero-open milestone evidence before tag creation, no-crates publication evidence, and a release report with plan-versus-actual, fallout, and next-thesis sections. | docs/topics/release-process/policy.toml, docs/topics/release-process/runbook.md |
| RELEASE-REQ-019 | policy | Structured release policy captures the `v0.7.0-alpha.1` file-backed authority-facts scope, governance-design boundary, policy hardening, review fallback, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-020 | policy | Structured release policy captures the `v0.8.0-alpha.1` minimal effectful compiler-spine scope, Core effect-node boundary, unsupported-form rejection boundary, pure-golden stability boundary, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-021 | policy | Structured release policy captures the `v0.9.0-alpha.1` first Target IR scope, Echo and git-warp target artifact boundaries, lowerability bridge, stable failure boundary, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-022 | policy | Structured release policy captures the `v0.10.0-alpha.1` first public CLI scope, JSONL check workflow, stream record schemas, stable diagnostic kind codes, golden fixture corpus, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-023 | policy | Structured release policy captures the `v0.11.0-alpha.1` contract-bundle assembly and canonical Target IR artifact freeze scope, checked digest/byte goldens, computed bundle integration, and explicit non-goal boundaries. | docs/topics/release-process/policy.toml |
| RELEASE-REQ-024 | implemented | `cargo xtask release-prep <version>` scaffolds the mechanical release-prep files that have drifted before: workspace package versions, lockfile package versions, a changelog section dated `--date` or today's UTC date, the release policy boundary block, release notes stub, and paired release-process test-plan rows. It does not scaffold Rust test stubs and does not extrapolate the date from release history. | xtask/src/release_prep.rs |
| RELEASE-REQ-025 | implemented | Every `[release_notes.*]` policy block is structurally complete and uniquely keyed, with per-block field lookup rather than whole-file substring matching. | docs/topics/release-process/policy.toml, xtask/src/release_dates.rs |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| docs/releases/v0.1.0-alpha.1.md | Published release notes for the first front-end alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.2.0-alpha.1.md | Published release notes for the Core semantic model and schema alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.3.0-alpha.1.md | Published release notes for the compiler-spine and canonical Core alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.4.0-alpha.1.md | Published release notes for the target-profile, lowerability, and contract-bundle validation alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.5.0-alpha.1.md | Published release notes for the Gate C admission-boundary alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.6.0-alpha.1.md | Published release notes for the developer-tooling alpha. | The release workflow looks up this file by full tag name. |
| docs/releases/v0.7.0-alpha.1.md | Published release notes for the file-backed authority-facts alpha. | The release workflow looked up this file by full tag name after the release-prep PR merged. |
| docs/releases/v0.8.0-alpha.1.md | Published release notes for the minimal effectful compiler-spine alpha. | The release workflow looked up this file by full tag name after the release-prep PR merged. |
| docs/releases/v0.10.0-alpha.1.md | Published release notes for the first public CLI alpha. | The release workflow looked up this file by full tag name after the release-prep PR merged. |
| docs/releases/v0.9.0-alpha.1.md | Published release notes for the first Target IR alpha. | The release workflow looked up this file by full tag name after the release-prep PR merged. |
| docs/releases/v0.11.0-alpha.1.md | Published notes for the contract-bundle assembly and canonical Target IR artifact freeze alpha. | The release workflow looked up this file by full tag name after the release-prep PR merged. |
| .github/workflows/auto-release-tag.yml | Successful main-CI release-prep merges create immutable release tags and dispatch publication. | The workflow derives tags only from merged `release/*-prep` branches and refuses tag mutation. |
| CHANGELOG.md | Release history for published and release-prep alpha releases. | Published alpha release sections use the creation date of the annotated git tag that published them, checked by `cargo xtask release-dates`. |
| docs/topics/release-process/policy.toml | Structured release-tag, runbook, and alpha boundary policy. | Tag mutation is forbidden, runbook phases are named, and release scope/non-goals are structured. |
| docs/topics/release-process/runbook.md | Operator steps for preparing, tagging, publishing, and recovering releases. | The structured policy names the phases and checks the runbook must cover. |
| `cargo xtask release-prep <version>` | Mechanical scaffold for the next release-prep branch. | The xtask regression exercises the writer against a temp repo skeleton and checks every generated file surface. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| RELEASE-TP-001 | implemented | Golden path | RELEASE-REQ-001, RELEASE-REQ-002, RELEASE-REQ-003, RELEASE-REQ-004, RELEASE-REQ-005 | The workflow contains the tag trigger, main reachability guard, full-tag release-notes path, verified GitHub Release creation, prerelease flag, and no package publish command. | release_workflow_publishes_only_main_reachable_tags | docs/releases/v0.1.0-alpha.1.md | Static workflow contract regression. |
| RELEASE-TP-002 | implemented | Policy guard | RELEASE-REQ-006 | Structured policy forbids tag mutation and names existing-valid-tag publication as recovery. | release_tag_recovery_policy_is_structured | docs/topics/release-process/policy.toml | Policy evidence is structured, not prose. |
| RELEASE-TP-003 | policy | Boundary guard | RELEASE-REQ-007 | Review confirms structured policy captures the v0.2 Core schema scope and non-goals for lowering, encoder, bytes, digests, targets, and admission. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the Core milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-004 | implemented | Consistency guard | RELEASE-REQ-008 | Reconciliation reports drift when a release's policy, changelog, and release-notes dates agree with each other but contradict the git tag, and reports uncovered surfaces separately from drift. | release_date_reconciliation_reports_internally_consistent_wrong_dates, release_date_reconciliation_accepts_dates_matching_their_tags | CHANGELOG.md, docs/topics/release-process/policy.toml | The previous guard compared changelog against policy, which release-prep writes from one field, so both drifted together undetected. |
| RELEASE-TP-005 | implemented | Runbook guard | RELEASE-REQ-009 | Structured policy names the release-prep phases and required checks for local verification, PR checks, and release existence. | release_runbook_policy_is_structured | docs/topics/release-process/policy.toml, docs/topics/release-process/runbook.md | Keeps the human runbook tied to a stable release contract. |
| RELEASE-TP-006 | policy | Boundary guard | RELEASE-REQ-010 | Review confirms structured policy captures the v0.3 compiler-spine, canonical encoder, reviewed golden fixture, exact digest, target-lowering, and admission boundaries. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the compiler-spine milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-007 | policy | Boundary guard | RELEASE-REQ-011 | Review confirms structured policy captures the v0.4 target-profile, lowerability, contract-bundle validation, target-lowering, admission, and publication boundaries. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the target-profile and lowerability milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-008 | implemented | Automation guard | RELEASE-REQ-012, RELEASE-REQ-013 | The auto-release workflow watches successful `main` CI, derives tags from merged release-prep PRs, refuses tag moves, dispatches release publication, and the Release workflow closes zero-open milestones after publication. | release_automation_policy_is_structured, auto_release_tag_workflow_is_guarded, release_workflow_supports_dispatch_and_milestone_closure | .github/workflows/auto-release-tag.yml, .github/workflows/release.yml, docs/topics/release-process/policy.toml | Keeps release automation deterministic and non-mutating. |
| RELEASE-TP-009 | policy | Boundary guard | RELEASE-REQ-014 | Review confirms structured policy captures the v0.5 admission-boundary scope and explicit non-goals for Continuum-owned policy, identity, delegation, revocation, ledger persistence, signature verification, target lowering, and crates.io publication. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the Gate C admission milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-010 | implemented | Recovery guard | RELEASE-REQ-015 | The manual auto-release recovery path requires a successful main-CI release-prep merge, derives the tag from the merged release-prep PR, and rejects mismatched operator tag input before writing release outputs. | auto_release_tag_manual_dispatch_checks_verified_main_sha, auto_release_tag_workflow_is_guarded | .github/workflows/auto-release-tag.yml, docs/topics/release-process/policy.toml | Keeps manual recovery idempotent without allowing arbitrary tag/SHA pairing. |
| RELEASE-TP-011 | implemented | Audit guard | RELEASE-REQ-016 | Structured policy defines the `docs/topics/` coverage and accuracy formulas, requires issue-or-PR evidence before merge, records release-blocking evidence fields, requires stale current-truth claims to be corrected or removed before counting as accurate, and sets both floors to at least 90%. | release_topic_audit_policy_sets_minimums | docs/topics/release-process/policy.toml | Keeps release preparation from shipping stale or under-reviewed topic shelves. |
| RELEASE-TP-012 | policy | Boundary guard | RELEASE-REQ-017 | Review confirms structured policy captures the v0.6 developer-tooling scope, supported VS Code/Cursor integration, topic-shelf audit, and explicit non-goals for compiler CLI, language-server diagnostics, marketplace publication, target lowering, and admission tooling. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the developer-tooling milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-013 | implemented | Runbook guard | RELEASE-REQ-018 | Structured policy requires release thesis, previous-tag diff reconciliation, milestone-zero evidence at tag time, no-crates verification, release-report sections, and next-release thesis evidence. | release_runbook_policy_is_structured | docs/topics/release-process/policy.toml | Makes release claim integrity durable before and after publication. |
| RELEASE-TP-014 | policy | Boundary guard | RELEASE-REQ-019 | Review confirms structured policy captures the v0.7 file-backed authority-facts scope, first compiler fact classes, governance design note, policy hardening, review fallback, and explicit non-goals for trusted authorship, full manifests, broader fact corpora, target IR, admission execution, and crates.io publication. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the authority-facts milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-015 | policy | Boundary guard | RELEASE-REQ-020 | Review confirms structured policy captures the v0.8 minimal effectful compiler-spine scope, Core effect-node model, file-backed fact dependency, unsupported-form rejection boundary, pure Core golden stability, and explicit non-goals for target IR, runtime execution, CLI, admission, governance, and crates.io publication. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the effectful compiler-spine milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-016 | policy | Boundary guard | RELEASE-REQ-021 | Review confirms structured policy captures the v0.9 first Target IR scope, Echo and git-warp review-artifact boundary, lowerability bridge, stable target-lowering failure boundary, and explicit non-goals for runtime execution, canonical target bytes, bundles, admission, v2 adapters, CLI, and crates.io publication. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the first Target IR milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-017 | policy | Boundary guard | RELEASE-REQ-022 | Review confirms structured policy captures the v0.10 first public CLI scope, JSONL check workflow, deterministic input expansion, stream record schemas, stable diagnostic kind codes, golden fixture corpus, and explicit non-goals for compile/lower/explain/bundle/admission commands, human-pretty output, embedded schema validation, language server, marketplace packaging, participant policy, and crates.io publication. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the first public CLI milestone. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-018 | policy | Boundary guard | RELEASE-REQ-023 | Review confirms structured policy captures the v0.11 contract-bundle assembly and canonical Target IR artifact freeze scope, including semantic/release bundle digest goldens, Target IR byte/digest goldens, computed bundle integration, and explicit non-goals for runtime execution, admission execution, participant policy logic, Echo verifier completeness, git-warp commit creation, git-warp CRDT reducer verification, general target plugin dispatch, additional target profiles, extra source-to-target fixtures, canonical bundle-manifest bytes, and crates.io publication. | - | docs/topics/release-process/policy.toml | Prevents the release metadata from overclaiming the v0.11 cryptographic freeze. Frozen historical record; reviewed, not string-tested. Block structure is covered by `release_policy_blocks_are_structurally_complete`. |
| RELEASE-TP-019 | implemented | Scaffolding guard | RELEASE-REQ-024 | Given a temp repo skeleton with the current release-process surfaces, `cargo xtask release-prep <version>` writes the version bump, lockfile package versions, dated changelog section, release policy boundary block, release notes stub, and paired planned release-process rows deterministically, and writes no Rust test stub. | release_prep_scaffolds_version_policy_changelog_and_notes | xtask/src/release_prep.rs, xtask/src/tests.rs | Keeps release-prep setup mechanical so review focuses on release thesis, scope, non-goals, and evidence rather than missed boilerplate. |
| RELEASE-TP-020 | implemented | Consistency guard | RELEASE-REQ-025 | Every `[release_notes.*]` block parses with a unique section and tag, an ISO `target_date`, a known status, and `scope`/`non_goals` lists, and published blocks retain no scaffold placeholders. | release_policy_blocks_are_structurally_complete | docs/topics/release-process/policy.toml | Replaces eleven near-duplicate per-release guards with one data-driven check. |
| RELEASE-TP-022 | implemented | Boundary guard | RELEASE-REQ-008 | A lightweight release tag fails reconciliation, because it has no tagger date and would otherwise report the tagged commit's committer date. | release_date_reconciliation_rejects_lightweight_release_tags | docs/topics/release-process/policy.toml | Tag placement on an older commit must not silently redate a release. |
| RELEASE-TP-023 | implemented | Boundary guard | RELEASE-REQ-008 | A tag whose policy block still reads `prep` reports an advisory gap rather than failing, covering the window before the post-publication change flips the status. | release_date_reconciliation_tolerates_prep_status_for_a_fresh_tag | docs/topics/release-process/policy.toml | Keeps `verify` green on `main` for unrelated branches between tagging and the evidence change. |
| RELEASE-TP-024 | implemented | Boundary guard | RELEASE-REQ-008 | Deleting a covered release's changelog heading, policy block, or release-notes file fails the gate, while the pre-policy `v0.1.0-alpha.1` omission stays advisory. | release_date_reconciliation_fails_when_a_covered_surface_disappears, release_date_reconciliation_allowlists_the_prepolicy_release | docs/topics/release-process/policy.toml | Removing the evidence must not be a way to make the check pass. |
| RELEASE-TP-021 | implemented | Boundary guard | RELEASE-REQ-025 | Block-scoped parsing reports a release's own `target_date` even when a different release in the same file carries the expected date string. | release_policy_block_parsing_scopes_fields_to_their_own_release | docs/topics/release-process/policy.toml | Regression guard: whole-file substring matching passed a wrong per-release date whenever any other release shared the expected string, which is normal for releases tagged the same day. |

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
- The scaffolded release date is `--date` or today's UTC date. It is still a
  prediction: a release-prep branch that sits unmerged past its scaffold date
  records a date earlier than its eventual tag, and `release-dates` only detects
  that once the tag exists. Flipping the block to `published` is what binds the
  recorded date to the tag.
- Per-release scope and non-goal content is reviewed rather than string-tested.
  `release_policy_blocks_are_structurally_complete` proves each block is present
  and complete, but nothing mechanically checks that a block's declared scope
  matches what the release actually shipped.
