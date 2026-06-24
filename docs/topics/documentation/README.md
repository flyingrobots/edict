# Documentation Standards Topic

Status: current HEAD workflow contract.

Documentation is a product interface. Its job is to help a specific reader do a
specific task, not to prove that the repository contains enough Markdown.

This policy adapts the reusable
`/Users/james/git/DOCUMENTATION_STANDARDS.md` reader-task standard to Edict's
current repository shape. It does not mass-convert existing pages. It gives new
and changed documentation a direction that fits the topic-shelf, fixture, spec,
and release workflow already in this repository.

## Reader jobs

Each documentation page should have one primary reader job. A page may link to
other jobs, but it should not try to be all of them at once. [DOCS-REQ-001]

Edict uses these page types:

- tutorial: a guided first-success path that teaches by doing;
- how-to: a task guide for a competent reader trying to finish one job;
- reference: exact facts such as syntax, schema fields, release gates,
  requirement IDs, fixtures, commands, and error identities;
- explanation: mental models, mechanisms, tradeoffs, and design boundaries;
- troubleshooting: symptom-led diagnosis and recovery;
- contributor: architecture, edit paths, invariants, impact rules, and
  verification evidence.

Topic shelves in `docs/topics/` are contributor and evidence material first.
They may contain explanation or reference facts, but they are not a substitute
for user-facing tutorials or task guides when Edict gains directly operated
surfaces such as a CLI. [DOCS-REQ-002]

## Edict documentation shape

Edict's current documentation set has these roles:

- [README.md](../../../README.md): product landing page and high-level
  explanation.
- [docs/README.md](../../README.md): documentation router.
- `docs/SPEC_*.md`: normative protocol, language, ABI, and admission reference.
- [docs/REQUIREMENTS.md](../../REQUIREMENTS.md): requirement and fixture
  registry.
- `docs/topics/*`: current-truth contributor/evidence shelves.
- `docs/releases/*`: versioned release notes.
- [CONTRIBUTING.md](../../../CONTRIBUTING.md): contributor entry point.
- [AGENTS.md](../../../AGENTS.md): agent workflow entry point.

New pages should extend this structure by reader job. Do not add a new omnibus
README when a focused tutorial, how-to, reference, explanation,
troubleshooting, or contributor page is the clearer shape. [DOCS-REQ-001]

## Documentation Coverage Matrix

Coverage is selected by capability. "Not needed" requires a reason; it does not
require an empty placeholder page. [DOCS-REQ-003]

| Capability | Audience | First success | Task guides | Reference | Explanation | Troubleshooting | Contributor |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Edict language | user, integrator, maintainer | planned when a runnable compiler surface exists | planned around compile/canonicalize tasks | current specs and requirement registry | current README and assurance guide | gap until user-facing failures exist | current topic shelves |
| Syntax and surface validation | maintainer, integrator | not needed: library surface is narrow and test-driven | optional parser/validator recipes | current syntax and semantic-validation shelves | current topic READMEs | gap for diagnostics beyond stable error kinds | current test plans and fixtures |
| Compiler spine and Core IR | maintainer, integrator | planned for source-to-Core/canonicalization once #21 lands | planned for canonicalize and inspect workflows | current compiler-spine and Core IR shelves | current topic READMEs and specs | gap for compiler-stage diagnostics | current test plans and CDDL fixtures |
| Release process | maintainer | not needed: release is operator workflow, not newcomer product use | current release process topic and release notes | current release workflow and roadmap | current release-process shelf | current tag recovery policy, limited to known release failures | current release-process test plan |
| Contributor workflow | maintainer, agent | current `CONTRIBUTING.md` and `AGENTS.md` orientation | current testing and documentation workflow topics | current local verification commands | current topic-shelf policy | gap for local environment failures | current `xtask` evidence |

When a capability gains a CLI, public API, generated reference, visual UI, or
operational failure mode, update this matrix before claiming the documentation
set is complete. [DOCS-REQ-003]

## Examples

Examples are contract material. Runnable examples must be valid, use supported
behavior, include required context, and show expected output when the output is
part of the reader's task. Illustrative examples must be labeled or clearly
framed as illustrative. Abridged examples must explain what was omitted when the
omission could affect interpretation. [DOCS-REQ-004]

Copyable shell commands must not include prompts. Present commands and output as
separate blocks. Use `text` for exact or representative output, and say which
parts vary when output is nondeterministic. [DOCS-REQ-004]

Use safe fictional values such as `example.com`, `example-target`, and
`test_token_example`. Placeholder digest prose such as `sha256:...` is useful in
explanations, but runnable Edict fixtures must use lexable digest strings. The
fixture corpus documents that distinction. [DOCS-REQ-004]

## Change impact

Documentation must move with behavior. A contract-bearing change must do one of
the following before it is claimed complete:

1. update affected documentation;
2. show that existing documentation remains accurate;
3. declare `docs-impact: none` with a concise rationale.

Changed documentation must preserve page type. Do not turn a reference page into
a tutorial, or a topic-shelf contributor page into a marketing page, as an
incidental side effect. [DOCS-REQ-005]

Do not copy live issue lists, pull request lists, CI timestamps, or dashboards
into prose as current truth. Link to live systems or use generated artifacts
when those facts matter. [DOCS-REQ-005]

## Deterministic checks and editorial review

The local gate already checks links, topic metadata, evidence names, fixture
paths, doctests, Rust tests, formatting, and diff whitespace. Markdown lint is
available for changed Markdown files. These checks block on facts the repo can
determine reliably. [DOCS-REQ-006]

Human review is still required for reader-task quality. A page is not done only
because Markdown syntax passes. Review should ask whether the intended reader can
complete the page's primary job without source archaeology. [DOCS-REQ-006]

The verification matrix is tracked in [test-plan.md](./test-plan.md).
