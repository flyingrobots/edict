# Changelog

All notable changes to the Edict specifications are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Edict now has executable Rust implementation slices alongside the design specs;
versions still track specification maturity rather than a released product.

## [Unreleased]

### Changed

- Replaced sentinel external-request schema and reconciliation identities
  with generator-owned canonical artifacts. External-action application builds
  now require an exact `externalActionResources` closure, independently validate
  each closed resource meta-contract and domain-framed digest, and fail before
  publication on missing, duplicate, disconnected, substituted, opaque,
  non-canonical, sentinel, mutated, or over-budget resources.
- Hardened result-projection admission with immutable emitted artifacts,
  bounded recursive decoding, shared compiler input identity, reuse of the
  compiler-computed semantic closure, and provider-schema parity for positive
  output bounds, flat-record size, and source-path depth.
- Rejected many-to-one target obstruction mappings before Target IR emission so
  failure-coordinate collisions cannot silently discard an obstruction arm.
- Hardened standalone application builds around complete lawpack dependency
  closures, selected target-adapter identity, all provider-bound schema roles,
  pure-helper call closure, canonical settings and application paths, mapped
  target obstructions, exact verifier acceptance, safe provider roles, shared
  domain-framed artifact identities, and rollback-safe package/report
  publication.

### Added

- Added a public application-owned lawpack authoring boundary. One bounded
  `edict.lawpack-build/v1` review document now emits deterministic canonical
  manifests, exports, adapters, local resources, and digest sidecars through
  the existing public decoders and complete dependency-graph validator.
  JSONL `build` requests support transactional owned-directory replacement and
  non-repairing, filesystem-read-only `checkOnly` drift detection with a
  before/after ownership-basis check. A
  standalone external witness authors the workspace-snapshot closure without
  `xtask`, reproduces its reviewed
  bytes, and feeds those exact generated artifacts into the public application
  build. Tagged inputs and output paths fail closed, dependency inputs cannot
  resolve under the replaced tree, duplicate JSON keys reject before typed
  authoring, ownership indexes must be real files, every generated artifact and
  drift read stays bounded, check-only classifies a missing nested output as
  drift without creating its parent, empty lawpack document paths reject as
  invalid settings, artifact paths containing filesystem NUL bytes reject
  before output or dependency I/O, the pure preflight includes fixed artifacts,
  sidecars, reserved namespaces, duplicates, and ancestor collisions, artifact
  paths follow a length-bounded lowercase portable filename policy, proper
  ancestor intent locks are acquired top-down and shared by disjoint sibling
  outputs, and ownership and real-directory confinement are rechecked before
  activation, so only overlapping output footprints conflict,
  lawpack-authored JSON accepts a scalar at a 48-container boundary while
  accounting for enclosing export structures and retaining normal-thread stack
  headroom, emitted artifact-path collision checks use an ordering-independent
  ancestor set, and activation is the publication commit point;
  provider invocation and runtime execution remain outside authoring.
- Added the generator-owned `workspace.patch.applyValidated@1` request closure.
  Exact compiler-owned Core and Target IR bind the canonical patch-input schema,
  workspace-root basis, writable-path-policy authority, CI-workflow exclusion,
  bounded settlement, postcondition, and reconciliation contracts as one
  non-callable request with no filesystem-write authority.
- Added explicit `externalAction` application builds that validate one exact
  request-only source, lawpack/adapter/configuration closure, and provider-owned
  target profile before publishing canonical `core.cbor` and `target-ir.cbor`.
  The route requires a typed request, rejects callable Target IR steps and
  substituted capability manifests, binds authority by exact root-reachable
  manifest digest without inventing a coordinate-version relationship, invokes
  no provider component, replaces the output pair transactionally, and clears
  stale executable-operation outputs. Request-only lawpack profiles now bind
  their own exact budget and opaque target configuration while carrying no
  semantic effect or target intrinsic; compilation rejects another profile's
  budget, and application builds reject supplied lawpacks outside the ordered
  root's dependency closure. A generator-owned workspace-snapshot closure and
  mirrored Echo-owned target profile make the full public build reproducible in
  Edict.
- Added typed external-action request values without adding external execution
  authority to Edict. Digest-locked capability imports and `request` statements
  preserve exact operation, schema, scope, basis, budget, input, reconciliation,
  and awaiting-settlement data through canonical Core and Target IR. Operation
  families remain bound in the semantic capability closure and the current
  allowlist admits only the domain-specific `workspace` root. Floating
  capabilities, empty request resources, duplicate request ids, capability
  aliases outside request position, and raw ambient operation families reject
  with stable kinds. Target steps remain non-callable, the provider seam gains
  no host import, and checked Core/Target request goldens bind the complete
  public request identity.
- Added the compiler-owned `edict.result-projection/v1` artifact for preserving
  typed application results across the Echo target boundary. The bounded,
  canonical projection names only declared application input and
  capability-step result paths; an independent verifier reconstructs the
  authored Core result and requires exact Core, Target IR, semantic-closure,
  schema, and digest agreement. Target lowering separates admitted projections
  from explicit per-intent projection failures without narrowing general Target
  IR support. The application build requires one projection, independently
  verifies it, and binds the same artifact into both provider closures. The
  provider contract pack publishes its CDDL root, and the Hello Echo lawpack
  generator owns reviewed projection bytes and identity fixtures.
- Added the public `edict` CLI `build` operation for standalone applications.
  A settings-only JSONL request loads one exact `edict.application/v1`
  manifest, source, complete lawpack dependency closure, direct target adapter,
  target configuration, selected provider profile, and checked provider
  package. Edict compiles and lowers the real source, invokes the provider's
  lowerer and structurally separate verifier through the capability-denied
  Wasmtime host, and writes only the accepted provider-emitted package and
  verification-report bytes. It does not reimplement the provider encoder or
  execute the package.
- Added the generator-owned portable `causal.cell@1.createIfAbsent` lawpack
  closure for external applications. `cargo xtask lawpack-goldens` now
  reproduces its canonical manifest, exports, direct Echo adapter, target
  configuration, and digest sidecars only after validating the closure and
  compiling a digest-pinned Edict witness through Target IR.
