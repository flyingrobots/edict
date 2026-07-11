# Provider Artifact Pipeline Alpha

Status: accepted design for issue #139 and goalpost #138.

## Scope

This design records the first Edict provider boundary: a provider package is an
assembled set of generated artifacts plus provider-owned executable components.
Edict consumes that package through explicit, digest-locked references.

In scope for this slice:

- provider manifest vocabulary;
- artifact roles;
- generated artifact provenance;
- component provenance for lowerers and verifiers;
- generic envelope validation;
- fixture-backed validation tests.

Out of scope:

- Echo-specific lawpack semantics;
- Wesley execution or lawpack generation inside Edict;
- WIT component loading;
- target lowering through providers;
- verifier execution;
- runtime execution;
- admission or registration.

## Doctrine

Lawpacks, target profiles, authority facts, provider manifests, review payloads,
and generated artifact profiles are provider artifacts. They are usually
generated from runtime-owned semantic sources by runtime-owned generators.

The lowerer and verifier are provider-owned components. They are not generated
metadata claims, and they must be identified as components in the provider
manifest.

Edict validates:

- manifest ABI version;
- provider identity digest lock;
- artifact identity digest locks;
- generated artifact semantic-source provenance;
- generated artifact generator provenance;
- component provenance for component roles;
- unique artifact role slots.

Edict does not validate:

- Echo graph semantics;
- git-warp reducer semantics;
- lawpack export meaning;
- target-profile runtime behavior;
- lowerer correctness;
- verifier correctness;
- runtime admission or execution.

Runtime-specific correctness remains provider-owned and must eventually be
expressed as verifier evidence that Edict can bind without interpreting.

## Provider Flow

```text
runtime-owned semantic source
  -> runtime-owned generator
  -> generated lawpack/profile/facts/provider artifacts
  -> provider package
  -> Edict compiler consumption
```

For Echo, the intended downstream shape is:

```text
Echo semantic source
  -> Echo-Wesley generator
  -> Echo lawpack
  -> Echo target profile
  -> Echo authority facts
  -> Echo provider manifest
  -> edict-echo lowerer and verifier components
```

Edict sees only the resulting provider package artifacts and their digest-locked
provenance. Edict does not understand Wesley internals and does not hardcode
Echo law.

## Manifest Shape

The initial typed manifest is:

```text
TargetProviderManifest {
  api_version,
  provider,
  artifacts[]
}
```

Each artifact has:

```text
ProviderArtifactRef {
  role,
  artifact_kind,
  resource,
  source
}
```

`role` is a unique package role slot. It can distinguish multiple lawpack or
target-profile artifacts without forcing Edict to understand their runtime
meaning.

`artifact_kind` is a generic routing category:

- `lawpack`;
- `targetProfile`;
- `authorityFacts`;
- `providerManifest`;
- `reviewArtifact`;
- `generatedArtifactProfile`;
- `lowerer`;
- `verifier`.

Generated metadata roles must use generated provenance:

```text
Generated {
  semantic_source,
  generator
}
```

Component roles must use component provenance:

```text
Component {
  component
}
```

All referenced resources must be digest-locked with lowercase
`sha256:<64 lowercase hex>` review strings.

## Current Fixture

The first fixture is
`fixtures/providers/echo-generated/provider-manifest.json`. It is deliberately
not an Echo semantics fixture. It is a provider-envelope fixture using
Echo-shaped coordinates to prove that lawpack, target-profile, authority-facts,
lowerer, and verifier roles can be represented without making Edict interpret
Echo.

## Follow-On Work

Issue #140 wraps current in-tree target lowerers behind provider-shaped APIs and
must prove direct/provider Target IR parity.

Issue #141 adds the WIT provider host alpha.

Issue #142 creates the downstream Echo-owned provider implementation issues once
the Edict provider ABI is concrete enough to scope them accurately.
