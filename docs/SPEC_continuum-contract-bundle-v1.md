---
title: "SPEC - Continuum Contract Bundle v1"
legend: "SPEC|TRANSMUTE|PLATFORM"
lane: "design"
packet: "0021-continuum-yolo-runtime-neutral-edict-sha-lock-assurance"
issue: "https://github.com/flyingrobots/wesley/issues/611"
status: "draft"
owners:
  - "@flyingrobots"
created: "2026-06-18"
updated: "2026-06-18"
---

<!-- markdownlint-disable MD025 -->

# SPEC - Continuum Contract Bundle v1

<!-- markdownlint-enable MD025 -->

## Purpose

This specification defines participant-neutral contract bundle identity.
Admission is external and is specified in
[SPEC - Continuum Admission v1](./SPEC_continuum-admission-v1.md).

## Artifact Layers

Continuum separates four artifact layers:

```text
ContractBundle
  semanticBundleDigest      # executable semantics
  releaseBundleDigest       # semantic + source provenance + toolchain
  Core IR digest
  target IR digest
  generated artifacts
  compiler/lowerer/verifier evidence references

AdmissionRequest
  bundleSubject { kind: semantic | release, digest }
  participantDescriptorDigest
  catalogSnapshotDigest
  admissionPolicyDigest
  policyEpoch
  requested capabilities/budget

AdmissionReceiptBody
  admissionRequestDigest
  bundleSubject { kind: semantic | release, digest }   # echoes the request
  participant identity
  decision
  admitted bounds/capabilities
  (no signature reference: a body never points to the envelope signing it)

DistributionEnvelope
  contract bundle
  attestations
  zero or more (AdmissionReceiptBody + its DSSE envelope)
  transparency evidence
```

The receipt body is hashed to `AdmissionReceiptBodyDigest`; a DSSE envelope
signs that digest. The signature lives in the distribution envelope, never
inside the body it authenticates (`CONTINUUM-RECEIPT-ACYCLIC-001`).

Both bundle digests (`semanticBundleDigest`, `releaseBundleDigest`) are computed
before admission. The same contract bundle may be submitted to multiple
participants without recompilation and without changing its digests.

## Contract Bundle Contents

A contract bundle binds:

- source artifact descriptors and digests;
- source-profile semantic facts digest;
- Core IR digest;
- target profile digest;
- target IR digest;
- lawpack digests;
- generated artifact digests;
- compiler identity and digest;
- lowerer identity and digest;
- verifier identity and digest;
- semantic compile options digest (semantic-affecting options only; nonsemantic
  diagnostic options are bound by release/sidecar, not semantic);
- canonicalization profile digest;
- normative conformance fixture corpus digests;
- verifier report digest;
- compile explanation artifact digest.

The bundle binds only the **compile explanation** artifact, which is
participant-neutral. The **admission explanation** artifact is policy-epoch
specific, references the bundle and a receipt, and lives outside the bundle (see
[GUIDE - Edict Assurance and Transparency](./GUIDE_edict-assurance-transparency.md)).

It does not include admission requests, admission receipts, admission
explanations, participant policy, participant descriptors, participant catalog
snapshots, display metadata, or signatures over itself.

## Semantic And Release Bundle Digests

A contract bundle exposes two digests so participant policy can decide which it
admits. The two preimages are exact (`CONTINUUM-BUNDLE-SUBJECT-001`).

`semanticBundleDigest` identifies the executable semantics. Its preimage is:

```text
semanticBundleDigest = digest(domain "edict.bundle.semantic/v1", [
  coreIrDigest,
  targetProfileDigest,
  targetIrDigest,
  lawpackDigests,
  sourceProfileSemanticFactsDigest,
  generatedArtifactDigests,
  canonicalizationProfileDigest,
  semanticCompileOptionsDigest,
  conformanceFixtureCorpusDigests,
  verifierReportDigest
])
```

Only **semantic** compile options enter `semanticCompileOptionsDigest` — those
that can change Core IR, target IR, or canonical encoding. Nonsemantic options
whose wording is not operation law (notably `diagnosticPolicy`, repair text, and
other diagnostic selectors) are excluded; they are bound, if at all, by
`releaseBundleDigest` or a diagnostic sidecar. Changing only diagnostics must not
change `semanticBundleDigest` (`EDICT-CORE-NODIAG-001`,
`CONTINUUM-SEMANTIC-OPTIONS-001`).

A different but conforming lowerer/verifier must produce the **same**
`semanticBundleDigest` (this is what the two-lowerer trial checks); therefore
toolchain identities are **not** in the semantic preimage. Formatting-only or
comment-only source changes also do not change it.

`releaseBundleDigest` additionally binds source provenance and the exact
toolchain. Its preimage is:

```text
releaseBundleDigest = digest(domain "edict.bundle.release/v1", [
  semanticBundleDigest,
  rawSourceArtifactDescriptors,     # incl. logical source paths
  compilerIdentityAndDigest,
  lowererIdentityAndDigest,
  verifierIdentityAndDigest,
  nonSemanticCompileOptionsDigest,
  buildProvenance,
  compileExplanationDigest
])
```

