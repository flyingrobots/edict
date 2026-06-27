# Authority Fact Governance Design Note

Status: future design note. This is not a topic shelf contract for landed
behavior, and no trusted lawpack or target-profile authorship workflow is
implemented in HEAD.

Edict's file-backed fact model creates a trust surface. Once lawpacks and target
profiles can supply operation profiles, budgets, write classes, effect facts,
obligations, adapter facts, and target capabilities, the compiler can prove only
against those supplied facts. The hard question is who authored those facts, who
reviewed them, how revisions are audited, and why a participant should trust
them.

This design track does not block `v0.7.0-alpha.1` file-backed fact loading. It
starts there so the project can observe the exact fact classes, digest
boundaries, ambiguity points, and structured error kinds before turning the
workflow into executable validation in a later alpha.

## Boundary

Edict owns deterministic validation of fact provenance, artifact shape, digest
binding, review references, compatibility markers, and explainable structured
diagnostics.

Continuum and participants own trust policy, identity, delegation, revocation,
and final acceptance decisions.

In other words, Edict may prove:

```text
This lawpack was authored, reviewed, versioned, digest-bound, and evidence-linked
according to its declared process.
```

Edict must not decide:

```text
This participant accepts that author, reviewer, identity system, trust root, or
policy.
```

That boundary keeps Edict as the authority-fact validation layer rather than a
global governance or identity system.

## Fact Classes That Need Provenance

The initial governance design must account for these authored facts:

- operation profiles;
- profile write-class allowances;
- effect write classes;
- obstruction payloads;
- obligations;
- adapter facts;
- target capabilities;
- budget and footprint facts;
- compatibility declarations between lawpacks, target profiles, Core ABI
  versions, and adapter ABI versions.

Each fact class needs enough metadata to answer:

- who authored the claim;
- who reviewed it;
- which artifact digest the review covered;
- what evidence supports the claim;
- when the claim changed;
- whether the change altered authority, write behavior, obligations, or target
  assumptions.

## Open Questions

- Who is allowed to author a lawpack?
- Who reviews effect write-class claims?
- What evidence supports an effect classification?
- How are target-profile capability claims reviewed?
- How are lawpack and target-profile revisions recorded?
- How are write-class upgrades and downgrades detected and explained?
- Can multiple lawpacks claim the same effect coordinate?
- How are conflicting fact owners rejected or selected without ambient trust?
- How does a participant require specific provenance before admission?
- How are disputes, corrections, and withdrawn claims represented?
- Which provenance checks belong to Edict, and which acceptance decisions belong
  to Continuum or participants?

## Planned Alpha Shape

`v0.13.0-alpha.1` is the first planned executable release for this track. Its
theme is to make authority facts reviewable, not merely loadable.

Must ship:

- lawpack authoring manifest shape;
- target-profile authoring manifest shape;
- author, reviewer, and provenance fields;
- review digest binding;
- revision-history fields for fact changes;
- stable validation errors for missing author provenance;
- stable validation errors for missing reviewer provenance;
- stable validation errors for unsigned or digest-unbound review evidence;
- stable validation errors for fact changes without revision notes;
- stable validation errors for write-class changes without explicit review
  markers;
- stable validation errors for conflicting effect ownership;
- stable validation errors for stale lawpack and target-profile compatibility;
- fixture corpus for accepted reviewed lawpacks;
- fixture corpus for rejected unreviewed lawpacks;
- fixture corpus for rejected conflicting lawpacks;
- fixture corpus for accepted lawpack revisions with explicit review;
- fixture corpus for rejected silent write-class changes;
- CLI support for `edict lawpack check`;
- CLI support for `edict lawpack diff`;
- CLI support for `edict target-profile check`;
- CLI support for `edict authority explain`.

Optional, but valuable:

- an authority nutrition label for lawpacks and target profiles, including
  package coordinate, maintainer, reviewer, review epoch, effects classified,
  write classes, target assumptions, evidence references, and digest.

Non-goals:

- no global registry;
- no public trust root;
- no legal identity model;
- no distributed revocation;
- no Continuum participant policy;
- no HOLMES, Watson, or Moriarty implementation.

The alpha should not pretend to solve global trust. It should make trust claims
inspectable, structured, and digest-bound.

## v1 Implication

The v1 end-to-end slice must not merely prove that source can compile through
facts into a bundle. It must prove the route through trusted file-backed facts.

The v1 target becomes:

```text
one small intent goes source -> trusted file-backed facts -> Core -> target IR
-> bundle -> admission evidence, with lawpack and target-profile provenance
visible and digest-bound
```

That means v1 must fail loudly for untrusted, ambiguous, stale, or conflicting
authority facts. Without that requirement, Edict would only prove claims made by
arbitrary input files.
