# Lowerability Test Plan

Status: current verification design for v1 lowerability checks.

## Scope

In scope:

- typed `LoweringRequirements` and `TargetProfileFacts` values;
- `check_lowerability` classification behavior;
- native support;
- exactly-one direct adapter support;
- unsupported missing support;
- unsupported chained/composite adapter claims;
- unsupported ambiguous direct adapter claims;
- explicit non-claim that lowerability checks produce Target IR or admission
  artifacts.

Out of scope:

- CLI commands;
- canonical-CBOR encode/decode helpers for `LoweringRequirements`;
- target-profile manifest file loading;
- Target IR generation;
- verifier reports;
- bundle/admission validation;
- v2 adapter-composition search.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| LOWER-REQ-001 | implemented | `LoweringRequirements` is a typed contract containing operation profile, semantic effects, write classes, guards, atomicity, postcondition support, obstruction coordinates, footprint obligations, cost obligations, and optic contract. | issue #5, docs/abi/edict-target-profile.cddl |
| LOWER-REQ-002 | implemented | `check_lowerability` classifies requirements as `Native` only when explicit target-profile facts directly support every requirement, semantic effect, and required per-effect guard. | issue #5 |
| LOWER-REQ-003 | implemented | `check_lowerability` classifies requirements as `Adapted` only when each non-native semantic effect is discharged by exactly one digest-locked direct adapter and all other requirements, including per-effect guards, are supported. | issue #5, EDICT-LAWPACK-ADAPTER-DIRECT-001 |
| LOWER-REQ-004 | implemented | Missing operation-profile or semantic-effect support returns `Unsupported` with stable failure kinds. | issue #5 |
| LOWER-REQ-005 | implemented | Undigested adapter references, chained/composite adapter claims, and ambiguous direct adapters return `Unsupported`; v1 does not perform adapter-chain search. | issue #5, EDICT-LAWPACK-ADAPTER-DIRECT-001 |
| LOWER-REQ-006 | implemented | Lowerability checks do not create Target IR, verifier reports, bundles, admission requests, or admission receipts. | ROADMAP.md |
| LOWER-REQ-007 | implemented | Per-effect guard requirements must be supported by the selected native intrinsic or direct adapter support fact. | docs/SPEC_edict-target-profile-abi-v1.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| crates/edict-syntax/tests/lowerability.rs | In-memory typed lowering requirements, target-profile facts, and direct-adapter facts. | The same requirements classify as native, adapted, or unsupported depending only on explicit target-profile facts. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LOWER-TP-001 | implemented | Golden path | LOWER-REQ-001, LOWER-REQ-002, LOWER-REQ-006 | Explicit native target facts classify as `LowerabilityStatus::Native`, return no failures, and select a native per-effect result without Target IR or admission artifacts. | native_target_facts_satisfy_lowering_requirements | crates/edict-syntax/tests/lowerability.rs | Tests structured values only. |
| LOWER-TP-002 | implemented | Golden path | LOWER-REQ-003, LOWER-REQ-006 | Removing native support and adding exactly one direct adapter classifies as `LowerabilityStatus::Adapted` and records the adapter coordinate. | one_direct_adapter_satisfies_v1_lowering_requirements | crates/edict-syntax/tests/lowerability.rs | Direct adapter, not chain search. |
| LOWER-TP-003 | implemented | Error handling | LOWER-REQ-004 | Removing native support without adding an adapter classifies as `Unsupported` with `MissingEffectSupport`. | missing_native_or_adapter_support_is_unsupported | crates/edict-syntax/tests/lowerability.rs | Stable failure kind, not diagnostic prose. |
| LOWER-TP-004 | implemented | Boundary guard | LOWER-REQ-005 | A direct adapter without a digest-lock classifies as `Unsupported` with `UndigestedAdapter`. | v1_rejects_floating_direct_adapter_claims | crates/edict-syntax/tests/lowerability.rs | Prevents floating lawpack adapter references. |
| LOWER-TP-005 | implemented | Boundary guard | LOWER-REQ-007 | A native effect support fact without the required per-effect guard classifies as `Unsupported` with `UnsupportedEffectGuard`. | native_effects_must_support_required_per_effect_guards | crates/edict-syntax/tests/lowerability.rs | Checks per-intrinsic guard support, not only global profile guard availability. |
| LOWER-TP-006 | implemented | Boundary guard | LOWER-REQ-005 | A direct adapter that emits unresolved semantic effects classifies as `Unsupported` with `ChainedAdapterUnsupported`. | v1_rejects_chained_adapter_claims | crates/edict-syntax/tests/lowerability.rs | Keeps v2 adapter composition out of v1. |
| LOWER-TP-007 | implemented | Boundary guard | LOWER-REQ-005 | Two direct adapters for the same semantic effect classify as `Unsupported` with `AmbiguousAdapter`. | v1_rejects_ambiguous_direct_adapters | crates/edict-syntax/tests/lowerability.rs | Exactly-one adapter rule. |

## Determinism Obligations

- Tests build requirements and profile facts from in-memory constants.
- Tests assert structured statuses and failure kinds.
- Tests do not inspect stdout, stderr, diagnostic prose, serialized bytes,
  filesystem ordering, network state, or wall-clock time.
- The contract graph is checked by `cargo xtask contract-check`.

## Open Gaps

- Canonical-CBOR encode/decode helpers for `edict.lowering-requirements/v1`.
- File-backed target-profile manifest loading and conformance fixtures.
- CLI explanation surface.
- Target IR and verifier report generation.
- v2 adapter-composition search.
