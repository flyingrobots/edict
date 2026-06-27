# Project History And Orientation

This document is the narrative front door for Edict. It explains what Edict is
trying to solve, how the repository reached its current form, how the pieces fit
together, and how contributors should think about roadmap direction.

It is not a specification, roadmap, or topic shelf. The normative contracts
remain in `docs/topics/`, the release plan remains in `ROADMAP.md`, and release
evidence remains in `docs/releases/`.

Use this file when you need to answer:

- what Edict is for;
- how the system works;
- which surfaces exist today;
- which claims are still future work;
- where contributors can help without breaking the architecture;
- how to choose the next roadmap slice.

The history below is based on mainline Git history, release notes,
`CHANGELOG.md`, `ROADMAP.md`, topic shelves, and the durable Think memories from
Claude and Codex. Think is useful context, but the repo remains the source of
truth.

## What Edict Is Trying To Solve

Edict exists to turn an operation's authority into a compiler-visible contract.

Ordinary functions run with the ambient authority of their host process. A
function named `readThread` can still reach any table, make network calls, write
files, or mutate data if the process can. The function name, TypeScript type,
comment, test fixture, or code review convention may describe a narrower
authority, but the runtime does not prove that the implementation stays inside
that authority.

That mismatch matters for normal software. It matters more for autonomous
systems. When an agent is handed a callable tool, the real capability is not the
tool's friendly name. The real capability is everything the underlying code and
process can reach.

Edict's answer is a restricted deterministic language for declared operations.
An Edict program states what it reads, what it writes, how it can fail, which
budget governs it, and which imported law or target facts it depends on. The
project goal is to make those claims inspectable before an operation is admitted
for execution.

## Five-Minute Mental Model

Edict is a ladder of contracts. Each rung owns a different kind of truth.

```text
Edict source
  human-authored intent, declared authority, typed failures

Compiler spine
  proves accepted source can become typed Core for the supported subset

Core IR
  runtime-neutral meaning

Canonical bytes and digests
  stable identity for reviewed Core meaning

Target profiles and lowerability
  facts that say whether Core meaning can run on a specific target

Authority fact provenance
  evidence for who authored, reviewed, revised, and digest-bound those facts

Contract bundle
  participant-neutral package of artifacts, hashes, and evidence references

Continuum admission
  participant-specific acceptance, policy epoch, and capability evidence
```

The most common contributor error is to collapse rungs. Target facts do not
belong in Core. Admission artifacts do not belong in participant-neutral
bundles. Future v2 adapter design does not belong in current-truth topic
READMEs. README claims should not run ahead of executable evidence.

Current implementation coverage is deliberately narrower than the long-term
vision:

```text
Implemented today:
parse
  -> validate_surface
  -> resolve_module
  -> type_check
  -> lower_core
  -> canonical Core bytes and digest fixtures
  -> lowerability checks
  -> bundle and admission-boundary validation
  -> editor-facing syntax tooling

Not implemented yet:
file-backed lawpack and target fact loading
trusted lawpack and target-profile authorship workflow
full effectful source lowering
target IR generation
runtime execution
participant policy execution
compiler CLI and language server
```

## Glossary

Intent
: The source-level operation Edict compiles. It declares inputs, output,
  profile, budget, basis, constraints, and body.

Profile
: The operation-mode claim, such as read-only behavior. Today the compiler can
  check known profile/effect write-class conflicts when explicit context facts
  are supplied.

Lawpack
: A digest-locked package of domain law: types, pure helpers, semantic effect
  signatures, obstruction payloads, obligations, and adapter facts. Full
  file-backed lawpack loading is not implemented yet.

Target Profile
: A digest-locked description of target runtime capabilities and lowering
  facts. Edict validates target-profile manifests and lowerability facts, but
  does not yet execute target lowerers.

Authority Fact Governance
: The planned workflow for authoring, reviewing, revising, auditing, and
  trusting lawpack and target-profile facts. Edict should validate provenance,
  digest binding, review references, and artifact shape; Continuum and
  participants should decide trust policy and acceptance.

Core IR
: Edict's runtime-neutral semantic representation. Core is meaning, not target
  execution and not participant admission.

Lowerability
: The v1 check that classifies whether required Core obligations are satisfied
  natively, by exactly one direct adapter, or unsupported. Chained and composite
  adapter search is future v2 work.

