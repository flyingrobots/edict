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

### Notes

Applies the Phase 0 design review (external "ChatGPT" feedback): SHOULD/COULD
treated as MUST. Design baseline marked non-normative historical context. Grammar
and Core schema remain **unfrozen**; v1 is not yet stable.
