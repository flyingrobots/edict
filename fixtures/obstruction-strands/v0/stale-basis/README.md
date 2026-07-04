# Stale-Basis Obstruction Strand Fixture

Status: corridor manifest for issue #116. This fixture directory grows one
layer at a time as the obstruction-strand implementation advances.

## Purpose

This fixture records one stale-basis obstruction strand from source syntax
through later Core, Target IR, Echo receipt, and projection surfaces.

Each implementation PR may add only the artifact for the layer it owns:

| Artifact | Expected Layer | Authority |
| --- | --- | --- |
| `source.edict` | PR 1 | Edict parser |
| `source.parse.review` | PR 1 | Edict parser |
| `core.review` | PR 2 | Edict Core |
| `core.canonical.bytes` | PR 2, if canonical Core bytes change | Edict Core |
| `core.digest` | PR 2, if canonical Core bytes change | Edict Core |
| `echo-target-ir.review` | PR 3 | Edict target lowerer |
| `echo-target-ir.canonical.bytes` | PR 3 | Edict target lowerer |
| `echo-target-ir.digest` | PR 3 | Edict target lowerer |
| `echo-receipt.review` | PR 4 | Echo runtime |
| `echo-receipt.canonical.bytes` | Future, only if receipt bytes freeze | Echo runtime |
| `echo-receipt.digest` | Future, only if receipt bytes freeze | Echo runtime |

## Non-Claims

This manifest does not claim that Core obstruction semantics, Echo Target IR,
Echo execution receipts, canonical receipt bytes, Graft projection, or jedit UI
support exist before the corresponding layer lands.
