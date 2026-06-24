# Changelog

All notable changes to the Edict specifications are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Edict now has executable Rust implementation slices alongside the design specs;
versions still track specification maturity rather than a released product.

## [Unreleased]

### Added

- Added the first reference `edict.canonical-cbor/v1` Core encoder for the
  current in-memory Core module model, plus canonical byte validation through
  decode/re-encode stability checks. This slice intentionally does not freeze
  reviewed golden bytes or exact Core digests.
- Added the first executable compiler-spine slice for `v0.3.0-alpha.1`:
  explicit `resolve_module`, `type_check`, `lower_core`, and `compile_to_core`
  APIs; deterministic `CompilerContext` profile/budget facts; a typed module
  boundary distinct from source AST; and in-memory Core IR lowering for the
  initial pure local-record subset. The slice intentionally makes no canonical
  byte, exact digest, target lowering, or admission claim.
- Added `validate_surface` as the explicit source/surface semantic-validation
  compiler stage, with deterministic tests proving that import/name resolution,
  contextual typing, loop-bound proof, and target/lawpack obstruction
  exhaustiveness remain downstream of this pass. `validate_module` remains a
  compatibility alias for the same stage.
- Added the repository rule that issue-closing PRs must include GitHub
  auto-close text such as `Closes #123` in the pull request body.

## [v0.2.0-alpha.1] - 2026-07-01

### Added

- Added the `edict.core/v1` Core IR topic shelf and normative CDDL schema for
  the `v0.2.0-alpha.1` Core semantic-model milestone, with local `xtask`
  regressions proving required schema declarations and the explicit no-byte/hash
  freeze boundary.
- Added a repo-local `AGENTS.md` topic-shelf policy, a release-process topic
  shelf, and a structured release-tag recovery policy covering tag-triggered
  GitHub Release publication.

### Changed

- Extended `cargo xtask contract-check` evidence discovery to include `xtask`
  tests, so workflow/process shelves can cite executable `xtask` regressions.
- Relaxed Markdown heading duplication checks to allow changelog section
  headings to repeat across different release versions.

## [v0.1.0-alpha.1] - 2026-06-24

### Added

- **Release roadmap.** Added `ROADMAP.md` as the scheduled alpha-release plan,
  linked it from the README/docs index, and mapped GitHub milestones, release
  labels, and issue #16 for the `v0.1.0-alpha.1` release-prep checklist.
- **Phase 2 — source-AST semantic validation (`edict-syntax`).** Added
  `validate_module`, stable `SemanticErrorKind` categories, deterministic tests,
  and a semantic-validation topic shelf for checks that do not require Core IR:
  bounded runtime `String`/`Bytes`, intent operation-mode/budget/basis
  requiredness, duplicate singleton intent clauses, module namespace collision
  checks, and scoped binder shadowing checks.
- **Topic shelf pilot (`docs/topics/syntax/`).** Added the first current-truth
  topic chapter and verification matrix for the Phase 1 syntax front end,
  library-hosted doctest coverage for the external Markdown example, and
  `cargo xtask verify` / `cargo xtask contract-check` as the local contract
  graph gate.
- **Phase 1 — first executable slice (`crates/edict-syntax`).** A standalone,
  std-only Rust workspace with a hand-written deterministic lexer and a
  recursive-descent parser for the `edict.implementation/minimal-v1` surface.
  Now parses: package/imports (shape/lawpack/target/core, optional `digest`);
  `type` records and refined scalars; `enum` declarations; `variant` types with
  optional payloads; `intent`s with their clauses; `let`/`return`; calls and
  type-calls (`echo.ref<T>(...)`); effect statements with single- and
  map-form `else` obstruction handlers; `require`/`guarantee`/`assert`; the full
  `if` family (ternary `if … then … else …`, effectful branch-yield in
  `let`-rhs, and `if`/`else if`/`else` control flow); bounded
  `for … in … bounded …` loops; variant-literal constructors
  (`Qual.Type::Case(payload)`); boolean and `digest("sha256:…")` literals; and
  `match` expressions. Keywords are reserved as bare identifiers but remain
  legal as member names after `.` (§1510-1511). Conformance fixtures under
  `fixtures/lang/`; 55 tests green under
  `cargo fmt --check`, `clippy` deny-all + pedantic, and CI. See
  `docs/RETRO_phase1-parser.md`.
