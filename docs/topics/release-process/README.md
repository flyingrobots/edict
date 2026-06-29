# Release Process Topic

Status: current HEAD contract.

This chapter describes the release process Edict has today. It is not a future
publishing proposal. Release automation is part of the project contract because
it decides which commits become published artifacts.

## Public Surface

Edict publishes GitHub releases through `.github/workflows/release.yml`. A
release can be initiated by pushing a version tag that matches `v*` to GitHub,
or by dispatching the workflow with an existing `v*` tag. [RELEASE-REQ-001]

The normal alpha path is automated after release-prep PR merge. When `CI`
passes on `main`, `.github/workflows/auto-release-tag.yml` checks whether the
successful commit came from a merged `release/vX.Y.Z-alpha.N-prep` pull request.
If so, it derives `vX.Y.Z-alpha.N`, creates an annotated tag on the verified
`main` commit, and dispatches the Release workflow with that tag.
[RELEASE-REQ-012]

The Release workflow validates that the tag target is reachable from
`origin/main` before it publishes anything. Tags that point outside `main` are
rejected. [RELEASE-REQ-002]

Release notes are loaded by full tag name from `docs/releases/${TAG}.md`. For
example, `v0.1.0-alpha.1` loads
`docs/releases/v0.1.0-alpha.1.md`. [RELEASE-REQ-003]

The Release workflow creates a GitHub Release with
`gh release create --verify-tag`. Tags whose version contains a prerelease
suffix publish as GitHub prereleases. [RELEASE-REQ-004]

After publication, the Release workflow closes the matching GitHub milestone
only when a milestone with the same title exists and has zero open issues.
[RELEASE-REQ-013]

The current release process does not publish to crates.io. Workspace packages
remain `publish = false`, and the release workflow only creates GitHub releases.
[RELEASE-REQ-005]

Release tags are durable once pushed. If a release workflow fails after a signed
tag is created, the recovery path is to fix the workflow or publish the GitHub
release against the existing valid tag. Do not move, delete, or recreate release
tags to paper over workflow mistakes. The stable recovery policy is captured in
[`policy.toml`](./policy.toml). [RELEASE-REQ-006]

Release preparation follows the operator runbook in [`runbook.md`](./runbook.md):
prepare a release branch, write the release thesis, reconcile the diff since the
previous tag, refresh release artifacts, audit topic shelves, verify locally,
merge a normal pull request to `main`, let automation tag from verified `main`,
watch publication, and capture evidence. The structured runbook contract is
captured in [`policy.toml`](./policy.toml). [RELEASE-REQ-009]

Every release-prep branch must audit `docs/topics/` coverage and accuracy before
the release-prep pull request opens. Coverage is audited topic shelves divided
by total topic shelves; accuracy is accurate audited topic shelves divided by
audited topic shelves after fixes. Both metrics must be at least 90%, and the
release-prep issue must record the denominators, percentages, and findings
before the pull request opens. The pull request body mirrors or updates the
evidence before merge when review fixes change the counts.
[RELEASE-REQ-016]

Every release-prep branch must record a release thesis, previous-tag diff
reconciliation, zero-open milestone evidence before tag creation, no-crates.io
publication evidence, and a final release report with released/not-released
scope, plan-versus-actual notes, evidence, fallout issues, and the next release
thesis. [RELEASE-REQ-018]

## Release Notes

Release notes are checked in under `docs/releases/` and are loaded by the
release workflow by full tag name. Current release-note files:

- [`v0.10.0-alpha.1`](../../releases/v0.10.0-alpha.1.md): published first
  public CLI and structured-diagnostics alpha notes.
- [`v0.9.0-alpha.1`](../../releases/v0.9.0-alpha.1.md): published first
  Target IR alpha notes.
- [`v0.8.0-alpha.1`](../../releases/v0.8.0-alpha.1.md): published minimal
  effectful compiler-spine alpha notes.