- Added the first executable `edict.lawpack/v1` loader. Exact canonical
  manifests and export surfaces decode into an opaque typed bundle, corroborate
  the export digest, validate the closed verifier, helper, effect, obstruction,
  and operation-profile shapes, and reject incomplete, substituted, or cyclic
  dependency sets. The reviewed Hello Echo fixture includes canonical bytes,
  exact digests, and a real `createGreeting` source import checked by
  `cargo xtask lawpack-goldens`.
- Added the direct declarative `edict.lawpack-adapter/v1` ABI. Exact canonical
  adapter bytes are selected and digest-bound by a validated lawpack, must
  completely discharge exported operation-profile, runtime-effect, footprint,
  cost, budget, and named-failure obligations, bind each runtime effect to an
  exact target-owned configuration resource, and derive compiler and Target IR
  facts through the source module's exact digest-locked import. Edict preserves
  the configuration identity without interpreting target semantics. The Hello
  Echo source now lowers to `echo.span-ir/v1` without a caller-built
  `CompilerContext` or `TargetIrLoweringFacts`, and its compiler-produced Core
  and Target IR bytes are reviewed goldens. Target profiles accept only the
  exact direct adapter ABI, and the self-contained provider contract pack
  publishes its CDDL root.

- Added `EDICT.md`, a comprehensive cited introduction and deep-dive report:
  hello-world walkthrough, feature deep dive, plain-English walkthrough with
  glossary and diagrams, unique technical details, roadmap discussion, and a
  claims-to-citations appendix pinned to commit `56f82ec`. It is a
  reader-facing report, not a normative spec or topic shelf. Revision 2 adds
  a theory-grounding walkthrough level (AION / Observer Geometry / Continuum:
  the WARP optic five-tuple, footprint lineage, the four-outcome law,
  support ledgers and witness debt), sixteen cited theory claims, a corrected
  "pluralize" evidence-gap entry, and a personal reflections appendix.
  Revision 3 adds a hands-on lab of six transcripts captured live from the
  built CLI (including a digest invariance/sensitivity experiment over the
  `project` operation), a quick-reference appendix (syntax skeleton,
  wire-stable diagnostic code inventory, digest domains, xtask verbs), and a
  wider-world positioning appendix with a suggested reading order.
- Added guarded recursive CDDL admission to the manifest-bound provider schema
  registry. Productive recursion through map key/value and array-element child
  values now admits the published Core schema, while alias-only cycles,
  choice-only cycles, ambiguous recursive alternatives, and non-progressing
  repetitions still fail before a registry exists. Recursive variable
  occurrences and tagged choices use an Edict-owned specialization pass;
  tagged choices dispatch by a required literal map key without depending on
  declaration or encoded-entry order. A two-arm recursive map choice may also
  dispatch on one exact required text key that the other closed arm cannot
  accept, admitting the closed-versus-legacy Target IR compatibility union
  without weakening ambiguous same-tag choices. Optional or wildcard overlap
  still rejects. Construction rejects any recursive shape the finite
  specializer cannot preserve exactly, including ambiguous map-key assignment
  and multiple or non-final variable array members. Scalar map-key predicates
  retain exact pinned-validator semantics, including `.regexp`. Specialized
  values select an arm before child traversal, then cross canonical encoding
  and the exact 50-container limit before `cddl-cat 0.7.1` validation; duplicate
  keys and one-over-limit values return the stable schema mismatch.

- Added a deterministic Apache-2.0 provider contract pack for runtime-owned
  generators. The checked manifest binds one self-contained CDDL document,
  every logical and artifact-domain root, and the exact bytes, raw and
  coordinate-framed digests, and provenance of all five Edict-owned
  target-profile resources. The new `target-ir-artifact` root is checked
  against both reviewed canonical Target IR fixtures, and
  `cargo xtask provider-contract-pack --check` detects schema or manifest drift
  without rewriting either artifact. Schema controls whose nested rule graph
  cannot be inspected reject with a distinct stable failure.

- Added canonical, content-addressed contract resources for all five
  Edict-owned target-profile slots: encoding, component sandbox, fuel,
  diagnostics, and deterministic execution. Runtime-owned generators pass the
  exact bytes, digest, coordinate, and review provenance explicitly through an
  all-or-nothing validator before a sealed resource set can bind a target
  profile. Reviewed byte/digest fixtures are checked by
  `cargo xtask target-profile-resource-goldens` and the full local gate.

- Added the Edict-owned `edict.authority-facts/v1` canonical-CBOR ABI and CDDL
  root. Canonical fact maps use coordinate keys, source digests use typed
  SHA-256 bytes, write-class sets normalize independent of declaration order,
  and stable byte/shape/duplicate/semantic failures protect the existing
  `AuthorityFactsDocument` to `CompilerContext` path. A reviewed neutral
  byte/digest fixture is checked by `cargo xtask authority-facts-goldens` and
  the full local verification gate.
- Added the generic provider artifact kind `generationProvenance` for a
  generator's deterministic build-provenance document. Provider manifest
  validation treats it as generated metadata with digest-locked semantic-source
  and generator provenance; Edict routes the envelope without interpreting the
  provider-owned evidence schema.
- Added the first runtime-neutral provider manifest boundary:
  `TargetProviderManifest`, provider artifact provenance types, stable
  provider-manifest validation failure kinds, a checked Echo-shaped provider
  manifest fixture, and provider topic/design documentation. This models
  lawpacks, target profiles, authority facts, and provider manifests as
  generated provider artifacts with digest-locked semantic-source and generator
  provenance, while keeping lowerer/verifier entries as provider-owned
  components. It does not load providers, execute WIT components, interpret Echo
  semantics, run verifiers, or perform runtime execution/admission.
- Added an explicit built-in lowerer compatibility seam for the current
  Echo and git-warp lowerers. The borrowed request API distinguishes
  target-profile selection incompatibility from the existing structured target
  refusal report, while parity tests prove direct and compatibility paths retain
  identical Target IR values, canonical bytes, and digests, plus identical
  bundle identities under identical explicit assembly inputs. This is an
  in-process migration adapter, not manifest-backed resolution, WIT component
  loading, or a public Rust provider plugin trait.
