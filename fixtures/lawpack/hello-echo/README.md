# Hello Echo Lawpack Fixture

This is the first reviewed `edict.lawpack/v1` fixture. It exists to expose the
next real Edict-to-Echo crossing without GraphQL, native application callbacks,
a fake transport, or a handwritten Echo executable package.

- `manifest.cbor` is the canonical lawpack manifest.
- `manifest.sha256` is its `edict.lawpack/v1` domain-framed identity.
- `exports.cbor` is the canonical export surface.
- `exports.sha256` is its `hello.echo.exports/v1` domain-framed identity.
- `create-greeting.edict` imports the exact manifest digest and declares the
  bounded `createGreeting` action with typed `AlreadyExists` mapping.

Regenerate only through:

```sh
cargo xtask lawpack-goldens --write
```

Check reviewed artifacts without modifying them:

```sh
cargo xtask lawpack-goldens --check
```

The lawpack loader proves the module boundary. It does not yet load or execute
the referenced Echo target adapter; that is the next compiler crossing.
