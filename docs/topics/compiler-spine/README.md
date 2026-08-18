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
Those first compiler context facts may be supplied with builder methods or by
loading explicit authority-facts files through
`load_compiler_context_from_authority_fact_files`. Canonical
`edict.authority-facts/v1` bytes decode to the same validated document model and
enter the same `compiler_context_from_authority_facts` path. [CSPINE-REQ-010]

## Current Contract

- The lowerable subset is deliberately narrow: local record type declarations,
  one-parameter intents, `profile`, `basis none` or one input-derived explicit
  basis, `budget <=`, `where` predicates, pure `let` bindings, one annotated
  effectful `let ... else` shape, lowerable `require ... else` obstruction
  arms, `return`, bounded strings and bytes, booleans, fixed-width integers,
  field access, record literals, equality predicates, string concatenation, and
  pure conditional expressions whose branches have compatible bounded types.
  Statement conditionals lower to isolated branch blocks, and literal- or
  coordinate-bounded loops lower over bounded lists when the resolved cap
  covers the list maximum without exceeding the operation step budget.
  [CSPINE-REQ-006] [CSPINE-REQ-011] [CSPINE-REQ-017] [CSPINE-REQ-023]
  [CSPINE-REQ-025] [CSPINE-REQ-026]
- The fixed-width source scalar set is `I32`, `I64`, `U32`, and `U64`.
  Explicitly suffixed literals retain their exact width and signedness;
  bare literals inherit an unambiguous expected width from supported comparison,
  annotation, and record-return contexts. Unconstrained bare literals, overflow,
  negative unsigned values, and cross-width assignments reject in type checking;
  signed minima are accepted through unary-negative literal folding. Statically
  bounded `Bytes<max=N>` lowers with its exact bound.
  [CSPINE-REQ-019] [CSPINE-REQ-021]
- An explicit basis expression is checked in the pure pre-body environment
  containing the intent parameter, before body locals exist. The typed
  expression is preserved in Core; this is authoring evidence, not runtime
  basis resolution or admission. [CSPINE-REQ-020]
- Core lowering produces structured in-memory `CoreModule` values with module
  coordinate, imports, types, intents, input constraints, budgets, locals,
  ordered nodes, and result expressions. [CSPINE-REQ-003]
- Resolver/type-checker failures use stable `CompilerErrorKind` and
  `CompilerStage` values. Tests assert those structured values rather than
  diagnostic prose. [CSPINE-REQ-007]
- Effectful source bodies are checked against the resolved operation profile's
  allowed write classes before Core lowering. A write-class effect under a
  read-only profile rejects with `ProfileEffectMismatch`. [CSPINE-REQ-009]
- The first lowerable effectful body shape is an annotated
  `let name: Type = effect(arg) else { failure(binder) => Obstruction };`
  where `effect` is an untyped plain dotted callee. It lowers to a semantic
  Core effect node with the effect coordinate, input expression, result binding,
  and deterministic obstruction map. [CSPINE-REQ-011] [CSPINE-REQ-014]
  [CSPINE-REQ-015]
- Effectful branch-yield and other unsupported effectful forms still reject
  with stable compiler stage and kind identities before Core lowering.
  [CSPINE-REQ-012]
- Duplicate failure keys in an obstruction map reject with
  `DuplicateObstructionFailure` before Core lowering. [CSPINE-REQ-013]
- Lowerable `require ... else <obstruction>` statements lower to Core
  terminal require-failure arms, and
  `require ... else continue obstructed { reason: ... }` lowers to a preserved
  obstruction require-failure arm. Duplicate preserved-obstruction payload
  fields reject with `DuplicateObstructionPayloadField` before Core digesting.
  [CSPINE-REQ-017] [CSPINE-REQ-018]
- File-backed authority facts can supply the same profile, budget, profile
  write-class, and effect write-class facts consumed by the compiler spine.
  [CSPINE-REQ-010]
- Pure-helper calls resolve only from explicit compiler facts. The exact
  lawpack preparation path derives those facts from the source import alias and
  validated export signature, while Core records the canonical exported
  coordinate. Missing helpers and incompatible arguments reject before Core;
  the imported lawpack digest remains the helper implementation identity.
  [CSPINE-REQ-024]
- Coordinate loop bounds resolve only from explicit compiler facts. Exact
  lawpack preparation projects exported `U32` and `U64` constants through the
  source alias, uses their numeric values for static soundness and budget
  checks, and preserves their canonical exported coordinates in Core.
  [CSPINE-REQ-027]
- The lowerer output carries no embedded canonical bytes, exact digest, target
  IR, or admission fields. Canonical encoding is a separate Core IR surface, and
  reviewed golden bytes and exact digests are separate Core IR artifacts.
  [CSPINE-REQ-008]

## Deferred

The following are not implemented by this compiler-spine slice:

- target-profile lowering;
- obstruction exhaustiveness against target/lawpack failure facts;
- effect obstruction payload lowering;
- bare effect-statement lowering;
- effectful branch-yield lowering;
- shape/lawpack schema loading;
- full lawpack or target-profile manifest loading beyond authority-facts
  documents;
- full source language lowering.
- pure-helper cost-template accumulation.

Those items remain assigned to later lowerability/admission milestones.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
