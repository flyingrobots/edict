# Topic Shelves

Topic shelves are current reference material for landed subsystems. They are not
RFCs, proposals, or dated retrospectives.

Each topic uses this shape:

- `README.md`: what is true in HEAD.
- `test-plan.md`: how those truths are verified, including fixtures, oracles,
  implemented evidence, planned cases, and known gaps.
- `architecture.md`: optional machinery/dataflow notes when the structure earns
  a separate page.
- `rationale.md`: optional still-relevant tradeoffs, with historical decisions
  cited rather than duplicated.

The local contract graph is checked by:

```text
cargo xtask contract-check
```

Test-plan row statuses are `implemented`, `planned`, `gap`, and `policy`.
`policy` is reserved for human-review workflow contracts; it is not a substitute
for executable evidence when software behavior is at stake.

The full local gate is:

```text
cargo xtask verify
```

## Topics

- [Admission](./admission/README.md): typed Gate C admission-boundary checks
  for Edict-owned artifact and invocation evidence semantics.
- [Compiler Spine](./compiler-spine/README.md): executable source-AST to
  in-memory Core IR stage contract for the initial lowerable subset.
- [Contract Bundles](./contract-bundles/README.md): typed v1
  participant-neutral contract bundle and assurance evidence manifest
  validation.
- [Core IR](./core-ir/README.md): `edict.core/v1` semantic model and normative
  CDDL schema boundary for the Core contract.
- [Documentation Standards](./documentation/README.md): reader-task page types,
  documentation coverage, examples, and docs-impact rules.
- [Developer Tooling](./developer-tooling/README.md): editor-facing source
  highlighting roles, Tree-sitter grammar source, TextMate grammar scopes, and
  fixture-backed tooling behavior.
- [Lowerability](./lowerability/README.md): typed v1 lowering requirements,
  target-profile facts, and direct-only support classification.
- [Release Process](./release-process/README.md): tag-triggered GitHub Release
  publication contract, release runbook, and verification matrix.
- [Semantic Validation](./semantic-validation/README.md): source/surface
  `validate_surface` stage contract and verification matrix.
- [Syntax](./syntax/README.md): Phase 1 `edict-syntax` lexer/parser contract and
  verification matrix.
- [Target Profiles](./target-profiles/README.md): typed v1 target-profile
  manifest conformance and runtime-neutral profile validation.
- [Testing Workflow](./tests/README.md): RED/GREEN development discipline,
  fixture reuse, and local verification workflow.