- [`v0.7.0-alpha.1`](../../releases/v0.7.0-alpha.1.md): published
  file-backed authority-facts alpha notes.
- [`v0.6.0-alpha.1`](../../releases/v0.6.0-alpha.1.md): published
  developer-tooling alpha notes.
- [`v0.5.0-alpha.1`](../../releases/v0.5.0-alpha.1.md): published Gate C
  admission-boundary alpha notes.
- [`v0.4.0-alpha.1`](../../releases/v0.4.0-alpha.1.md): published
  target-profile, lowerability, and contract-bundle validation alpha notes.
- [`v0.3.0-alpha.1`](../../releases/v0.3.0-alpha.1.md): published
  compiler-spine, canonical Core encoder, reviewed golden bytes, and exact
  digest alpha notes.
- [`v0.2.0-alpha.1`](../../releases/v0.2.0-alpha.1.md): published Core semantic
  model and normative schema alpha notes.
- [`v0.1.0-alpha.1`](../../releases/v0.1.0-alpha.1.md): published front-end
  alpha notes.

The `v0.10.0-alpha.1` release is published as a GitHub prerelease:

- Release issue: <https://github.com/flyingrobots/edict/issues/76>
- Release URL: <https://github.com/flyingrobots/edict/releases/tag/v0.10.0-alpha.1>
- Tag object: `11e516c8ea8be5fa6739efd545c5b8fb40cbc46d`; peeled commit:
  `622834138af249e70d717d6b7a940e4b01e23f4d`.
- Main CI run: `28348355987`; Auto Release Tag run: `28348383070`; Release
  workflow run: `28348397035`.
