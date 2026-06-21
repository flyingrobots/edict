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

## Current Release

`v0.1.0-alpha.1` is published as a GitHub prerelease:

- GitHub release: <https://github.com/flyingrobots/edict/releases/tag/v0.1.0-alpha.1>
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
