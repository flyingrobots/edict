# v2 Obligation-Closure Design Note

Status: future design note for issue #4. This is not a topic shelf contract for
landed behavior, and no v2 adapter-composition behavior is implemented in HEAD.

This note records the planned post-v1 design direction for adapter composition
and obligation-closure resolution. The current executable contract remains in
the lowerability and lawpack topic shelves.

## Terminology

Use **lowering obligation**, not capability, for compiler legalization facts.
Capability remains reserved for participant authority, capability receipts, and
`CapabilityRef<T>` style admission concepts.

An obligation is a typed lowering requirement that must be discharged before a
valid target artifact can exist. Obligations include semantic effects, target
intrinsic needs, guard support, obstruction mappings, footprint facts, cost
facts, atomicity requirements, postcondition support, and optic-preservation
facts.

## Adapter Declarations

A future v2 adapter declaration has three obligation sets:

- `consumes`: obligations the adapter can accept from the current unresolved
  obligation set;
- `provides`: obligations discharged or made available by selecting the adapter;
- `requires`: prerequisite obligations that must already be in the current
  closure before the adapter can be selected.

Each adapter declaration must be identified by a digest-locked coordinate and
must name the target profile, lawpack subject, accepted Core ABI, accepted
adapter ABI, and compatibility bounds it was authored for. Floating adapter
references are never candidates.

The future adapter ABI must keep target-owned intrinsics separate from
lawpack-owned semantic effects. An adapter may compose through declared
obligations; it must not smuggle ambient runtime authority or participant
capabilities into compiler legalization.

## Closure Resolution

The v2 resolver starts with the root native obligations from the target profile
and the root operation obligations from `LoweringRequirements`. Given a closed
candidate adapter set, it computes a monotonic fixed point over:

- `facts`: obligations currently known to the resolver;
- `unresolved`: obligations that still need native support or an adapter;
- `selected`: digest-locked adapters selected into the solution.

A candidate adapter may be selected only when all of the following are true:

- the candidate is digest-locked and version-compatible;
- `consumes(a)` is non-empty;
- `consumes(a)` intersects `unresolved`;
- `consumes(a)` is a subset of `facts`;
- `requires(a)` is a subset of `facts`;
- selecting the adapter would not introduce an adapter dependency cycle.

Selecting an adapter removes its consumed obligations from `unresolved`, adds
its provided obligations to `facts`, and adds any newly provided obligations
that still require discharge to `unresolved`. The fixed point succeeds only when
the deterministic selected adapter set leaves no unresolved semantic effects or
other root obligations.

Adapters with empty or unrelated `consumes` sets may not contribute provided
facts to the closure. This prevents an unrelated candidate with empty
requirements from discharging obligations it was not selected to accept.

Closure computation must be monotonic: every iteration may add facts, but it
must not remove or reinterpret earlier facts. The finite universe is the
operation requirement set plus the obligations reachable from digest-locked
candidate adapters. A resolver that would require an unbounded search rejects
before selecting adapters.

## Candidate Ordering And Ambiguity

Candidate discovery must be deterministic. Before solving, candidates are
ordered by stable keys:

1. target-profile coordinate and digest;
2. lawpack coordinate and digest;
3. adapter coordinate and digest;
4. declared adapter ABI version;
5. canonical obligation-set encoding.

Ordering is a tie-breaker for reproducibility, not authority to choose
semantics silently. If two distinct candidate sets discharge the same root
obligations but produce different selected adapter sets, closure evidence, or
target semantics, the resolver reports ambiguity instead of selecting one by
input order.

Version and conflict solving happens before closure selection. A candidate is
eligible only when its declared lawpack, target profile, Core ABI, adapter ABI,
and compatibility bounds are mutually satisfiable. Conflicting eligible
candidates reject with a structured ambiguity diagnostic.

## Cycles And Dependency Boundaries

Adapter dependencies form a directed graph over digest-locked adapter
coordinates. Selected adapter graphs must be acyclic. Any cycle rejects with a
structured cycle diagnostic; v2 does not define a benign-cycle exception.

The resolver must not fetch packages, search registries, or mutate dependency
state while computing closure. It receives a closed candidate universe and
either proves closure inside that universe or returns structured missing
obligation diagnostics.

## Closure Evidence

Successful v2 resolution must produce closure evidence that can be hash-bound
into later artifacts. The evidence records:

- root operation obligations;
- root target native obligations;
- candidate adapter universe digests;
- selected adapter coordinates and digests;
- consumed obligations for each selected adapter;
- iteration steps or an equivalent canonical proof of the fixed point;
- discharged obligations;
- residual obligations, which must be empty on success;
- ambiguity and cycle decisions.

The selected adapter chain and closure evidence must be canonicalized before
hashing. Reordering equivalent candidate input must not change the evidence
digest. Any semantic change to a selected adapter, obligation declaration,
target profile, or lawpack subject must change the evidence digest.

## Diagnostics

Failure diagnostics are structured values, not prose-only messages. At minimum,
future v2 diagnostics distinguish:

- unsatisfied root obligation;
- unsatisfied transitive obligation;
- ambiguous adapter set;
- adapter dependency cycle;
- incompatible adapter ABI;
- incompatible lawpack or target-profile version;
- non-monotonic obligation declaration;
- non-digest-locked candidate.

Diagnostics may render human-readable explanations, but executable tests must
assert stable diagnostic kinds and structured obligation coordinates.
