# Lawpacks Topic

Status: current HEAD contract.

This shelf describes the lawpack boundary that exists today. A lawpack is an
external, digest-locked source of portable Edict semantics. Edict can parse
lawpack imports, carry lawpack references through bundle and lowerability
contracts, validate the direct declarative adapter ABI, load first compiler
context facts from authority-facts documents whose source kind is `lawpack`,
and decode exact canonical lawpack manifests and export surfaces into an opaque
validated bundle. Complete supplied dependency sets are checked for exact
digest binding, missing dependencies, and cycles before use.

Provider manifests can describe lawpacks as generated provider artifacts with
digest-locked semantic-source and generator provenance. This validates only the
provider envelope and provenance lock; it does not load or interpret the
lawpack manifest. [LAWPACKS-REQ-007]

## Public Surface

The source syntax accepts lawpack imports of the form:

```text
use lawpack hello.optics@1 digest "sha256:<64 lowercase hex>" as hello;
```

The parser preserves the import as a lawpack import with the package coordinate,
version label, alias, and digest review string. [LAWPACKS-REQ-001]

The machine-readable lawpack manifest and export surface are specified in
[`docs/abi/edict-lawpack.cddl`](../../abi/edict-lawpack.cddl), with explanatory
reference material in
[`docs/SPEC_edict-lawpack-abi-v1.md`](../../SPEC_edict-lawpack-abi-v1.md).
`decode_lawpack_bundle` enforces the closed canonical shape and local semantic
obligations. `validate_lawpack_dependency_graph` validates the complete supplied
dependency set independent of input order. [LAWPACKS-REQ-005]

The current executable Rust surfaces touching lawpacks are:

- parser support for `ImportKind::Lawpack`;
- canonical manifest/export loading through `ValidatedLawpackBundle`;
- complete dependency-set validation with exact manifest-digest edges;
- canonical direct-adapter loading with exact target selection, adapter digest
  corroboration, complete callable profile/effect/budget coverage, and
  request-only profiles whose exact budget and target configuration confer no
  callable effect authority;
- compiler and Target IR fact derivation from the exact
  module/lawpack/adapter closure;
- reproducible canonical Core and Target IR artifacts for the standalone Hello
  Echo crossing;
- reproducible request-only workspace-snapshot closure and public application
  build artifacts with one external request and zero callable Target IR steps;
- authority-facts loading for budget and effect write-class facts whose source
  identity is a digest-locked lawpack reference;
- target-profile validation for the exact `edict.lawpack-adapter/v1` ABI;
- lowerability checks for digest-locked, one-hop direct adapter support;
- contract-bundle manifest validation that can carry lawpack artifact
  references as participant-neutral resources.
- provider manifest validation that can carry a generated lawpack artifact
  reference and provenance without interpreting lawpack semantics.

## Current Contract

- Lawpack source imports require lexically valid digest review strings when a
  digest is present. Invalid digest strings reject at the parser boundary.
  [LAWPACKS-REQ-001]
- Canonical manifests and export surfaces reject non-canonical bytes, missing
  or unknown fields, malformed typed digests, substituted exports, invalid
  discriminants, duplicate identities, unmappable failure names, opaque Edict
  helper bodies, unbounded executable components, and runtime effects with no
  target-adapter descriptor. The successful wrapper exposes typed values but
  cannot be fabricated or mutated by callers. [LAWPACKS-REQ-005]
- Dependency validation resolves the complete supplied set by `(id, version)`,
  detects cycles independent of input ordering, then corroborates every edge
  against the exact resolved manifest digest. [LAWPACKS-REQ-005]
- v1 target profiles accept the exact `edict.lawpack-adapter/v1` identifier.
  Unknown and duplicate declarations reject. [LAWPACKS-REQ-003]
- `decode_lawpack_adapter` accepts only canonical adapter bytes selected by one
  exact digest-locked target descriptor. It requires exact operation-profile,
  runtime-effect, budget, footprint, cost, and named-failure coverage before
  returning an opaque validated adapter. Each callable effect carries one
  typed, digest-locked target-configuration reference. A profile with no
  semantic effects is request-only and must carry its own exact budget
  obligation and target configuration. Edict preserves those references but
  does not interpret their target-owned semantics.
  `prepare_lawpack_compilation` then derives compiler and Target IR facts
  through the source import's exact alias and manifest digest.
  [LAWPACKS-REQ-008]
- The Hello Echo golden generator compiles the exact source and lawpack closure,
  lowers the resulting Core module, and pins canonical Core and Target IR bytes
  under their native domain-framed identities. [LAWPACKS-REQ-009]
- The workspace-snapshot generator binds a requestable capability to the exact
  lawpack manifest, compiles one request through a request-only profile, and
  pins canonical Core and Target IR with zero callable steps.
  [LAWPACKS-REQ-011]
- Lowerability may classify an operation as adapted when exactly one
  digest-locked direct adapter satisfies the required semantic effect, write
  class, and guard facts. Floating, chained, or ambiguous adapter claims reject
  with stable failure kinds. [LAWPACKS-REQ-002]
- Contract bundles may reference lawpacks as digest-locked participant-neutral
  artifacts, but validation does not load, rehash, or execute lawpack manifests.
  [LAWPACKS-REQ-004]
- Authority-facts documents may identify a `lawpack` source and provide the
  first compiler facts consumed by `CompilerContext`, such as budgets and effect
  write classes. This is not full lawpack manifest validation.
  [LAWPACKS-REQ-006]
- Provider manifests may identify lawpacks as generated artifacts. The provider
  validator checks lowercase digest locks for the artifact, semantic source, and
  generator, and rejects component provenance for lawpack metadata roles.
  [LAWPACKS-REQ-007]

## Deferred

The following are not implemented:

- executable target-adapter component loading; v1 currently specifies and
  implements the direct declarative adapter class only;
- target-owned configuration interpretation;
- Echo admission or execution of compiler-emitted external-action requests;
- lawpack conformance fixtures and two-lowerer differential trials.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