- `SPEC_edict-lawpack-abi-v1.md`: the Lawpack ABI (manifest, dependency graph,
  exported types/constants, pure helper and semantic effect signatures,
  `executionClass` × `writeClass` classification, typed obstruction payloads,
  footprint/cost obligations, target adapters, compatibility matrix, the v1
  direct-adapter resolution rule).
- `docs/abi/`: machine-readable schemas as the single source of truth —
  `edict-common.cddl` (shared types), `edict-target-profile.cddl`,
  `edict-target-lowerer.wit`, `edict-lawpack.cddl`.
- `docs/REQUIREMENTS.md`: the Fixture Constitution — every normative requirement
  gets a stable ID bound to its owner spec and positive/negative/golden fixtures.
- `spec.lock.json` (schema/registry digest lock for a doc-build gate);
  `fixtures/` Phase 0 corpus layout and conventions.
- Minimal normative **optic contract** in Core (`opticKind`, `basis`,
  `boundaryKind`, `apertureRequirement`, `supportPolicy`, `lossDisposition`),
  each with one deterministic source; richer Observer Geometry evidence
  (Aperture Ledger, witness debt, degeneracy) as derived verifier evidence.
- **Partial Lowerability** section: lowering is a partial, semantics-preserving
  relation classified `native` / `adapted` / `composite` / `unsupported`;
  unsupported is a compiler error, never a silent approximation. README gains the
  lowerability value-proposition statement.
- Language semantics: refined scalar types `String<max=,canonical=>` /
  `Bytes<max=>` (bytes max-only) and pinned `len` units; typed integer-literal
  elaboration with propagation contexts; closed minimal-v1 prelude with pinned
  **integer safety** (overflow-safe, `checked*`, no wrap/saturate/trap);
  `where` input refinement; `basis` clause (pure/effect-free); bounded `for`
  loops; short-circuit booleans; Option-only refinement; typed obstruction
  payloads with failure binders + exhaustive matching; `CapabilityRef<T>`.
- Profiles & packaging: `edict.language/v1` vs `edict.implementation/minimal-v1`
  capability flags (source vs Core split); Core/target/admitted budget split with
  pinned units; semantic vs release bundle digests; logical source-path rules;
  namespace/shadowing and enum-vs-variant rules; `postconditionSupport` target
  field.
- Assurance guide: hash ladder, Aperture Ledger, Lawfulness Certificate,
  obstruction coverage, two-lowerer differential trial.

### Changed

- Roadmap correction: inserted the explicit compiler-spine milestone between
  Core schema work and target/admission work, split the Core IR issue scope
  across schema, compiler-spine, encoder, and golden-digest artifacts, and moved
  developer tooling to `v0.6.0-alpha.1`.
- Updated the `edict-syntax` package description to include source-level
  semantic validation, not only the Phase 1 lexer/parser.
- Purified Core IR: removed the Core self-hash (now `canonicalizationProfile`);
  removed lowerer/verifier digests and packaging fields from the preimage; moved
  `verifiedOperationMode` to the verifier report (Core keeps
  `requiredOperationProfile`); demoted `preconditions`/`postconditions` to derived
  indices and `diagnosticPolicy` to a compile option; reconciled I-010; added a
  positive exhaustive preimage inclusion list and excluded human `name` fields.
- Turned the Target Profile ABI into a real ABI (CDDL manifest + WIT plugin
  boundary + exchange types); enforced `pure`/`effect` intrinsic union;
  named/typed `effectFailures`; intrinsics corpus-document shape; classified
  lawpack verifier (executable ⇒ sandbox+fuel); removed the duplicated manifest
  from the language spec; extracted shared types to `edict-common.cddl`.
- Canonical digest is the typed pair `[algorithm, bytes]` everywhere;
  `"sha256:<hex>"` is review-JSON only.
- Made the artifact graph explicitly acyclic: split compile vs admission
  explanations; split admission receipt body from its DSSE signature; defined
  exact `semanticBundleDigest`/`releaseBundleDigest` preimages (toolchain identity
  in release, not semantic); requests/receipts/explanations carry
  `bundleSubject {kind,digest}`; split semantic vs nonsemantic compile options.
