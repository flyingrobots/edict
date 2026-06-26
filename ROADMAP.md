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
- No canonical digest is frozen from a paper encoding plan. Meaning freezes
  before bytes; bytes freeze before hashes; hashes freeze before admission.

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

## v0.2.0-alpha.1 - Core Semantic Model And Normative Schema

Target date: 2026-07-01

Milestone: `v0.2.0-alpha.1`

Primary issues: #3, #19

Release labels: `release:core-ir`

Scope:

- Core semantic algebra and invariants: expressions, predicates, types,
  blocks/nodes, input constraints, and local-reference normalization.
- Normative `edict.core/v1` CDDL/schema validation for that algebra.
- Topic shelf and test-plan evidence for what is normative versus scaffolding.

Exit gates:

- #3 and #19 land or have reviewed release-scope splits.
- Core meaning and schema boundaries are documented.
- Release notes explicitly state that no canonical bytes or digests are frozen.

Non-goals:

- No source-to-Core compiler spine.
- No canonical encoder.
- No golden bytes.
- No exact Core digests.

## v0.3.0-alpha.1 - Compiler Spine Alpha

Target date: 2026-07-15

Milestone: `v0.3.0-alpha.1`

Primary issues: #10, #20, #21, #22

Release prep issue: #35

Release labels: `release:semantic-validation`, `release:compiler-spine`,
`release:core-ir`

Scope:

- Keep source/surface validation, resolution, typing, Core lowering, and
  canonicalization as explicit compiler stages:

  ```text
  parse
    -> validate_surface
    -> resolve
    -> type_check
    -> lower_core
    -> canonicalize
  ```

- First executable resolver and typed representation boundary.
- Source-to-Core lowering for the initial executable subset.
- Reference canonical encoder.
- First reviewed Core golden bytes and exact digest fixtures.
- Release notes, changelog, and reusable release runbook for the compiler-spine
  alpha.

Exit gates:

- #10, #20, #21, #22, and #35 land or have reviewed release-scope splits.
- Tests prove the stage boundaries do not collapse into one semantic pass.
- Golden Core bytes are produced by an executable encoder, not a prose plan.
- Golden tests cover alpha-renaming invariance where applicable, map-order
  independence, encode/decode stability, mutation sensitivity, and platform
  independence.

Non-goals:

- No target lowering.
- No bundle/admission claim.

## v0.4.0-alpha.1 - Target Profile And Lowerability Alpha

Target date: 2026-07-29

Milestone: `v0.4.0-alpha.1`

Primary issues: #1, #5

Release prep issue: #39

Release labels: `release:admission`, `release:lowerability`

Scope:

- Runtime-neutral target-profile and assurance flow.
- Direct-only v1 target adaptation facts.
- Typed `LoweringRequirements` contract and fixture-driven explanation model.
- Typed participant-neutral contract bundle and assurance evidence manifest
  validation.
- `edict explain lowerability` and profile check-requirements surface, if the
  CLI boundary is ready.

Exit gates:

- #1, #5, and #39 land or have reviewed release-scope splits.
- The release reconciles lowerability with the v1 direct-adapter rule.
- Any bundled assurance evidence is hash-bound to the selected bundle subject,
  target profile, and target IR.
- Any `composite` or chained-adapter behavior is explicitly deferred to
  `v2-design`.
- Lowerability output is documented as a contract or explicitly marked
  experimental.

## v0.5.0-alpha.1 - Bundle And Admission Alpha

Target date: 2026-08-12

Milestone: `v0.5.0-alpha.1`

Primary issues: #6, #11

Release issue: #42

Release labels: `release:admission`

Scope:

- Edict-owned Continuum participation boundary.
- Gate C artifact admission contract and evidence expectations.
- Typed Gate C admission-boundary checks for bundle subject, operation
  requirements, receipts, and invocation capability evidence.
- Bundle/admission fixtures that consume compiler-spine artifacts rather than
  paper-only sketches.

Exit gates:

- #6, #11, and #42 land or have reviewed release-scope splits.
- Admission claims are backed by fixtures, schema, or topic-shelf evidence.
- Release notes distinguish Edict-owned artifacts from Continuum-owned
  participant policy.

## v0.6.0-alpha.1 - Developer Tooling Alpha

Target date: 2026-08-26

Milestone: `v0.6.0-alpha.1`

Primary issue: #7

Release issue: #50

Release labels: `release:developer-tools`

Scope:

- Editor-facing lexical highlighting roles that editor adapters can consume
  before full grammar packages land.
- Tree-sitter grammar source, generated parser source, highlight query, and
  current-subset corpus for the accepted fixture families.
- TextMate grammar artifact for `.edict` lexical scopes in TextMate-compatible
  editors.
- Thin VS Code/Cursor extension package wrapping the TextMate grammar.
- Differential grammar fixture path after the compiler/Core spine stabilizes
  beyond the current Tree-sitter corpus.
- Tooling documentation for VS Code, Vim, Zed, jedit, or the first supported
  subset.

Exit gates:

- #7 and #50 land or have reviewed release-scope splits.
- Highlighting roles are backed by deterministic fixtures before editor
  adapters consume them.
- Tree-sitter grammar fixtures are deterministic, versioned with the source
  grammar, and aligned with the reference parser.
- TextMate grammar scopes are backed by executable checks against public
  highlighter roles.
- The VS Code/Cursor package registers `.edict` files and uses the canonical
  TextMate grammar artifact.
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
- `v0.2.0-alpha.1`: #3, #19, #28
- `v0.3.0-alpha.1`: #10, #20, #21, #22
- `v0.4.0-alpha.1`: #1, #5, #39
- `v0.5.0-alpha.1`: #6, #11, #42
- `v0.6.0-alpha.1`: #7, #50
- `v2-design`: #4

Alpha-train release labels:

- `release:front-end`
- `release:core-ir`
- `release:semantic-validation`
- `release:compiler-spine`
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