Contract Bundle
: A participant-neutral artifact package tying source, Core, target profile,
  target IR references, digests, and optional assurance evidence together.

Assurance Evidence
: Optional hash-bound references to HOLMES, Watson, and Moriarty evidence at the
  bundle boundary. Edict validates references; it does not yet implement those
  tools.

Admission
: The participant-specific acceptance boundary. Edict validates its own Gate C
  request, receipt, operation-requirement, and invocation evidence shapes.
  Continuum owns participant policy, identity, delegation, and revocation.

Topic Shelf
: A current-truth contract chapter under `docs/topics/`. A shelf README says
  what is true in `HEAD`; its test plan says how that truth is verified and
  where gaps remain.

RED/GREEN
: The local development discipline for nontrivial behavior changes: write a
  failing behavior test first, make the smallest coherent fix, then verify the
  relevant suite.

Current Truth
: Behavior or policy that is true in this branch now. Topic README files are
  current truth, not wish lists.

## Origin

The repo began as a design and specification packet. Early mainline commits
seeded the Edict specifications, clarified the Edict/Continuum boundary, framed
intents as optics, and added the README primer. Claude's earliest remembered
orientation described the design as a restricted deterministic language whose
core path was source to Core IR to target-profile lowering to a
participant-neutral bundle to Continuum admission.

That first phase was intentionally documentation-heavy. It established the
language ambition before there was a real compiler:

- Edict owns the language and artifact contracts.
- Continuum owns participant admission and policy vocabulary.
- Core IR is runtime-neutral.
- Target profiles and lawpacks provide external facts rather than hidden
  ambient authority.
- Unsupported lowering is a compiler error, not a silent approximation.

The project then pivoted from promising language to executable contract graph.
That pivot is visible in the alpha release train.

## The Alpha Train

### v0.1.0-alpha.1: Front-End Alpha

The first alpha turned the language front end into a real Rust crate:
`edict-syntax`. It added a deterministic lexer, recursive-descent parser,
source AST, source-level semantic validation, syntax and semantic-validation
topic shelves, and the local `cargo xtask verify` gate.

The release notes were explicit about the boundary: this was not yet Core
lowering, target lowering, canonical hashing, admission, or crates.io
publication. That precision became a recurring project habit.

### v0.2.0-alpha.1: Core Semantic Model And Schema

The second alpha established `edict.core/v1` as a semantic model and normative
schema boundary. It added Core IR topic material, CDDL checks, and release
process scaffolding.

The important move was conceptual: Core meaning was allowed to freeze before
canonical bytes or hashes. The roadmap later summarized that discipline as:

```text
meaning freezes before bytes;
bytes freeze before hashes;
hashes freeze before admission.
```

### v0.3.0-alpha.1: Compiler Spine And Canonicalization

The third alpha made the source-to-Core path executable for a narrow subset. It
added explicit compiler stages:

```text
validate_surface
  -> resolve_module
  -> type_check
  -> lower_core
  -> canonicalize
```

This release also added the canonical CBOR encoder, domain-separated Core
module digest computation, reviewed golden bytes, exact digest fixtures, and
`cargo xtask core-goldens`.

Several review-driven fixes hardened the canonical contract:

- unresolved import digests cannot enter the canonical preimage;
- source import alias spelling does not affect Core bytes;
- imports and input constraints are sorted before encoding;
- source binder spelling is excluded from canonical local references;
- compiler-owned local IDs remain part of Core byte identity;
- oversized CBOR declared lengths are rejected before allocation.

This is where the project became more than a parser. It could compile a small,
pure, local-record subset into in-memory Core and prove canonical byte behavior
for that Core model.

### v0.4.0-alpha.1: Lowerability, Target Profiles, And Bundles

The fourth alpha added the v1 lowerability and packaging boundary:

- typed `LoweringRequirements`;
- typed `TargetProfileFacts`;
- direct-only v1 lowerability classification;
- target-profile manifest conformance checks;
- participant-neutral contract-bundle manifest validation;
- optional hash-bound assurance evidence references.

It also deliberately rejected the tempting overreach: chained adapters,
composite profile behavior, floating adapter references, and ambiguous support
facts were kept out of v1. Future adapter composition was separated into a v2
design track instead of being smuggled into the v1 checker.

This release made a clear architectural distinction. Edict can validate
participant-neutral bundle and lowerability facts, but it does not yet execute
target lowerers or admit operations.

