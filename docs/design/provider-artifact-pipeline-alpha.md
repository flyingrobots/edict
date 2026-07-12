# Provider Artifact Pipeline Alpha

Status: current design and implemented boundary for issues #139, #140, and #148
under goalpost #138.

## Scope

This design records the intended first Edict provider boundary: a provider
package is an assembled set of generated artifacts plus provider-owned
executable components. The current slices validate a typed manifest envelope and
invoke explicitly selected in-tree lowerers through a compatibility adapter.
Manifest-backed package resolution and consumption remain follow-on work.

The first manifest/provenance slice includes:

- provider manifest vocabulary;
- artifact roles;
- generated artifact provenance;
- component provenance for lowerers and verifiers;
- generic envelope validation;
- fixture-backed validation tests.

The built-in compatibility slice adds:

- explicit selection of the existing in-tree Echo or git-warp lowerer;
- a borrowed request over an already-built Core module and target-lowering
  facts;
- a structured target-profile compatibility failure before invocation;
- unchanged passthrough of the existing target-lowering report; and
- executable parity evidence for Target IR values, canonical bytes, digests,
  and bundle identities.

The WIT envelope slice adds:

- the distinct `edict:target-provider@1.0.0` package identity;
- explicit protocol versions and digest-bound opaque input artifacts;
- authority-separated lowerer and verifier output requests;
- digest-free provider outputs with optional logical paths;
- typed provider diagnostics and refusal;
- deterministic response limits; and
- parser-backed structural contract evidence.

Out of scope:

- Echo-specific lawpack semantics;
- Wesley execution or lawpack generation inside Edict;
- WIT component loading;
- manifest-backed provider/lowerer resolution;
- external provider component dispatch;
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
                                                    \
provider-owned lowerer/verifier components --------> provider package
  -> Edict compiler consumption
```

For Echo, the intended downstream shape is:

```text
Echo semantic source
  -> Echo-Wesley generator
  -> generated Echo lawpack/profile/facts/manifest
                                                    \
edict-echo lowerer and verifier components --------> Echo provider package
```

In the intended external-provider flow, Edict sees only the resulting package
artifacts and their digest-locked provenance. Edict does not understand Wesley
internals and does not hardcode Echo law.

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

## Built-in Lowerer Compatibility

`BuiltinTargetLowerer` and `lower_with_builtin_lowerer` route the current
in-tree Echo and git-warp lowerers through an explicit in-process migration
seam. The adapter validates only that the selected lowerer serves the requested
target-profile coordinate, then delegates to `lower_to_target_ir` without
reclassifying target failures or adding provider identity to semantic artifacts.

This is not a complete target provider. It does not consume a
`TargetProviderManifest`, resolve lawpack or target-profile bytes, select a
manifest-declared component, load WIT/WASM, or invent a lowerer digest. The
explicit lowerer identity supplied to contract-bundle assembly remains the
release-identity authority.

Direct and compatibility paths are required to produce byte-identical canonical
Target IR and the same `edict.target-ir.artifact/v1` digest. With identical
bundle inputs they produce identical semantic and release bundle digests;
changing only lowerer identity changes the release digest only.

## External Provider WIT Envelope

The normative component transport is
`docs/abi/edict-target-provider.wit`. Its `lowerer` and `verifier` worlds expose
one request/result function each. The transport composes existing canonical
artifact domains rather than copying their semantic schemas into WIT.

Each request carries an explicit `{ major, minor, patch }` protocol version.
Inputs use `bound-artifact`, which pairs opaque `{ domain, bytes }` transport
with a digest-locked resource reference already resolved by the host. Core and
target profile are explicit. Additional lawpack, authority-fact,
lowerability-fact, and auxiliary artifacts carry deterministic roles. Requested
outputs declare the exact role, routing kind, and artifact domain. Separate
lowering and verification output types prevent either world from claiming the
other world's evidence roles.

Every role-keyed list has one spelling: semantic inputs, requested outputs, and
returned outputs contain non-empty unique roles sorted by ascending UTF-8
bytes. Success returns exactly one output for every request, matching its
`(role, kind, domain)`, and no undeclared output. Present logical paths are
unique by exact case-sensitive UTF-8 bytes. Diagnostics use the lexicographic
tuple fixed in WIT. Response limits constrain both success and refusal. The
total byte limit is a checked `u64` sum over every provider-authored byte list
and UTF-8 string in the selected result arm; overflow or any exceeded bound is a
future host-owned failure. Limits cannot alter the canonical result. If that
result fits two supplied limit sets, the entire selected result arm is
byte-identical.

Provider outputs deliberately omit digests. A component can return bytes and a
declared role plus an optional logical path, but only the future host may
validate canonicality and path safety, recompute the digest, bind the result to
the invocation's Core and semantic inputs, and build an authoritative output
manifest. Provider refusal is target-owned evidence; load errors, denied
imports, traps, resource exhaustion, malformed responses, binding failures, and
replay mismatches remain host-owned failures.

The earlier `edict:target-profile@1.0.0` WIT package shipped as an unhosted alpha
direction. This slice supersedes it with a new package identity because its
opaque two-argument context, single-artifact result, unused digest type, and
fine-grained RPC surface cannot satisfy the explicit provider-host boundary
without a breaking change.

## Follow-On Work

Issue #140 introduced the built-in compatibility seam described above. The
broader manifest-backed provider resolver remains future composition work.

Issue #141 tracks four native slices: #148 freezes the WIT envelope, #146 adds
pure envelope validation, #145 adds the capability-constrained component host,
and #147 adds deterministic replay and negative conformance.

Issue #142 creates the downstream Echo-owned provider implementation issues once
the Edict provider ABI is concrete enough to scope them accurately.
