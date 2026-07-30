# Workspace Snapshot Lawpack Fixture

This generator-owned closure gives a typed external-action application the
profile, budget, and opaque target-configuration facts needed to construct a
bounded workspace-observation request.

It grants no callable semantic effect:

- `operationProfiles` declares `workspace.snapshot@1.observeRequest`;
- `semanticEffects` is empty;
- `budgetObligation` binds the compiler budget;
- `targetConfiguration` binds one exact request-profile resource;
- `effectImplementations` is empty.

`observe-workspace.edict` imports both the exact lawpack manifest and the
requestable `workspace.snapshot.observe@1` capability using the same manifest
digest. The generated Core and Target IR therefore preserve the complete
capability closure while Target IR contains one external-action request and
zero callable steps.

Artifacts are generated only through:

```sh
cargo xtask lawpack-goldens --write
```

Checked bytes are verified through:

```sh
cargo xtask lawpack-goldens --check
```

The fixture constructs request data only. It does not observe a workspace,
perform I/O, invoke a target intrinsic, admit a settlement, or resume an Edict
program.
