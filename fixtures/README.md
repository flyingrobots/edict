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

The first Core canonical fixture lives in
[`core/canonical/`](./core/canonical/). Additional source coverage, relapse-zoo
cases, target fixtures, and admission fixtures remain planned as their owning
implementation slices land.

**Placeholder digests:** prose in the README/specs writes `sha256:...` as a
human ellipsis, which is **not lexable** — the grammar's `digest-lit` requires
`sha256:` followed by exactly 64 hex characters. A GREEN fixture is therefore
**not** the prose verbatim: it uses a syntactically valid **dummy digest**
(e.g. `sha256:` + 64 `0`s) so it parses and typechecks. The placeholder only
defers **digest-lock validation** (matching the import against a *real* pinned
digest), not compilation. Until the Phase 0 tooling (the `spec.lock.json`
generator and a fixture validator) computes and pins real digests, that lock
step is skipped; the example is otherwise a valid GREEN fixture.
