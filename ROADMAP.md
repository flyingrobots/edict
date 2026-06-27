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

## v0.7.0-alpha.1 - File-Backed Authority Facts Alpha

Target date: TBD

Planned milestone: `v0.7.0-alpha.1`

Primary issues: TBD

Release labels: `release:authority-facts`, `release:compiler-spine`,
`release:lawpacks`, `release:target-profiles`

Scope:

- File-backed lawpack and target-profile fact loading for the first compiler
  facts the in-memory `CompilerContext` already models.
- Deterministic loading of operation profiles, budgets, write classes, effect
  facts, obstruction facts, obligation facts, adapter facts, and target
  capability facts for the supported subset.
- Executable harness that proves the compiler path consumes loaded facts rather
  than caller-constructed memory fixtures.
- Structured diagnostics for missing, ambiguous, malformed, stale, or
  incompatible authority facts.
- A non-topic Authority Fact Governance design note that identifies fact classes,
  provenance questions, review boundaries, and the later trusted-authorship
  alpha scope.

Exit gates:

- Compiler-spine profile/effect compatibility tests cover file-backed fact
  input, not only caller-supplied context.
- Lawpack and target-profile fixture corpora cover accepted and rejected fact
  loading paths.
- The loaded-fact harness is deterministic and does not fetch registries or
  mutate dependency state.
- `docs/design/authority-fact-governance.md` captures the trusted-authorship
  questions opened by this release without claiming implemented workflow.

Non-goals:

- No trusted lawpack or target-profile authorship workflow.
- No global registry, trust root, identity system, or revocation model.
- No target IR generation.
- No full effectful source lowering.
- No admission execution workflow.

## v0.8.0-alpha.1 - Minimal Effectful Compiler Spine Alpha

Target date: TBD

Planned milestone: `v0.8.0-alpha.1`

Primary issues: TBD

Release labels: `release:compiler-spine`, `release:semantic-validation`,
`release:core-ir`

Scope:

- Minimal effectful source subset lowered through the compiler spine.
- Stable Core representation for the first accepted effectful body shape.
- Obstruction mapping and effect-order preservation for the supported subset.
- Profile, budget, effect, and obstruction checks wired to file-backed facts
  where v0.7 made those facts available.

Exit gates:

- At least one effectful intent compiles from source through typed Core with
  deterministic fixtures.
- Unsupported effectful forms fail with stable diagnostics before lowering.
- Existing pure Core golden behavior remains unchanged except for intentional,
  reviewed schema or canonicalization changes.

Non-goals:

- No target IR generation.
- No target-runtime execution.
- No adapter composition.

## v0.9.0-alpha.1 - First Target IR Alpha

Target date: TBD

Planned milestone: `v0.9.0-alpha.1`

Primary issues: TBD

Release labels: `release:target-ir`, `release:lowerability`,
`release:target-profiles`

Scope:

- First target-specific IR for one deliberately narrow runtime profile.
- Lowering evidence that connects accepted Core obligations to the selected
  target IR artifact.
- Structured target-lowering diagnostics for unsupported Core obligations.
- Fixture-backed target IR identity and hash-impact expectations where the
  artifact format is stable enough to bind.

Exit gates:

- One source intent can compile to Core and lower to a concrete target IR
  artifact for the selected profile.
- Lowerability evidence and target IR generation agree on the same fact set.
- Unsupported target features fail loudly rather than falling back silently.

Non-goals:

- No general target-lowering framework.
- No runtime execution guarantee.
- No v2 chained or composite adapter resolution.

## v0.10.0-alpha.1 - Public CLI And Structured Diagnostics Alpha

Target date: TBD

Planned milestone: `v0.10.0-alpha.1`

Primary issues: TBD

Release labels: `release:cli`, `release:developer-tools`,
`release:compiler-spine`

Scope:

- Public `edict` CLI boundary for the first compile, check, explain, and
  diagnostic workflows.
- Machine-readable diagnostic output for stable error kinds and structured
  coordinates.
- Human-readable explanations layered over structured diagnostics.
- Golden CLI fixtures for success, rejection, and explain paths.

Exit gates:

- CLI tests assert output semantics through stable structured artifacts, not
  incidental wording.
