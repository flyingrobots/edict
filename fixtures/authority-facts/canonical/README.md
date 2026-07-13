# Canonical Authority-Facts Fixture

This directory freezes the first `edict.authority-facts/v1` canonical artifact:

- `example-effectful.authority-facts.json` is the strict JSON review/input form;
- `example-effectful.authority-facts.cbor` is the normative
  `edict.canonical-cbor/v1` byte representation;
- `example-effectful.authority-facts.sha256` is the lowercase review rendering of
  the domain-framed artifact digest.

The fixture supplies one runtime-neutral operation profile, its allowed write
classes, one semantic effect write class, and one Core evaluation budget. It is
compatibility evidence for the generic Edict authority-facts ABI, not a
deployable provider artifact.

Check the fixture with:

```bash
cargo xtask authority-facts-goldens --check
```

Regenerate it only after an intentional ABI change:

```bash
cargo xtask authority-facts-goldens --write
```
