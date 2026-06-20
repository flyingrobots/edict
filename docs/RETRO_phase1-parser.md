# Retrospective — Phase 1: Edict lexer + minimal-v1 parser

**Branch:** `feat/phase1-parser` · **PR:** #9 · **Crate:** `crates/edict-syntax`

Phase 1 turned the Phase 0 design baseline into something that actually runs: a
standalone Rust workspace that lexes and parses the
`edict.implementation/minimal-v1` surface into a source AST. This retro records
what shipped, what was deliberately left out, the decisions worth remembering,
and where the next phase picks up.

## What shipped

A hand-written, `std`-only, deterministic frontend:

- **Lexer** (`token.rs`) — contextual-keyword identifiers (keywords stay usable
  after `.`), typed integer literals with suffixes, UTF-8 string literals with
  escapes, refined-scalar/operator punctuation, line/block comments.
- **AST** (`ast.rs`) — source AST that preserves source order and surface
  spelling; canonicalization is deferred to Core IR lowering.
- **Parser** (`parser.rs`) — recursive descent with full operator precedence.

Parsed surface, by grammar production:

| Area | Productions parsed |
|---|---|
| Module | `package`, `use shape/lawpack/target/core` (+ optional `digest`) |
| Types | `type` records, refined `String`/`Bytes`, `Option`/`List`/`Map`/`CapabilityRef`, `enum`, `variant` (with payloads) |
| Intents | params, `returns`, `profile`/`implements`/`basis`/`footprint`/`budget`/`where` clauses |
| Statements | `let` (+ effect-`else`), `return`, `require`, `guarantee`, `assert`, `if`/`else if`/`else`, bounded `for`, effect statements |
| Expressions | full precedence chain; calls + type-calls; field access; records (incl. shorthand + spread); ternary `if … then … else`; branch-yield; variant literals (`::`); `match` |
| Obstructions | single-target and full map-form `else { failure(binder) => target, … }` |

**Conformance fixtures** (`fixtures/lang/`): `bounds/bounded-hello`,
`effects/read-greeting`, `effects/conditional-blob`, `types/color-match`, plus a
negative parse-reject corpus. **34 tests green**; `cargo fmt --check` clean;
`clippy` clean under deny-all + pedantic; CI (`fmt` · `clippy -D warnings` ·
`test`) guards every push.

## Decisions worth remembering

- **Contextual keywords.** Keywords are matched by identifier text, not reserved
  at the lexer. This keeps `.read`, `.match`, `.Transparent` usable as member
  names while `match`/`if`/`for` act as keywords in leading position.
- **Ternary vs. branch-yield disambiguation.** Both start `if predicate`. The
  parser commits on the token *after* the predicate: `then` → pure ternary
  (legal anywhere, sits at the top of `expr`); `{` → effectful branch-yield
  (legal **only** as a `let`-rhs, each branch ending in `yield`). The two share
  no grammar past the predicate, so no backtracking is needed here.
- **`<` is overloaded; type-calls backtrack.** A leading `<` after a callee is
  ambiguous between generics and comparison. `try_type_call_args` speculatively
  parses `<type-args>` and only commits if it is immediately followed by `(`;
  otherwise it restores the cursor and lets the relational operator win.
- **Variant literals via a `::` postfix suffix.** `Qual.Type::Case(payload)` is
  detected in the postfix loop: the accumulated expression must flatten to a
  pure dotted path (`expr_to_path`), or it is a parse error. Enum-case
  *selection* stays ordinary field access (`Qual.Enum.CASE`) — the `::`/`.`
  split mirrors `EDICT-LANG-ENUMVARIANT-001`.
- **Tests are the spec.** Every slice landed test-first with a conformance
  fixture plus targeted positive/negative cases, including rejections
  (unbounded `for`, branch with no `yield`, empty `match`).

## What was found and fixed

The strict self-review on the first parser drop caught a **real latent bug**:
string lexing used `char::from(u8)`, a byte→Latin-1 cast rather than a UTF-8
decode, silently corrupting any non-ASCII literal (proven: `0xC3` → `Ã`). ASCII
fixtures had hidden it. Fixed by buffering raw bytes and decoding once at the
close quote, with a multibyte regression fixture (`café — naïve 🦀`). The same
pass added CI, promoted `clippy::pedantic` to deny, tightened integer-underscore
placement, and parsed multi-part package versions.

## Deliberately out of scope (deferred)

minimal-v1's grammar is larger than this phase. Still unparsed, by design:

- `const` and `fn` (pure-block) declarations;
- `record` semantic-effect statements;
- `list-lit` (`[ … ]`) and `map-lit` (`map<K,V>{ … }`) literals;
- `bool`/`unit`/`digest(...)` literal forms;
- **all semantic validation** — naked unbounded `String`/`Bytes`, clause
  requiredness, `migration`/`projection` as *reserved* words, read-only proofs,
  exhaustive `match`/obstruction coverage, integer-width resolution, loop-bound
  provability. This is a separate pass over the AST/Core and is tracked, with
  the relapse-zoo reject corpus, in **#10**.

Grammar and the Core schema both remain unfrozen.

## Next

1. Close the remaining surface gaps above (literals, `const`/`fn`, `record`).
2. Begin Core IR lowering against the `edict.core/v1` CDDL (**#3**) — A-normal
   effect form, structured branches, explicit guard nodes, loop bounds.
3. Stand up the semantic-validation layer + relapse zoo (**#10**).

The frontend now parses both README worked examples (hello, readGreeting) plus
conditional effects, bounded loops, and sum types — enough shape to start
lowering.