- CLI commands do not depend on ambient repo layout beyond explicit input paths.
- README and topic shelves describe only supported commands.

Non-goals:

- No full language server.
- No marketplace packaging.
- No participant policy execution.

## v0.11.0-alpha.1 - Contract Bundle Assembly Alpha

Target date: TBD

Planned milestone: `v0.11.0-alpha.1`

Primary issues: TBD

Release labels: `release:bundles`, `release:admission`, `release:target-ir`

Scope:

- Bundle assembly from real compiler, Core, target IR, target-profile, lawpack,
  and assurance-reference inputs.
- Digest-bound artifact graph that ties source, Core, target IR, target profile,
  lawpack facts, and optional assurance references together.
- Generated bundle summary or nutrition-label material for the supported slice.
- Bundle fixture corpus that proves mutation sensitivity across the artifact
  graph.

Exit gates:

- One compiled and lowered intent can be assembled into a participant-neutral
  contract bundle.
- Bundle validation consumes the assembled artifact, not only hand-written
  bundle fixtures.
- Any unsupported bundle field fails explicitly or remains absent.

Non-goals:

- No participant-specific admission policy.
- No global publication registry.
- No HOLMES, Watson, or Moriarty implementation.

## v0.12.0-alpha.1 - Edict Admission Workflow Harness Alpha

Target date: TBD

Planned milestone: `v0.12.0-alpha.1`

Primary issues: TBD

Release labels: `release:admission`, `release:bundles`

Scope:

- End-to-end Edict-owned admission workflow harness over the assembled bundle
  slice.
- Admission request, receipt, operation requirement, and invocation evidence
  generated or validated from the same bundle graph.
- Structured failure cases for missing capability evidence, hidden inputs,
  mismatched participants, stale receipts, and unsupported operation requests.

Exit gates:

- One bundle can travel through the Edict-owned admission-boundary harness.
- The harness remains participant-neutral and does not implement Continuum trust
  policy.
- Admission evidence is digest-bound to the same bundle subject that the
  compiler and assembler produced.

Non-goals:

- No Continuum participant policy, identity, delegation, or revocation.
- No runtime execution.
- No global admission service.

## v0.13.0-alpha.1 - Trusted Lawpack And Target-Profile Authorship Alpha

Target date: TBD

Planned milestone: `v0.13.0-alpha.1`

Primary issues: TBD

Release labels: `release:authority-governance`, `release:authority-facts`,
`release:lawpacks`, `release:target-profiles`

Scope:

- Lawpack authoring manifest shape.
- Target-profile authoring manifest shape.
- Author, reviewer, and provenance fields for authority facts.
- Review digest binding.
- Changelog or revision-history field for fact changes.
- Stable validation errors for missing author provenance, missing reviewer
  provenance, unsigned or digest-unbound review evidence, fact changes without
  revision notes, write-class changes without explicit review markers,
  conflicting effect ownership, and stale lawpack/target-profile compatibility.
- Fixture corpus for accepted reviewed lawpacks, rejected unreviewed lawpacks,
  rejected conflicting lawpacks, accepted reviewed revisions, and rejected silent
  write-class changes.
- CLI support for `edict lawpack check`, `edict lawpack diff`,
  `edict target-profile check`, and `edict authority explain`.
- Optional authority nutrition label for lawpacks and target profiles if the
  structured fields are stable enough.

Exit gates:

- Trusted-authorship validation is executable and fixture-backed.
- Provenance validation is digest-bound and does not depend on prose review
  claims.
- Conflicting or silently revised authority facts fail loudly with stable
  structured diagnostics.
- The Edict/Continuum boundary is explicit: Edict validates provenance shape and
  evidence binding; Continuum and participants decide acceptance policy.

Non-goals:

- No global registry.
- No public trust root.
- No legal identity model.
- No distributed revocation.
- No Continuum participant policy.
- No HOLMES, Watson, or Moriarty implementation.

## v0.14.0-alpha.1 - Publication Readiness And crates.io Policy Alpha

Target date: TBD

Planned milestone: `v0.14.0-alpha.1`

Primary issues: TBD

Release labels: `release:publication`, `release:developer-tools`

