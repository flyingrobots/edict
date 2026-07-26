# Hello Echo Lawpack Fixture

This is the first reviewed `edict.lawpack/v1` fixture. It exists to expose the
next real Edict-to-Echo crossing without GraphQL, native application callbacks,
a fake transport, or a handwritten Echo executable package.

- `manifest.cbor` is the canonical lawpack manifest.
- `manifest.sha256` is its `edict.lawpack/v1` domain-framed identity.
- `exports.cbor` is the canonical export surface.
- `exports.sha256` is its `hello.echo.exports/v1` domain-framed identity.
- `adapter.cbor` is the canonical direct declarative Echo target adapter.
- `adapter.sha256` is its manifest-bound domain-framed identity.
- `echo-operation-configuration.cbor` is the target-owned, adapter-bound
  program, budget, authority, and invocation-binding configuration that Echo
  may interpret.
- `echo-operation-configuration.sha256` is its
  `hello.echo.echo-operation-configuration/v1` domain-framed identity.
- `create-greeting.edict` imports the exact manifest digest and declares the
  bounded `createGreeting` action with typed `AlreadyExists` mapping.
- `create-greeting.core.cbor` is the canonical Core module compiled from that
  exact source and lawpack closure.
- `create-greeting.core.sha256` is its `edict.core.module/v1`
  domain-framed identity.
- `create-greeting.target-ir.cbor` is the canonical `echo.span-ir/v1`
  artifact lowered from that exact Core module.
- `create-greeting.target-ir.sha256` is its
  `edict.target-ir.artifact/v1` domain-framed identity.

Regenerate only through:

```sh
cargo xtask lawpack-goldens --write
```

Check reviewed artifacts without modifying them:

```sh
cargo xtask lawpack-goldens --check
```

The loader validates the manifest, exports, and exact direct adapter. The
compiler derives Core and `echo.span-ir/v1` facts from that closure without a
handwritten compiler context. The golden command compiles and lowers the source
before reproducing the reviewed Core and Target IR bytes and identities. This
fixture does not yet emit an Echo executable package or execute an Echo Action.
Edict corroborates the target-configuration reference but deliberately leaves
its Echo-specific semantics to the Echo-owned target provider.