- `require` always carries `else` (grammar + semantics); `where`/`require`/
  `assert` roles disjoint; CoreGuard is `targetAtomic` and always carries an
  obstruction, with verifier proofs as separate `CoreProofObligation` nodes.
- README/docs drift fixes: `edict` code fences, corrected ER-diagram cardinality,
  bounded `hello`/`repo` examples, fixed the alias-shadowing example.
- Design baseline marked non-normative historical context.

### Fixed

- Applied the external Phase 0 design review and two follow-up review rounds
  (Codex + CodeRabbit): closed every flagged contradiction and normative hole as
  bounded clarifications, one commit per finding. Notable: lowerer compares
  cost/footprint vs the **declared** ceiling (admission is external); lawpack
  adapters map failures by **coordinate**; `acceptedLawpackAdapterAbi`
  schema-enforced empty until its ABI exists; `targetBudget` carries both the
  hash-significant `costAlgebra` ref and resolved `ceiling`; bound violations are
  integrity/internal faults, never silent truncation; defined
  `CanonicalEncodedMax<T>` and `edict.core-cost/v1`; deduped requirement IDs.
- Self-review nits: dropped an unused WIT import; de-duplicated the
  `basis`-requiredness wording; locked `edict-common.cddl` in `spec.lock.json`;
  corrected the `edict-common.cddl` header.
- Second-order ripples from the above (Codex + CodeRabbit round): an intent may
  carry **both** `profile` and `implements` (was wrongly "exactly one"); pure
  expressions may call **pure** target/lawpack constructors (only effect
  intrinsics forbidden); integer-literal propagation reaches binary operands;
  field-constraint and refined-type bounds are both valid; `requiredCoreCapabilities`
  is a hash-significant Core module field; operation-profile records get a
  publication slot in the target/lawpack ABIs; exported pure helpers require a
  hash-bound implementation; residual singular bundle-digest references replaced
  with `bundleSubject`; Core/README examples updated to the new rules
  (ObstructionConstruct, `basis` clauses); registry `deferred` status defined and
  the int-literal-mismatch ID numbered (`EDICT-LANG-INTLIT-002`).
- Further ripple round: `edict`-source pure helpers must carry an inline body
  (CDDL union); `operationProfiles` added to the target-profile manifest example;
  `optic-template` can publish `apertureRequirement`; target adapters digest-lock
  their accepted target profile + Target IR; GREEN fixtures use syntactically
  valid dummy digests (the prose `sha256:...` is an un-lexable ellipsis).
- Schema/example/prose alignment round (+ proactive same-class sweep): lawpack
  manifest example carries adapter target-locks; export-surface summary lists
  `operationProfiles`; component pure helpers carry their own digest-locked
  `implementation`; language operation-mode `custom` bullet mirrors the ABI;
  README fixture promise accounts for digest substitution; compile explanation
  surfaces `apertureRequirement`; LawfulnessCertificate proves only core+target
  declared ceilings (never `admitted`); obstruction coverage includes lawpack
  effects; portable example gains a `basis`; Appendix A scoped as exploratory
  non-fixtures; `effectFailures` coordinates must be unique per effect.
- **jedit appendix brought to clause-conformance** (it is the intended first
  real-world use case): added correct `basis` clauses to all 12 rope-package and
  structural-history intents; the Product Text Buffer Optic sketch remains the
  one deliberate non-v1 example (uses rejected `invoke`/`use capability` to show
  design pressure). Appendix note rewritten accordingly.

### Deferred

- The complete `edict.core/v1` CoreExpr/CorePredicate CDDL and canonical encoding
  → issue #3. The spec marks JSON expression examples illustrative and forbids
  freezing any Core hash golden before that schema lands. Adapter
  obligation-closure composition → issue #4; `edict explain lowerability` CLI →
  issue #5.

### Notes

Applies the Phase 0 design review (external "ChatGPT" feedback): SHOULD/COULD
treated as MUST. Grammar and Core schema remain **unfrozen** but the five
yellow-light joints are now determined; next step is Phase 0 implementation
(parser fixtures, Core CDDL, canonical-CBOR goldens, tiny KV target). v1 is not
yet stable.