`releaseBundleDigest`'s preimage references `semanticBundleDigest`, never the
reverse, preserving acyclicity (`CONTINUUM-BUNDLE-DAG-001`).

Wherever an artifact, request, or receipt references "the bundle", it carries an
explicit `bundleSubject`:

```text
bundleSubject = { kind: "semantic" / "release", digest: <Digest> }
```

## Acyclicity

The artifact graph is a DAG. One universal rule governs it
(`CONTINUUM-BUNDLE-DAG-001`):

> No artifact participating in a subject's digest may directly or transitively
> reference an attestation, signature, receipt, or envelope whose subject is
> that digest.

Concretely: Core IR never references bundle fields; the bundle never references
admission requests/receipts/signatures over itself; a receipt body never
references its own signing envelope; the compile explanation never references a
policy epoch. Acyclicity is a Moriarty fixture target: any introduced cycle must
be caught (`CONTINUUM-BUNDLE-DAG-MORIARTY-001`).

## Core Versus Provenance

Raw source bytes, source paths, comments, locations, and formatting are bundle
provenance. They do not participate in the Core IR digest. Source-profile
semantic facts, Shape IR, Law IR, and digest-locked compile options are semantic
inputs to Core compilation.

Formatting-only source changes may change the raw source artifact digest and the
`releaseBundleDigest` while leaving the Core IR digest, target IR digest, and
`semanticBundleDigest` unchanged (`CONTINUUM-BUNDLE-SUBJECT-001`).

### Logical Source Paths

Source paths recorded as bundle provenance must be logical, package-relative
URIs, never machine-local paths (`CONTINUUM-SOURCEPATH-001`). The content digest
is identity; the logical path is reproducible provenance. A logical source path:

- is UTF-8;
- uses forward slashes only;
- contains no `.` or `..` segments;
- contains no drive letters and no leading slash;
- is not derived from a symlink target;
- never contains an absolute machine path such as `/Users/<name>/...`.

A path such as `contracts/jedit/rope.graphql` is a valid locator. An absolute or
machine-local path rejects locked-bundle production.

## Display Sidecars

Display metadata is never hash-significant for semantic artifact identity. A
display sidecar is keyed by the authoritative digest:

```json
{
  "subject": "sha256:<target-profile-or-bundle-digest>",
  "display": {
    "codename": "YOLO",
    "expansion": "You Only Lawfully Operate"
  }
}
```

The historical packet slug contains `yolo` as a non-authoritative design
locator. It is not a runtime coordinate and does not violate the codename
coordinate invariant.

## Canonical CBOR

`edict.canonical-cbor/v1` is the authoritative byte encoding for Edict and
Continuum bundle digests.

The v1 profile pins:

- CBOR definite lengths only;
- shortest preferred integer encodings only;
- duplicate map keys rejected;
- indefinite strings, arrays, and maps rejected;
- NaN and floating-point values rejected by Edict Core v1;
- unknown CBOR tags rejected;
- map keys sorted by deterministic encoded-byte lexical order;
- no length-first map ordering variant;
- all text labels encoded as UTF-8 without normalization;
- digests encoded as the typed pair `[algorithm, bytes]` (e.g.
  `["sha256", h'..32 bytes..']`), never as a `"sha256:<hex>"` string, in
  authoritative CBOR (`EDICT-DIGEST-WIRE-001`);
- human hex rendering `"sha256:<64 lowercase hex>"` allowed only in review JSON.

Generic canonical values carry scalar type explicitly, so `U32(1)` and `U64(1)`
have distinct preimages.

## Hash Framing

All Edict and Continuum artifact hashes use SHA-256 over a canonical tuple:

```text
SHA-256(canonical-cbor([
  "edict.digest/v1",
  "<domain>",
  <typed artifact value without self digest>
]))
```

Rules:

- hash algorithm: SHA-256;
- domain labels: UTF-8 text strings, exact bytes, no normalization;
- self-digest fields are omitted from the preimage;
- manifests should prefer external resource descriptors over embedded `digest`
  fields;
- if an embedded digest field is unavoidable, the preimage must use exactly one
  specified placeholder representation.

## Signatures And Attestations

Edict does not define a bespoke signature envelope.

V1 roles:

- in-toto Statement plus typed Edict predicate describes compiler, lowerer,
  verifier, HOLMES, Moriarty, and admission attestations;
- DSSE authenticates those attestation envelopes;
- COSE_Sign1 may be used later for a directly signed CBOR distribution envelope,
  but v1 does not require both DSSE and COSE for the same claim.

The bundle digests remain independent of their external signatures and
attestations.

## Distribution Envelope

A distribution envelope may aggregate:

- one contract bundle;
- compiler/lowerer/verifier attestations;
- HOLMES/Watson/Moriarty attestations;
- zero or more admission receipts;
- transparency-log inclusion or consistency evidence;
- display sidecars.

Changing the distribution envelope does not change the enclosed contract bundle
identity.