- Added the parser-checked `edict:target-provider@1.0.0` WIT transport ABI for
  external lowerer and verifier components. Explicit versioned requests carry
  digest-bound Core, target-profile, and semantic artifacts, world-specific
  requested output roles, and deterministic response limits. Provider outputs
  carry role-tagged bytes and optional logical paths without authoritative
  digests. The private host described below now loads, executes, validates, and
  replays this transport contract; target runtime execution remains separate.
  The new package identity explicitly supersedes the previously shipped but
  unhosted `edict:target-profile@1.0.0` WIT direction rather than changing its
  meaning.
- Added pure provider invocation-envelope validation. Owned Rust values mirror
  the WIT lowerer and verifier contracts; host-authored bindings constrain every
  input before protocol, canonical bytes, owning-schema compatibility,
  domain-framed digests, roles, outputs, paths, diagnostics, success/refusal
  limits, and pairwise limit independence are checked. The required explicit
  schema validator is a deterministic host capability retained by opaque
  validated-request wrappers. Only a fully valid success produces a sealed
  host-authored output manifest binding its inputs, requested outputs, and
  recomputed output digests; limits and diagnostics stay outside the manifest,
  and valid refusal remains distinct from host failure. The pure validator adds
  no component instantiation, ambient I/O, or Echo-specific semantic
  interpretation.
- Completed the hostable provider-manifest v1 authority boundary with exact
  `providerAbi`, domain-to-schema bindings, selected component contract identity,
  generated schema provenance, and an immutable concrete CDDL registry built
  only from the exact manifest-bound closure of explicit digest-locked bytes.
  The registry proves required-domain closure, rejects structurally unusable or
  non-progressing schema roots before invocation, and performs real
  canonical-CBOR schema-instance validation without discovery or lazy loading.
- Added the capability-denied Wasmtime component host for the frozen lowerer and
  verifier worlds. It independently checks component digest, exact digest-covered
  contract attestation, callable-import denial, and structural WIT compatibility;
  binds prepared components to their creating engine; creates a fresh bounded
  store per invocation; distinguishes stable host-owned digest, decode,
  contract, instantiation, fuel, resource, lifting, trap, and admission
  failures; and exposes only pure-validator-admitted outcomes.
- Added sealed provider replay and failure-isolation evidence. Lowerer and
  verifier replay execute identical authority through distinct fresh stores,
  compare complete admitted outcomes or stable failure identities, and return
  structured mismatch categories without treating opaque Wasmtime diagnostics
  as semantic identity. Tests cover concurrent calls, failure recovery,
  cross-provider isolation, named filesystem/network/environment/clock/random
  capability denial, noncanonical and unauthorized outputs, and independent
  process parity with the reviewed Echo-shaped Target IR bytes and digest.
- Added checked conforming and malicious provider component fixtures plus
  `cargo xtask provider-component-fixtures --check/--write`. The inventory binds
  source and component digests, while fixtures cover typed success/refusal,
  infinite work, memory pressure, output and diagnostic floods, schema-invalid
  output, guest traps, instantiation failure, instantiation-time fuel exhaustion,
  and malformed canonical-ABI lifting. Both Rust CI matrix jobs check the
  inventory explicitly.

### Changed

- The local `cargo xtask verify` gate now schedules one default workspace test
  pass, which already includes doctests, instead of repeating every workspace
  doctest in a second Cargo invocation.
- Selected provider component identities now borrow their validated manifest
  directly rather than the temporary proof handle used to authorize selection,
  so callers can discard that handle after obtaining the opaque selection.
- Canonical-CBOR encoding and decoding now accept at most 128 nested values and
  return the stable `NestingLimitExceeded` kind beyond that bound. Provider
  artifact validation uses the same bounded decoder before digest computation.
- The parser now accepts first-class obstruction-strand source syntax:
  `require ... else continue obstructed { reason: ... }`. The form is preserved
  as a distinct `RequireElseArm::ContinueObstructed` source AST arm, requires
  exactly one `reason` field, rejects duplicate `reason` fields, and remains
  contextual to a `require ... else` arm. Helper-shaped constructors such as
  `continueInObstructedStrand(...)` remain ordinary terminal obstruction
  targets. Echo receipts, runtime execution, and editor projection remain
  deferred.
- Core lowering now represents `require` statements as explicit Core require
  nodes with terminal and preserved-obstruction failure arms. The canonical Core
  preimage distinguishes `else <obstruction>` from
  `else continue obstructed { ... }`, binds stable reason kinds and canonical
  payload fields, rejects duplicate payload fields before Core digesting, and
  keeps non-semantic formatting out of Core digests. Runtime receipt behavior
  for obstruction strands remains deferred.
- Echo Target IR lowering now represents supported Core `require` guards as
  explicit Target IR requirements with terminal and `continueObstructed` failure
  dispositions. Canonical Target IR bytes and digests now bind requirement
  predicates, reason kinds, reason payload values, and terminal-vs-preserved
  disposition, while targets without requirement support reject with a stable
  `UnsupportedTargetFeature` failure before artifact emission. Requirements
  after a target step, including requirements whose predicate or reason payload
  reads a target step output, reject until Target IR owns an ordered or
  step-attached guard shape. Echo runtime receipts, admission, scheduler
  counterfactuals, and editor projection remain deferred.
- The obstruction-strands design note now formalizes the cross-project taxonomy
  separating not-admitted scheduler counterfactuals, admitted obstructed strands,
  and hard rejections. The taxonomy records authority boundaries only; it does
  not add Edict-owned runtime execution, scheduler exploration, Graft
  projection, jedit display, Continuum settlement, or XYPH settlement behavior.
- Added the `edict` CLI `project` operation for editor-facing JSONL
  projection over dirty source records. It can emit syntax spans, diagnostics,
  Core review JSON plus canonical Core digest, and Echo Target IR review JSON
  plus canonical Target IR digest without requiring the source to exist on disk;
  compiler and lowering failures are structured projection data on stdout, not
  CLI transport failures. The projection review JSON is not a canonical hash
  contract. Syntax-only lexical failures now emit visible diagnostics projection
  data, and CLI-input failures for known `project` requests report
  `command: "project"` in their diagnostic and status records. Explicit `null`
  values for object-valued compiler settings such as `compilerContext` and
  `target` are rejected before serde can treat them as absent values, and the
  settings schema now rejects empty `project` emit lists. The help record now
  scopes exit code `1` to `check` diagnostics because `project` compiler
  diagnostics are projection data and exit `0`.
