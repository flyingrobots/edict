# AGENTS

## Git Rules

NEVER amend git commits. Make a new commit instead.

NEVER use `git rebase` unless the user explicitly approves a rare exception.
Use regular merge commits.

NEVER force any git operation. If a force operation appears necessary, stop and
explain what happened and what options remain.

NEVER create draft pull requests.

NEVER use a `codex` prefix in branch names, PR titles, or commit messages.

Pull request bodies for issue work MUST include GitHub auto-close text such as
`Closes #123` for every issue the PR is intended to close.

## Think

Think is durable memory for cross-session coordination.

- Use `codex-think --remember --json` when starting a new session, changing into
  this repository, or regaining context after a context shift.
- Use `codex-think "..." --json` when a cycle closes or a significant event
  should survive across turns.
- Treat Think as memory, not repo truth. Anchor strong claims back to files,
  commits, commands, issues, or pull requests.
- Claude memories are read-only. Use `claude-think --remember --json` only for
  additional context.

## Topic Shelves

`docs/topics/` contains the living contract graph for landed behavior. Topic
shelves are not proposals, retrospectives, or design archaeology.

Each shelf may contain:

- `README.md`: what is true in HEAD.
- `test-plan.md`: how those truths are verified, including requirements, cases,
  fixtures, oracles, implemented evidence, planned cases, and known gaps.
- `architecture.md`: optional structure or dataflow notes when the machinery
  earns a separate page.
- `rationale.md`: optional still-relevant tradeoffs and rejected approaches.

### When To Update Topic Shelves

For every nontrivial behavior, contract, workflow, release, schema, validation,
or public-surface change:

1. Identify the owning topic shelf before editing code.
2. If no shelf owns durable behavior, create one.
3. Update `test-plan.md` before or alongside tests with requirement IDs, case
   IDs, fixtures, and oracles.
4. Write executable evidence: deterministic tests, fixtures, doctests, or
   contract checks as appropriate.
5. Update the topic `README.md` only after behavior exists in the branch. The
   README describes current branch truth, not intended future behavior.
6. Mark planned cases as implemented only when executable evidence exists.
7. Run `cargo xtask verify` before claiming the shelf is current.

### When Not To Update Topic Shelves

Do not churn topic shelves for purely mechanical edits that do not change a
contract, such as formatting, typo fixes, dependency pin updates with no
observable behavior change, or internal refactors whose existing tests and topic
claims remain accurate.

When a change intentionally does not update a topic shelf, state why in the pull
request body or final report.

### Topic Shelf Discipline

- Topic `README.md` files must not describe intended behavior before it lands.
- `test-plan.md` may include planned cases and known gaps.
- `policy` rows are for human-review workflow contracts. They must not be used
  to avoid writing behavior tests for software behavior.
- Tests assert code behavior and stable contract artifacts, not prose.
- Negative tests should assert stable error kinds or structured artifacts, not
  merely `is_err()` or diagnostic text.
- Release, CI, and publication workflows count as behavior when they define a
  project contract.
- Avoid ceremonial documentation. Update shelves because the contract changed,
  not because a path changed.

## RED/GREEN Testing Discipline

Edict uses RED/GREEN test-driven development for nontrivial changes. The shared
contract lives in [docs/topics/tests/](docs/topics/tests/README.md).

For behavior, contract, workflow, release, schema, validation, or public-surface
changes:

1. Update the owning topic `test-plan.md` with planned requirement and case rows
   before or alongside the first test.
2. Write the deterministic test, fixture, doctest, or contract check before the
   implementation that makes it pass.
3. Run the narrowest relevant command and observe the RED failure.
4. Implement the smallest coherent change that turns that test GREEN.
5. Mark planned rows as implemented only after executable evidence exists.
6. Report the RED command and GREEN command in the final report or pull request
   body.

Tests must assert software behavior. Do not write tests that assert
implementation detail, documentation detail, or repository structure. Tests may
exercise documentation tooling behavior, such as a validator rejecting invalid
input, but they must not pass merely because prose contains a phrase or a file
appears at a particular path.

Do not use after-the-fact tests as a substitute for RED/GREEN. If a change is a
purely mechanical edit with no contract impact, state that no RED/GREEN cycle
was required.

## Documentation Standards

Documentation is a product interface, not a Markdown inventory. The shared
documentation policy lives in
[docs/topics/documentation/](docs/topics/documentation/README.md).

When creating or changing documentation:

- Give each page one primary reader job: tutorial, how-to, reference,
  explanation, troubleshooting, or contributor guidance.
- Keep user-facing task help separate from contributor architecture and evidence
  maps.
- Use concrete, valid examples and show expected results when the result matters.
- Put exact public facts in reference material and validate or generate them
  from authoritative sources when practical.
- Update affected documentation in the same change as behavior, schema, release,
  workflow, or public-surface changes, or state `docs-impact: none` with a
  concise rationale.

## Rust Standards

Rust engineering policy lives in
[docs/topics/rust-standards/](docs/topics/rust-standards/README.md).

For Rust changes:

- Preserve claim integrity: no public claim without executable evidence.
- Keep compiler and validation paths deterministic and free of hidden I/O.
- Prefer structured public failures with stable error kinds over prose-only
  diagnostics.
- Do not add dependencies without PR-body rationale and contract-impact notes.
- Treat planned lint, dependency, and fuzzing ratchets as planned until their
  executable checks land.

## Pull Request Review Policy

Review policy lives in
[docs/topics/review-process/](docs/topics/review-process/README.md).

- If CodeRabbit is actively reviewing, its approval is required before merge.
- If CodeRabbit is rate limited, out of credits, or reports insufficient usage
  credits, post `@codex review please` on the pull request and wait for the
  alternate review response.
- Do not treat CodeRabbit unavailability as approval. Without CodeRabbit
  approval or an alternate review response, merge is blocked unless a maintainer
  explicitly overrides the review-bot gate.

## Release Discipline

Release policy lives in
[docs/topics/release-process/](docs/topics/release-process/README.md).

For release-prep work:

- Write the release thesis before editing release artifacts.
- Reconcile the diff from the previous version tag before finalizing signposts.
- Update structured release policy and matching release-policy tests.
- Ensure the matching milestone has zero open issues before tag automation runs.
- Verify no crates.io publication happened unless publication policy changes.
- Capture a durable release report with released/not-released scope,
  plan-versus-actual notes, evidence, fallout issues, and the next release
  thesis.

## Local Verification

Use the local gate before claiming a branch is ready:

```text
cargo xtask verify
```
