//! Shared helpers for the parser integration tests.
//!
//! Lives in a `common/` subdirectory so Cargo treats it as a module included by
//! each test binary (`mod common;`) rather than its own test target. Not every
//! test uses every helper, hence the crate-level `dead_code` allowance.
#![allow(dead_code)]

use edict_syntax::ast::{Decl, IntentDecl, Module};
use edict_syntax::parse_module;

/// Wrap a statement sequence in a minimal, well-formed intent body.
pub fn body(stmts: &str) -> String {
    format!(
        "package a.b@1;\n\
         intent t(input: shape.In) returns shape.Out basis none budget <= p.b {{\n\
         {stmts}\n\
         }}"
    )
}

/// Parse a source string, panicking with the parse error on failure.
pub fn parse_ok(src: &str) -> Module {
    parse_module(src).unwrap_or_else(|e| panic!("expected a parse, got error: {e}"))
}

/// The first declaration as an intent.
pub fn intent_of(m: &Module) -> &IntentDecl {
    let Decl::Intent(intent) = &m.decls[0] else {
        panic!("decl 0 is an intent");
    };
    intent
}
