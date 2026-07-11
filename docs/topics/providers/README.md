# Providers Topic

Status: current HEAD contract.

This shelf describes the provider manifest boundary that exists today. A target
provider package is an assembled collection of generated artifacts plus
provider-owned components. Edict can validate the generic provider manifest
envelope and provenance locks, but it does not load providers, execute lowerers,
run verifiers, or interpret runtime-specific semantics.

## Current Contract

The `edict_syntax` crate exposes:

- `TargetProviderManifest`;
- `ProviderArtifactRef`;
- `ProviderArtifactKind`;
- `ProviderArtifactSource`;
- `validate_target_provider_manifest`;
- `ProviderManifestValidationStatus`;
- `ProviderManifestValidationFailureKind`.

Provider manifests use API version `edict.provider-manifest/v1`.

The validator checks that:

- the manifest API version is current;
- the provider reference is digest-locked;
- at least one artifact or component is present;
- artifact role slots are non-empty and unique;
- every artifact resource is digest-locked;
- generated metadata artifacts carry digest-locked semantic-source and
  generator provenance;
- lowerer and verifier artifacts carry digest-locked component provenance;
- generated metadata roles are not represented as executable components; and
- component roles are not represented as generated metadata.

Digest review strings on this boundary are strict artifact references:
`sha256:<64 lowercase hex>`.

## Authority Boundary

Providers own runtime-specific semantics. Edict validates the provider package
envelope and digest bindings only.

For example, an Echo provider may supply an Echo lawpack, Echo target profile,
Echo authority facts, Echo lowerer, and Echo verifier. Edict can validate that
those artifacts are explicitly identified and digest-locked. Edict does not
decide whether Echo graph DPO semantics are correct.

## Fixture

The current fixture is:

```text
fixtures/providers/echo-generated/provider-manifest.json
```

The fixture uses Echo-shaped coordinates to exercise the provider envelope. It
is not an Echo semantics claim and does not imply Echo provider loading.

## Deferred

The following are not implemented:

- provider file discovery or package loading;
- WIT component loading;
- provider-shaped target lowering;
- provider verifier execution;
- lawpack generation through Wesley;
- Echo-owned provider implementation;
- runtime execution;
- admission or registration.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