- Milestone `v0.10.0-alpha.1` (#11) closed at `2026-06-29T04:21:11Z` with zero
  open issues.
- Topic-shelf audit evidence: 21 of 21 shelves audited; 21 of 21 accurate after
  release-prep edits; coverage 100%; accuracy 100%.
- No crates.io publication occurred: `edict-syntax` and `edict-cli` remain
  `publish = false`, and the GitHub prerelease has zero package assets.

The `v0.9.0-alpha.1` release is published as a GitHub prerelease:

- Release issue: <https://github.com/flyingrobots/edict/issues/70>
- Release URL: <https://github.com/flyingrobots/edict/releases/tag/v0.9.0-alpha.1>
- Tag object: `c6a4ea6b10d438cd407cd7f273fecf1fd012b2d3`; peeled commit:
  `81bacc5a240bd3ea50af934a3611ce6b3f505043`.
- Main CI run: `28314566818`; Auto Release Tag run: `28314582826`; Release
  workflow run: `28314590143`.
- Milestone `v0.9.0-alpha.1` (#10) closed at `2026-06-28T07:04:06Z` with zero
  open issues.
- Topic-shelf audit evidence: 20 of 20 shelves audited; 20 of 20 accurate after
  release-prep edits; coverage 100%; accuracy 100%.
- No crates.io publication occurred: `edict-syntax` remains `publish = false`,
  and the GitHub prerelease has zero package assets.

The `v0.7.0-alpha.1` release is published as a GitHub prerelease:

- Release issue: <https://github.com/flyingrobots/edict/issues/59>
- Release URL: <https://github.com/flyingrobots/edict/releases/tag/v0.7.0-alpha.1>
- Tag object: `f7888160f7f9a0d7b9b82d4f78bb38b886856a1e`; peeled commit:
  `6f9c731b4f36d3283dcb448b14761832ab916b07`.
- Main CI run: `28303787401`; Auto Release Tag run: `28303801200`; Release
  workflow run: `28303809157`.
- Milestone `v0.7.0-alpha.1` (#8) closed at `2026-06-27T22:31:50Z` with zero
  open issues.
- Topic-shelf audit evidence: 19 of 19 shelves audited; 19 of 19 accurate after
  release-prep edits; coverage 100%; accuracy 100%.
- No crates.io publication occurred: `edict-syntax` remains `publish = false`,
  and the GitHub prerelease has zero package assets.

The `v0.8.0-alpha.1` release is published as a GitHub prerelease:

- Release issue: <https://github.com/flyingrobots/edict/issues/62>
- Release URL: <https://github.com/flyingrobots/edict/releases/tag/v0.8.0-alpha.1>
- Tag object: `32e843c5e5f7b9252078c2b8a99afa23daeab411`; peeled commit:
  `c6a166ccea0fcb61fff9b8d76bfb5d51d613e2eb`.
- Main CI run: `28307840316`; Auto Release Tag run: `28307856119`; Release
  workflow run: `28307864582`.
- Milestone `v0.8.0-alpha.1` (#9) closed at `2026-06-28T01:41:16Z` with zero
  open issues.
- Topic-shelf audit evidence: 19 of 19 shelves audited; 19 of 19 accurate after
  release-prep edits; coverage 100%; accuracy 100%.
- No crates.io publication occurred: `edict-syntax` remains `publish = false`,
  and the GitHub prerelease has zero package assets.

The `v0.6.0-alpha.1` release is published as a GitHub prerelease:

- GitHub release:
  <https://github.com/flyingrobots/edict/releases/tag/v0.6.0-alpha.1>
- Release issue: <https://github.com/flyingrobots/edict/issues/50>
- Tag target: `ec1ce8a5186801caa3c627c6714cedff7fdf43bc`
- Topic-shelf audit evidence: 15 of 15 shelves audited; 15 of 15 accurate after
  release-process status fixes; coverage 100%; accuracy 100%.

The `v0.5.0-alpha.1` release is published as a GitHub prerelease:

- GitHub release:
  <https://github.com/flyingrobots/edict/releases/tag/v0.5.0-alpha.1>
- Release issue: <https://github.com/flyingrobots/edict/issues/42>
- Tag target: `e680adf0a3ccc72559d1c4710e81010738918532`

The `v0.4.0-alpha.1` release is published as a GitHub prerelease:

- GitHub release:
  <https://github.com/flyingrobots/edict/releases/tag/v0.4.0-alpha.1>
- Release issue: <https://github.com/flyingrobots/edict/issues/39>
- Tag target: `65c80ce4660b384ebf9fd482c59fff402f34d47b`

The `v0.3.0-alpha.1` release is published as a GitHub prerelease:

- GitHub release:
  <https://github.com/flyingrobots/edict/releases/tag/v0.3.0-alpha.1>
- Release issue: <https://github.com/flyingrobots/edict/issues/35>
- Tag target: `4ea3d993f74490b495fe6e6a9ec2d52f889ccceb`

The `v0.2.0-alpha.1` release is published as a GitHub prerelease:

- GitHub release:
  <https://github.com/flyingrobots/edict/releases/tag/v0.2.0-alpha.1>
- Release issue: <https://github.com/flyingrobots/edict/issues/28>
- Tag target: `029f43435fae9639a18c0288793dd47dda6f8946`

The `v0.1.0-alpha.1` release is published as a GitHub prerelease:

- GitHub release:
  <https://github.com/flyingrobots/edict/releases/tag/v0.1.0-alpha.1>
- Release issue: <https://github.com/flyingrobots/edict/issues/16>
- Tag target: `e9226344bf12699d744f5d066949a8d0da327fe8`

The first tag-triggered run failed because the workflow looked for
`docs/releases/0.1.0-alpha.1.md` instead of
`docs/releases/v0.1.0-alpha.1.md`. The signed tag was not moved. The workflow
was fixed on `main`, and the GitHub prerelease was published against the
existing signed tag. [RELEASE-REQ-003] [RELEASE-REQ-006]

## Deferred

- No crates.io publishing policy is implemented.
- No package-signing or binary-asset publication policy is implemented.
- No automated retry path exists for a tag event that ran a broken historical
  workflow file.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
