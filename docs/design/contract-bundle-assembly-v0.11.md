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

### 1a. Canonical preimage byte shape

The assembler does not hash `sha256:<hex>` review strings directly. For bundle
digest preimages, every review digest is parsed into the authoritative canonical
digest value `["sha256", h'..32 bytes..']` (the same typed `[algorithm, bytes]`
shape the canonical encoder already uses). Each bundle digest is then SHA-256
over:

```text
canonical-cbor([
  "edict.digest/v1",
  "<bundle-domain>",
  <typed bundle preimage value without self digest>
])
```

where `<bundle-domain>` is `edict.bundle.semantic/v1` or
`edict.bundle.release/v1`, and the typed bundle preimage value is the ordered
list of component digests (each a typed `["sha256", <bytes>]` value), never the
human review strings. This mirrors the Core digest path exactly, which frames the
canonical Core module value inside
`["edict.digest/v1", "edict.core.module/v1", <canonical Core module value>]`.
This is the byte-level contract, not just an ingredient list.

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

Top-level digest mutation tests apply to **semantic/release preimage
components**. Excluded external artifacts, optional assurance evidence, and
admission artifacts are handled by validation, subject binding, or explicit
rejection — not by pretending every nearby artifact changes the bundle digest.

| Mutation | semanticBundleDigest | releaseBundleDigest |
| --- | --- | --- |
| Core semantic change (Core digest changes) | changes | changes |
| `targetIrDigest` **reference** changed | changes | changes |
| target profile / lawpack / generated / fixture-corpus / verifier-report digest | changes | changes |
| `sourceProfileSemanticFactsDigest` changed | changes | changes |
| `canonicalizationProfileDigest` changed | changes | changes |
| semantic compile options digest | changes | changes |
| provenance-only source digest / logical path | unchanged | changes |
| compiler / lowerer / verifier identity (artifacts unchanged) | unchanged | changes |
| nonsemantic compile options digest | unchanged | changes |
| build provenance | unchanged | changes |
| compile explanation digest | unchanged | changes |
| optional assurance evidence artifact changed | unchanged | unchanged (no top-level digest claim) |
| assurance evidence subject / target mismatch | rejected | rejected |
| admission artifact inserted | rejected | rejected |

Notes: semantic source *edits* are covered by the Core-digest row (Core digest
changes ⇒ both change); the provenance-only row is exactly that — provenance,
not meaning. A lowerer/verifier identity change is release-only *only when the
produced artifacts are unchanged*; if the target IR digest also changes, the
semantic digest changes through `targetIrDigest`. Optional assurance evidence
and admission artifacts are not top-level preimage components, so they are
governed by validation/subject-binding/rejection, not by digest propagation.

Plus a round-trip: `assemble_contract_bundle(...)` →
`validate_contract_bundle_manifest` returns `Valid`.

The bundle digest goldens are checked in with an `xtask` check/regenerate path,
mirroring the Core golden discipline. **This slice freezes the v0.11 bundle
digest preimage shape and the resulting semantic/release golden digest values.
It does not freeze canonical `ContractBundleManifest` bytes** unless that
manifest encoder is explicitly added as a separate reviewed scope item.
Target-IR byte identity is a separate freeze (#105).

## Documentation discipline

Per `AGENTS.md`: planned cases land in the contract-bundles **test plan** first
(observed RED), behavior is implemented (GREEN), and only then are the planned
rows marked implemented and the topic **README** updated to describe current-HEAD
truth. No README prose describes intended behavior as current truth before it
lands.