Scope:

- Deliberate crates.io publication policy for workspace crates.
- Package metadata, license, README, and API-stability review for any crate
  proposed for publication.
- Semver and alpha-versioning rules that distinguish specification maturity
  from public API stability.
- Reproducible package dry-run or pack verification for publishable crates.

Exit gates:

- Any crate flipped from `publish = false` has an explicit policy rationale.
- Publication docs distinguish crate availability from v1 language stability.
- Release automation and manual runbooks agree on publish and no-publish paths.

Non-goals:

- No promise of stable v1 API.
- No publishing without an explicit release decision.

## v0.15.0-alpha.1 - Language-Server Semantic Diagnostics Alpha

Target date: TBD

Planned milestone: `v0.15.0-alpha.1`

Primary issues: TBD

Release labels: `release:lsp`, `release:developer-tools`,
`release:semantic-validation`

Scope:

- First language-server semantic diagnostics over parser and compiler-spine
  structured errors.
- Editor-facing diagnostics that reuse CLI/compiler diagnostic kinds rather
  than inventing editor-only semantics.
- Fixture-backed diagnostic ranges and severity mapping for the supported
  source subset.

Exit gates:

- LSP diagnostics are deterministic and asserted through structured artifacts.
- Editor diagnostics reflect compiler truth and do not overclaim unsupported
  lowering, target IR, bundle, or admission behavior.
- Existing syntax-highlighting surfaces keep their current fixture contract.

Non-goals:

- No full formatter.
- No code actions unless backed by structured diagnostics and deterministic
  edits.
- No marketplace publication unless v0.14 policy already permits it.

## Parallel Design Track: Authority Fact Governance

Edict's file-backed fact model creates a new trust surface. Once lawpacks and
target profiles can supply operation profiles, budgets, write classes, effect
facts, obligations, adapter facts, and target capabilities, the project must
define how those facts are authored, reviewed, revised, audited, and trusted.

This track does not block `v0.7.0-alpha.1` fact loading, but it must begin with
that release. The initial design output is a non-topic design note, not a
current-truth topic README:
[`docs/design/authority-fact-governance.md`](./docs/design/authority-fact-governance.md).

Open questions:

- Who authors a lawpack?
- Who reviews effect write-class claims?
- What evidence supports an effect classification?
- How are target-profile capability claims reviewed?
- How are lawpack and target-profile revisions recorded?
- How are write-class changes detected and explained?
- How are conflicting fact owners rejected or selected?
- Which provenance checks belong to Edict, and which acceptance decisions belong
  to Continuum or participants?

Edict owns deterministic validation of fact provenance, digest binding, review
references, and artifact shape. Continuum and participants own trust policy,
identity, delegation, revocation, and acceptance decisions.

## v2-design - Future Design Track

Milestone: `v2-design`

Primary issue: #4

Release labels: `release:v2-design`

Scope:

- Adapter composition and obligation-closure resolution design, tracked as a
  future design note in
  [`docs/design/v2-obligation-closure.md`](./docs/design/v2-obligation-closure.md)
  with current HEAD boundaries in
  [`docs/topics/v2-design/`](./docs/topics/v2-design/).
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
- `v0.7.0-alpha.1`: planned, issues TBD
- `v0.8.0-alpha.1`: planned, issues TBD
- `v0.9.0-alpha.1`: planned, issues TBD
- `v0.10.0-alpha.1`: planned, issues TBD
- `v0.11.0-alpha.1`: planned, issues TBD
- `v0.12.0-alpha.1`: planned, issues TBD
- `v0.13.0-alpha.1`: planned, issues TBD
- `v0.14.0-alpha.1`: planned, issues TBD
- `v0.15.0-alpha.1`: planned, issues TBD
- `v2-design`: #4

Alpha-train release labels:

- `release:front-end`
- `release:core-ir`
- `release:semantic-validation`
- `release:compiler-spine`
- `release:lowerability`
- `release:admission`
- `release:developer-tools`
- `release:authority-facts`
- `release:target-ir`
- `release:cli`
- `release:bundles`
- `release:authority-governance`
- `release:lawpacks`
- `release:target-profiles`
- `release:publication`
- `release:lsp`
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
