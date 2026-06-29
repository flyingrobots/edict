---
report_id: "AUD-2026-06-28-V02"
title: "Documentation Quality Audit: edict README, topic shelves, and standard repo docs"
status: "Final"
audit:
  date_started: 2026-06-28
  date_completed: 2026-06-28
  type: "Full"
  scope: "README.md, docs/**, CONTRIBUTING.md, crate rustdoc, standard community files"
  compliance_frameworks: ["Keep a Changelog", "Standard OSS repo hygiene", "flyingrobots house rules"]
target:
  repository: "github.com/flyingrobots/edict"
  branch: "main"
  commit_hash: "d607605"
  language_stack: ["Markdown", "Rust rustdoc", "Mermaid"]
  environment: "Docs-as-contract (docs/topics/ verified by cargo xtask contract-check)"
methodology:
  automated_tools: ["markdownlint-cli2", "cargo xtask contract-check", "cargo test --doc"]
  manual_review_hours: 2
  false_positive_rate: "n/a"
summary:
  total_findings: 8
  severity_count:
    critical: 0
    high: 2
    medium: 4
    low: 2
  remediation_status: "Pending"
related_reports:
  previous_audit: null
  tracking_ticket: "n/a"
---

**Subject:** the project's documentation surface: a 584-line `README.md`, an
unusually rigorous `docs/topics/` system (21 topic shelves whose prose is gated
against code by `cargo xtask contract-check`), full language/ABI/bundle/admission
specs under `docs/`, dated release notes under `docs/releases/`, and crate-level
rustdoc.

> **Standout strength.** Edict treats internal docs as a *contract*: every topic
> shelf must cite executable evidence (tests/fixtures), broken links fail the
> build, and a release cannot cut without a topic-shelf coverage/accuracy audit
> (RELEASE-REQ-016). This is far above typical OSS hygiene. The findings below
> are almost entirely about the **outward / onboarding** face, not the internal
> contract docs.

---

## 1. Accuracy & Effectiveness Assessment

### 1.1 Core Mismatch (most critical inaccuracy)

The hero **"Edict in 10 Seconds"** Mermaid diagram (`README.md:7`) presents the
*envisioned end-to-end architecture* as the product: `Compile & Prove → Cryptographic
Seal (Core IR + Hash-Locked Manifest) → Participant Admission ("Nutrition Label")
→ WASM Sandbox (enforced limits & auto-rollback)`. At `v0.10.0-alpha.1` the
implementation is the **front-end + `edict check`** only — there is no runtime,
no WASM sandbox, no admission *execution*, and no bundle *assembly*. The later
"What Edict Is Not" and "Current Status" sections do disclaim this, but the
first thing a reader sees overstates what runs today. This is an
expectation/accuracy gap, not a falsehood — it needs an *implemented-vs-envisioned*
marker.

### 1.2 Audience & Goal Alignment

Two distinct audiences are served unevenly:

- **Conceptual reader / evaluator** (why does Edict exist?) — **served very well.**
  The narrative, the intent example, the "failure is a typed outcome" framing,
  and the AION pointer answer "what is it / why / how does it fit".
- **Hands-on developer** (how do I run it?) — **not served.** Their top-3
  questions — *How do I build/install it? How do I run `edict check`? What does a
  request/response look like?* — are answered **nowhere**. There are zero
  `cargo build`/run instructions in `README.md` or `CONTRIBUTING.md`, and the
  binary name `edict` and its stdin JSONL contract appear only in
  `docs/topics/cli/`, which the README does not link from an onboarding context.

### 1.3 Time-to-Value (TTV) Barrier

The dominant bottleneck is the **absence of any runnable path**. Because
`publish = false`, there is no `cargo install edict`; because there is no build
or usage section, a developer who finishes the README has no command to type.
The CLI's JSONL request shape lives in `docs/schemas/` + `docs/topics/cli/` but
is never surfaced as a copy-pasteable example.

---

## 2. Required Updates & Completeness Check

### 2.1 README.md Priority Fixes (top 3)

1. **Mark the hero diagram as a vision/roadmap view** (or add a parallel
   "Shipping today" line): label which stages exist at the current alpha
   (Write Intent → Compile & Validate via `edict check`) versus envisioned
   (Seal/Admission/Sandbox), so the first impression matches `Current Status`.
