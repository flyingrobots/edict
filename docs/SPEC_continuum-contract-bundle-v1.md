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
  source artifact digest
  Core IR digest
  target IR digest
  generated artifacts
  compiler/lowerer/verifier evidence references

AdmissionRequest
  contractBundleDigest
  participantDescriptorDigest
  catalogSnapshotDigest
  admissionPolicyDigest
  policyEpoch
  requested capabilities/budget

AdmissionReceipt
  admissionRequestDigest
  contractBundleDigest
  participant identity
  decision
  admitted bounds/capabilities
  signature

DistributionEnvelope
  contract bundle
  attestations
  zero or more admission receipts
  transparency evidence
```

The contract bundle digest is computed before admission. The same contract
bundle may be submitted to multiple participants without recompilation and
without changing the contract bundle digest.

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
- compile options digest;
- canonicalization profile digest;
- normative conformance fixture corpus digests;
- verifier report digest;
- explanation artifact digest.

It does not include admission requests, admission receipts, participant policy,
participant descriptors, participant catalog snapshots, display metadata, or
signatures over itself.

## Core Versus Provenance

Raw source bytes, source paths, comments, locations, and formatting are bundle
provenance. They do not participate in the Core IR digest. Source-profile
semantic facts, Shape IR, Law IR, and digest-locked compile options are semantic
inputs to Core compilation.

Formatting-only source changes may change the raw source artifact digest and the
contract bundle digest while leaving the Core IR digest and target IR digest
unchanged.

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
- digest bytes represented as byte strings in authoritative CBOR;
- human hex strings allowed only in review JSON.

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

The contract bundle digest remains independent of its external signatures and
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