- The `edict` CLI now bounds stdin before request parsing with a default 8 MiB
  cap and an `EDICT_CLI_MAX_STDIN_BYTES` override. Over-limit input fails with
  the stable `InputTooLarge` CLI diagnostic and exit 2, pinned by
  `CLI-REQ-010` / `CLI-TP-016` and `fixtures/cli/12-input-too-large`.
- The `edict` CLI now documents its trusted local request boundary and accepts
  optional compiler setting `inputRoot` to confine path, path-list, directory,
  and glob inputs. Inputs resolving outside that root fail with
  `InputPathOutsideRoot`, exit 2, and are pinned by `CLI-REQ-011` /
  `CLI-TP-017` plus `fixtures/cli/13-input-root-outside`; explicit JSON `null`
  for `inputRoot` is rejected as `InvalidSettings`, and non-file glob matches
  are skipped before root-confined canonicalization.
- The `edict` CLI now builds its JSONL check-result, diagnostic, status, and
  info records from typed `Serialize` structs instead of post-construction
  `serde_json::Value` mutation, while preserving the existing byte-for-byte
  golden output.
- The `edict-cli` production targets now deny `clippy::unwrap_used` and
  `clippy::expect_used`, and the parser's `self.expect` helper is documented as
  a fallible token-matching combinator rather than a panic primitive.
- CI now includes a dedicated `cargo deny check` supply-chain job backed by
  `deny.toml`, enforcing RustSec advisories, yanked crates, license allowlisting,
  duplicate-version warnings, and source restrictions.
- Raised the Rust MSRV to 1.94 for Wasmtime 46.0.1, with every workspace package
  inheriting that value into Cargo metadata. Wasmtime is isolated to the private
  provider host with default features disabled, no `wasmtime-wasi`, an executable
  direct/resolved feature ratchet, reviewed permissive license additions, and
  cargo-deny coverage for both the root and nested fixture guest lockfiles.
- Directory expansion in the `edict` CLI no longer allocates a temporary dotted
  extension string per visited file; behavior and golden output are unchanged.
- Added `cargo xtask cli-goldens --check/--write` and wired check mode into
  `cargo xtask verify`, giving the CLI golden corpus the same check/write
  regeneration path as the Core, Target IR, and bundle goldens. The CLI golden
  runner resolves the `edict` binary through Cargo metadata so custom target
  directories are honored.
- Added `cargo xtask release-prep <version>` to scaffold the mechanical release
  prep surfaces that must move together: workspace package versions, lockfile
  package versions, dated changelog section, release policy boundary block,
  release notes stub, boundary test stub, changelog date guard entry, and paired
  release-process test-plan rows. Generated boundary tests now require operators
  to replace scaffolded scope/non-goal placeholders before the branch can pass.
- Added a root `ARCHITECTURE.md` workspace map covering current crate
  responsibilities, dependency direction, the `edict-syntax` crate-scope caveat,
  and current non-claims.
- Added a Core IR canonical encoding explainer covering the canonical value
  model, canonical CBOR subset, Core digest frame, reviewed golden fixtures, and
  byte/hash change discipline.
- Recorded the crate-scope decision to prefer an eventual layered split behind
  an umbrella crate over a simple `edict-syntax` rename, while documenting the
  current crate-scope caveat in `ARCHITECTURE.md`.
- Recorded the schema-as-source-of-truth codegen decision: defer generator work
  until cross-language drift or fixture-authoring pain is measurable, and do not
  reintroduce GraphQL semantics as the contract source.
- Split `xtask` out of its former single-file shape: command dispatch,
  contract checks, golden management, release scaffolding, shared utilities, and
  harness tests now live in focused `xtask/src/*.rs` modules with command
  behavior preserved.
- Marked `v0.11.0-alpha.1` as published in the release-process contract and
  release notes, recording the immutable tag, workflow evidence, milestone
  closure, release URL, and no-crates publication evidence.

## [v0.11.0-alpha.1] - 2026-11-04

### Added

- `edict_syntax::check(&str) -> CheckOutcome`: a one-call front-end entry point
  that parses and surface-validates a source string, returning `Valid`,
  `ParseFailed`, or `SemanticFailed`. The `edict` CLI now routes its check
  pipeline through it (single owner for the parse→validate sequence), and the
  crate docs declare the supported alpha API surface.
- README "Build & Run" and "Using the library" onboarding: how to build the
  `edict` CLI, a copy-pasteable `edict check` example with expected output and
  the exit-code contract, and a runnable `edict_syntax` parse/validate snippet
  (with a matching `cargo test --doc` example in `edict-syntax`). The hero
  diagram now marks shipping-today versus envisioned stages.
- Standard repository files: `SECURITY.md` (private vulnerability reporting +
  supported alpha versions), `NOTICE` (Apache-2.0 attribution), and
  `CODE_OF_CONDUCT.md` (Contributor Covenant 2.1).
- The `edict` CLI now supports `--help`/`-h` and `--version`/`-V`, which emit a
  single `edict.cli.info/v1` JSONL record on stdout (the `help` topic carries the
  usage summary, accepted request schemas, and exit-code contract) and exit 0.
  The new record family has a checked-in JSON Schema and contract-guard test
  (`CLI-REQ-009`, `CLI-TP-012`..`CLI-TP-014`).
- Contract bundle assembly now builds `ContractBundleManifest` values from a
  real `CoreModule` plus supplied digest-locked references, computes the
  semantic and release bundle digests with the v0.11 preimage order, validates
  the assembled manifest before returning it, rejects assembly inputs that would
  produce invalid required bundle structure, and checks a reviewed
  semantic/release digest golden through `cargo xtask bundle-goldens --check`.
  The low-level bundle preimage helper also rejects invalid machine-local source
  paths before hashing. This freezes bundle preimage/digest values without
  freezing canonical `ContractBundleManifest` bytes.
