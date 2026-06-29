//! Edict CLI support crate.
//!
//! The `edict` binary owns the JSONL stream contract for compiler request,
//! result, status, and diagnostic records.

/// Stable schema identifier for compiler settings records.
pub const COMPILER_SETTINGS_SCHEMA: &str = "edict.compiler.settings/v1";

/// Stable schema identifier for compiler input records.
pub const COMPILER_INPUT_SCHEMA: &str = "edict.compiler.input/v1";

/// Stable schema identifier for `check` success result records.
pub const CHECK_RESULT_SCHEMA: &str = "edict.cli.check-result/v1";

/// Stable schema identifier for CLI diagnostic records.
pub const DIAGNOSTIC_SCHEMA: &str = "edict.cli.diagnostic/v1";

/// Stable schema identifier for CLI terminal status (event) records.
pub const EVENT_SCHEMA: &str = "edict.cli.event/v1";

/// Stable schema identifier for CLI informational (`--help`/`--version`) records.
pub const INFO_SCHEMA: &str = "edict.cli.info/v1";
