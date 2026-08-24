# Public Rust API

Status: current HEAD contract.

The `flyingrobots-edict` package exposes the Rust library name `edict` as a
curated facade over Edict's implementation crates. The facade is the supported
Rust entry point for source checking, stable diagnostic kinds, and canonical
artifact identity operations. It does not expose the implementation crate's
module tree as an accidental public API.

The package remains `publish = false`. This topic defines a reversible release-
engineering boundary; it does not authorize or claim crates.io publication.

The `edict` CLI remains the stable process boundary for complete application
builds. The Rust facade does not duplicate the CLI's JSONL protocol, provider
host, filesystem publication, or application-build orchestration.