- Canonical Target IR artifact bytes and digests now cover the current Echo and
  git-warp Target IR envelope with the `edict.target-ir.artifact/v1` digest
  frame, reviewed byte/digest goldens under `fixtures/target-ir/canonical/`,
  and `cargo xtask target-ir-goldens --check` wired into `cargo xtask verify`.
  Bundle assembly also has a computed Target IR path that derives
  `targetIrDigest` from a real `TargetIrArtifact` instead of a caller-supplied
  digest reference, rejecting Target IR artifacts whose source Core coordinate
  does not match the supplied Core module and reporting invalid embedded target
  profile digest references with a stable field.

### Changed

- The CLI now rejects compiler input records that the published
  `edict.compiler.input/v1` schema rejects — unrecognized fields and records
  that mix fields from two input kinds both fail with `InvalidInputRecord` —
  so the binary accepts exactly what the schema accepts. Added golden cases
  `10-input-extra-field` and `11-input-hybrid-kind` (`CLI-TP-015`).
- The CLI `InvalidArguments` diagnostic now gives actionable guidance, pointing
  at `edict --help` and `docs/topics/cli/README.md` instead of a bare statement.
- Marked `v0.10.0-alpha.1` as published in the release-process contract and
  recorded durable release evidence for its tag, workflow runs, milestone
  closure, and no-crates publication boundary.

## [v0.10.0-alpha.1] - 2026-10-21

### Added

- Added the first JSONL-only public `edict` CLI surface in `edict-cli`: the
  `check` operation reads compiler settings and compiler input records from
  stdin as JSON Lines, accepts inline source, file paths, directories, ordered
  path lists, and glob patterns, and emits only JSONL records on stdout and
  stderr with stable status, result, and diagnostic fields. Added the CLI topic
  shelf and the checked `edict.compiler.settings/v1` JSON Schema artifact.
- Added checked-in JSON Schema contract artifacts for the remaining CLI stream
  record families — `edict.compiler.input/v1`, `edict.cli.check-result/v1`,
  `edict.cli.diagnostic/v1`, and `edict.cli.event/v1` — so every record on the
  CLI boundary has a stable schema. Added matching contract-artifact guard tests
  (`CLI-TP-006`..`CLI-TP-009`) and documented all five schemas in the CLI topic
  shelf. Hoisted the CLI stream schema identifiers into `edict_cli` as the single
  source of truth and added a test that pins each runtime constant to its
  checked-in JSON Schema artifact, guarding against record/schema drift. Made
  the compiler-input schema's input kinds mutually exclusive (`oneOf`) and
  coupled the event schema's terminal `status` to its `exitCode`, with xtask
  guards pinning both invariants, so the artifacts reject hybrid input records
  and contradictory terminal status records.
- Added a checked-in golden CLI fixture corpus under `fixtures/cli/`, replayed
  end-to-end through the `edict` binary by `golden_cli_fixtures_replay_exactly`,
  matching stdout, stderr, and exit code byte-for-byte across success, parse
  rejection, semantic rejection, CLI-input rejection, and the deterministic
  path, directory, path-list, and glob expansion paths (`CLI-TP-010`,
  `CLI-TP-011`).

### Changed

- Derived the CLI diagnostic `kind` field from explicit, exhaustive
  `ParseErrorKind::code` / `SemanticErrorKind::code` mappings instead of `Debug`
  formatting, so the `edict.cli.diagnostic/v1` wire contract cannot change when a
  variant is renamed and a new variant forces a compile error until it is given a
  stable code. The emitted strings are unchanged.
- Marked `v0.9.0-alpha.1` as published in the release-process contract and
  recorded durable release evidence for its tag, workflow runs, milestone
  closure, and no-crates publication boundary.

## [v0.9.0-alpha.1] - 2026-10-07

### Added

- Added the first Target IR lowering surface in `edict_syntax`: explicit
  `echo.dpo@1` target facts can lower the supported effectful Core shape into a
  deterministic in-memory `echo.span-ir/v1` review artifact, while unsupported
  targets and unsupported Core nodes reject with stable target-lowering failure
  kinds before any artifact is emitted. Target IR preserves effect result
  bindings, effect inputs, obstruction failure keys, obstruction arm values, and
  intent result expressions. Target IR lowering facts can also be derived from
  selected native lowerability results, keeping target artifact paths
  tied to the lowerability report's target profile, operation profile, and
  selected native effect support. The lowerer rejects unsupported Core ABI
  versions, unsupported Core capability flags, undigested target-profile
  references, non-Echo target intrinsics, operation profiles unsupported by the
  selected target facts, and intents with no target-owned steps before Target IR
  emission. Target IR intents also preserve Core input constraints and Core
  evaluation budgets so supported artifacts do not drop preconditions or
  evaluation limits. Added the second supported target slice:
  `gitwarp.ref_crdt@1` can lower the same supported effectful Core shape into a
  deterministic in-memory `gitwarp.commit-reducer-ir/v1` review artifact without
  runtime execution, commit creation, reducer verification, or general target
  plugin dispatch.

### Changed

- Marked `v0.8.0-alpha.1` as published in the release-process contract and
  recorded durable release evidence for its tag, workflow runs, milestone
  closure, and no-crates publication boundary.

## [v0.8.0-alpha.1] - 2026-09-23

### Added

- Added the first minimal effectful compiler-spine path: an annotated
  `let ... = effect(arg) else { failure(binder) => Obstruction }` source shape
  can lower through file-backed authority facts into typed Core with a semantic
  effect node and a source-order-stable obstruction map. Unsupported effectful
  branch-yield, chained effect calls, typed effect calls, and duplicate
  obstruction failure keys still fail before Core lowering with stable compiler
  error kinds.

### Changed

- Marked `v0.7.0-alpha.1` as published in the release-process contract and
  recorded durable release evidence for its tag, workflow runs, milestone
  closure, and no-crates publication boundary.

## [v0.7.0-alpha.1] - 2026-09-09

### Added

- Added the Authority Fact Governance design note and planned
  `v0.7.0-alpha.1` through `v0.15.0-alpha.1` roadmap train.
- Added the first file-backed authority-facts loader in `edict_syntax`, covering
  digest-bound `lawpack` and `targetProfile` source identity, operation-profile
  facts, profile write-class allowances, effect write classes, budgets,
  deterministic merging, and stable load failure kinds.
