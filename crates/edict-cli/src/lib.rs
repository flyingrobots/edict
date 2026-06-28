//! Edict CLI support crate.
//!
//! The `edict` binary owns the JSONL stream contract for compiler request,
//! result, status, and diagnostic records.

/// Stable schema identifier for compiler settings records.
pub const COMPILER_SETTINGS_SCHEMA: &str = "edict.compiler.settings/v1";
