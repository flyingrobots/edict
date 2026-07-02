# Schema As Source Of Truth Codegen

Status: deferred decision for the audit-remediation branch.

## Decision

Do not add a schema-as-source-of-truth code generator now.

The idea remains plausible, but the current repository does not yet have enough
cross-language consumers or fixture-authoring pain to justify a new generator
stack. The immediate risk would be adding a second meta-language before Edict's
own artifact boundaries finish stabilizing.

If this idea is reopened, the source of truth should be an Edict-owned schema or
canonical value model, not GraphQL semantics.

## Why This Is Not GraphQL

The useful mechanism to borrow is:

```text
one reviewed schema/value source -> generated artifacts -> checked fixtures
```

The mechanism is separate from GraphQL's runtime semantics, resolver model,
query language, and ecosystem assumptions. Edict was created because those
semantics were not the right authority boundary for this project. A future
codegen tool must not smuggle them back in through convenience.

## Reopen Criteria

Revisit this only when at least one of these becomes a repeated cost:

- Rust, TypeScript, Zod, JSON Schema, or Edict fixture types drift from each
  other in reviewed PRs.
- Fixture authoring becomes slow enough that generated accepted/rejected cases
  would remove real maintenance load.
- A downstream Echo, Continuum, or editor integration needs the same artifact
  shape in another language and cannot safely consume the Rust crate.
- A release gate starts depending on multiple manually synchronized schema
  artifacts for the same public contract.

## Required Shape If Implemented

A future implementation should have:

- one reviewed input model with explicit ownership;
- deterministic generation through `xtask`;
- checked-in generated artifacts only when reviewable;
- `--check` and `--write` modes;
- contract tests that fail on drift;
- clear generated-file headers;
- no canonical hash changes unless the owning canonical value model changes;
- no runtime or admission claims.

Generated outputs may include:

- Rust types;
- TypeScript types;
- Zod validators;
- JSON Schema artifacts;
- Edict source fixtures;
- accepted/rejected golden fixture cases.

## Current Rule

Manual schema and fixture maintenance remains the source of truth for this
branch. When drift is found, fix the owned artifact directly and add a targeted
test. Do not introduce a generator to avoid writing the contract down.
