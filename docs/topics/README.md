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

The full local gate is:

```text
cargo xtask verify
```

## Topics

- [Semantic Validation](./semantic-validation/README.md): Phase 2 source-AST
  semantic validation contract and verification matrix.
- [Syntax](./syntax/README.md): Phase 1 `edict-syntax` lexer/parser contract and
  verification matrix.
