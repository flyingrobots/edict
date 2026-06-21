# Syntax Topic

Status: current HEAD contract.

This chapter describes what the Phase 1 Edict syntax front end does today. It is
not a proposal document. Future syntax and semantic behavior live in
[test-plan.md](./test-plan.md) as planned or open gaps until the code lands.

## Public Surface

The parser crate is `edict-syntax`. Its public boundary is:

```rust
use edict_syntax::{parse_module, ParseErrorKind};

let module = parse_module(
    "package examples.hello@1;\n\
     type HelloInput = { name: String<max=256>, };\n",
)
.expect("valid module parses");

assert_eq!(module.package.version, "1");
assert_eq!(module.decls.len(), 1);

let err = parse_module("package a.b@1;\nuse lawpack x.y@1 as return;")
    .expect_err("reserved import aliases reject");
assert_eq!(err.kind, ParseErrorKind::ReservedKeyword);
```

`parse_module` returns a source AST. It preserves source order and surface
spelling needed by later lowering. It does not resolve imports, type-check
programs, prove bounds, or lower to Core IR. [SYNTAX-REQ-001]

## Current Contract

- A module starts with one `package` declaration, then zero or more imports, then
  declarations. Package and import versions preserve source-significant version
  spelling, including `_beta` style labels. [SYNTAX-REQ-002]
- Supported imports are `shape`, `lawpack`, `target`, and `core`; `capability`
  import syntax is rejected in minimal-v1. Digest clauses accept only
  `sha256:` plus 64 hex characters. [SYNTAX-REQ-003]
- Type declarations parse record types, refined `String`, max-only `Bytes`,
  `Option`, `CapabilityRef`, bounded `List`, bounded `Map`, enum declarations,
  and payload-carrying `variant` types. Empty enums reject. [SYNTAX-REQ-004]
- Integer literal suffixes are preserved where syntax carries them, including
  static bounds. [SYNTAX-REQ-005]
- Intent declarations parse parameters, return type, clause surface, statement
  blocks, and expression bodies. Clause requiredness is semantic validation, not
  parser validation. [SYNTAX-REQ-006]
- Statements parse `let`, `return`, `require`, `guarantee`, `assert`, effect
  call statements, `if` / `else if` / `else`, and bounded `for`. Effect
  positions must be calls. [SYNTAX-REQ-007]
- Expressions parse the full Phase 1 precedence chain, calls, type-calls, field
  access, record literals, booleans, digest literals, pure ternary
  `if ... then ... else`, branch-yield conditional effects in `let` right-hand
  sides, variant literals, and `match`. [SYNTAX-REQ-008]
- Keywords are reserved as bare identifiers, binders, shorthand fields, import
  aliases, and coordinate roots. Keywords remain legal after `.` as member
  names. [SYNTAX-REQ-009] [EDICT-LANG-RECORD-SHORTHAND-001]
- Negative coverage asserts stable `ParseErrorKind` values rather than message
  prose or stdout/stderr. [SYNTAX-REQ-010]

## Deferred

These are deliberately not part of the syntax parser contract:

- semantic checks beyond parsing. The landed source-AST subset is documented in
  [semantic-validation](../semantic-validation/); import resolution, resolved
  type checking, shadow checks, and bound proofs remain deferred;
- Core IR lowering, CDDL, canonical encoding, and golden Core fixtures;
- pure `fn` and `const` declarations;
- `record` semantic-effect statements;
- list, map, and unit expression literals;
- exhaustive source fixture coverage for every `docs/REQUIREMENTS.md` language
  row.

The current verification strategy, planned cases, fixtures, oracles, and open
gaps are tracked in [test-plan.md](./test-plan.md).
