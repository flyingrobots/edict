//! `edict-syntax`: the Edict language front end.
//!
//! Scope is `edict.implementation/minimal-v1` (see SPEC - Edict Language v1).
//! Phase 1 parses: package and imports; `type` records and refined scalars;
//! `enum` declarations and `variant` types; `intent`s with their clauses;
//! `let`/`return`/`require`/`guarantee`/`assert`; the `if` family; bounded
//! `for`; calls and type-calls; variant-literal constructors; and `match`.
//! Phase 2 currently validates source-AST constraints that do not require import
//! resolution or Core IR: bounded runtime `String`/`Bytes`, required intent
//! operation-mode/budget/basis clauses, duplicate singleton intent clauses,
//! module namespace collisions, and source binder shadowing.
//! Pure `fn`/`const` declarations, `record` semantic-effect statements,
//! list/map/unit expression literals, import resolution, resolved type checking,
//! and Core IR validation are deferred.
//!
//! Assurance tooling (HOLMES / Watson / Moriarty) is shared platform machinery
//! in `flyingrobots/wesley`; it operates on bundles and evidence, downstream of
//! this crate, and is wired in at the assurance phase — not depended on here.

pub mod ast;
pub mod parser;
pub mod semantic;
pub mod token;

pub use parser::{parse_module, ParseError, ParseErrorKind};
pub use semantic::{validate_module, SemanticError, SemanticErrorKind};
pub use token::{lex, IntSuffix, LexError, Span, Token, TokenKind};

#[cfg(doctest)]
mod topic_shelf_doctests {
    #[doc = include_str!("../../../docs/topics/core-ir/README.md")]
    pub struct CoreIrTopicDocs;

    #[doc = include_str!("../../../docs/topics/semantic-validation/README.md")]
    pub struct SemanticValidationTopicDocs;

    #[doc = include_str!("../../../docs/topics/syntax/README.md")]
    pub struct SyntaxTopicDocs;
}
