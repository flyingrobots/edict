---
title: "GUIDE - Edict Assurance and Transparency"
legend: "GUIDE|TRANSMUTE|PLATFORM"
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

# GUIDE - Edict Assurance and Transparency

<!-- markdownlint-enable MD025 -->

## Purpose

This guide describes platform assurance artifacts around Edict and Continuum:
HOLMES, Watson, Moriarty, transparency logs, profile diff, and bundle nutrition
labels. These are platform milestones, not prerequisites for the Edict Language
v1 parser.

## Explanation Artifacts

The explanation artifact is split so a participant-neutral, pre-admission bundle
never contains policy-epoch-specific data (`CONTINUUM-BUNDLE-DAG-001`).

### Compile Explanation (participant-neutral, bundle-bound)

```text
operation coordinate
operation mode (verified)
assurance lane
target profile
source artifact digest
Core IR digest
target IR digest
optic contract (opticKind, basis, boundaryKind, supportPolicy, lossDisposition)
reads and writes with bounds
core/target budget
obstruction taxonomy and mappings
runtime guards
determinism proof status
floating imports
native plugins and sandbox identity
```

The compile explanation digest is bound by the contract bundle.

### Admission Explanation (participant-specific, external)

```text
bundleSubject { kind: semantic | release, digest }
admission receipt body digest
participant identity
participant policy epoch
admitted operation set, bounds, capabilities
decision / obstruction taxonomy
```

The admission explanation references the bundle and a receipt; it lives outside
the bundle and is never hashed into it. Both artifacts are Watson and Moriarty
input. Neither is part of Edict Core.

## Moriarty Hash-Impact Matrix

Moriarty outcomes must be vectors because Edict intentionally has multiple
hashes. For each mutation, Moriarty records:

```text
parse: accepted | rejected
rawSourceDigest: same | changed | not-produced
CoreIrDigest: same | changed | not-produced
targetIrDigest: same | changed | not-produced
semanticBundleDigest: same | changed | not-produced
releaseBundleDigest: same | changed | not-produced
admissionReceipt(semantic): still-valid | invalidated | not-applicable
admissionReceipt(release): still-valid | invalidated | not-applicable
result: accept | reject
```

Example: a comment-only source change should produce (note the split makes the
outcome precise — semantics are untouched, only the release identity moves):

```text
parse: accepted
rawSourceDigest: changed
CoreIrDigest: same
targetIrDigest: same
semanticBundleDigest: same
releaseBundleDigest: changed
admissionReceipt(semantic): still-valid
admissionReceipt(release): invalidated
result: accept
```

There is no mysterious fourth result. Every mutation is explained as same hash,
changed hash, missing artifact, or reject for each layer.

## Relapse Zoo

The relapse fixture corpus should include:

- graph primitive in Core;
- ambient clock;
- randomness;
- host callback;
- unbounded closure;
- read-only hidden append;
- duplicate content-addressed create;
- target profile digest swap;
- JSON integer ambiguity;
- source path substitution;
- codename coordinate;
- cross-bundle invoke in v1;
- participant policy used as Core type bound.

## Profile Diff

`edict profile diff old new` should classify:

- hash compatibility;
- source compatibility;
- Core compatibility;
- Target IR compatibility;
- verifier compatibility;
- admission impact.

Profile diff is a platform tool and fixture source. It is not part of the Edict
Language parser.

## Transparency

Bundle, lawpack, target profile, admission request, and admission receipt digests
should be suitable for transparency-log publication. A transparency entry should
reference the artifact digest and may include inclusion and consistency proofs.

Transparency evidence lives in a distribution envelope or external attestation.
It does not change contract bundle identity.

## Hash Ladder

`edict explain --hash-ladder <artifact>` shows each rung of the artifact graph
and what changed at each rung and why. This operationalizes the Moriarty matrix
instead of leaving it as prose:

```text
Raw Source
  v
Source Facts
  v
Core IR
  v
Target IR
  v
Contract Bundle (semantic / release)
  v
Admission Request
  v
Admission Receipt
```

`edict explain --hash <artifact>` additionally shows the exact domain label, the
canonical value tree, the excluded fields, the encoded canonical-CBOR bytes, and
the resulting digest, so any digest is reproducible by inspection.

## Aperture Ledger

The Aperture Ledger is the derived verifier evidence that bridges Observer
Geometry to the compiler without forcing every concept into source syntax. For
each operation the verifier emits:

```text
basis
declared aperture
inferred aperture
footprint overlap findings
support carried
support lost
support blocked
support refuting
witness debt
degeneracy findings
```

These are derived evidence, not Core fields. The Core optic contract
(`opticKind`, `basis`, `boundaryKind`, `supportPolicy`, `lossDisposition`)
remains the normative surface; the ledger is the analyzed result. A lowering that
would silently erase support loss, degeneracy, footprint overlap, or witness
debt must instead record it here or reject (`EDICT-OPTIC-PRESERVE-001`).

## Lawfulness Certificate

A `LawfulnessCertificate` summarizes exactly which obligations were proven for a
bundle:

```text
operation coordinate
required operation profile -> verified operation mode
guard obligations proven
footprint inequalities proven (computed <= ceiling)
cost inequalities proven (core/target/admitted)
optic-preservation claims proven
profile predicates proven
```

It is derived evidence (HOLMES output), never runtime authority (I-014).

## Obstruction Coverage

`edict obstruction coverage` proves that every domain-mappable failure class of
every imported intrinsic an operation uses is exhaustively handled
(`EDICT-LANG-OBSTRUCT-EXHAUST-001`). An unhandled domain-mappable class is a
compile error, not a runtime surprise.

## Two-Lowerer Trial

A target profile (or a lawpack target adapter) is not considered stable until
**two independently written implementations** produce byte-identical Target IR
and verifier reports for the normative corpus
(`EDICT-CONFORMANCE-DIFFERENTIAL-001`). One lowerer passing its own fixtures is
confidence; two lowerers agreeing on bytes is a specification. The differential
conformance harness runs both against the corpus and byte-compares outputs.

## Assurance Roles

HOLMES assesses exact bundle evidence and emits the LawfulnessCertificate.

Watson explains compiler, verifier, admission, HOLMES, and Moriarty failures.

Moriarty mutates source, manifests, policies, and envelopes to test that every
change produces the expected hash-impact matrix or rejection, including injected
artifact-graph cycles (`CONTINUUM-BUNDLE-DAG-MORIARTY-001`).
