# Workspace Patch Lawpack Fixture

This generator-owned closure defines the compiler side of one basis-bound
validated workspace patch request. It binds:

- `workspace.patch.applyValidated@1` as the requestable operation;
- `workspace.patch.input@1` as the canonical patch-input schema;
- the exact workspace-root basis and writable-path-policy authority classes;
- CI-workflow exclusion and canonical validated-patch policy identities;
- exact settlement and reconciliation resources;
- postcondition evidence as an exact resulting workspace root; and
- bounded request construction and settlement budgets.

The closure grants no callable write effect. Its operation profile has empty
`semanticEffects`, its adapter has empty `effectImplementations`, and generated
Target IR contains one external-action request with zero callable steps.
Edict constructs request data; it does not open, validate, or mutate a
workspace.

Artifacts are owned by:

```sh
cargo xtask lawpack-goldens --write
```

Checked bytes are verified by:

```sh
cargo xtask lawpack-goldens
```

Echo remains responsible for dynamic schema admission, exact basis and path
policy validation, request-before-write durability, bounded adapter authority,
settlement, ambiguous-outcome reconciliation, recovery, and effect-free
replay.
