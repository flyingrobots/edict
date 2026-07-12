# Providers Topic

Status: current HEAD contract.

This shelf describes the provider manifest boundary, built-in lowerer
compatibility seam, external component transport ABI, and pure invocation
envelope validator that exist today. A target provider package is an assembled
collection of generated artifacts plus provider-owned components. Edict can
validate provider manifests and invocation values, invoke the existing in-tree
Echo and git-warp lowerers through an explicit migration adapter, and parse the
frozen WIT contract. It cannot load or invoke manifest-declared components, run
verifiers, or interpret runtime-specific semantics.

## Current Contract

The `edict_syntax` crate exposes:

- `TargetProviderManifest`;
- `ProviderArtifactRef`;
- `ProviderArtifactKind`;
- `ProviderArtifactSource`;
- `validate_target_provider_manifest`;
- `ProviderManifestValidationStatus`;
- `ProviderManifestValidationFailureKind`;
- `BuiltinTargetLowerer`;
- `BuiltinLowererRequest`;
- `BuiltinLowererCompatibilityFailure`;
- `BuiltinLowererCompatibilityFailureKind`;
- `BuiltinLoweringResult`;
- `lower_with_builtin_lowerer`.

The `provider_invocation` module mirrors the WIT request and result values and
exposes:

- `ProviderLoweringInvocationContract` and
  `ProviderVerificationInvocationContract` for host-authored input bindings;
- `ProviderArtifactSchemaValidator` for an explicitly injected, deterministic
  owning-schema capability;
- `ProviderLoweringRequest` and `ProviderVerificationRequest` plus opaque
  validated request wrappers;
- `validate_provider_lowering_request` and
  `validate_provider_verification_request`;
- `validate_provider_lowering_result` and
  `validate_provider_verification_result`;
- pairwise lowerer and verifier limit-independence validators;
- structured validation reports and stable failure kinds; and
- `ValidatedProviderOutcome` plus `ProviderOutputManifest` for results that have
  crossed the complete validation boundary.

Provider manifests use API version `edict.provider-manifest/v1`.

The manifest validator checks that:

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

The compatibility adapter requires the caller to select either the Echo or
git-warp built-in lowerer explicitly and pass an already-built `CoreModule` plus
`TargetIrLoweringFacts`. A selected lowerer whose target-profile coordinate does
not match the facts rejects with `TargetProfileMismatch`. Once the profile
matches, the adapter returns the existing `TargetLoweringReport` unchanged,
including structured target refusal. Direct and adapter paths produce identical
Target IR artifacts, canonical bytes, and digests. The adapter does not consume
or resolve a `TargetProviderManifest`, invent component identity, or define the
external provider ABI. Its structured compatibility failure implements
`Display` and `std::error::Error` for standard Rust error propagation.

The external component contract is
[`edict:target-provider@1.0.0`](../../abi/edict-target-provider.wit). It replaces
the previously shipped but unhosted `edict:target-profile@1.0.0` WIT direction
with a distinct ABI identity rather than assigning incompatible meanings to the
old package version.

The `lowerer` and `verifier` worlds each expose one function over an explicit
versioned request. Every request carries the semantic protocol version. Inputs
are opaque canonical artifacts wrapped in digest-locked resource references.
Semantic inputs carry generic roles and kinds; requested outputs carry an
expected role, kind, and domain. Lowering and verification use distinct output
types so neither world can claim the other's evidence roles. Deterministic
response limits cross the ABI, while memory, fuel, and interruption remain
future host configuration. Component outputs carry role-tagged bytes and an
optional logical path, but no digest. The pure validator checks canonicality and
logical-path syntax and computes authoritative output identity before a future
host may expose it. Typed provider refusal remains distinct from future host
failures such as denied imports, traps, exhaustion, malformed envelopes, or
replay mismatch.

Semantic inputs, requested outputs, and returned outputs use non-empty, unique
roles in strict ascending UTF-8 byte order. A success matches every requested
`(role, kind, domain)` exactly once and returns no undeclared output. Present
logical paths are unique by exact case-sensitive UTF-8 bytes. Diagnostic lists
use the WIT-declared lexicographic order. Response limits apply to success and
refusal: the aggregate byte cap is the checked sum of every provider-authored
byte list and UTF-8 string in the selected result arm. Overflow or any exceeded
cap is a host-owned limit failure. Limits cannot change the canonical result;
when it fits two limit sets, the entire result arm is byte-identical.

