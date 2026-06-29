# Fixtures Topic

Status: current HEAD contract.

This shelf describes the fixture corpus as a shared verification surface. The
fixtures are not examples copied from prose; they are executable inputs or
reviewed artifacts consumed by parser, validation, compiler, tooling, and Core
golden checks.

## Public Surface

The root fixture constitution is [`fixtures/README.md`](../../../fixtures/README.md).
It defines positive, negative, and golden fixture roles and records the current
placeholder-digest rule for runnable source fixtures.

Current checked-in fixture families are:

- source fixtures under [`fixtures/lang/`](../../../fixtures/lang/) for parser,
  semantic-validation, compiler-spine, and developer-tooling behavior;
- reviewed Core artifacts under
  [`fixtures/core/canonical/`](../../../fixtures/core/canonical/) for exact
  canonical bytes and the `edict.core.module/v1` digest;
- golden CLI cases under [`fixtures/cli/`](../../../fixtures/cli/) replayed
  end-to-end through the `edict` binary for byte-exact stdout, stderr, and exit
  code.

`cargo xtask core-goldens --check` verifies the reviewed Core artifacts against
the executable compiler and encoder. `cargo xtask core-goldens --write`
regenerates them after an intentional Core semantic or canonical-encoding
change. [FIXTURES-REQ-002]

## Current Contract

- Source fixtures are executable behavior inputs. Tests consume them through
  public parser, validator, compiler, highlighter, grammar, and encoder APIs.
  [FIXTURES-REQ-001]
- Source fixture digests use lexable `sha256:` review strings. Prose ellipses
  such as `sha256:...` are illustrative only and are not valid runnable fixture
  input. [FIXTURES-REQ-001]
- Reviewed Core golden fixtures are derived from executable behavior, then
  checked in as exact bytes and exact digest review renderings.
  [FIXTURES-REQ-002]
- Topic-shelf test plans may cite fixtures as executable evidence inputs. The
  contract graph check rejects fixture paths that do not exist.
  [FIXTURES-REQ-003]

## Deferred

The fixture constitution names future families for target profiles, lawpacks,
bundles, admission, and conformance. Those directories are not populated yet.
They should be added only when the owning implementation slice has executable
behavior to verify. [FIXTURES-REQ-004]

The verification matrix is tracked in [test-plan.md](./test-plan.md).
