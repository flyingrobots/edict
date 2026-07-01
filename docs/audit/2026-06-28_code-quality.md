---
report_id: "AUD-2026-06-28-V01"
title: "Code Quality Audit: edict workspace (front-end, CLI, tooling)"
status: "Final"
audit:
  date_started: 2026-06-28
  date_completed: 2026-06-28
  type: "Full"
  scope: "crates/edict-syntax/*, crates/edict-cli/*, xtask/*"
  compliance_frameworks: ["Rust API Guidelines", "flyingrobots house rules"]
target:
  repository: "github.com/flyingrobots/edict"
  branch: "main"
  commit_hash: "d607605"
  language_stack: ["Rust 2021 (MSRV 1.85)", "serde", "serde_json", "glob"]
  environment: "Library + CLI (no network, DB, server, or runtime)"
methodology:
  automated_tools: ["cargo clippy (all+pedantic, deny)", "cargo xtask verify", "cargo xtask contract-check", "markdownlint-cli2"]
  manual_review_hours: 3
  false_positive_rate: "High on naive grep (see §4.1); all findings hand-verified"
summary:
  total_findings: 9
  severity_count:
    critical: 0
    high: 0
    medium: 4
    low: 5
  remediation_status: "Pending"
related_reports:
  previous_audit: null
  tracking_ticket: "n/a"
---

**Subject:** an early-alpha (`v0.10.0-alpha.1`) Rust workspace implementing the Edict
restricted deterministic language: `edict-syntax` (front-end + compiler spine +
canonical encoder + Core IR + lowerability + target IR + contract-bundle +
admission checks), `edict-cli` (the public `edict check` binary), and `xtask`
(release/contract tooling). 19,013 LOC of Rust; 278 tests across 22 files; three
runtime dependencies.

> **Framing.** This is a pure compiler library + local CLI. There is no network,
> database, server, UI, authentication, or persisted state. Audit-template items
> that assume those surfaces are marked **N/A** rather than answered with
> invented findings.

---

## 0. 🏆 Executive Report Card

| Metric | Score (1-10) | Recommendation |
| --- | --- | --- |
| **Developer Experience (DX)** | 7 | **Best of:** the CLI's JSONL-only, machine-first contract is rigorously deterministic — every record family has a checked-in JSON Schema and a byte-exact golden replay. |
| **Internal Quality (IQ)** | 9 | **Watch Out For:** scope sprawl in a single crate — `edict-syntax` quietly owns admission, bundles, and target IR, and `xtask/src/main.rs` is a 3,052-line catch-all. |
| **Overall Recommendation** | **👍 THUMBS UP** | **Justification:** a disciplined, defensively-written, heavily-tested codebase whose gaps are additive polish (onboarding surface, module boundaries), not rot. |

---

## 1. DX: Ergonomics & Interface Clarity (Advocate View)

### 1.1 Time-to-Value (TTV) — Score: 6/10

The CLI's TTV is good once you know the contract (`printf '<settings>\n<input>\n' | edict`). The **library's** TTV is the weak point: there is no single entry call. To check a string a consumer must chain `parse_module` → `validate_surface` → (optionally) `compile_to_core`, and `lib.rs` re-exports ~150 symbols flat across 14 modules with no curated "start here" surface. The biggest removable boilerplate is this manual chain — the CLI already encapsulates it privately in `check_sources` (`crates/edict-cli/src/main.rs`), so the capability exists but is not exposed.

