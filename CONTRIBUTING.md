# Contributing

Edict treats tests and fixtures as contract evidence. Before changing behavior,
public APIs, schemas, release workflow, validation rules, compiler stages, or
other durable project contracts, read the testing workflow topic:

- [Testing Workflow](docs/topics/tests/README.md)

Documentation changes follow the same contract-oriented model. Before adding or
rewriting docs, read the documentation workflow topic:

- [Documentation Standards](docs/topics/documentation/README.md)

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
