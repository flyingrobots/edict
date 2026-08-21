# Lawpack Authoring Test Plan

Status: planned contract for Edict issue #195.

## Requirements

| ID | Status | Requirement | Evidence |
| --- | --- | --- | --- |
| LAUTH-REQ-001 | implemented | A public, typed authoring boundary accepts application-owned lawpack semantics without requiring callers to construct `CanonicalValue`, canonical CBOR, or derived local-artifact digests. | LAUTH-TP-001 |
| LAUTH-REQ-002 | partial | Identical semantic inputs and exact dependency bundles produce byte-identical canonical artifacts and lowercase digest sidecars independent of caller working directory. | LAUTH-TP-001, LAUTH-TP-006 |
| LAUTH-REQ-003 | partial | Emitted manifest, exports, and adapter bytes pass the same public decoders and complete dependency-closure checks used by application builds before publication. | LAUTH-TP-002, LAUTH-TP-003 |
| LAUTH-REQ-004 | partial | Local resources receive Edict-derived identities; external resources and dependency edges require exact caller-authored digest pins that are corroborated against supplied bytes. | LAUTH-TP-003, LAUTH-TP-004 |
| LAUTH-REQ-005 | partial | Duplicate coordinates or output paths, namespace escapes, malformed pure Core, invalid adapters, incomplete or disconnected dependency closures, and digest substitution fail with stable structured authoring failures. | LAUTH-TP-003, LAUTH-TP-004 |
| LAUTH-REQ-006 | planned | The public CLI supports write and check-only lawpack builds with confined relative paths, bounded reads, stale-output detection, and failure-atomic publication. | LAUTH-TP-005, LAUTH-TP-006 |
| LAUTH-REQ-007 | planned | A consumer invoked from outside the Edict checkout can author, publish, check, and feed its exact lawpack closure into the public application-build boundary without invoking `xtask`. | LAUTH-TP-006 |

## Test Cases

| ID | Status | Behavior | Planned executable evidence |
| --- | --- | --- | --- |
| LAUTH-TP-001 | implemented | Minimal deterministic authoring | A typed minimal definition emits canonical manifest/exports pairs twice with identical bytes and digests; changing one exported semantic type moves exports and manifest identity. |
| LAUTH-TP-002 | implemented | Full export and adapter surface | Types, constants, component pure helpers, effects, obstructions, profiles, verifier metadata, adapter mappings, budgets, and local target configuration all round-trip through the existing decoders. Edict-authored pure bodies remain covered by LAUTH-TP-004. |
| LAUTH-TP-003 | planned | Exact dependency closure | Root dependencies are corroborated against supplied bundles; missing, substituted, cyclic, and disconnected bundles reject before artifacts are returned. |
| LAUTH-TP-004 | partial | Fail-closed authoring values | Invalid digest pins, unresolved local resources, and path escape return stable failure kinds. Remaining negative cases are still required. |
| LAUTH-TP-005 | planned | Atomic write and check | Publication replaces one owned output set atomically, removes stale owned artifacts, preserves the previous set on injected failure, and check-only mode reports drift without writes. |
| LAUTH-TP-006 | planned | External-directory CLI witness | The built `edict` binary runs with a temporary external working directory; repeated write/check runs are deterministic and the resulting closure is consumed by `edict application build`. |

## Oracles

- Canonical bytes are decoded by `decode_lawpack_bundle` and `decode_lawpack_adapter`; a second authoring run must match them byte for byte.
- Digest sidecars must equal `digest_canonical_artifact` under the artifact's owning domain and end in one newline.
- Negative tests assert `LawpackAuthoringFailureKind` or the public CLI diagnostic kind, not prose.
- Publication tests compare the complete previous output tree after failure rather than inspecting transaction internals.
