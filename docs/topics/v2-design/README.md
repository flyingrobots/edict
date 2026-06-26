# v2 Design Topic

Status: current design boundary for future v2 work. No v2 adapter-composition
behavior is implemented in HEAD.

This shelf owns the post-v1 design for adapter composition and
obligation-closure resolution. It does not change the v1 lowerability contract:
v1 remains native-or-direct-adapter only, with adapter chains rejected before
Target IR exists. [V2DESIGN-REQ-007]

## Terminology

Use **lowering obligation**, not capability, for compiler legalization facts.
Capability remains reserved for participant authority, capability receipts, and
`CapabilityRef<T>` style admission concepts. [V2DESIGN-REQ-001]

An obligation is a typed lowering requirement that must be discharged before a
valid target artifact can exist. Obligations include semantic effects, target
intrinsic needs, guard support, obstruction mappings, footprint facts, cost
facts, atomicity requirements, postcondition support, and optic-preservation
facts. [V2DESIGN-REQ-001]

## Adapter Declarations

A future v2 adapter declaration has three obligation sets:

- `consumes`: root obligations the adapter can accept from the current closure;
- `provides`: obligations discharged or made available by selecting the
  adapter;
- `requires`: additional obligations that must already be in the closure or be
  discharged by other selected adapters. [V2DESIGN-REQ-002]

Each adapter declaration must be identified by a digest-locked coordinate and
must name the target profile, lawpack subject, accepted Core ABI, accepted
adapter ABI, and compatibility bounds it was authored for. Floating adapter
references are never candidates. [V2DESIGN-REQ-002]

The future adapter ABI must keep target-owned intrinsics separate from
lawpack-owned semantic effects. An adapter may compose through declared
obligations; it must not smuggle ambient runtime authority or participant
capabilities into compiler legalization. [V2DESIGN-REQ-001]
[V2DESIGN-REQ-002]

## Closure Resolution

The v2 resolver starts with the root native obligations `N` from the target
profile and the root operation obligations from `LoweringRequirements`. Given a
candidate adapter set `A`, it computes the fixed point:

```text
closure(N, A) = fixed-point of:
  N union union { provided(a) | a in A, required(a) subset current_closure }
```

Resolution succeeds only when the deterministic, digest-locked, acyclic adapter
set discharges all root obligations and leaves no unresolved semantic effects.
[V2DESIGN-REQ-003]

Closure computation must be monotonic: every iteration may add facts, but it
must not remove or reinterpret earlier facts. The finite universe is the
operation requirement set plus the obligations reachable from digest-locked
candidate adapters. A resolver that would require an unbounded search rejects
before selecting adapters. [V2DESIGN-REQ-003]

## Candidate Ordering And Ambiguity

Candidate discovery must be deterministic. Before solving, candidates are
ordered by stable keys:

1. target-profile coordinate and digest;
2. lawpack coordinate and digest;
3. adapter coordinate and digest;
4. declared adapter ABI version;
5. canonical obligation-set encoding. [V2DESIGN-REQ-004]

Ordering is a tie-breaker for reproducibility, not authority to choose
semantics silently. If two distinct candidate sets discharge the same root
obligations but produce different selected adapter sets, closure evidence, or
target semantics, the resolver reports ambiguity instead of selecting one by
input order. [V2DESIGN-REQ-004]

Version and conflict solving happens before closure selection. A candidate is
eligible only when its declared lawpack, target profile, Core ABI, adapter ABI,
and compatibility bounds are mutually satisfiable. Conflicting eligible
candidates reject with a structured ambiguity diagnostic. [V2DESIGN-REQ-004]

## Cycles And Dependency Boundaries

Adapter dependencies form a directed graph over digest-locked adapter
coordinates. Cycles are rejected unless every cycle edge is proven to add no new
obligations and no new target semantics. The default rule is reject cycles.
[V2DESIGN-REQ-005]

The resolver must not fetch packages, search registries, or mutate dependency
state while computing closure. It receives a closed candidate universe and
either proves closure inside that universe or returns structured missing
obligation diagnostics. [V2DESIGN-REQ-005]

## Closure Evidence

Successful v2 resolution must produce closure evidence that can be hash-bound
into later artifacts. The evidence records:

- root operation obligations;
- root target native obligations;
- candidate adapter universe digests;
- selected adapter coordinates and digests;
- iteration steps or an equivalent canonical proof of the fixed point;
- discharged obligations;
- residual obligations, which must be empty on success;
- ambiguity and cycle decisions. [V2DESIGN-REQ-006]

The selected adapter chain and closure evidence must be canonicalized before
hashing. Reordering equivalent candidate input must not change the evidence
digest. Any semantic change to a selected adapter, obligation declaration, target
profile, or lawpack subject must change the evidence digest. [V2DESIGN-REQ-006]

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
- non-digest-locked candidate. [V2DESIGN-REQ-005]

Diagnostics may render human-readable explanations, but executable tests must
assert stable diagnostic kinds and structured obligation coordinates.
[V2DESIGN-REQ-005]

## v1 Boundary

Nothing in this design permits v1 lowerability to accept adapter chains. The
current executable behavior remains:

- native support succeeds only from explicit target-profile facts;
- exactly one digest-locked direct adapter may discharge a semantic effect;
- floating, chained, composite, or ambiguous adapters classify as unsupported;
- no Target IR, verifier report, bundle, admission request, or admission receipt
  is produced by the lowerability checker. [V2DESIGN-REQ-007]

The v2 design can only become executable in a later change that adds typed
adapter declarations, fixtures, stable failure kinds, closure evidence, and
behavior tests. [V2DESIGN-REQ-007]

The verification plan is tracked in [test-plan.md](./test-plan.md).
