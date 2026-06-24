# Release Runbook

Status: current operator runbook for alpha releases.

Use this runbook for every Edict alpha release until the process is replaced by
an executable release-preflight command. The structured policy fields in
[`policy.toml`](./policy.toml) are the machine-checkable guardrail; this page is
the human execution path.

## 1. Prepare The Branch

Start from a clean, up-to-date `main`:

```bash
export GH_PAGER=cat
git status --porcelain
git switch main
git fetch origin
git merge --ff-only origin/main
git status --porcelain
```

Create a release-prep branch named for the release:

```bash
git switch -c release/vX.Y.Z-alpha.N-prep
```

Open or identify the release-prep issue in the matching GitHub milestone. The
issue must name the release scope, documentation updates, local gate, CI gate,
and milestone closure requirement.

## 2. Refresh Release Artifacts

Update these artifacts together:

- crate/package version metadata;
- `CHANGELOG.md`;
- `docs/releases/vX.Y.Z-alpha.N.md`;
- `docs/README.md`;
- `README.md`, when its current-status signposts change;
- `ROADMAP.md`, when issue or milestone status changed;
- `docs/topics/release-process/README.md`;
- `docs/topics/release-process/test-plan.md`;
- topic shelves that own newly landed behavior.

Each release notes file must state the release type, version policy, included
scope, explicit non-goals, required verification, and tagging plan. Do not claim
target lowering, admission, bundle integrity, or publication behavior before the
owning topic shelf and tests exist.

## 3. Verify Locally

Run the focused release checks first when the release process changed:

```bash
cargo test -p xtask release_
```

Then run the full local gate:

```bash
cargo xtask verify
```

Fix failures on the branch. Do not tag from a branch that has not passed the
local gate.

## 4. Open And Merge The Release-Prep PR

Push the branch and open a normal pull request against `main`. The PR body must
include GitHub auto-close text for the release-prep issue, for example:

```text
Closes #35
```

Before merge, verify:

```bash
gh pr checks --watch
```

Merge only when CI is green, required reviews are satisfied, and there are no
unresolved blocking review threads.

## 5. Tag From Main

After the PR merges, update local `main` and re-run the local gate on the exact
commit that will be tagged:

```bash
git switch main
git fetch origin
git merge --ff-only origin/main
cargo xtask verify
```

Create and push the annotated release tag:

```bash
git tag -a vX.Y.Z-alpha.N -m "vX.Y.Z-alpha.N"
git push origin vX.Y.Z-alpha.N
```

The release workflow rejects tags whose target commit is not reachable from
`origin/main`.

## 6. Watch Publication

Find and watch the release workflow run:

```bash
gh run list --workflow Release --limit 5
gh run watch <run-id>
```

Confirm the GitHub Release exists:

```bash
gh release view vX.Y.Z-alpha.N
```

Record the release URL, tag object or target commit, workflow run, and release
issue in the release-process topic or final release report.

## 7. Recover Without Tag Mutation

Release tags are durable once pushed. If a workflow fails after a valid tag is
pushed, do not move, delete, recreate, or force-push the tag.

Allowed recovery paths:

- fix the workflow on `main` and publish against the existing valid tag;
- manually publish the GitHub Release against the existing valid tag when the
  workflow failure is historical and the checked-in release notes are correct;
- document the failed run and successful recovery evidence.

The recovery policy is structured in [`policy.toml`](./policy.toml).
