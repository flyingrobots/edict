# Release Process Topic

Status: current HEAD contract.

This chapter describes the release process Edict has today. It is not a future
publishing proposal. Release automation is part of the project contract because
it decides which commits become published artifacts.

## Public Surface

Edict publishes GitHub releases through the tag-triggered workflow in
`.github/workflows/release.yml`. A release is initiated by pushing a version tag
that matches `v*` to GitHub. [RELEASE-REQ-001]

The workflow validates that the tag target is reachable from `origin/main`
before it publishes anything. Tags that point outside `main` are rejected.
[RELEASE-REQ-002]

Release notes are loaded by full tag name from `docs/releases/${TAG}.md`. For
example, `v0.1.0-alpha.1` loads
`docs/releases/v0.1.0-alpha.1.md`. [RELEASE-REQ-003]

The workflow creates a GitHub Release with `gh release create --verify-tag`.
Tags whose version contains a prerelease suffix publish as GitHub prereleases.
[RELEASE-REQ-004]

The current release process does not publish to crates.io. Workspace packages
remain `publish = false`, and the release workflow only creates GitHub releases.
[RELEASE-REQ-005]

Release tags are durable once pushed. If a release workflow fails after a signed
tag is created, the recovery path is to fix the workflow or publish the GitHub
release against the existing valid tag. Do not move, delete, or recreate release
tags to paper over workflow mistakes. The stable recovery policy is captured in
[`policy.toml`](./policy.toml). [RELEASE-REQ-006]

Release preparation follows the operator runbook in [`runbook.md`](./runbook.md):
prepare a release branch, refresh release artifacts, verify locally, merge a
normal pull request to `main`, tag from verified `main`, watch publication, and
capture evidence. The structured runbook contract is captured in
[`policy.toml`](./policy.toml). [RELEASE-REQ-009]

## Release Notes

Release notes are checked in under `docs/releases/` and are loaded by the
release workflow by full tag name. Current release-note files:

- [`v0.3.0-alpha.1`](../../releases/v0.3.0-alpha.1.md): publish-ready
  compiler-spine, canonical Core encoder, reviewed golden bytes, and exact
  digest alpha notes.
- [`v0.2.0-alpha.1`](../../releases/v0.2.0-alpha.1.md): published Core semantic
  model and normative schema alpha notes.
- [`v0.1.0-alpha.1`](../../releases/v0.1.0-alpha.1.md): published front-end
  alpha notes.

The `v0.3.0-alpha.1` release notes are publish-ready:

- Release issue: <https://github.com/flyingrobots/edict/issues/35>
- Required tag after release-prep merge: `v0.3.0-alpha.1`
- Target date: 2026-07-15

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
