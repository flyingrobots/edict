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

## Nutrition Label

Every compiled operation should be able to emit a compact explanation artifact:

```text
operation coordinate
operation mode
assurance lane
target profile
source artifact digest
Core IR digest
target IR digest
reads and writes with bounds
budget
obstruction taxonomy and mappings
runtime guards
determinism proof status
floating imports
native plugins and sandbox identity
participant policy epoch, when admitted
```

This artifact is Watson and Moriarty input. It is not part of Edict Core.

## Moriarty Hash-Impact Matrix

Moriarty outcomes must be vectors because Edict intentionally has multiple
hashes. For each mutation, Moriarty records:

```text
parse: accepted | rejected
rawSourceDigest: same | changed | not-produced
CoreIrDigest: same | changed | not-produced
targetIrDigest: same | changed | not-produced
contractBundleDigest: same | changed | not-produced
admissionReceipt: still-valid | invalidated | not-applicable
result: accept | reject
```

Example: a comment-only source change should produce:

```text
parse: accepted
rawSourceDigest: changed
CoreIrDigest: same
targetIrDigest: same
contractBundleDigest: changed
admissionReceipt: invalidated
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

## Assurance Roles

HOLMES assesses exact bundle evidence.

Watson explains compiler, verifier, admission, HOLMES, and Moriarty failures.

Moriarty mutates source, manifests, policies, and envelopes to test that every
change produces the expected hash-impact matrix or rejection.