### v0.5.0-alpha.1: Bundle And Admission Boundary

The fifth alpha added Edict-owned Gate C admission-boundary checks:

- admission requests;
- admission receipts;
- invocation capabilities;
- request and receipt digest binding;
- operation requirement checks;
- hidden execution input rejection;
- scope and participant matching.

The release kept participant policy, identity, delegation, and revocation on
the Continuum side of the boundary. That boundary matters: Edict validates the
artifact and invocation evidence it owns; Continuum remains the protocol and
policy layer for participant decisions.

The release process itself also matured here. Automation around release tags,
milestone readiness, release workflow dispatch, and non-mutating recovery was
tested and hardened after review.

### v0.6.0-alpha.1: Developer Tooling Alpha

The sixth alpha made the language inspectable in editors:

- public highlight roles;
- deterministic highlighting fixtures;
- Tree-sitter grammar source and corpus;
- TextMate grammar scopes;
- a thin VS Code/Cursor extension package;
- fixture, lawpack, and assurance topic shelves;
- a release-prep topic-shelf audit gate requiring at least 90% coverage and
  90% accuracy.

The release notes again kept the boundary honest. This was not a compiler CLI,
language server, marketplace publication, target lowerer, or admission tooling
release.

## Post-v0.6 Correction And Design Work

After v0.6, the repository had three important follow-up moves.

First, v2 adapter composition was documented as future design rather than
current truth. The v2 work now lives in a non-topic design note, with the
`docs/topics/v2-design/` shelf describing only the current HEAD boundary and
planned evidence. Review feedback forced this split to stay clean.

Second, the compiler spine was corrected to enforce a claim the README had been
approaching too aggressively. Edict now rejects known profile/effect write-class
conflicts from deterministic compiler context facts. A read-only profile with a
known write-class effect fails with stable `ProfileEffectMismatch`.

That correction matters because it moves one of Edict's central value
propositions from aspiration toward executable behavior. It is still bounded:
the compiler can check profile/effect compatibility only when the caller
supplies explicit in-memory facts. Loading those facts from target and lawpack
files remains future work.

Third, the roadmap gained a named Authority Fact Governance design track. That
track exists because the compiler can prove only against facts it receives. Once
lawpacks and target profiles supply profiles, budgets, write classes, effects,
obligations, adapters, and target capabilities, Edict must make fact authorship,
review, revision, and provenance inspectable. The design track does not block
the next file-backed fact-loading release, but it prevents the project from
mistaking "some JSON said so" for authority.

## What Exists Today

As of `main` at merge commit `821f610`, Edict has these landed surfaces.

### Language Front End

`edict-syntax` provides a deterministic lexer, parser, source AST, and
source-level semantic validation for the landed source subset. The parser
covers packages, imports, records, refined scalars, enums, variants, intents,
clauses, pure and effectful statements, bounded loops, conditionals, matches,
digest literals, and related fixtures.

### Compiler Spine

The compiler spine exposes:

- `validate_surface`;
- `resolve_module`;
- `type_check`;
- `lower_core`;
- `compile_to_core`.

It lowers the initial pure local-record subset to in-memory Core IR. It also
checks known profile/effect write-class compatibility from deterministic
compiler context facts before Core lowering.

The lowerer does not embed canonical bytes, exact digests, target IR, or
admission artifacts into Core modules.

### Core IR And Canonicalization

The repo has `edict.core/v1`, a normative CDDL schema, a reference canonical
CBOR path for the current in-memory Core module model, reviewed Core golden
bytes, and exact digest fixtures.

### Lowerability And Target Profiles

The v1 lowerability checker validates native support, exactly one compatible
direct adapter, or unsupported status. Composite and chained adapter behavior is
rejected in v1 and tracked as future v2 design.

Target-profile manifest validation checks runtime-neutral Echo and KV profile
conformance, digest-locked components, accepted Core ABI, and atomic application
semantics.

### Contract Bundles And Assurance

The bundle validator checks participant-neutral contract-bundle manifests,
logical source paths, digest-locked artifacts, canonicalization profile pinning,
release-only provenance input binding, and optional HOLMES, Watson, and
Moriarty evidence references.

### Admission Boundary

The Gate C admission-boundary checks validate Edict-owned request, receipt,
capability, and invocation evidence. They intentionally stop before Continuum
participant policy.