- Added the authority-facts topic shelf and connected lawpack, target-profile,
  and compiler-spine test plans to the new file-backed compiler context path.
- Added the v2 design boundary topic shelf and non-topic obligation-closure
  design note, while preserving the v1 direct-adapter lowerability boundary.
- Added compiler-spine enforcement for operation-profile write-class
  compatibility with effectful source bodies.
- Added the Rust standards topic shelf, tightened release-prep policy around
  release thesis, previous-tag diff reconciliation, durable release reports, and
  no-crates publication evidence, and promoted missing `Debug` implementations
  to a deny-level workspace lint.
- Added the review-process topic shelf and structured CodeRabbit-to-Codex
  fallback policy for review-bot outages, rate limits, and credit exhaustion.

## [v0.6.0-alpha.1] - 2026-08-26

### Added

- Added editor-facing lexical highlighting in `edict_syntax`:
  `highlight_source`, `HighlightToken`, and stable `HighlightRole` values for
  comments, identifiers, keywords, numbers, operators, punctuation, strings, and
  type identifiers. The highlighter keeps comments visible for editor adapters
  while leaving parsing, resolution, Core lowering, and admission behavior
  unchanged.
- Added the developer-tooling topic shelf and a deterministic highlighting
  fixture for the `v0.6.0-alpha.1` tooling milestone.
- Added initial Tree-sitter artifacts for the developer-tooling milestone:
  grammar source, generated parser source, node metadata, highlight queries, and
  a current-subset corpus aligned with Edict's reference parser.
- Added a TextMate grammar artifact for `.edict` lexical scopes aligned with the
  public editor-facing highlight roles.
- Added a thin VS Code/Cursor extension package that registers `.edict` files
  and uses the canonical TextMate grammar for syntax highlighting.
- Added fixture, lawpack, and assurance topic shelves so the cross-cutting
  contract surfaces have current-truth verification maps.
- Added a release-prep topic-shelf audit gate requiring `docs/topics/` coverage
  and accuracy to both meet at least 90% before release.

## [v0.5.0-alpha.1] - 2026-08-12

### Added

- Added typed Gate C admission-boundary checks in `edict_syntax`:
  `AdmissionRequest`, `AdmissionReceiptBody`, `GateCInvocation`,
  `digest_admission_request`, `validate_admission_request`,
  `validate_admission_receipt`, and `check_gate_c_invocation`. The checks
  validate Edict-owned bundle-subject, operation-requirement, hidden execution
  input, request-digest echoing, receipt echoing, receipt acyclicity,
  invoked-operation, admitted capability scope, participant-matched capability,
  and invocation evidence semantics while leaving participant policy, identity,
  delegation, and
  revocation to Continuum.
- Added the admission topic shelf for the Edict/Continuum admission-boundary
  contract and verification matrix.

## [v0.4.0-alpha.1] - 2026-07-29

### Added

- Added typed v1 lowerability checks in `edict_syntax`: `LoweringRequirements`,
  `TargetProfileFacts`, `check_lowerability`, native/direct-adapter/unsupported
  classifications, and stable lowerability failure kinds. The checker rejects
  ambiguous native support, floating adapter references, chained/composite
  adapter claims, and ambiguous direct adapters, checks required guards on
  selected native/direct support facts, and does not produce Target IR or
  admission artifacts.
- Added typed v1 target-profile manifest conformance checks in `edict_syntax`:
  `TargetProfileManifest`, `validate_target_profile_manifest`, runtime-neutral
  Echo/KV profile acceptance, SHA-256 digest-locked component validation, Core
  ABI validation, deferred lawpack-adapter ABI rejection, and atomic
  application doctrine validation. Composite `multiTarget` profile validation
  remains deferred, so `multiTarget: true` is rejected in v1 conformance.
- Added typed v1 contract-bundle manifest validation in `edict_syntax`:
  `ContractBundleManifest`, `validate_contract_bundle_manifest`,
  runtime-neutral Echo/KV bundle acceptance, SHA-256 digest-locked artifact
  validation, lowercase digest review rendering, release-only provenance input
  binding, canonicalization-profile pinning, logical source path validation,
  optional HOLMES/Watson/Moriarty evidence binding, and explicit rejection of
  admission artifacts from the participant-neutral bundle.
- Added the lowerability topic shelf and the `edict.lowering-requirements/v1`
  CDDL shape in the target-profile ABI.
- Added the target-profiles topic shelf for the manifest conformance contract
  and verification matrix.
- Added the contract-bundles topic shelf for the participant-neutral bundle and
  assurance evidence validation contract.

### Changed

- Clarified the language and target-profile specs so v1 lowerability uses only
  native support, exactly one direct adapter, or unsupported. General composite
  adapter-chain search remains future v2 design work.

## [v0.3.0-alpha.1] - 2026-07-15

### Added

- Added the first reference `edict.canonical-cbor/v1` Core encoder for the
  current in-memory Core module model, plus canonical byte validation through
  decode/re-encode stability checks.
- Added domain-separated `edict.core.module/v1` SHA-256 digest computation,
  reviewed Core golden bytes, exact digest fixtures, and
  `cargo xtask core-goldens --check/--write` for deterministic fixture
  regeneration.
- Added the first executable compiler-spine slice for `v0.3.0-alpha.1`:
  explicit `resolve_module`, `type_check`, `lower_core`, and `compile_to_core`
  APIs; deterministic `CompilerContext` profile/budget facts; a typed module
  boundary distinct from source AST; and in-memory Core IR lowering for the
  initial pure local-record subset. The slice intentionally makes no canonical
  byte, exact digest, target lowering, or admission claim.
- Added `validate_surface` as the explicit source/surface semantic-validation
  compiler stage, with deterministic tests proving that import/name resolution,
  contextual typing, loop-bound proof, and target/lawpack obstruction
  exhaustiveness remain downstream of this pass. `validate_module` remains a
  compatibility alias for the same stage.
- Added publish-ready v0.3 release notes, repeatable release runbook, and
  structured release policy metadata for alpha release preparation, tagging,
  publication, and non-mutating tag recovery.
