# Edict Fixture Corpus

This is the Phase 0 conformance corpus. Each fixture is owned by a requirement
ID in the [requirements registry](../docs/REQUIREMENTS.md).

> A requirement without a fixture is advisory. A fixture without a requirement
> is folklore.

## Layout

Fixtures are grouped by requirement domain, mirroring the registry's ID scheme:

```text
fixtures/
  lang/        # EDICT-LANG-*  source that parses, typechecks, or is rejected
  core/        # EDICT-CORE-*  golden Core IR canonicalization + hash cases
  optic/       # EDICT-OPTIC-* optic-contract preservation cases
  target/      # EDICT-TARGET-* target profile ABI cases
  lawpack/     # EDICT-LAWPACK-* lawpack ABI cases
  abi/         # EDICT-ABI-*  no-duplication / display-sidecar cases
  bundle/      # CONTINUUM-BUNDLE-* + CONTINUUM-SOURCEPATH-* identity/DAG cases
  admission/   # CONTINUUM-RECEIPT-* admission acyclicity cases
  conformance/ # EDICT-CONFORMANCE-* two-lowerer differential cases
```

## Fixture kinds

- **Positive (GREEN):** source/artifact that must be accepted; for hash-bearing
  cases a golden canonical artifact and expected digest accompany it.
- **Negative (RED):** source/artifact that must be rejected, with the expected
  diagnostic code (e.g. `EDICT-INPUT-CONSTRAINT`, or the owning requirement ID).
- **Golden:** exact expected canonical-CBOR bytes / digest, so formatting,
  comment, and alias changes can be proven not to change semantic identity.

## Status

Empty pending the Phase 0 fixture campaign. The first fixtures to land are the
canonicalization golden cases and the relapse zoo (see the assurance guide), per
the Language spec implementation plan, Phase 0.

**Placeholder digests:** README/spec code examples currently use `sha256:...`
placeholders. The fixtures derived from those examples will carry placeholder
digests until the Phase 0 tooling (the `spec.lock.json` generator and a fixture
validator) exists to compute and pin real digests. A GREEN example fixture is
not expected to compile cleanly until that tooling lands.
