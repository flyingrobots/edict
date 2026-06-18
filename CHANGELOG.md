# Changelog

All notable changes to the Edict specifications are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Edict is in Phase 0 (design only); versions track specification maturity, not a
released implementation.

## [Unreleased]

### Added

- `SPEC_edict-lawpack-abi-v1.md`: the Lawpack ABI (manifest, dependency graph,
  exported types/constants, pure helper and semantic effect signatures,
  proof-only vs runtime-materialized classification, typed obstruction payloads,
  footprint/cost obligations, target adapters, compatibility matrix).
- `docs/abi/`: machine-readable schemas as the single source of truth —
  `edict-target-profile.cddl`, `edict-target-lowerer.wit`, `edict-lawpack.cddl`.
- `docs/REQUIREMENTS.md`: the Fixture Constitution — every normative requirement
  gets a stable ID bound to its owner spec and positive/negative/golden fixtures.
- `spec.lock.json`: generated lock of schema/registry digests for a doc-build
  gate.
- `fixtures/`: Phase 0 corpus layout and conventions.
- Minimal normative **optic contract** in Core (`opticKind`, `basis`,
  `boundaryKind`, `supportPolicy`, `lossDisposition`); richer Observer Geometry
  evidence (Aperture Ledger, witness debt, degeneracy) as derived verifier
  evidence.
- Refined scalar types `String<max=, canonical=>` / `Bytes<max=>`; typed integer
  literal elaboration; pinned `len` units.
- `where` as input refinement; mandatory `else` on runtime `require`/`guarantee`;
  short-circuit boolean semantics; defined bounded `for` loop behavior.
- Typed obstruction payload construction with failure binders and exhaustive
  matching.
- `edict.language/v1` vs `edict.implementation/minimal-v1` profiles with
  conformance capability flags.
- Core/target/admitted budget split; `CapabilityRef<T>`; semantic vs release
  bundle digests; logical source-path rules; namespace/shadowing and
  enum-vs-variant syntax rules.
- Hash ladder, Aperture Ledger, Lawfulness Certificate, obstruction coverage,
  and the two-lowerer differential trial in the assurance guide.

### Changed

- Purified Core IR: removed Core self-hash (now `canonicalizationProfile`),
  removed lowerer/verifier digests and packaging fields from the Core preimage,
  moved `verifiedOperationMode` to the verifier report (Core keeps
  `requiredOperationProfile`), demoted `preconditions`/`postconditions` to
  derived indices and `diagnosticPolicy` to a compile option.
- Turned the Target Profile ABI into a real ABI (CDDL manifest + WIT plugin
  boundary + exchange types); added `pure`/`effect` intrinsic class; removed the
  duplicated manifest from the language spec.
- Made the artifact graph explicitly acyclic: split compile vs admission
  explanations, split admission receipt body from its signature, added a
  universal acyclicity rule.
- README/docs drift fixes: `edict` code fences, corrected artifact ER-diagram
  cardinality, bounded the `hello` example, fixed the alias-shadowing example.

### Changed (yellow-light review, round 2)

Five underdetermined contracts nailed before implementation:

- **Optic source production**: added the `basis` source clause (incl.
  `basis none`); pinned one deterministic source for each Core optic field
  (basis clause / profile template / canonical coordinate / footprint ceiling);
  `basis` is a typed Core expression, `apertureRequirement` an explicit field;
  `supportPolicy`/`lossDisposition` are coordinates, not strings.
- **Lawpack classification**: replaced `materialization` with orthogonal
  `executionClass` (`proofOnly`/`runtime`) × authoritative `writeClass`;
  `readOnly` now permits runtime reads (writeClass=read).
- **Named typed failures**: ABIs declare `effectFailures` (coordinate +
  authorityClass + bounded payloadType) per effect; obstruction map keyed by
  failure coordinate; target intrinsic CDDL is an enforced pure/effect union;
  lawpack components made appropriately optional.
