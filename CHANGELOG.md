# Changelog

All notable changes to the Edict specifications are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Edict is in Phase 0 (design only); versions track specification maturity, not a
released implementation.

## [Unreleased]

### Added

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
