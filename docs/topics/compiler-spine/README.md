# Compiler Spine Topic

Status: current HEAD contract.

This chapter describes the executable compiler-spine stages that exist today.
The spine is the path from parsed source AST to in-memory Core IR. The lowerer
does not embed canonical bytes or hashes into Core modules, and it is not a hash
freezer, target lowerer, or admission tool.

## Public Surface

The public compiler-spine surface lives in `edict_syntax`:

- `validate_surface` checks context-free source-AST invariants.
- `resolve_module` resolves source names that can be resolved from the module
  plus explicit compiler context facts. [CSPINE-REQ-001]
- `type_check` builds a typed module boundary distinct from source AST.
  [CSPINE-REQ-002]
- `lower_core` lowers the typed initial subset to in-memory Core IR.
  [CSPINE-REQ-003]
- `compile_to_core` runs the full executable path:
  `validate_surface -> resolve_module -> type_check -> lower_core`.
  [CSPINE-REQ-004]

`CompilerContext` is intentionally explicit. Source clauses such as
`profile hello.readOnly` and `budget <= hello.tinyBudget` do not magically
become Core facts; the caller must supply deterministic profile and budget facts
before the resolver can produce Core-ready metadata. [CSPINE-REQ-005]
The caller must also supply deterministic write-class facts for operation
profiles and imported effect calls before the compiler can check profile/effect
compatibility. [CSPINE-REQ-009]

## Current Contract

- The initial lowerable subset is deliberately narrow: local record type
  declarations, one-parameter intents, `profile`, `basis none`, `budget <=`,
  `where` predicates, pure `let` bindings, `return`, strings, booleans,
  integers, field access, record literals, equality predicates, and string
  concatenation. [CSPINE-REQ-006]
- Core lowering produces structured in-memory `CoreModule` values with module
  coordinate, imports, types, intents, input constraints, budgets, locals,
  ordered nodes, and result expressions. [CSPINE-REQ-003]
- Resolver/type-checker failures use stable `CompilerErrorKind` and
  `CompilerStage` values. Tests assert those structured values rather than
  diagnostic prose. [CSPINE-REQ-007]
- Effectful source bodies are checked against the resolved operation profile's
  allowed write classes before Core lowering. A write-class effect under a
  read-only profile rejects with `ProfileEffectMismatch`. [CSPINE-REQ-009]
- The lowerer output carries no embedded canonical bytes, exact digest, target
  IR, or admission fields. Canonical encoding is a separate Core IR surface, and
  reviewed golden bytes and exact digests are separate Core IR artifacts.
  [CSPINE-REQ-008]

## Deferred

The following are not implemented by this compiler-spine slice:

- target-profile lowering;
- obstruction exhaustiveness against target/lawpack failure facts;
- shape/lawpack schema loading;
- target/lawpack file loading for operation-profile or effect facts;
- full source language lowering.

Those items remain assigned to later lowerability/admission milestones.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
