# Contract Bundle Assembly — v0.11 Assembly Boundary

Status: design record for the `v0.11.0-alpha.1` contract-bundle assembly slice.
This is a scope boundary note, not a bundle manifesto. The normative contract is
[SPEC - Continuum Contract Bundle v1](../SPEC_continuum-contract-bundle-v1.md);
the existing validation surface is in `crates/edict-syntax/src/contract_bundle.rs`.

## Why this note exists

Before v0.11 the crate could *validate* a hand-written `ContractBundleManifest`
but could not *assemble* one. v0.11 moves bundle digest recomputation from
"out of scope" to implemented behavior. The one thing this slice must not do is
silently absorb a second, unrelated freeze — **canonical Target IR byte
identity** — under bundle pressure. That freeze belongs to the target-IR track
and is tracked separately (#105).

## 1. Scope

This slice implements **bundle digest derivation and assembly**:

- The assembler **computes** `semanticBundleDigest` and `releaseBundleDigest`
  from digest-locked artifact references, using the existing
  `edict.digest/v1` domain-separated SHA-256 framing (the same primitive that
  produces the Core digest).
- The assembler **computes** `coreIrDigest` from the actual compiled Core
  artifact (`digest_core_module`), so the bundle is anchored on a real Core
  freeze.
- The assembler **accepts** `targetIrDigest` and every other layer hash (target
  profile, lawpacks, source-profile semantic facts, generated artifacts,
  canonicalization profile, semantic/nonsemantic compile options, conformance
  fixture corpora, verifier report, compiler/lowerer/verifier identities, source
  provenance, build provenance, compile explanation) as **supplied,
  digest-locked references**.
- The assembled manifest is consumed by the existing
  `validate_contract_bundle_manifest` (exit gate: validation consumes the
  assembled artifact, not a hand-written fixture).

The two bundle preimages are exact (`CONTINUUM-BUNDLE-SUBJECT-001`):

```text
semanticBundleDigest = digest("edict.bundle.semantic/v1", [
  coreIrDigest, targetProfileDigest, targetIrDigest, lawpackDigests,
  sourceProfileSemanticFactsDigest, generatedArtifactDigests,
  canonicalizationProfileDigest, semanticCompileOptionsDigest,
  conformanceFixtureCorpusDigests, verifierReportDigest ])

releaseBundleDigest = digest("edict.bundle.release/v1", [
  semanticBundleDigest, rawSourceArtifactDescriptors,
  compilerIdentityAndDigest, lowererIdentityAndDigest, verifierIdentityAndDigest,
  nonSemanticCompileOptionsDigest, buildProvenance, compileExplanationDigest ])
```

`releaseBundleDigest` references `semanticBundleDigest`, never the reverse
(`CONTINUUM-BUNDLE-DAG-001`).

### Provenance is typed, not commented

The assembly input API makes computed-versus-supplied **unrepresentable as a
mistake**: the Core digest enters as a *computed* value derived from a real
`CoreModule`, and every supplied hash enters through a distinct *supplied
reference* type. A reader of the assembled bundle (or of the input struct) can
never mistake a supplied `targetIrDigest` for one this slice computed from
bytes.

## 2. Non-claim

This slice deliberately does **not**:

- define a canonical Target IR encoding or byte layout;
- claim target-IR **byte** tamper detection — it cannot rehash target IR bytes,
  because those bytes are not yet canonical (#105);
- load files, run target verifiers, or perform admission.

It **does** detect target-IR **digest-reference** changes in the bundle graph:
if the supplied `targetIrDigest` reference changes, the bundle digests change.
That is digest-graph tamper evidence at the bundle layer, not target-IR byte
identity.

## 3. Follow-up

[#105](https://github.com/flyingrobots/edict/issues/105) tracks canonical Target
IR bytes/digests on the target-IR track: a canonical CBOR/value model, byte
fixtures, a digest function, reviewed goldens, then integration into assembly so
`targetIrDigest` can be **computed** from a real artifact (upgrading the matrix
row below from "reference changed" to "bytes rehashed"). It is not punted into
this slice.

## 4. Test matrix

Mutation sensitivity honors the spec's semantic/release split — not every
mutation changes everything (`CONTINUUM-SEMANTIC-OPTIONS-001`,
`EDICT-CORE-NODIAG-001`):

| Mutation | semanticBundleDigest | releaseBundleDigest |
| --- | --- | --- |
| Core semantic change (Core digest changes) | changes | changes |
| `targetIrDigest` **reference** changed | changes | changes |
| target profile / lawpack / generated / fixture-corpus / verifier-report digest | changes | changes |
| semantic compile options digest | changes | changes |
| raw source descriptor / logical path / source digest | unchanged | changes |
| compiler / lowerer / verifier identity | unchanged | changes |
| nonsemantic compile options digest | unchanged | changes |
| build provenance | unchanged | changes |
| compile explanation digest | unchanged | changes |
| admission artifact inserted | rejected | rejected |

Plus a round-trip: `assemble_contract_bundle(...)` →
`validate_contract_bundle_manifest` returns `Valid`.

The frozen bundle digest goldens are checked in with an `xtask` check/regenerate
path, mirroring the Core golden discipline. That checked-in freeze is the
deliberate v0.11 byte freeze; target-IR byte identity is a separate freeze (#105).

## Documentation discipline

Per `AGENTS.md`: planned cases land in the contract-bundles **test plan** first
(observed RED), behavior is implemented (GREEN), and only then are the planned
rows marked implemented and the topic **README** updated to describe current-HEAD
truth. No README prose describes intended behavior as current truth before it
lands.
