# Contributing

Edict treats tests and fixtures as contract evidence. Before changing behavior,
public APIs, schemas, release workflow, validation rules, compiler stages, or
other durable project contracts, read the testing workflow topic:

- [Testing Workflow](docs/topics/tests/README.md)

Documentation changes follow the same contract-oriented model. Before adding or
rewriting docs, read the documentation workflow topic:

- [Documentation Standards](docs/topics/documentation/README.md)

Rust changes must preserve the project safety and determinism contract. Before
changing public Rust APIs, validation behavior, compiler paths, dependencies, or
generated artifacts, read:

- [Rust Standards](docs/topics/rust-standards/README.md)

Release-prep work follows the repo-local release process, not a generic tag
flow. Before preparing a release branch, read:

- [Release Process](docs/topics/release-process/README.md)
- [Release Runbook](docs/topics/release-process/runbook.md)

The short rule is RED/GREEN:

1. Add or update the relevant topic-shelf `test-plan.md` first.
2. Write the deterministic test or fixture that describes the intended contract.
3. Run the narrowest command and observe it fail.
4. Implement the smallest coherent change that makes the test pass.
5. Run the narrow test again, then `cargo xtask verify` before claiming the
   branch is ready.

The short documentation rule is reader-task first:

1. Decide whether the page is a tutorial, how-to, reference, explanation,
   troubleshooting guide, or contributor guide.
2. Keep that primary job intact instead of making one page serve every reader.
3. Use runnable, illustrative, or abridged examples honestly and label the
   difference when it matters.
4. Update affected docs with behavior changes, or state `docs-impact: none` with
   a concise rationale.

Pull request bodies for issue work must include GitHub auto-close text such as
`Closes #123` for every issue the PR is intended to close.

The local verification gate is:

```text
cargo xtask verify
```
