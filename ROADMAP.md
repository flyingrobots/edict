# Edict Roadmap

This roadmap is the current release schedule for Edict. GitHub milestones and
issues are the operational trackers; this file is the human-readable release
plan that keeps those artifacts visible from the repository.

Dates are target dates, not promises. A release can move, but the GitHub
milestone and this roadmap must move together.

## Release Gates

Every release cut must satisfy these gates on the release commit:

- `cargo xtask verify` passes locally.
- The available GitHub Actions checks for `main` are green.
- Publication is triggered by pushing a `v*` tag whose target commit is reachable
  from `origin/main`; release tags must not target unmerged branch commits.
- `CHANGELOG.md` has a dated section for the release.
- The GitHub milestone has no open issue that blocks the release scope.
- Release notes state exactly what the release claims and what it does not
  claim.
- No target, Core, admission, or bundle-integrity claim is made before the
  release that owns the corresponding contract.

The crate remains `publish = false` until package policy and API stability are
deliberately changed.

## v0.1.0-alpha.1 - Front-End Alpha

Target date: 2026-06-24

Milestone: `v0.1.0-alpha.1`

Primary issue: #16

Release labels: `release:front-end`

Scope:

- Phase 1 `edict-syntax` lexer/parser for the landed source-AST subset.
- Phase 2 source-AST semantic validation through `SEMVAL-REQ-007`.
- Syntax and semantic-validation topic shelves.
- Local contract-graph verification through `cargo xtask verify`.

Exit gates:

- #16 release checklist is complete.
- Version policy for `v0.1.0-alpha.1` is explicit.
- `CHANGELOG.md` has a dated `v0.1.0-alpha.1` section.
- The release notes say this is a front-end milestone only.

Non-goals:

- No crates.io publish.
- No Core IR lowering.
- No canonical Core hash goldens.
- No target lowerers or admission tooling.

## v0.2.0-alpha.1 - Core IR Contract Alpha

Target date: 2026-07-01

Milestone: `v0.2.0-alpha.1`

Primary issue: #3

Release labels: `release:core-ir`

Scope:

- `edict.core/v1` CDDL for Core expressions, predicates, types, and blocks.
- Canonical encoding plan for the Core contract.
- Golden fixture scaffolding for future byte-stable artifacts.

Exit gates:

- #3 lands or has a reviewed release-scope split.
- Core schema and fixture locations are documented.
- The release notes state which artifacts are golden and which are scaffolding.

## v0.3.0-alpha.1 - Semantic Closure And Lowerability Alpha

Target date: 2026-07-15

Milestone: `v0.3.0-alpha.1`

Primary issues: #10, #5

Release labels: `release:semantic-validation`, `release:lowerability`

Scope:

- Remaining source-AST semantic validation planned in the semantic-validation
  topic shelf.
- Relapse-zoo fixture path for unlawful or unsupported constructs.
- `edict explain lowerability` and profile check-requirements surface.

Exit gates:

- #10 and #5 land or have reviewed release-scope splits.
- Negative semantic coverage asserts stable error kinds or contractual
  artifacts, not diagnostic prose.
- Lowerability output is documented as a contract or explicitly marked
  experimental.

## v0.4.0-alpha.1 - Artifact Admission Alpha

Target date: 2026-07-29

Milestone: `v0.4.0-alpha.1`

Primary issues: #1, #6, #11

Release labels: `release:admission`

Scope:

- Runtime-neutral target-profile and assurance flow.
- Edict-owned Continuum participation boundary.
- Gate C artifact admission contract and evidence expectations.

Exit gates:

- #1, #6, and #11 land or have reviewed release-scope splits.
- Admission claims are backed by fixtures, schema, or topic-shelf evidence.
- Release notes distinguish Edict-owned artifacts from Continuum-owned
  participant policy.

## v0.5.0-alpha.1 - Developer Tooling Alpha

Target date: 2026-08-12

Milestone: `v0.5.0-alpha.1`

Primary issue: #7

Release labels: `release:developer-tools`

Scope:

- Editor syntax-highlighting plan and first supported grammar artifacts.
- Differential grammar fixture path after the Core grammar stabilizes.
- Tooling documentation for VS Code, Vim, Zed, jedit, or the first supported
  subset.

Exit gates:

- #7 lands or has a reviewed release-scope split.
- Grammar fixtures are deterministic and versioned with the source grammar.
- The release notes identify exactly which editor integrations are supported.

## v2-design - Future Design Track

Milestone: `v2-design`

Primary issue: #4

Release labels: `release:v2-design`

Scope:

- Adapter composition and obligation-closure resolution design.
- Future work that must not block the v0.x alpha release train.

This track has no release date until the v1 foundation is stable enough for the
design to stop moving underneath it.

## GitHub Artifact Map

Milestones:

- `v0.1.0-alpha.1`: #16
- `v0.2.0-alpha.1`: #3
- `v0.3.0-alpha.1`: #10, #5
- `v0.4.0-alpha.1`: #1, #6, #11
- `v0.5.0-alpha.1`: #7
- `v2-design`: #4

Alpha-train release labels:

- `release:front-end`
- `release:core-ir`
- `release:semantic-validation`
- `release:lowerability`
- `release:admission`
- `release:developer-tools`
- `release:v2-design`

Related cross-repo label retained on #11:

- `release:continuum-stack`

## Scheduling Rules

- Topic shelves remain current-truth documents for landed subsystems.
- Release issues carry operational checklists for cutting tags and notes.
- Historical RFCs, decisions, and retrospectives do not become current release
  truth unless their still-relevant content is folded into a topic shelf or this
  roadmap.
- If a target date changes, update the GitHub milestone and this file in the
  same change.
- If a release-scope issue splits, move the new issues into the same milestone
  or explicitly document the deferral here.
