# Canonical Target IR v0.11

## Scope

This design record freezes canonical Target IR artifact bytes and digests for
the current `TargetIrArtifact` envelope emitted by Edict's Echo and git-warp
target-lowering slices.

In scope:

- canonical value shape for the current `TargetIrArtifact` envelope;
- canonical CBOR bytes for `echo.span-ir/v1` and
  `gitwarp.commit-reducer-ir/v1` artifacts;
- domain-separated SHA-256 digest function for Target IR artifacts;
- reviewed Echo and git-warp byte/digest golden fixtures;
- `xtask target-ir-goldens --check` and `--write`;
- bundle assembly integration from a computed Target IR artifact digest after
  the Target IR digest exists.

## Non-Claims

This slice does not implement or claim:

- Echo runtime execution;
- Echo verifier completeness or verifier reports;
- git-warp runtime execution, commit object creation, or CRDT reducer
  verification;
- admission execution;
- general target-lowering plugin dispatch;
- additional target profiles beyond Echo and git-warp;
- canonical `ContractBundleManifest` bytes.

## Canonical Value Shape

The digest domain is the Edict Target IR artifact envelope:

```text
edict.target-ir.artifact/v1
```

The artifact's own `domain` field remains inside the canonical value, so
`echo.span-ir/v1` and `gitwarp.commit-reducer-ir/v1` artifacts are separated by
value as well as by their content.

The canonical value is an intentional map, not a direct Rust struct
serialization:

```text
{
  "kind": "targetIrArtifact",
  "domain": <artifact domain>,
  "targetProfile": { "id": <coordinate>, "digest": ["sha256", <32 bytes>] },
  "sourceCoreCoordinate": <Core coordinate>,
  "intents": {
    <intent name>: {
      "operationProfile": <operation profile>,
      "inputConstraints": [<canonical Core input constraints>],
      "coreEvaluationBudget": <canonical Core budget>,
      "steps": [
        {
          "id": <step id>,
          "binding": <canonical Core local ref>,
          "effect": <Core effect coordinate>,
          "targetIntrinsic": <selected target intrinsic>,
          "input": <canonical Core expression>,
          "obstructionFailures": [<failure key>, ...],
          "obstructionArms": {
            <failure key>: <canonical Core obstruction arm>
          }
        }
      ],
      "result": <canonical Core expression>
    }
  }
}
```

Target profile references must be digest locked before canonicalization. The
digest review string must be exactly `sha256:` followed by 64 lowercase
hexadecimal characters.

## Ordering

- Intent maps are sorted by intent name.
- Obstruction arm maps are sorted by failure key.
- Input constraints are sorted by canonical value because they are semantic
  constraints, not an execution order.
- Obstruction failure key lists are sorted and deduplicated by text.
- Target IR step lists preserve semantic Core execution order.
- No filesystem order, ambient process state, debug formatting, or prose
  serialization contributes to the bytes.

## Digest Frame

The reviewed digest is SHA-256 over canonical CBOR for:

```text
["edict.digest/v1", "edict.target-ir.artifact/v1", <canonical Target IR value>]
```

Review strings are parsed into typed digest values before they enter the
canonical value. The digest never hashes a review string directly.