- Added the repository rule that issue-closing PRs must include GitHub
  auto-close text such as `Closes #123` in the pull request body.

### Fixed

- Rejected Core canonical encoding when an import resource digest is unresolved,
  preventing floating imports from entering the canonical preimage.
- Excluded source-local import alias spelling from Core canonical bytes.
- Sorted resolved Core imports before canonical encoding so source import order
  does not affect canonical bytes.
- Excluded source binder spelling from canonical local references while keeping
  compiler-owned local IDs, normalized `alphaName`, and type references in the
  Core byte identity.
- Canonicalized `requiredCoreCapabilities` as a sorted set before encoding.
- Rejected oversized CBOR declared lengths before allocation in the canonical
  decode validation path.
- Normalized uppercase SHA-256 hex review forms to the same canonical digest
  bytes as lowercase hex.
- Sorted Core input constraints before canonical encoding so constraint vector
  order does not affect canonical bytes.
- Stabilized the compiler-generated input binding ID as `arg.0`, so source
  parameter renaming stays hash-invariant while Core local identity mutations
  still change canonical bytes and digests.

## [v0.2.0-alpha.1] - 2026-07-01

### Added

- Added the `edict.core/v1` Core IR topic shelf and normative CDDL schema for
  the `v0.2.0-alpha.1` Core semantic-model milestone, with local `xtask`
  regressions proving required schema declarations and the explicit no-byte/hash
  freeze boundary.
- Added a repo-local `AGENTS.md` topic-shelf policy, a release-process topic
  shelf, and a structured release-tag recovery policy covering tag-triggered
  GitHub Release publication.

### Changed

- Extended `cargo xtask contract-check` evidence discovery to include `xtask`
  tests, so workflow/process shelves can cite executable `xtask` regressions.
- Relaxed Markdown heading duplication checks to allow changelog section
  headings to repeat across different release versions.

## [v0.1.0-alpha.1] - 2026-06-24

### Added

- **Release roadmap.** Added `ROADMAP.md` as the scheduled alpha-release plan,
  linked it from the README/docs index, and mapped GitHub milestones, release
  labels, and issue #16 for the `v0.1.0-alpha.1` release-prep checklist.
- **Phase 2 — source-AST semantic validation (`edict-syntax`).** Added
  `validate_module`, stable `SemanticErrorKind` categories, deterministic tests,
  and a semantic-validation topic shelf for checks that do not require Core IR:
  bounded runtime `String`/`Bytes`, intent operation-mode/budget/basis
  requiredness, duplicate singleton intent clauses, module namespace collision
  checks, and scoped binder shadowing checks.
- **Topic shelf pilot (`docs/topics/syntax/`).** Added the first current-truth
  topic chapter and verification matrix for the Phase 1 syntax front end,
  library-hosted doctest coverage for the external Markdown example, and
  `cargo xtask verify` / `cargo xtask contract-check` as the local contract
  graph gate.
- **Phase 1 — first executable slice (`crates/edict-syntax`).** A standalone,
  std-only Rust workspace with a hand-written deterministic lexer and a
  recursive-descent parser for the `edict.implementation/minimal-v1` surface.
  Now parses: package/imports (shape/lawpack/target/core, optional `digest`);
  `type` records and refined scalars; `enum` declarations; `variant` types with
  optional payloads; `intent`s with their clauses; `let`/`return`; calls and
  type-calls (`echo.ref<T>(...)`); effect statements with single- and
  map-form `else` obstruction handlers; `require`/`guarantee`/`assert`; the full
  `if` family (ternary `if … then … else …`, effectful branch-yield in
  `let`-rhs, and `if`/`else if`/`else` control flow); bounded
  `for … in … bounded …` loops; variant-literal constructors
  (`Qual.Type::Case(payload)`); boolean and `digest("sha256:…")` literals; and
  `match` expressions. Keywords are reserved as bare identifiers but remain
  legal as member names after `.` (§1510-1511). Conformance fixtures under
  `fixtures/lang/`; 55 tests green under
  `cargo fmt --check`, `clippy` deny-all + pedantic, and CI. See
  `docs/RETRO_phase1-parser.md`.
- `SPEC_edict-lawpack-abi-v1.md`: the Lawpack ABI (manifest, dependency graph,
  exported types/constants, pure helper and semantic effect signatures,
  `executionClass` × `writeClass` classification, typed obstruction payloads,
  footprint/cost obligations, target adapters, compatibility matrix, the v1
  direct-adapter resolution rule).
- `docs/abi/`: machine-readable schemas as the single source of truth —
  `edict-common.cddl` (shared types), `edict-target-profile.cddl`,
  `edict-target-lowerer.wit`, `edict-lawpack.cddl`.
- `docs/REQUIREMENTS.md`: the Fixture Constitution — every normative requirement
  gets a stable ID bound to its owner spec and positive/negative/golden fixtures.
- `spec.lock.json` (schema/registry digest lock for a doc-build gate);
  `fixtures/` Phase 0 corpus layout and conventions.
- Minimal normative **optic contract** in Core (`opticKind`, `basis`,
  `boundaryKind`, `apertureRequirement`, `supportPolicy`, `lossDisposition`),
  each with one deterministic source; richer Observer Geometry evidence
  (Aperture Ledger, witness debt, degeneracy) as derived verifier evidence.
- **Partial Lowerability** section: lowering is a partial, semantics-preserving
  relation classified `native` / `adapted` / `composite` / `unsupported`;
  unsupported is a compiler error, never a silent approximation. README gains the
  lowerability value-proposition statement.
- Language semantics: refined scalar types `String<max=,canonical=>` /
  `Bytes<max=>` (bytes max-only) and pinned `len` units; typed integer-literal
  elaboration with propagation contexts; closed minimal-v1 prelude with pinned
  **integer safety** (overflow-safe, `checked*`, no wrap/saturate/trap);
  `where` input refinement; `basis` clause (pure/effect-free); bounded `for`
  loops; short-circuit booleans; Option-only refinement; typed obstruction
  payloads with failure binders + exhaustive matching; `CapabilityRef<T>`.
