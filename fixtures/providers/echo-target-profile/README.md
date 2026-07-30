# Echo Target Profile Fixture

`generated/primary/target-profile.echo-dpo.cbor` is the exact Echo-owned target
profile consumed by the Edict public application-build integration test.

Provenance:

- repository: `flyingrobots/echo`;
- source commit: `5413f55316e5baf2d3af93fd64bb71dc7f84e27d`;
- source path:
  `crates/echo-wesley-gen/assets/v1/edict-provider/package/v1/generated/primary/target-profile.echo-dpo.cbor`;
- Echo generator identity:
  `echo-wesley-gen.provider-artifact-generator@1`;
- Edict domain-framed identity:
  `sha256:2e2494121aecf5e6a2d920f5fb85408825d394765fad41484c416397c920fb04`;
- raw file SHA-256:
  `1b105d1b1f6cdf5fecdef98b7adeb238525047d43581fe9fd8c44fd213e1788e`.

The fixture is metadata authority only. Edict does not interpret Echo runtime
semantics and does not invoke a provider component on the external-action build
route.
