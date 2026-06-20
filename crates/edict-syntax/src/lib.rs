//! `edict-syntax`: the Edict language front end (Phase 1).
//!
//! Scope is `edict.implementation/minimal-v1` (see SPEC - Edict Language v1).
//! The front end currently parses: package and imports; `type` records and
//! refined scalars; `enum` declarations and `variant` types; `intent`s with
//! their clauses; `let`/`return`/`require`/`guarantee`/`assert`; the `if`
//! family (ternary, branch-yield conditional effects, and `if`/`else if`/`else`
//! control flow); bounded `for`; calls and type-calls; variant-literal
//! constructors; and `match`. Pure `fn`/`const` declarations, `record`
//! semantic-effect statements, list/map/unit literals, and the entire
//! semantic-validation layer are deferred (see `docs/RETRO_phase1-parser.md`).
//!
//! Assurance tooling (HOLMES / Watson / Moriarty) is shared platform machinery
//! in `flyingrobots/wesley`; it operates on bundles and evidence, downstream of
//! this crate, and is wired in at the assurance phase — not depended on here.

pub mod ast;
pub mod parser;
pub mod token;

pub use parser::{parse_module, ParseError, ParseErrorKind};
pub use token::{lex, IntSuffix, LexError, Span, Token, TokenKind};
