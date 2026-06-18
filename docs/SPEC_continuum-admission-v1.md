---
title: "SPEC - Continuum Admission v1"
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

# SPEC - Continuum Admission v1

<!-- markdownlint-enable MD025 -->

## Purpose

Continuum admission is participant policy over a participant-neutral contract
bundle. Admission requests and receipts reference a contract bundle digest; they
are not components of that digest.

## Admission Request

An admission request contains:

- `contractBundleDigest`;
- participant descriptor digest;
- catalog snapshot digest;
- admission policy digest;
- policy epoch or monotonic policy version;
- requested operation set;
- requested capabilities;
- requested runtime budgets and ceilings;
- requester identity or provenance, if required by participant policy;
- attestation references required by the participant.

Admission requests are canonical artifacts with their own digest.

## Admission Receipt

An admission receipt is split into a hashed body and an external signature so
the body never references the envelope that signs it
(`CONTINUUM-RECEIPT-ACYCLIC-001`).

An `AdmissionReceiptBody` contains:

- `admissionRequestDigest`;
- `contractBundleDigest`;
- participant identity;
- decision;
- admitted operation set;
- admitted bounds and budgets;
- admitted capabilities;
- obstruction or rejection taxonomy for non-accept decisions;
- policy epoch.

The body is hashed to `AdmissionReceiptBodyDigest`. A DSSE envelope signs that
digest and is carried by the distribution envelope, **not** by the body. The
body contains no signature-envelope reference.

Receipts are participant-owned evidence. Multiple participants may issue
receipts for the same contract bundle digest.

## Admission Explanation

An admission explanation is the policy-epoch-specific counterpart to the
bundle's participant-neutral compile explanation. It references the contract
bundle digest and the `AdmissionReceiptBodyDigest`, records the participant
policy epoch and admitted ceilings, and lives **outside** the contract bundle.
It must never be hashed into the participant-neutral bundle
(`CONTINUUM-BUNDLE-DAG-001`).

## Admission Evidence Is External

A contract bundle digest is computed before admission. Admission requests and
receipts reference the contract bundle digest but are not components of that
digest. A distribution envelope may aggregate bundles, attestations, and receipts
without changing the identity of the enclosed contract bundle.

## Capability Receipts

Opaque handles are participant/app capability receipts, not ambient authority.
A receipt should include:

- handle id;
- issuer bundle hash;
- participant identity;
- scope;
- basis coordinate, such as worldline/head for jedit;
- capability kind;
- admitted bounds;
- revocation policy;
- expiry policy;
- participant policy epoch.

Expiry must use explicit participant epochs or supplied provenance, not ambient
wall-clock time.

## Participant Policy

Participant policy may accept, reject, or lower admitted runtime ceilings. It may
not reinterpret Edict Core types or fill in missing Core semantic bounds during
Core compilation. Missing semantic bounds are compiler errors or locked-bundle
production errors, not participant-specific language meaning.

## Policy Replay Defense

Admission evidence must bind:

- participant descriptor digest;
- catalog snapshot digest;
- admission policy digest;
- policy epoch or monotonic policy version;
- admitted target profile digests;
- admitted lawpack digests;
- admitted bundle digest.

Replay under a newer participant policy is detectable because the request and
receipt remain bound to their original policy epoch and digest.
