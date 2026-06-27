# Lawpacks Topic

Status: current HEAD contract.

This shelf describes the lawpack boundary that exists today. A lawpack is an
external, digest-locked source of portable Edict semantics. Edict can parse
lawpack imports, carry lawpack references through bundle and lowerability
contracts, reject unsupported v1 adapter claims, and load first compiler context
facts from authority-facts documents whose source kind is `lawpack`. It does not
yet validate full lawpack manifest instances or load complete lawpack export
surfaces.

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
Those files are current design/reference material, not an executable validator.
[LAWPACKS-REQ-005]

The current executable Rust surfaces touching lawpacks are:

- parser support for `ImportKind::Lawpack`;
- authority-facts loading for budget and effect write-class facts whose source
  identity is a digest-locked lawpack reference;
- target-profile validation that keeps the deferred
  `accepted_lawpack_adapter_abi` slot empty for v1;
- lowerability checks for digest-locked, one-hop direct adapter support;
- contract-bundle manifest validation that can carry lawpack artifact
  references as participant-neutral resources.

## Current Contract

- Lawpack source imports require lexically valid digest review strings when a
  digest is present. Invalid digest strings reject at the parser boundary.
  [LAWPACKS-REQ-001]
- v1 target profiles do not yet accept a lawpack adapter ABI declaration. The
  field exists for byte-level `edict.lawpack-adapter/v1` ABI ids once that ABI
  is specified, and non-empty values reject until then. [LAWPACKS-REQ-003]
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

## Deferred

The following are not implemented:

- full `edict.lawpack/v1` manifest file loading beyond authority-facts
  documents;
- full `edict.lawpack/v1` CDDL instance validation;
- export-surface validation for pure functions, semantic effects,
  obstructions, and operation profiles;
- dependency DAG validation;
- target adapter ABI validation;
- lawpack conformance fixtures and two-lowerer differential trials.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