- Profiles & packaging: `edict.language/v1` vs `edict.implementation/minimal-v1`
  capability flags (source vs Core split); Core/target/admitted budget split with
  pinned units; semantic vs release bundle digests; logical source-path rules;
  namespace/shadowing and enum-vs-variant rules; `postconditionSupport` target
  field.
- Assurance guide: hash ladder, Aperture Ledger, Lawfulness Certificate,
  obstruction coverage, two-lowerer differential trial.

### Changed

- Roadmap correction: inserted the explicit compiler-spine milestone between
  Core schema work and target/admission work, split the Core IR issue scope
  across schema, compiler-spine, encoder, and golden-digest artifacts, and moved
  developer tooling to `v0.6.0-alpha.1`.
- Updated the `edict-syntax` package description to include source-level
  semantic validation, not only the Phase 1 lexer/parser.
- Purified Core IR: removed the Core self-hash (now `canonicalizationProfile`);
  removed lowerer/verifier digests and packaging fields from the preimage; moved
  `verifiedOperationMode` to the verifier report (Core keeps
  `requiredOperationProfile`); demoted `preconditions`/`postconditions` to derived
  indices and `diagnosticPolicy` to a compile option; reconciled I-010; added a
  positive exhaustive preimage inclusion list and excluded human `name` fields.
- Turned the Target Profile ABI into a real ABI (CDDL manifest + WIT plugin
  boundary + exchange types); enforced `pure`/`effect` intrinsic union;
  named/typed `effectFailures`; intrinsics corpus-document shape; classified
  lawpack verifier (executable ⇒ sandbox+fuel); removed the duplicated manifest
  from the language spec; extracted shared types to `edict-common.cddl`.
- Canonical digest is the typed pair `[algorithm, bytes]` everywhere;
  `"sha256:<hex>"` is review-JSON only.
- Made the artifact graph explicitly acyclic: split compile vs admission
  explanations; split admission receipt body from its DSSE signature; defined
  exact `semanticBundleDigest`/`releaseBundleDigest` preimages (toolchain identity
  in release, not semantic); requests/receipts/explanations carry
  `bundleSubject {kind,digest}`; split semantic vs nonsemantic compile options.
- `require` always carries `else` (grammar + semantics); `where`/`require`/
  `assert` roles disjoint; CoreGuard is `targetAtomic` and always carries an
  obstruction, with verifier proofs as separate `CoreProofObligation` nodes.
- README/docs drift fixes: `edict` code fences, corrected ER-diagram cardinality,
  bounded `hello`/`repo` examples, fixed the alias-shadowing example.
- Design baseline marked non-normative historical context.

### Fixed

- Applied the external Phase 0 design review and two follow-up review rounds
  (Codex + CodeRabbit): closed every flagged contradiction and normative hole as
  bounded clarifications, one commit per finding. Notable: lowerer compares
  cost/footprint vs the **declared** ceiling (admission is external); lawpack
  adapters map failures by **coordinate**; `acceptedLawpackAdapterAbi`
  schema-enforced empty until its ABI exists; `targetBudget` carries both the
  hash-significant `costAlgebra` ref and resolved `ceiling`; bound violations are
  integrity/internal faults, never silent truncation; defined
  `CanonicalEncodedMax<T>` and `edict.core-cost/v1`; deduped requirement IDs.
- Self-review nits: dropped an unused WIT import; de-duplicated the
  `basis`-requiredness wording; locked `edict-common.cddl` in `spec.lock.json`;
  corrected the `edict-common.cddl` header.
- Second-order ripples from the above (Codex + CodeRabbit round): an intent may
  carry **both** `profile` and `implements` (was wrongly "exactly one"); pure
  expressions may call **pure** target/lawpack constructors (only effect
  intrinsics forbidden); integer-literal propagation reaches binary operands;
  field-constraint and refined-type bounds are both valid; `requiredCoreCapabilities`
  is a hash-significant Core module field; operation-profile records get a
  publication slot in the target/lawpack ABIs; exported pure helpers require a
  hash-bound implementation; residual singular bundle-digest references replaced
  with `bundleSubject`; Core/README examples updated to the new rules
  (ObstructionConstruct, `basis` clauses); registry `deferred` status defined and
  the int-literal-mismatch ID numbered (`EDICT-LANG-INTLIT-002`).
- Further ripple round: `edict`-source pure helpers must carry an inline body
  (CDDL union); `operationProfiles` added to the target-profile manifest example;
  `optic-template` can publish `apertureRequirement`; target adapters digest-lock
  their accepted target profile + Target IR; GREEN fixtures use syntactically
  valid dummy digests (the prose `sha256:...` is an un-lexable ellipsis).
- Schema/example/prose alignment round (+ proactive same-class sweep): lawpack
  manifest example carries adapter target-locks; export-surface summary lists
  `operationProfiles`; component pure helpers carry their own digest-locked
  `implementation`; language operation-mode `custom` bullet mirrors the ABI;
  README fixture promise accounts for digest substitution; compile explanation
  surfaces `apertureRequirement`; LawfulnessCertificate proves only core+target
  declared ceilings (never `admitted`); obstruction coverage includes lawpack
  effects; portable example gains a `basis`; Appendix A scoped as exploratory
  non-fixtures; `effectFailures` coordinates must be unique per effect.
- **jedit appendix brought to clause-conformance** (it is the intended first
  real-world use case): added correct `basis` clauses to all 12 rope-package and
  structural-history intents; the Product Text Buffer Optic sketch remains the
  one deliberate non-v1 example (uses rejected `invoke`/`use capability` to show
  design pressure). Appendix note rewritten accordingly.

### Deferred

- The complete `edict.core/v1` CoreExpr/CorePredicate CDDL and canonical encoding
  → issue #3. The spec marks JSON expression examples illustrative and forbids
  freezing any Core hash golden before that schema lands. Adapter
  obligation-closure composition → issue #4; `edict explain lowerability` CLI →
  issue #5.

### Notes

Applies the Phase 0 design review (external "ChatGPT" feedback): SHOULD/COULD
treated as MUST. Grammar and Core schema remain **unfrozen** but the five
yellow-light joints are now determined; next step is Phase 0 implementation
(parser fixtures, Core CDDL, canonical-CBOR goldens, tiny KV target). v1 is not
yet stable.