- **Digest wire format**: canonical digest is the typed pair
  `[algorithm, bytes]` everywhere (CDDL, WIT, bundle spec); `"sha256:<hex>"` is
  review-JSON only.
- **Bundle subject propagation**: defined exact `semanticBundleDigest` and
  `releaseBundleDigest` preimages (toolchain identity lives in release, not
  semantic); requests/receipts carry `bundleSubject {kind,digest}`; Moriarty
  matrix tracks both; lowerer compares cost against the declared target ceiling,
  not an admitted participant budget.

Cleanups: `canonicalEncode<T>` returns bounded `Bytes`; bounded `repo` in the
normative example; missing lawpack target adapter is a compiler/lowering error,
not admission-class.

### Fixed (PR #2 review audit)

Resolved 9 Codex review threads + 1 self-discovered issue, one commit each:

- Lowerer compares cost/footprint vs the **declared target ceiling**, never an
  admitted participant budget (prose now matches the WIT).
- `CoreGuard`/`ObstructionMap` carry a typed obstruction payload construct so
  guard `expected`/`observed` round-trip through Core.
- Core `inputConstraints` carries the typed `where` predicate trees (hash-
  significant), not a validator coordinate.
- `CapabilityRef<T>` added to the Core type universe, `type-ref` grammar, and
  map-key exclusions.
- Lawpack adapter maps failures by **coordinate** (named failures sharing an
  authority class stay distinct).
- `acceptedLawpackAdapterAbi` deferred/optional until `edict.lawpack-adapter/v1`
  is specified.
- Semantic vs nonsemantic compile options split so diagnostics cannot change
  `semanticBundleDigest`.
- Admission explanation carries `bundleSubject {kind,digest}`.
- Deduped the orphan intrinsic requirement ID; registered all audit IDs.
- Defined `CanonicalEncodedMax<T>` (self-discovered).

### Fixed (PR #2 review round 2 — CodeRabbit)

Triage ruling: fix 15, defer 1. Bounded semantic clarifications, one commit each:

- CapabilityRef carried digest is canonical/hash-significant; obstruction
  shorthand legal only for exactly one unmapped class (zero omits `else`).
- Core budget units pinned; `targetBudget` = costAlgebra + resolved ceiling;
  cost checked vs declared ceiling (admission stays external).
- Minimal-v1 prelude closed (integer/string/bytes ops; unlisted ops don't
  exist); integer-type propagation contexts enumerated + mismatch diagnostic.
- `basis` is pure/effect-free over inputs/constants/capability-refs/pure-fns.
- Pure-helper bounds enforced at import + call-site.
- Every `require` carries `else`; input-only require allowed for domain
  obstructions; `where`/`require`/`assert` roles disjoint.
- Option refinement only (isSome/unwrap, lexical); no general narrowing.
- Bound violation = integrity/internal fault, never silent truncation.
- CoreGuard is targetAtomic + always carries an obstruction; verifier proofs are
  separate `CoreProofObligation` nodes.
- Positive exhaustive Core-intent hash-preimage inclusion list added.
- operation-profile optic template owned in `edict-common.cddl`;
  `apertureRequirement` is a typed reference.
- Source vs Core-ABI capability flags split.
- Shared CDDL types extracted to `edict-common.cddl`; intrinsics corpus-document
  shape defined; lawpack verifier classified (executable ⇒ sandbox+fuel).

Deferred (1): the complete `edict.core/v1` CoreExpr/CorePredicate CDDL →
issue #3; spec marks JSON expression examples illustrative and forbids freezing
any Core hash golden before that schema lands.

### Notes

Applies the Phase 0 design review (external "ChatGPT" feedback): SHOULD/COULD
treated as MUST. Design baseline marked non-normative historical context. Grammar
and Core schema remain **unfrozen** but the five yellow-light joints are now
determined; next step is Phase 0 implementation (parser fixtures, Core CDDL,
canonical-CBOR goldens, tiny KV target). v1 is not yet stable.