### Developer Tooling

Editor-facing tooling exists for lexical visibility:

- stable highlight roles;
- Tree-sitter grammar artifacts;
- TextMate grammar artifact;
- thin VS Code/Cursor extension package.

These are syntax-highlighting surfaces, not a full language-server or formatter
stack.

### Documentation And Verification Infrastructure

The repo now has 16 topic shelves under `docs/topics/`. Each shelf records
current HEAD truth and a verification matrix. `cargo xtask contract-check`
validates the contract graph, and `cargo xtask verify` is the local full gate.

The release process is also part of the product. Release tags must point at
`main`, release notes must state explicit non-goals, and topic-shelf coverage
and accuracy are audited before release.

Future design notes live outside the topic shelves when the project needs to
record planned direction without claiming landed behavior. Authority Fact
Governance and v2 obligation closure currently use that pattern.

## Contributor Reading Path

If you have 15 minutes:

- read the current status and "Where To Go Next" sections in `README.md`;
- read this document through "What Exists Today";
- scan `ROADMAP.md`.

If you have one hour:

- read `docs/topics/README.md`;
- read `docs/topics/compiler-spine/README.md`;
- read `docs/topics/core-ir/README.md`;
- read `docs/topics/lowerability/README.md`;
- read `docs/topics/release-process/README.md`.

If you want to change behavior:

- read `AGENTS.md`;
- identify the owning topic shelf before editing;
- update the topic `test-plan.md` before or alongside the test;
- write a deterministic RED behavior test;
- implement the smallest coherent GREEN change;
- run `cargo xtask verify`;
- keep README and topic README claims behind executable evidence.

## Roadmap Decision Frame

Roadmap slices should be chosen by asking whether they move Edict from declared
authority toward compiled or verified authority. The roadmap now has one more
test: when a slice depends on lawpack or target-profile facts, it must ask who
authored those facts, who reviewed them, how revisions are bound, and which
trust decisions belong outside Edict.

A strong roadmap slice should:

- reduce the gap between a public claim and executable enforcement;
- move future behavior into a topic-shelf test plan with evidence;
- preserve layer boundaries between source, Core, target facts, bundles, and
  admission;
- unlock downstream work without overclaiming the current surface;
- add reusable fixtures, stable error kinds, schema checks, or contract checks;
- retire a known gap rather than only adding new vocabulary.

Near-term candidate directions:

- File-backed facts: load operation profiles, budgets, write classes, and
  effect facts from lawpack or target artifacts instead of caller-supplied
  in-memory context.
- Authority fact governance: define how lawpacks and target profiles are
  authored, reviewed, revised, audited, and trusted without making Edict a
  participant-policy engine.
- Effectful source lowering: compile handled effect bodies into Core rather
  than only rejecting incompatible known effects.
- Target IR generation: move from lowerability checks to target-specific output
  for an initial runtime.
- Compiler CLI: expose the existing compiler spine and validation surfaces as a
  user-facing tool.
- Language server or editor diagnostics: build on the v0.6 tooling alpha.
- Admission execution path: connect bundle and admission-boundary validation to
  a runnable flow.
- v2 adapter composition: continue design work, but only after the v1 boundary
  remains stable enough to support it.

## Gap Register

| Gap | Why It Matters | Likely Owning Topics |
| --- | --- | --- |
| File-backed target and lawpack facts | Current profile, budget, and effect checks depend on caller-supplied context. | compiler-spine, lawpacks, target-profiles |
| Trusted lawpack and target-profile authorship | File-backed facts are not enough; participants need inspectable provenance for who authored, reviewed, revised, and digest-bound authority claims. | lawpacks, target-profiles, assurance |
| Effectful source lowering | Edict can reject some bad effects but cannot compile most useful effectful intents. | compiler-spine, core-ir |
| Target IR generation | Lowerability is validation, not execution. | lowerability, target-profiles |
| Compiler CLI | Existing compiler APIs are not yet a user-facing tool. | compiler-spine, developer-tooling |
| Language-server diagnostics | Editor syntax visibility exists, but semantic feedback does not. | developer-tooling, semantic-validation |
| Admission execution | Boundary checks exist, but no full admission workflow runs. | admission, release-process |
| Publication policy | Crates remain unpublished and versions track maturity, not API stability. | release-process |
| v2 adapter composition | Future composition rules are designed but not implemented. | v2-design, lowerability |