## Pure Invocation Validation

The request validators accept only semantic protocol version `1.0.0`. A caller
must supply a complete host-authored invocation contract and a
`ProviderArtifactSchemaValidator` separately from the WIT-shaped request. That
trusted capability's contract requires deterministic, side-effect-free
validation over in-memory values.
Validation requires the request resource references, domains, semantic-input
closure, canonical bytes, and decoded values under their owning schemas to
reproduce that contract before it returns an opaque
`ValidatedProviderLoweringRequest` or
`ValidatedProviderVerificationRequest`. The wrapper privately retains the
schema validator for result validation.

Core, target-profile, lawpack, authority-fact, and verifier Target IR inputs use
the fixed domains `edict.core.module/v1`, `edict.target-profile/v1`,
`edict.lawpack/v1`, `edict.authority-facts/v1`, and
`edict.target-ir.artifact/v1`. The host-authored contract supplies the domains
for lowerability facts and auxiliary inputs. Artifact identity is SHA-256 over
the canonical-CBOR frame:

```text
["edict.digest/v1", "<artifact-domain>", <decoded canonical artifact>]
```

The shared canonical decoder rejects malformed or noncanonical CBOR and pins a
maximum nesting depth of 128. After decoding, the explicit host validator must
accept the value as an instance of the immutable schema named by its domain.
Unsupported domains and schema mismatches remain distinct structured host
failures. Edict does not implement Echo, lawpack, target-profile, generated
artifact, review, or verifier-report semantics inside this generic envelope
module; the host-supplied validator owns those schema checks. The trait contract
requires total, deterministic, side-effect-free, in-memory behavior, but this
generic boundary cannot enforce the effects of an arbitrary implementation.
Issue #145 requires executable evidence from the concrete host registry.

Result validation preserves the separate lowerer and verifier output
vocabularies. It enforces exact requested outputs, role and diagnostic order,
logical paths, canonical bytes, owning-schema compatibility, and the checked
success/refusal limits. Requested output domains are preflighted before a
validated request can authorize invocation. A valid provider refusal remains
target-owned refusal evidence rather than becoming a host failure. The pairwise
validators compare otherwise-identical requests with different sufficient
limits and reject a changed complete result.

A fully valid success yields a host-authored `ProviderOutputManifest`. It binds
the invocation kind and protocol version, validated inputs, requested outputs,
and host-computed output digests. Limits and diagnostics remain validation and
review data outside that authoritative manifest. An invalid success yields no
partial manifest, and a valid refusal yields no output manifest.

## Authority Boundary

Providers own runtime-specific semantics. The manifest validator checks the
provider envelope and digest-locked reference syntax without receiving artifact
bytes. The invocation validator receives opaque bytes and proves their
canonicality, declared domains, owning-schema compatibility through an explicit
trusted host validator, digest bindings, ordering, and envelope contracts. It
does not prove the runtime-specific semantic meaning or correctness of those
bytes. The built-in compatibility adapter delegates to the existing target
lowerers without adding semantic interpretation.

For example, an Echo provider may supply an Echo lawpack, Echo target profile,
Echo authority facts, Echo lowerer, and Echo verifier. Edict can validate that
those artifacts are explicitly identified and digest-locked. Edict does not
decide whether Echo graph DPO semantics are correct.

## Fixtures

The provider manifest fixture is:

```text
fixtures/providers/echo-generated/provider-manifest.json
```

The fixture uses Echo-shaped coordinates to exercise the provider envelope. It
is not an Echo semantics claim and does not imply Echo provider loading.

Invocation validation reuses the reviewed canonical artifacts:

```text
fixtures/core/canonical/bounded-hello.core.cbor
fixtures/target-ir/canonical/echo-effectful.target-ir.cbor
```

`crates/edict-syntax/tests/provider_invocation.rs` binds those bytes into typed
lowering and verification requests, then exercises accepted envelopes and
structured negative mutations without loading a component.

## Deferred

The following are not implemented:

- manifest-backed provider resolution, file discovery, or package loading;
- a concrete production artifact-schema registry (issue #145 now requires
  digest-locked schema provenance, immutable domain bindings, and executable
  determinism and schema-instance evidence before the host injects it);
- WIT component loading;
- external provider component dispatch;
- provider verifier execution;
- lawpack generation through Wesley;
- Echo-owned provider implementation;
- runtime execution;
- admission or registration.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