2. **Add a "Build & Run" / "Using the CLI" section**: how to build the binary
   (`cargo build -p edict-cli` → `target/debug/edict`), a copy-pasteable
   `edict check` invocation (JSONL settings + source on stdin), the expected
   JSONL stdout/stderr, and the exit-code table. Link `docs/topics/cli/` and the
   `docs/schemas/` contracts.
3. **Add a "Using the library" snippet**: the minimal `edict_syntax` call to
   parse + validate a source string (ideally the new `check` façade proposed in
   the code-quality audit), so the README has at least one runnable code path.

### 2.2 Missing Standard Documentation

Confirmed **absent** at repo root: **`SECURITY.md`**, **`NOTICE`**,
`CODE_OF_CONDUCT.md`, and `ARCHITECTURE.md`. Per the flyingrobots house rules
for new repositories, `SECURITY.md` and `NOTICE` are *expected* and their
absence is a genuine gap; `CONTRIBUTING.md` and `LICENSE` are present. The two
highest-priority additions:

- **`SECURITY.md`** — vulnerability-reporting contact/process. (House-rule
  expectation; also standard for a security-positioning project.)
- **`NOTICE`** — Apache-2.0 attribution companion to `LICENSE`. (House-rule
  expectation for Apache-2.0 repos.)

Secondary: `CODE_OF_CONDUCT.md`; and an **`ARCHITECTURE.md`** (or a README link
to one) since the layering — `edict-syntax` (front-end+compiler), `edict-cli`,
`xtask` — and the crate-scope sprawl noted in the code-quality audit are not
explained in one map for new contributors.

### 2.3 Supplementary Documentation (undocumented complex area)

The **canonical encoder + Core digest** (`crates/edict-syntax/src/canonical.rs`,
905 LOC: `encode_canonical_cbor`, `digest_core_module`, the
`edict.canonical-cbor/v1` framing and `edict.core.module/v1` domain-separated
SHA-256) is the most consequential area with the thinnest narrative docs. It is
the byte/hash foundation everything else will be admitted against, it has a
`core-ir` topic shelf and golden fixtures, but no prose walkthrough of the
encoding rules, domain-separation rationale, or the "meaning freezes before
bytes; bytes freeze before hashes" discipline that the release gates reference.
A dedicated explainer would de-risk every future change to that file.

---

## 3. Final Action Plan

### 3.1 Recommendation Type: **A — Incremental updates**

The documentation is structurally strong and largely accurate; it needs
*additive* accuracy markers, an onboarding path, and the missing standard files —
not a rewrite. A rewrite would destroy genuinely good conceptual content.

### 3.2 Deliverable (Incremental Update Prompt)

- **Action Prompt:** `Apply incremental documentation fixes to the edict repo without rewriting existing conceptual content: (1) In README.md, annotate the "Edict in 10 Seconds" diagram to distinguish shipping-today stages (Write Intent, Compile & Validate via 'edict check') from envisioned stages (Seal, Admission, WASM Sandbox), keeping it consistent with the Current Status section. (2) Add a "Build & Run" section with cargo build -p edict-cli, a copy-pasteable edict check JSONL example, expected stdout/stderr, and the 0/1/2 exit-code table, linking docs/topics/cli/ and docs/schemas/. (3) Add a "Using the library" rustdoc-backed snippet. (4) Create SECURITY.md (vulnerability reporting), NOTICE (Apache-2.0 attribution), and CODE_OF_CONDUCT.md at repo root. (5) Add a runnable crate-level example to crates/edict-syntax/src/lib.rs and ensure cargo test --doc passes. Run markdownlint-cli2 and cargo xtask verify before finishing.`

### 3.3 Mitigation Prompt (ready to execute next)

- **Action Prompt:** `In the edict repo, create the two house-rule-required standard files first as a self-contained change: SECURITY.md describing how to privately report a vulnerability and the supported-versions policy for the alpha train, and NOTICE containing the Apache-2.0 attribution for "Edict — part of the Continuum project, Copyright flyingrobots". Then add a top-level "Build & Run" section to README.md with a working 'edict check' example verified against the fixtures/cli/01-source-ok request. Run markdownlint-cli2 on changed files and cargo xtask verify; open a PR titled "docs: add Build & Run, SECURITY.md, and NOTICE".`
