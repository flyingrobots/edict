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
bundle. Admission requests and receipts reference a `bundleSubject`
(`{ kind: semantic | release, digest }`); they are not components of either
bundle digest (`CONTINUUM-BUNDLE-SUBJECT-001`).

## Ownership Boundary

Edict owns the artifact and operation semantics carried by admission artifacts:

- exact `bundleSubject { kind: semantic | release, digest }` semantics;
- semantic versus release bundle digest selection;
- operation coordinate meaning inside a bundle;
- basis, canonical variables digest, and instantiated operation requirements;
- bundle-declared footprint, aperture, and budget ceilings;
- hidden execution input rejection below the determinism boundary;
- compiler, lowering, verifier, and runtime-effect failure taxonomy.

Continuum owns participant protocol semantics:

- participant, principal, host, agent, and role vocabulary;
- participant policy, capability delegation, revocation, and policy epochs;
- effective-authority evaluation;
- admission, activation, invocation, and receipt lifecycle;
- participant descriptor and catalog snapshot interpretation.

Admission artifacts may carry Edict-owned fields, but Continuum policy does not
define Edict language meaning and Edict does not grant participant authority.

## Admission Request

An admission request contains:

- `bundleSubject` (`{ kind: semantic | release, digest }`) — the participant
  declares which bundle identity it is admitting (`CONTINUUM-BUNDLE-SUBJECT-001`);
- participant descriptor digest;
- catalog snapshot digest;
- admission policy digest;
- policy epoch or monotonic policy version;
- requested operation set;
- operation requirement references derived from the admitted `bundleSubject`,
  operation coordinate, basis, canonical variables digest, and instantiated
  requirements digest;
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
- `bundleSubject` — echoes the request's exact `{ kind, digest }`;
- participant identity;
- decision;
- admitted operation set;
- admitted bounds and budgets;
- admitted capabilities;
- obstruction or rejection taxonomy for non-accept decisions;
- policy epoch.

The admitted operation set must be a subset of the request's operation set.
Accepted receipts carry no obstruction or rejection taxonomy.

The body is hashed to `AdmissionReceiptBodyDigest`. A DSSE envelope signs that
digest and is carried by the distribution envelope, **not** by the body. The
body contains no signature-envelope reference.

Receipts are participant-owned evidence. Multiple participants may issue
receipts for the same `bundleSubject`.

## Admission Explanation

An admission explanation is the policy-epoch-specific counterpart to the
bundle's participant-neutral compile explanation. It references the admitted
`bundleSubject` (`{ kind, digest }`, echoing the request/receipt) and the
`AdmissionReceiptBodyDigest`, records the participant policy epoch and admitted
ceilings, and lives **outside** the contract bundle. Carrying the full
`bundleSubject` — not a singular digest — is required so replay and hash-impact
tooling can tell which bundle identity (semantic or release) the receipt
explains (`CONTINUUM-BUNDLE-SUBJECT-001`). It must never be hashed into the
participant-neutral bundle (`CONTINUUM-BUNDLE-DAG-001`).

## Admission Evidence Is External

Both the `semanticBundleDigest` and `releaseBundleDigest` are computed before
admission. Admission requests and receipts reference a `bundleSubject` but are
not components of either bundle digest. A distribution envelope may aggregate
bundles, attestations, and receipts without changing the identity of the
enclosed contract bundle.

## Capability Receipts

Opaque handles are participant/app capability receipts, not ambient authority.
A receipt should include:

- handle id;
- issuer `bundleSubject` (`{ kind: semantic | release, digest }`, the same
  shape as admission requests/receipts, so tooling can tell which bundle
  identity issued the capability) (`CONTINUUM-BUNDLE-SUBJECT-001`);
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

Registration evidence is not invocation authority. A runtime invocation requires
an accepted admission receipt plus a matching invocation capability receipt for
the selected bundle subject, operation coordinate, participant, and policy
epoch.

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
- the admitted `bundleSubject` (kind + digest).

Replay under a newer participant policy is detectable because the request and
receipt remain bound to their original policy epoch and digest.
