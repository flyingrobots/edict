---
report_id: "AUD-2026-06-28-V03"
title: "Ship-Readiness Assessment: edict v0.10.0-alpha.1"
status: "Final"
audit:
  date_started: 2026-06-28
  date_completed: 2026-06-28
  type: "Sanity"
  scope: "Full workspace at release commit; readiness for external consumption"
  compliance_frameworks: ["Rust API Guidelines", "OWASP-style local-CLI threat model", "flyingrobots release gates"]
target:
  repository: "github.com/flyingrobots/edict"
  branch: "main"
  commit_hash: "d607605"
  language_stack: ["Rust 2021 (MSRV 1.85)", "serde", "serde_json", "glob"]
  environment: "Local CLI + library; GitHub prerelease (publish = false, no crates.io)"
methodology:
  automated_tools: ["cargo xtask verify", "cargo clippy (all+pedantic, deny)", "manual panic/security review"]
  manual_review_hours: 3
  false_positive_rate: "n/a"
summary:
  total_findings: 11
  severity_count:
    critical: 0
    high: 3
    medium: 5
    low: 3
  remediation_status: "Pending"
related_reports:
  previous_audit: "AUD-2026-06-28-V01"
  tracking_ticket: "n/a"
---

> **Context correction.** This codebase **has already shipped** as a GitHub
> **prerelease** (`v0.10.0-alpha.1`, `publish = false`, explicit non-goals,
> milestone closed). So "ready to ship?" is reframed honestly as: *is HEAD safe
> to ship as an alpha, and what must change before external consumers can
> depend on it?* There is **no network, DB, auth, server, or persisted state** —
> classic production-service risks (injection, secrets, race conditions,
> centralized logging, health checks) are largely **N/A** and are marked as such
> rather than padded with invented findings.

---

## 1. Quality & Maintainability Assessment (Exhaustive)

### 1.1 Technical Debt Score: **2/10** (1 = excellent, 10 = unmaintainable)

Low debt. Justification — the three most problematic patterns, in a codebase
that is otherwise clean:

1. **God-file tooling module.** `xtask/src/main.rs` is 3,052 lines: ~637 of
   production logic and a single ~2,400-line `#[cfg(test)] mod tests` covering
   unrelated concerns (release policy, contract graph, core goldens, grammar,
   TextMate, VS Code manifests, schema shapes, link checks).
