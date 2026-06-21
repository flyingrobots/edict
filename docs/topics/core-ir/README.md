# Core IR Topic

Status: current HEAD contract.

This chapter describes the Core IR contract that exists today. It began as the
`v0.2.0-alpha.1` semantic model and schema contract; the compiler-spine topic
now owns the initial source-to-in-memory-Core lowerer. This shelf is still not a
canonical encoding or hashing claim.

## Public Surface

The machine-readable Core schema is [`docs/abi/edict-core.cddl`](../../abi/edict-core.cddl).
Its top-level artifact is `core-module`, and every Core module declares
`apiVersion: "edict.core/v1"`. [COREIR-REQ-001] [COREIR-REQ-008]

Core is downstream of the source AST. The source parser still returns source
AST, not Core; the compiler-spine shelf owns the initial executable lowering
from source AST to in-memory Core. The Core contract does not freeze canonical
encodings, golden bytes, exact Core digests, target IR, or admission bundles.
[COREIR-REQ-007]

## Current Contract

- Core modules carry imports, type definitions, intents, and required Core
  capabilities. Imports are digest-locked `resource-ref` values, but the Core
  module does not contain its own self-hash field. [COREIR-REQ-001]
  [COREIR-REQ-007]
- Core types cover bounded scalars, records, variants, options, lists, maps, and
  capability references. Runtime-sized collections remain explicitly bounded at
  the Core schema boundary. [COREIR-REQ-002]
- Core expressions and predicates are separate schema families. Expressions
  compute values; predicates express boolean obligations and input constraints.
  [COREIR-REQ-003]
- Core blocks contain explicit locals, ordered nodes, and a result expression.
  Nodes cover local binding, semantic effects, guards, branches, bounded loops,
  match blocks, and proof obligations. [COREIR-REQ-004]
- Local references are alpha-stable: each `local-ref` carries a compiler-owned
  `id`, normalized `alphaName`, and type reference. Source binder spelling is
  not identity. [COREIR-REQ-005]
- Input constraints are explicit `input-constraint` records containing a source
  coordinate, origin class, and predicate tree. They are not validator coordinate
  strings. [COREIR-REQ-006] [EDICT-CORE-WHERE-HASH-001]
- Intents state the required operation profile as `requiredOperationProfile`.
  Verifier reports and target/admission decisions are external to Core.
  [COREIR-REQ-009] [EDICT-CORE-VERIFIED-EXTERNAL-001]
- Edict-authored lawpack pure helper bodies use `core-fn-body`, a pure function
  body shape. They do not reuse the effect-capable `core-block` node algebra.
  [COREIR-REQ-011]
- Schema-shape fixtures prove minimal accepted Core module/intent shapes and
  rejected missing or external-evidence fields against the CDDL declarations.
  [COREIR-REQ-010]

## Deferred

The following are not implemented by the Core IR contract:

- import resolution and resolved type checking;
- full source-to-Core language coverage;
- full CDDL instance validation;
- canonical encoder implementation;
- golden Core bytes and exact digest fixtures;
- target-profile lowering;
- bundle/admission artifacts.

Those items belong to the compiler-spine, lowerability, and admission milestones
tracked in the roadmap.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