- **Action Prompt (TTV Improvement):** `In crates/edict-syntax, add a public convenience function check(source: &str) -> CheckOutcome that runs parse_module then validate_surface and returns a structured result enum (Parsed-and-valid, ParseError, or Vec<SemanticError>), mirroring the private check logic in crates/edict-cli/src/main.rs::check_sources. Re-export it from lib.rs, add a #[doc] example, and refactor the CLI to call it so the parse→validate sequence has one owner. Do not change existing public signatures.`
- **✅ Addressed (2026-06-29, #104):** `edict_syntax::check(&str) -> CheckOutcome` added (with a runnable rustdoc example and an API-stability note); the CLI's `check_sources` now routes through it, and the golden corpus proves byte-identical external behavior.

### 1.2 Principle of Least Astonishment (POLA)

The sharpest violation is the **public CLI rejecting `--help` and `--version`**. `crates/edict-cli/src/main.rs:69` treats *any* argument as fatal (`InvalidArguments`, exit 2) with the message "the first CLI slice reads JSONL request records from stdin only". A developer's first instinct — `edict --help` — yields a cryptic JSONL error. Every CLI convention leads them to expect usage text. (Secondary astonishment: a crate named `edict-syntax` that also performs admission and bundle validation — see §3.2.)

- **Action Prompt (Interface Refactoring):** `In crates/edict-cli/src/main.rs, before reading stdin, handle --help/-h and --version/-V explicitly: print a short usage summary (the check workflow, the JSONL request schema URIs edict.compiler.settings/v1 and edict.compiler.input/v1, and the exit-code contract) and exit 0. Keep all other arguments rejected as InvalidArguments. Add a golden CLI fixture under fixtures/cli/ for --help and --version, and a CLI-REQ/CLI-TP row in docs/topics/cli/test-plan.md.`
- **✅ Addressed (2026-06-29, #101):** `--help`/`-h` and `--version`/`-V` emit a single `edict.cli.info/v1` JSONL record and exit 0 (machine-first rather than plain text, to keep the JSONL-only stdout contract and the "no human-pretty output" non-goal). Covered by `CLI-REQ-009` / `CLI-TP-012`..`CLI-TP-014`.

### 1.3 Error Usability

CLI diagnostics are strong: structured, staged (`parse`/`semantic`/`cli`), stable `kind` codes (hardened this session in PR #77), with spans for source errors and human messages for CLI errors. The remaining cryptic case is the **`InvalidArguments`** message above — it explains what the tool *does* but not what the caller should *do*, and carries no pointer to the request schema. (Parse/semantic diagnostics deliberately omit prose `message`; that is fine because the `kind`+`span` are the contract.)

- **Action Prompt (Error Handling Fix):** `In crates/edict-cli/src/main.rs, enrich the InvalidArguments CliFailure message to name the supported invocation ("edict reads JSONL request records on stdin; run 'edict --help' for the request schema") and add a "docs" field pointing at the docs/topics/cli/README.md path. Update the matching golden fixture (fixtures/cli/04-cli-missing-settings or a new args case) and the CLI test plan.`
- **✅ Addressed (2026-06-29, #101):** the `InvalidArguments` message now names the supported invocation and points at `edict --help` and `docs/topics/cli/README.md`. Covered by `CLI-TP-013`. (The pointer is in the diagnostic `message` rather than a new `docs` field, to avoid widening the frozen `edict.cli.diagnostic/v1` schema.)

---

## 2. DX: Documentation & Extendability (Advocate View)

### 2.1 Documentation Gap

The friction point past basic usage is the absence of an **API/CLI quickstart**. The 584-line `README.md` is an excellent conceptual + spec narrative ("Edict in 10 Seconds", "What An Intent Looks Like") but has **no Installation, no CLI Usage, and no library Quickstart** heading, and the crates carry **zero runnable rustdoc examples** (0 fenced examples in either `lib.rs`). A developer who buys the pitch has nowhere to learn `edict check`. (Full treatment in the companion Documentation audit.)

- **Action Prompt (Documentation Creation):** `Add a "Using the CLI" section to README.md with a copy-pasteable edict check example (a JSONL settings + source request piped to the binary, the expected JSONL stdout, and the exit-code table), and add a crate-level rustdoc example to crates/edict-syntax/src/lib.rs showing parse_module + validate_surface. Ensure doc examples compile under cargo test --doc.`

### 2.2 Customization — Score: 8/10

Extensibility is good *by construction*: the front-end is pure functions over data types, target/lawpack/profile facts are passed in explicitly (e.g. `CompilerContext`, `TargetProfileFacts`, authority-fact files), and everything is digest-bound, so callers compose behavior without forking source. The most robust extension point is **explicit fact injection** (file-backed authority facts, target-profile manifests). The weakest is the **stringly-typed, hand-rolled JSON request parsing in the CLI** (`parse_compiler_input` in `crates/edict-cli/src/main.rs`): adding a new input kind means editing a `match` and several `require_string_field` calls rather than deriving from a typed model, and the lenient parsing diverges from the strict schema (noted in PR #74 review).

- **Action Prompt (Extension Improvement):** `In crates/edict-cli/src/main.rs, replace the hand-rolled parse_compiler_input/require_string_field logic with a serde-derived enum (#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]) for CompilerInput so new input kinds are added declaratively and unknown-field rejection matches docs/schemas/edict.compiler-input.v1.schema.json. Preserve the exact emitted diagnostics and exit codes; verify against the fixtures/cli/ golden corpus.`
- **✅ Addressed (2026-06-29, #103):** the parser now rejects any field outside the envelope and the kind's own variant fields, so the binary accepts exactly what `edict.compiler.input/v1` accepts (unknown fields and hybrid kinds both fail with `InvalidInputRecord`). Golden cases `10-input-extra-field` / `11-input-hybrid-kind` (`CLI-TP-015`). A full serde-tagged-enum rewrite was deferred as the lower-risk allow-list check achieves parity; the declarative-model cleanup remains available via #90's broader scope if desired.

---

## 3. Internal Quality: Architecture & Maintainability (Architect View)

### 3.1 Technical Debt Hotspot

**`xtask/src/main.rs` (3,052 lines).** Production logic is lines 1–637; a single `#[cfg(test)] mod tests` runs from line 638 to EOF (~2,400 lines). One module mixes release-policy validation, contract-graph checking, core-golden generation, schema-shape audits, grammar/TextMate/VS Code manifest tests, and link checks. It is the repo's largest file, the hardest to navigate, and a merge-conflict magnet. (By contrast, the *library* modules are healthy — see §3.3.)

- **Action Prompt (Debt Reduction):** `Refactor xtask/src/main.rs into focused modules (release.rs, contract_check.rs, core_goldens.rs, schema_audit.rs, plus main.rs dispatch) and relocate the inline #[cfg(test)] tests either next to their modules or into xtask/tests/*.rs integration files. Keep cargo xtask verify / contract-check / core-goldens behavior byte-identical; run the full suite before and after to prove parity.`

### 3.2 Abstraction Violation

The clearest Separation-of-Concerns issue is **crate-level, not function-level**: `edict-syntax` bundles lexing/parsing, semantic validation, the compiler spine, the canonical CBOR encoder, Core IR, lowerability, target profiles, **target IR lowering, contract-bundle validation, and Gate C admission checks** into one crate named for only the first of those. There is no compile-time boundary preventing, say, the parser from reaching into admission types. The fix is crate (or at least module-visibility) segmentation behind an umbrella.

- **Action Prompt (SoC Refactoring):** `Propose (do not execute without sign-off, per house rules) a crate split: edict-syntax (lex/parse/AST), edict-core (semantic + compiler spine + canonical + Core IR), edict-targets (lowerability + target profiles + target IR), edict-admission (contract bundle + Gate C), and an edict umbrella re-exporting a curated surface. Produce a dependency-direction diagram proving an acyclic layering and an inventory of which current pub items move where.`

### 3.3 Testability Barrier

**There is essentially no testability barrier** — this is a strength, recorded for completeness. The core is pure functions over owned data; I/O is confined to explicit `load_*_file` loaders and the CLI boundary; there are no statics, no globals, no ambient singletons, and `unsafe` is `forbid`. 278 tests run without fixtures-on-disk gymnastics. The one mild friction is CLI testing-by-subprocess (`run_edict` spawns the binary), which is appropriate for an end-to-end contract but slower than in-process calls; exposing the library façade (§1.1) would let most CLI behavior be tested in-process.

- **Action Prompt (Testability Improvement):** `After adding the edict_syntax::check façade (§1.1), add in-process unit tests for the check pipeline in crates/edict-cli or crates/edict-syntax so subprocess golden replay (golden_cli.rs) is reserved for true end-to-end stream/exit-code contract coverage rather than for exercising parse/validate logic.`
- **✅ Addressed (2026-06-29, #104):** in-process unit tests for `check` (valid / parse-failed / semantic-failed) added in `edict-syntax`; the subprocess golden replay now backs the end-to-end stream contract while parse/validate logic is exercised in-process.

---

## 4. Internal Quality: Risk & Efficiency (Auditor View)

### 4.1 The Critical Flaw

**None found.** The highest-value result of this audit is a *negative*: the parse/validate path is **panic-free on untrusted input**. A naive grep flags 71 `expect`-like calls in `parser.rs`, but every one is `self.expect(&TokenKind::…)?` — the parser's own graceful token-matching combinator returning `ParseError`, not `Result::expect`. Production code contains **zero `.unwrap()`**; the few real `.expect()` calls (4 in `canonical.rs`, 1 each in `token.rs`/`admission.rs`/`authority_facts.rs`) sit behind proven invariants (range-checked integer casts, infallible `String` writes) with self-documenting messages. `edict-cli` production code has **zero** panic sites and is entirely `Result`-driven.

- **Action Prompt (Risk Mitigation):** `No critical flaw to neutralize. To lock in the property, add a clippy gate to deny unwrap_used and expect_used in non-test code for crates/edict-cli (where input is untrusted), allowing expect only where a // SAFETY/invariant comment is present, and document the parser's self.expect combinator naming so future audits don't re-flag it.`
  - **✅ Addressed (2026-07-01, #93):** `edict-cli` production targets now deny
    `clippy::unwrap_used` and `clippy::expect_used`; the parser's
    `self.expect` helper is documented as a fallible token-matching combinator
    that returns structured `ParseError` values rather than panicking.

### 4.2 Efficiency Sink

No meaningful sink for the current scale (single-file/dir/glob check runs). The only micro-allocation worth noting: `directory_extension_matches` in `crates/edict-cli/src/main.rs` builds a `format!(".{extension}")` `String` per file during directory walks to compare against `directory_extensions`. Negligible today; would matter only on very large trees.

- **Action Prompt (Optimization):** `In crates/edict-cli/src/main.rs::directory_extension_matches, compare the path extension to the configured directory_extensions without allocating per file — e.g. strip the leading '.' from each configured extension once into a reusable set and compare against path.extension() directly. Add no new behavior; keep the .edict default. Benchmark only if a large-tree fixture is added.`

### 4.3 Dependency Health — Excellent

Three runtime dependencies total: `serde`, `serde_json`, `glob` — all current, widely-maintained, permissively licensed (MIT/Apache-2.0), Apache-2.0-compatible. `unsafe_code = "forbid"`, `clippy::all`+`pedantic` denied, warnings-as-errors. No deprecated APIs, no known-CVE dependencies, no transitive bloat. This is a model minimal-surface dependency posture.

- **Action Prompt (Dependency Update):** `No problematic dependency to update. Optionally add cargo-deny (advisories + licenses + bans) to CI to make the current clean posture an enforced gate rather than an incidental property.`
  - **✅ Addressed (2026-07-01, #94):** `deny.toml` now enforces advisories,
    yanked crates, license allowlisting, bans, and source restrictions, and CI
    runs `cargo deny check` as a dedicated supply-chain job.

---

## 5. Strategic Synthesis & Action Plan (Strategist View)

### 5.1 Combined Health Score: **8.5/10**

A genuinely high-quality early-alpha: disciplined, defensive, exhaustively tested, minimal-dependency, with a contract-graph gate (`cargo xtask contract-check`) tying 21 topic shelves to executable evidence. It loses points only for onboarding surface (no library façade / CLI `--help` / quickstart) and for deferred module/crate boundaries — both cheap to fix pre-1.0.

### 5.2 Strategic Fix (highest leverage, improves DX **and** IQ)

**Introduce the `edict_syntax::check(&str)` façade and route the CLI through it.** It removes the library's biggest TTV barrier (DX), gives the project a small stable surface to document (DX), kills the parse→validate duplication between CLI and library (IQ), and unlocks in-process testing of CLI behavior (IQ). One change, four wins.

### 5.3 Mitigation Prompt (Strategic Priority)

- **Action Prompt:** `Add a public check(source: &str) -> CheckOutcome to crates/edict-syntax (parse_module + validate_surface, returning a structured outcome covering parse failure, semantic failures, and success), re-export it from lib.rs with a runnable rustdoc example, and refactor crates/edict-cli/src/main.rs::check_sources to call it so the parse→validate sequence has a single owner. Add in-process unit tests for the façade, keep the fixtures/cli/ golden replay green to prove the CLI's external contract is unchanged, and add a README "Using the library" snippet that uses the new function. Run cargo xtask verify.`
- **✅ Addressed (2026-06-29, #104):** done exactly as prompted — `check`/`CheckOutcome` added, CLI routed through it, in-process façade tests, README "Using the library" snippet, golden corpus green, `cargo xtask verify` green.
