//! `edict-syntax`: the Edict language front end (Phase 1).
//!
//! Scope is `edict.implementation/minimal-v1` (see SPEC - Edict Language v1):
//! records, enums, `Option`, bounded `List`, pure first-order functions, `if`
//! expressions, branch-yield conditional effects, target/lawpack effects,
//! guards, typed obstructions, and budgets. Variants, maps, regex-lite
//! constraints, and bounded recursive imports come later behind capability
//! flags.
//!
//! Assurance tooling (HOLMES / Watson / Moriarty) is shared platform machinery
//! in `flyingrobots/wesley`; it operates on bundles and evidence, downstream of
//! this crate, and is wired in at the assurance phase — not depended on here.

pub mod ast;
pub mod parser;
pub mod token;

pub use parser::{parse_module, ParseError};
pub use token::{lex, IntSuffix, LexError, Span, Token, TokenKind};
