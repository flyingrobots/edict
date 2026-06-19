---
title: "REQUIREMENTS - Edict Fixture Constitution"
legend: "SPEC|TRANSMUTE|PLATFORM"
lane: "design"
packet: "0021-continuum-yolo-runtime-neutral-edict-sha-lock-assurance"
issue: "https://github.com/flyingrobots/wesley/issues/611"
status: "draft"
owners:
  - "@flyingrobots"
created: "2026-06-18"
updated: "2026-06-18"
---

<!-- markdownlint-disable MD025 MD013 -->

# REQUIREMENTS - Edict Fixture Constitution

<!-- markdownlint-enable MD025 -->

## Purpose

Every normative requirement gets a stable ID. Every ID gets a positive fixture,
a negative fixture, and (where it has hash impact) a golden artifact.

> A requirement without a fixture is advisory.
> A fixture without a requirement is folklore.

This registry is the join table between the prose specs and the
[`fixtures/`](../fixtures/) corpus (Phase 0 deliverable). It is also the input to
[`spec.lock.json`](../spec.lock.json): the documentation build fails when an
embedded snippet, schema, or fixture no longer matches the locked digests.

## ID Scheme

`<DOMAIN>-<TOPIC>-<NNN>`:

- `EDICT-LANG-*` — Edict Language v1 (syntax, types, evaluation).
- `EDICT-CORE-*` — Core IR identity and canonicalization.
- `EDICT-OPTIC-*` — optic contract and Observer Geometry preservation.
- `EDICT-TARGET-*` — Target Profile ABI.
- `EDICT-LAWPACK-*` — Lawpack ABI.
- `EDICT-ABI-*` — cross-ABI rules (no-duplication, display sidecars).
- `EDICT-CONFORMANCE-*` — conformance/differential testing.
- `CONTINUUM-*` — contract bundle and admission.

`status`: `spec` (prose landed), `fixture` (fixture exists), `golden` (golden
artifact locked), `impl` (implementation passes), `deferred` (requirement stated
but owned by a follow-up issue; no fixtures until its dependency lands).

## Registry