## Claims Ladder

Contributors should be precise about which rung a claim has reached:

1. Named in vision prose.
2. Tracked as planned work.
3. Described in a non-topic design note.
4. Listed as a topic-shelf gap.
5. Specified in a topic test plan.
6. Backed by executable evidence.
7. Released with explicit non-goals.

Do not write as if a claim is on rung 6 when it is still on rung 2. A major
part of Edict's credibility is preserving that distinction.

## Contributor Guardrails

Do not:

- put future behavior in topic README files;
- make README claims before executable evidence exists;
- write tests that assert documentation phrasing, repository layout, or
  implementation details instead of software behavior;
- treat unsupported lowering as approximate lowering;
- let target, lawpack, bundle, or admission facts leak into the wrong layer;
- solve v2 adapter composition inside v1 lowerability;
- move, delete, recreate, or force-push release tags as a recovery strategy;
- add ceremonial documentation with no contract impact.

Do:

- make gaps explicit;
- add stable error kinds or structured validation artifacts where callers need
  to depend on failures;
- cite topic-shelf requirement IDs when changing contracts;
- keep release notes specific about included behavior and non-goals;
- prefer small, reviewable slices with RED/GREEN evidence.

## What Is Yet To Be Done

The project has strong contract discipline, but the executable surface is still
intentionally narrow.

### Load Real External Facts

The compiler can use explicit in-memory context facts for profiles, budgets,
and effect write classes. It does not yet load those facts from file-backed
target profiles, lawpacks, or shape schemas.

### Govern Authority Facts

File-backed facts introduce a trust problem. Edict must define how lawpacks and
target profiles are authored, reviewed, revised, audited, and digest-bound. The
planned boundary is narrow: Edict validates provenance shape, review evidence,
digest binding, compatibility, and structured conflicts; Continuum and
participants decide trust policy, identity, delegation, revocation, and
acceptance.

### Expand Source-To-Core Lowering

The lowerable subset is intentionally narrow. Full source language lowering,
effectful lowering, obstruction exhaustiveness, richer expression support,
branch-yield lowering, and loops remain future compiler-spine work.

### Generate Target IR

Lowerability checks facts, but Edict does not yet execute a target lowerer or
produce target-specific IR for Echo, KV, or any other runtime.

### Execute Admission

The repo validates admission-boundary artifacts, but it does not provide full
admission execution tooling, participant policy evaluation, capability
delegation, or revocation.

### Complete Tooling

There is no compiler CLI, language server, formatter, semantic-token service,
editor diagnostics, marketplace publication, or published Tree-sitter package.

### Stabilize Publication Policy

Workspace crates remain `publish = false`. Versions currently track
specification maturity, not crates.io API stability.

### Follow The Next Alpha Train

The next conceptual train starts with `v0.7.0-alpha.1` file-backed authority
facts and opens the Authority Fact Governance design track. The keeper roadmap
then moves through minimal effectful compiler lowering, first target IR, public
CLI diagnostics, contract-bundle assembly, an admission workflow harness,
trusted lawpack and target-profile authorship, publication policy, and
language-server diagnostics. The v1 target is one honest source-to-admission
slice with trusted fact provenance visible and digest-bound.

## Primary Evidence

- `README.md`: public project introduction and current status.
- `ROADMAP.md`: scheduled alpha milestones, release gates, and artifact map.
- `CHANGELOG.md`: release-by-release change history.
- `docs/topics/README.md`: current topic-shelf index.
- `docs/releases/`: published alpha release notes.
- `docs/topics/compiler-spine/README.md`: current compiler-spine contract.
- `docs/topics/release-process/README.md`: release automation and runbook
  contract.
- `docs/design/authority-fact-governance.md`: future trusted fact authorship
  design note.
- `docs/design/v2-obligation-closure.md`: future v2 adapter composition design.

## The Shape Of The Project Now

Edict is no longer only a speculative language design. It is a Rust workspace
with executable slices for syntax, surface validation, Core, canonicalization,
lowerability, bundles, admission-boundary checks, and editor-facing syntax
tooling.

It is also not yet the full end-to-end system promised by the README's long-term
vision. The current truth is narrower and stronger: Edict has a mechanically
checked contract graph and a disciplined alpha train. Its next phase should keep
that discipline while closing the largest remaining gaps between declared
authority, trusted facts, and compiled enforcement.