2. **Crate scope sprawl.** `edict-syntax` owns far more than syntax — compiler
   spine, canonical encoder, Core IR, lowerability, target IR, contract-bundle
   validation, and Gate C admission — with no compile-time boundary between
   layers.
   - **✅ Decided (2026-07-01, #84):**
     `docs/design/crate-scope-v0.11.md` records the decision to prefer an
     eventual layered split behind an umbrella crate over a simple rename. The
     package split is deferred to its own migration slice; `ARCHITECTURE.md`
     documents the current crate-scope caveat and layer rule.
3. **Stringly-typed CLI envelope parsing.** `crates/edict-cli/src/main.rs`
   hand-rolls JSON field extraction (`parse_compiler_input`,
   `require_string_field`) that is *more lenient* than the checked-in schemas
   (which pin `additionalProperties:false` and mutually exclusive input kinds) —
   a contract-drift surface.

Everything else trends positively: no `unsafe`, zero production `.unwrap()`,
`clippy::all`+`pedantic` denied, 278 tests, golden fixtures, and a docs-as-contract
gate.

### 1.2 Readability & Consistency (3 onboarding frictions)

- **Issue 1 — No runnable examples / entry-point signposting.** `lib.rs`
  re-exports ~150 symbols across 14 modules with **zero** rustdoc code examples;
  a new engineer cannot tell where to start.
  - **Mitigation Prompt 1:** `Add a crate-level rustdoc example to crates/edict-syntax/src/lib.rs demonstrating the primary flow (parse_module then validate_surface, or the new check facade), grouped under a "# Examples" heading, and add module-level intro docs to the 3-4 most-used modules (parser, semantic, compiler). Ensure cargo test --doc passes.`
- **Issue 2 — Tooling tests obscure tooling logic.** Navigating
  `xtask/src/main.rs` means scrolling past ~2,400 lines of tests to reach
  ~637 lines of logic.
  - **Mitigation Prompt 2:** `Split xtask/src/main.rs production logic into modules (release.rs, contract_check.rs, core_goldens.rs, schema_audit.rs) and move the inline tests into xtask/tests/*.rs or per-module test blocks, keeping cargo xtask verify behavior identical.`
- **Issue 3 — CLI input-kind handling is implicit.** The supported input kinds
  live in a hand-written `match` rather than a typed model, so the supported set
  is not discoverable from a single type.
  - **Mitigation Prompt 3:** `Model CompilerInput as a serde-tagged enum (tag = "kind") so the supported input kinds are visible in one type and unknown-field rejection matches docs/schemas/edict.compiler-input.v1.schema.json.`

### 1.3 Code Quality Violations (3 instances, with rewrites)

**Violation 1 — Hand-rolled variant parsing (SRP/maintainability).**
`crates/edict-cli/src/main.rs::parse_compiler_input` mixes discrimination, field
extraction, and error construction by hand.

*Simplified Rewrite 1 (illustrative — envelope `schema`/`type` validated separately):*

```rust
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum CompilerInput {
    Source { #[serde(default = "default_inline_name")] name: String, source: String },
    Path { path: PathBuf },
    PathList { paths: Vec<PathBuf> },
    Directory { path: PathBuf },
    Glob { pattern: String },
}
```

- **Mitigation Prompt 4:** `Replace parse_compiler_input/require_string_field with the serde-tagged enum above, keeping the envelope check for schema=="edict.compiler.input/v1" and type=="compilerInput", preserving every emitted diagnostic kind and exit code, and verifying against the fixtures/cli/ golden corpus.`

**Violation 2 — Per-file allocation in a hot path.**
`directory_extension_matches` builds a `format!(".{extension}")` String for every
file during directory walks.

*Simplified Rewrite 2:*

```rust
fn directory_extension_matches(settings: &CompilerSettings, path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else { return false; };
    settings.directory_extensions.iter().any(|allowed| allowed.strip_prefix('.') == Some(ext))
}
```

- **Mitigation Prompt 5:** `Replace directory_extension_matches with the allocation-free version above; keep the .edict default and behavior identical; confirm fixtures/cli/06-directory-expansion-ok still passes.`
  - **✅ Addressed (2026-07-01, #91):** `directory_extension_matches` now
    compares `Path::extension()` directly against configured dotted extensions
    without allocating a `String` for every visited file; the directory
    expansion golden remains byte-identical.

**Violation 3 — Post-construction mutation of a JSON value (record builder smell).**
`diagnostic_record` builds a `json!` object then conditionally mutates it
(`record["span"] = …`, `record["line"] = …`), and `cli_diagnostic` further does
`record["message"] = …`. Stringly-indexed mutation of a `Value` is fragile.

*Simplified Rewrite 3:*

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic<'a> {
    schema: &'static str, r#type: &'static str, command: &'static str,
    severity: &'static str, stage: &'a str, kind: &'a str, input: &'a Value,
    #[serde(skip_serializing_if = "Option::is_none")] span: Option<SpanRec>,
    #[serde(skip_serializing_if = "Option::is_none")] line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")] message: Option<&'a str>,
}
```

- **Mitigation Prompt 6:** `Introduce typed Serialize structs for the diagnostic, check-result, and event records with #[serde(skip_serializing_if = "Option::is_none")] for optional fields, replacing the json! + index-mutation construction in crates/edict-cli/src/main.rs. Keep emitted bytes identical (BTreeMap key order is preserved by serde_json) and verify against the golden corpus.`
  - **✅ Addressed (2026-07-01, #92):** CLI check-result, diagnostic, status,
    and info records are now built from typed `Serialize` structs and converted
    through `serde_json::Value` to preserve the existing byte-for-byte golden
    output; the `record[...]` mutation guard and CLI golden corpus are green.

---

## 2. Production Readiness & Risk Assessment (Exhaustive)

### 2.1 Top 3 Ship-Affecting Risks

None are Critical/crash-class; all three are "fix before inviting external
dependents," not "the alpha is unsafe."

- **Risk 1 (High) — Public CLI cannot introduce itself.** `crates/edict-cli/src/main.rs:69`
  rejects `--help`/`--version` with a cryptic JSONL `InvalidArguments` (exit 2).
  For a release whose headline is "first public CLI," this fails the most basic
  user expectation.
  - **Mitigation Prompt 7:** `Add explicit --help/-h and --version/-V handling in crates/edict-cli/src/main.rs before the stdin read, emitting usage + request-schema URIs + exit-code contract and exiting 0; add golden fixtures and CLI-REQ/CLI-TP rows.`
  - **✅ Addressed (2026-06-29, #101):** `--help`/`--version` emit an `edict.cli.info/v1` record and exit 0; unknown args get an actionable diagnostic. Covered by `CLI-REQ-009` / `CLI-TP-012`..`CLI-TP-014`.
- **Risk 2 (High) — No stable, curated API surface.** ~150 flat re-exports and
  the `edict-syntax` name/scope mismatch mean early adopters bind to symbols
  likely to move pre-1.0, with no `check`-style façade to depend on.
  - **Mitigation Prompt 8:** `Add an edict_syntax::check(&str) facade and an "API stability" note in the crate docs declaring which items are the supported surface for the alpha train; route the CLI through the facade.`
  - **✅ Partly addressed (2026-06-29, #104):** the `check`/`CheckOutcome` façade and an API-stability note now exist, and the CLI routes through it — early adopters have a small supported surface to bind to. The `edict-syntax` name/scope mismatch and the ~150 flat re-exports remain open in **#84**.
- **Risk 3 (High) — Schema/parser leniency divergence.** The CLI accepts input
  records the published schemas reject (extra fields; hybrid input kinds),
  because `parse_compiler_input` is lenient while
  `docs/schemas/edict.compiler-input.v1.schema.json` is strict. Consumers who
  validate against the schema and consumers who feed the binary can disagree.
  - **Mitigation Prompt 9:** `Tighten CLI input parsing to match the schema: deny unknown fields and reject hybrid input-kind field combinations, so the binary and the checked-in schema accept exactly the same records; add negative golden fixtures for an extra-field and a hybrid-kind request.`
  - **✅ Addressed (2026-06-29, #103):** the parser now rejects unknown fields and hybrid input kinds with `InvalidInputRecord`; golden cases `10-input-extra-field` / `11-input-hybrid-kind` (`CLI-TP-015`). Binary and schema now accept the same records.

### 2.2 Security Posture (2 overlooked items — local-CLI threat model)

The current trust model is *local developer, trusted input*; both items below
become real only if `edict` is exposed to untrusted callers (e.g. wired into an
automated pipeline). Worth pre-empting now.

- **Vulnerability 1 — Arbitrary file read via request-specified paths/globs +
  symlink traversal.** `expand_path`/`expand_directory`/`expand_glob`
  (`crates/edict-cli/src/main.rs`) read any path/dir/glob the request names, and
  directory recursion follows symlinks when `followSymlinks` is true. Under
  untrusted input this is an information-disclosure / traversal vector.
  - **Mitigation Prompt 10:** `Document the CLI trust boundary (requests are trusted; paths are read with the caller's privileges) in docs/topics/cli/README.md, and add an optional root-confinement setting that rejects input paths resolving outside a configured root, with a stable CLI failure kind and a golden fixture.`
  - **✅ Addressed (2026-07-01, #95):** compiler settings now accept optional
    `inputRoot`; path, path-list, directory, and glob inputs resolving outside
    that root fail with `InputPathOutsideRoot`, exit 2, and are pinned by
    `CLI-REQ-011` / `CLI-TP-017` plus
    `fixtures/cli/13-input-root-outside`.
- **Vulnerability 2 — Unbounded stdin buffering (DoS).** `run()` does
  `io::stdin().read_to_string(&mut input)` — the entire stream is read into
  memory with no cap. A hostile or runaway producer can exhaust memory.
  - **Mitigation Prompt 11:** `Add a configurable maximum stdin size (sane default) and stream JSONL line-by-line instead of buffering the whole input; emit a stable "InputTooLarge" CLI diagnostic past the cap, with a golden fixture.`
  - **✅ Addressed (2026-07-01, #96):** stdin is bounded before request parsing
    with a default 8 MiB cap and an `EDICT_CLI_MAX_STDIN_BYTES` override.
    Over-limit input emits `InputTooLarge`, exits 2, and is pinned by
    `CLI-REQ-010` / `CLI-TP-016` plus `fixtures/cli/12-input-too-large`.

### 2.3 Operational Gaps (3)

Classic service ops (centralized logging, health checks, error reporting) are
**N/A** — there is no running service. The relevant operational gaps for a
library + CLI:

- **Gap 1 — No browsable API docs.** `publish = false` means no docs.rs; combined
  with zero rustdoc examples, consumers have no rendered reference.
- **Gap 2 — No supply-chain gate in CI.** The dependency posture is excellent
  (3 deps, clean licenses) but unenforced — no `cargo-deny`/advisory check, so a
  future bad dependency would not be caught automatically.
  - **✅ Addressed (2026-07-01, #94):** `deny.toml` plus the CI
    `supply-chain (cargo-deny)` job now gate advisories, yanked crates, license
    allowlisting, duplicate-version warnings, and unknown sources.
- **Gap 3 — No distributable artifact / install path.** The prerelease has zero
  attached assets and `publish = false`, so obtaining `edict` requires building
  from source; there is no documented install. (`SECURITY.md` reporting channel
  is also absent — see Documentation audit.)

---

## 3. Final Recommendations & Next Step

### 3.1 Final Ship Recommendation: **YES, BUT**

HEAD is **safe to ship as an alpha** — and already is, correctly, as a
`publish = false` GitHub prerelease with honest non-goals, a panic-free input
path, and a green release gate. **BUT** before inviting external consumers to
depend on it: give the public CLI `--help`/`--version`, publish a curated/stable
library surface (the `check` façade + an API-stability note), add `SECURITY.md`,
and reconcile the schema/parser leniency.

### 3.2 Prioritized Action Plan

- **Action 1 (High Urgency):** Make the public CLI self-describing — add
  `--help`/`--version` and a README "Build & Run" section. It is the literal
  headline feature of the release and the first thing any user touches.
- **Action 2 (Medium Urgency):** Introduce `edict_syntax::check(&str)` plus an
  API-stability note, and route the CLI through it — stabilizes the surface
  external adopters will bind to and removes parse/validate duplication.
- **Action 3 (Low Urgency):** Tighten CLI input parsing to match the published
  schemas (serde-derive, `deny_unknown_fields`, mutual-exclusivity) and add
  `cargo-deny` to CI to make the clean dependency posture an enforced gate.