| ID | Requirement | Owner spec | Positive fixture | Negative fixture | Status |
| --- | --- | --- | --- | --- | --- |
| EDICT-LANG-BOUNDS-001 | Naked unbounded `String`/`Bytes` rejected in checked lane; refined scalars bound every position | Language | `lang/bounds/bounded-hello` | `lang/bounds/naked-string` | spec |
| EDICT-LANG-LEN-001 | `len` units pinned (Unicode scalars vs bytes); canonicalize before measure; concat bound derivation | Language | `lang/len/unicode-vs-bytes` | `lang/len/concat-overflow` | spec |
| EDICT-LANG-INTLIT-001 | Integer width hash-significant; literal needs suffix or unambiguous type; bare ambiguous rejects | Language | `lang/intlit/suffix` | `lang/intlit/ambiguous-hash-arg` | spec |
| EDICT-LANG-WHERE-001 | `where` is input refinement → `EDICT-INPUT-CONSTRAINT`, not runtime precondition | Language | `lang/where/input-refine` | `lang/where/touches-runtime-state` | spec |
| EDICT-LANG-TOTAL-CHECK-001 | Runtime `require`/`guarantee` must carry `else`; faults are typed, never host exceptions | Language | `lang/require/with-else` | `lang/require/missing-else` | spec |
| EDICT-LANG-BOOL-001 | Boolean `and`/`or` operators short-circuit | Language | `lang/bool/short-circuit` | `lang/bool/effect-in-operand` | spec |
| EDICT-LANG-LOOP-001 | `for ... bounded N`: List only, list order, prove M<=N, never silently truncate | Language | `lang/loop/bounded-list` | `lang/loop/unprovable-bound` | spec |
| EDICT-LANG-OBSTRUCT-EXHAUST-001 | Obstruction mapping is exhaustive over domain-mappable classes, with typed payloads | Language | `lang/obstruct/exhaustive-payload` | `lang/obstruct/missing-class` | spec |
| EDICT-LANG-NOSHADOW-001 | Locals must not shadow import/package/type/prelude names | Language | `lang/shadow/distinct-names` | `lang/shadow/greeting-collision` | spec |
| EDICT-LANG-ENUMVARIANT-001 | Enum case `Type.CASE` vs variant constructor `Type::Case(payload)` | Language | `lang/enumvariant/both` | `lang/enumvariant/mixed-syntax` | spec |
| EDICT-LANG-FOOTPRINT-COST-001 | Footprint and cost are checked separately | Language | `lang/footprint/separate` | `lang/footprint/conflated` | spec |
| EDICT-LANG-BUDGET-SPLIT-001 | Core/target/admitted budget split; target dims not in Core | Language | `lang/budget/split` | `lang/budget/target-dim-in-core` | spec |
| EDICT-LANG-PROFILE-001 | `edict.language/v1` vs `edict.implementation/minimal-v1` capability flags | Language | `lang/profile/minimal` | `lang/profile/undeclared-capability` | spec |
| EDICT-LANG-CAPREF-001 | `CapabilityRef<T>` carries receipt digest only; inert until admitted | Language | `lang/capref/inert` | `lang/capref/ambient-authority` | spec |
| EDICT-LANG-READONLY-001 | Read-only inferred; executionClass (proofOnly/runtime) orthogonal to writeClass; runtime read is allowed | Language/Lawpack | `lang/readonly/runtime-read` | `lang/readonly/hidden-append` | spec |
| EDICT-OPTIC-SOURCE-001 | Each optic field has one deterministic source (basis clause / profile template / coordinate / footprint) | Language | `optic/source/basis-clause` | `optic/source/freeform-support` | spec |
| EDICT-DIGEST-WIRE-001 | Canonical digest is typed `[algorithm, bytes]`, never a hex string; hex only in review JSON | Bundle/ABI | `abi/digest/typed-pair` | `abi/digest/hex-string` | spec |
| EDICT-ABI-FAILURE-NAMED-001 | Effects declare named, typed low-level failures; obstruction map keyed by failure coordinate | Target/Lawpack | `abi/failure/named-payload` | `abi/failure/undeclared-arm` | spec |
| CONTINUUM-BUNDLE-SUBJECT-001 | Requests/receipts/explanations carry `bundleSubject {kind,digest}`; receipt echoes it; Moriarty tracks both | Bundle/Admission | `bundle/subject/echo` | `bundle/subject/mismatch` | spec |
| EDICT-TARGET-NEUTRAL-LOWERING-001 | Lowerer compares cost/footprint vs declared target ceiling, never an admitted participant budget | Target | `target/lowering/declared-ceiling` | `target/lowering/admitted-budget` | spec |
| EDICT-CORE-GUARD-PAYLOAD-001 | CoreGuard/ObstructionMap carry a typed obstruction payload construct (coordinate + binder + payload expr) | Language | `core/guard/payload-roundtrip` | `core/guard/coordinate-only` | spec |
| EDICT-CORE-WHERE-HASH-001 | Core carries typed `where` predicate trees in `inputConstraints`, not a validator coordinate | Language | `core/where/predicate-tree` | `core/where/coordinate-only` | spec |
| EDICT-ABI-LAWPACK-ADAPTER-DEFER-001 | `acceptedLawpackAdapterAbi` is optional/deferred until `edict.lawpack-adapter/v1` exists | Target | `abi/adapter/deferred-empty` | `abi/adapter/required-undefined` | spec |
| CONTINUUM-SEMANTIC-OPTIONS-001 | Only semantic compile options enter the semantic digest; diagnostic options excluded | Bundle | `bundle/options/semantic-only` | `bundle/options/diagnostic-in-semantic` | spec |
| EDICT-LANG-ENCODEMAX-001 | `CanonicalEncodedMax<T>` is the compiler-derived max canonical-CBOR size of T, composed structurally; rejected for unbounded T | Language | `lang/encodemax/bounded` | `lang/encodemax/unbounded-type` | spec |
| EDICT-LANG-BUDGET-UNITS-001 | Core budget units pinned (steps/peak live bytes/output octets); targetBudget = costAlgebra+resolved ceiling | Language | `lang/budget/units` | `lang/budget/undefined-units` | spec |
| EDICT-LANG-INTLIT-002 | Suffix disagreeing with contextual integer type rejects; propagation contexts enumerated | Language | `lang/intlit/context-propagate` | `lang/intlit/suffix-context-mismatch` | spec |
| EDICT-LANG-PRELUDE-001 | Minimal-v1 prelude op set is closed; unlisted prelude ops do not exist | Language | `lang/prelude/closed` | `lang/prelude/unlisted-op` | spec |
| EDICT-LANG-BASIS-PURE-001 | `basis` is pure/effect-free over inputs, constants, capability refs, pure fns; runtime reads validate not define | Language | `lang/basis/pure` | `lang/basis/runtime-read` | spec |
| EDICT-LANG-HELPER-BOUNDS-001 | Pure-helper bounds enforced at import validation + call-site instantiation | Language/Lawpack | `lang/helper/bounded` | `lang/helper/unbounded` | spec |
| EDICT-LANG-REQUIRE-ELSE-001 | Every `require` carries `else`; input-only require allowed for domain obstructions; `where`=input refinement; `assert`=proof | Language | `lang/require/input-only-else` | `lang/require/no-else` | spec |
| EDICT-CORE-EXPR-CDDL-001 | Core expression/predicate CDDL + canonical encoding (deferred to issue #3; no Core golden before it lands) | Language/Core | `(deferred → #3)` | `core/expr/golden-before-cddl` | deferred |
| EDICT-LANG-OPTION-REFINE-001 | Only Option refinement (isSome/unwrap), lexical and variable-specific; no general narrowing | Language | `lang/refine/isSome` | `lang/refine/flows-through-helper` | spec |
| EDICT-LANG-BOUND-VIOLATION-001 | Runtime value violating a proven static bound is integrity/internal fault, never silent truncation or resourceFault | Language | `lang/bound/integrity-fault` | `lang/bound/silent-truncate` | spec |
| EDICT-CORE-GUARD-CATEGORY-001 | CoreGuard is targetAtomic + always carries obstruction; verifier proofs are CoreProofObligation nodes | Language | `core/guard/atomic-obstruction` | `core/guard/obstructionless` | spec |
| EDICT-CORE-PREIMAGE-LIST-001 | Canonicalization rules carry a positive exhaustive Core-intent preimage inclusion list | Language | `core/preimage/inclusion-list` | `core/preimage/scavenger-hunt` | spec |
| EDICT-OPTIC-TEMPLATE-OWNER-001 | operation-profile optic template has a canonical shape in edict-common.cddl, exported via ABIs | Target/Lawpack | `optic/template/owned` | `optic/template/undefined` | spec |
| EDICT-OPTIC-APERTURE-REF-001 | `apertureRequirement` is a typed reference (footprintCeiling/abstractFootprintObligation), not a string | Language | `optic/aperture/typed-ref` | `optic/aperture/string` | spec |
| EDICT-LANG-CAPABILITIES-SPLIT-001 | requiredSourceCapabilities (compiler) vs requiredCoreCapabilities (hash-significant Core field) | Language/Bundle | `lang/caps/split` | `lang/caps/conflated` | spec |
| EDICT-LANG-INT-SAFETY-001 | Integer arithmetic overflow-safe and total; checked forms or static proof; no wrap/saturate/trap | Language | `lang/int/checked` | `lang/int/unproven-overflow` | spec |
| EDICT-TARGET-POSTCOND-001 | Target declares `postconditionSupport`; precommit `guarantee` requires it or rejects | Target/Language | `target/postcond/supported` | `target/postcond/unsupported-guarantee` | spec |
| EDICT-LOWERABILITY-PARTIAL-001 | Lowering is a partial semantics-preserving relation: native/adapted/composite/unsupported; unsupported is a compiler error | Language/Target | `lowering/partial/classified` | `lowering/partial/silent-approx` | spec |
| EDICT-LAWPACK-ADAPTER-DIRECT-001 | v1 direct adaptation only; exactly one adapter per (effect,target); no chained legalization/fixed-point | Lawpack | `lawpack/adapter/direct-one` | `lawpack/adapter/chained` | spec |
| EDICT-LANG-TARGETBUDGET-HASH-001 | targetBudget costAlgebra ref + ceiling both hash-significant; ceiling meaningless without its algebra | Language | `lang/targetbudget/both-hashed` | `lang/targetbudget/algebra-unhashed` | spec |
| EDICT-LANG-OBSTRUCT-EMPTY-001 | Bare obstruction-target normalizes to ObstructionConstruct with empty `{}` payload | Language | `lang/obstruct/bare-empty` | `lang/obstruct/bare-undefined` | spec |
| EDICT-LANG-INTENT-CLAUSES-001 | Intent clause requiredness (at least one of profile/implements — both allowed; budget; basis-unless-template); omission parseable but semantically rejected | Language | `lang/clauses/required-present` | `lang/clauses/missing-budget` | spec |
| EDICT-OPMODE-AUTHORITY-001 | Operation-mode predicates defined authoritatively in Target Profile ABI; language spec mirror must not diverge | Target/Language | `opmode/mirror-matches` | `opmode/mirror-diverges` | spec |
| EDICT-ABI-OPPROFILE-SLOT-001 | Target profiles + lawpacks publish operation-profile records via a hash-locked ABI slot | Target/Lawpack | `abi/opprofile/published` | `abi/opprofile/no-slot` | spec |
| EDICT-LAWPACK-PURE-IMPL-001 | Exported pure helper needs a hash-bound implementation (edict body or component+sandbox+fuel) | Lawpack | `lawpack/pure/has-impl` | `lawpack/pure/signature-only` | spec |
| EDICT-LAWPACK-ADAPTER-TARGETIR-001 | Target adapter digest-locks its accepted target profile + Target IR; resolution can't bind a republished IR | Lawpack | `lawpack/adapter/targetir-locked` | `lawpack/adapter/targetir-unlocked` | spec |
| EDICT-ABI-FAILURE-UNIQUE-001 | An effect's `effectFailures` coordinates must be unique (obstruction map keyed by coordinate) | Target/Lawpack | `abi/failure/unique-coords` | `abi/failure/dup-coord` | spec |
| EDICT-ABI-FAILURE-IDENT-001 | Failure coordinates must be bare Edict `ident`s (source obstruction map LHS only accepts ident) | Target/Lawpack | `abi/failure/ident-coord` | `abi/failure/hyphen-coord` | spec |
| EDICT-ABI-INTRINSIC-UNIQUE-001 | Intrinsic coordinates unique within the corpus (schema-enforced via coordinate-keyed map) | Target | `abi/intrinsic/unique-coords` | `abi/intrinsic/dup-coord` | spec |
| EDICT-ABI-OPPROFILE-UNIQUE-001 | Operation-profile coordinates unique (coordinate-keyed map) so resolution is deterministic | Target/Lawpack | `abi/opprofile/unique-coords` | `abi/opprofile/dup-coord` | spec |
| EDICT-LANG-RECORD-SHORTHAND-001 | Bare `ident` record entry `{ x }` is shorthand for `{ x: x }` | Language | `lang/record/shorthand` | `lang/record/shorthand-unbound` | spec |
| EDICT-LANG-BYTES-NOCANON-001 | `Bytes` refinement is max-only; `canonical=` on `Bytes` is rejected | Language | `lang/bytes/max-only` | `lang/bytes/canonical-rejected` | spec |
| EDICT-ABI-INTRINSICS-DOC-001 | The `intrinsics` resource has a fixed corpus-document shape | Target | `abi/intrinsics/document` | `abi/intrinsics/freeform` | spec |
| EDICT-ABI-VERIFIER-BOUND-001 | An executable verifier requires sandbox + fuel; declarative verifier is classified | Lawpack | `lawpack/verifier/executable-bounded` | `lawpack/verifier/unbounded` | spec |
| EDICT-CORE-SELFHASH-001 | No Core/manifest self-hash; digest is external descriptor | Language/Target | `core/hash/no-selfhash` | `core/hash/embedded-selfhash` | spec |
| EDICT-CORE-NOPACKAGING-001 | Lowerer/verifier digests are bundle fields, not Core | Language/Bundle | `core/hash/lowerer-swap-stable` | `core/hash/lowerer-in-preimage` | spec |
| EDICT-CORE-VERIFIED-EXTERNAL-001 | Core states `requiredOperationProfile`; `verifiedOperationMode` is verifier report | Language | `core/verified/external` | `core/verified/in-core` | spec |
| EDICT-CORE-NODUP-PREPOST-001 | `preconditions`/`postconditions` are derived indices, excluded from preimage | Language | `core/prepost/derived` | `core/prepost/double-hashed` | spec |
| EDICT-CORE-NODIAG-001 | `diagnosticPolicy` is compile option/sidecar, not hashed | Language | `core/diag/sidecar` | `core/diag/in-preimage` | spec |
| EDICT-OPTIC-PRESERVE-001 | Optic contract preserved; support loss/degeneracy/witness debt recorded or rejected | Language | `optic/preserve/affect` | `optic/preserve/silent-loss` | spec |
| EDICT-TARGET-INTRINSIC-CLASS-001 | Intrinsic `pure` vs `effect`, enforced as a CDDL union (pure constructors carry no effect kind/failures) | Target | `target/intrinsic/pure-ctor` | `target/intrinsic/pure-with-effectkind` | spec |
| EDICT-LAWPACK-PURE-001 | Lawpack pure helpers are authority-free, observe no runtime state | Lawpack | `lawpack/pure/clean` | `lawpack/pure/side-door` | spec |
| EDICT-LAWPACK-PURE-002 | Pure helpers bounded (no unbounded alloc/scan, no naked return) | Lawpack | `lawpack/pure/bounded` | `lawpack/pure/unbounded` | spec |
| EDICT-LAWPACK-DAG-001 | Lawpack dependency graph acyclic and digest-locked | Lawpack | `lawpack/dag/acyclic` | `lawpack/dag/cycle` | spec |
| EDICT-LAWPACK-ADAPTER-001 | Missing target adapter is a hard error, never silent fallback | Lawpack | `lawpack/adapter/present` | `lawpack/adapter/missing` | spec |
| EDICT-ABI-NODUP-001 | Normative manifest defined once (schema); no duplicate prose JSON | Target/Lawpack | `abi/nodup/generated` | `abi/nodup/drifted-copy` | spec |
| EDICT-ABI-DISPLAY-001 | Display metadata/codenames live in sidecars, never in manifests | Target/Lawpack/Bundle | `abi/display/sidecar` | `abi/display/codename-in-manifest` | spec |
| EDICT-CONFORMANCE-DIFFERENTIAL-001 | Two independent lowerers/verifiers must byte-match the corpus | Conformance | `conformance/two-lowerer/agree` | `conformance/two-lowerer/diverge` | spec |
| CONTINUUM-BUNDLE-DAG-001 | Artifact graph acyclic; no subject references an attestation over itself | Bundle | `bundle/dag/acyclic` | `bundle/dag/cycle` | spec |
| CONTINUUM-BUNDLE-DAG-MORIARTY-001 | Moriarty must catch any introduced cycle | Bundle/Assurance | `(negative-only)` | `bundle/dag/moriarty-injected-cycle` | spec |
| CONTINUUM-RECEIPT-ACYCLIC-001 | Receipt body never references its signing envelope; DSSE signs body digest | Bundle/Admission | `admission/receipt/body-then-sign` | `admission/receipt/self-signature` | spec |
| CONTINUUM-SOURCEPATH-001 | Source paths are logical package-relative URIs, never machine-local | Bundle | `bundle/sourcepath/logical` | `bundle/sourcepath/absolute` | spec |

## Backlog (IDs to allocate)

Existing acceptance criteria in the Language spec (e.g. FIDLAR rejection,
determinism, atomic application, codename coordinates, content-addressed
duplicate create) will be assigned `EDICT-*` IDs and rows here as their fixtures
are written, so the registry becomes the authoritative checklist for grammar and
Core-schema freeze.

**Deferred entries** (status `deferred`) intentionally have no positive fixture
until their dependent schema lands; their positive-fixture cell records the
blocking dependency (e.g. `(deferred → #3)`). A row whose only fixture is a
negative one (e.g. a Moriarty cycle-injection) marks its positive cell
`(negative-only)`. These markers keep the table's "every ID gets fixtures"
contract honest rather than papering over the gap with an em-dash.
