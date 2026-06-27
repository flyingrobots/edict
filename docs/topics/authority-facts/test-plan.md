# Authority Facts Test Plan

Status: current verification design for file-backed authority facts.

## Scope

In scope:

- JSON authority-facts documents with explicit source identity;
- file-backed loading into `CompilerContext`;
- profile, budget, profile write-class, and effect write-class facts;
- deterministic merging and structured load failures.

Out of scope:

- full lawpack manifest validation;
- full target-profile manifest loading;
- registry resolution or directory discovery;
- author/reviewer governance;
- Continuum participant trust policy.

## Requirements

| ID | Status | Requirement | Source |
| --- | --- | --- | --- |
| AUTHFACTS-REQ-001 | implemented | Authority-facts documents load from explicit file paths with digest-locked source identity and no registry, directory discovery, or environment fallback. | ROADMAP.md |
| AUTHFACTS-REQ-002 | implemented | Loaded operation-profile and budget facts can resolve compiler-spine source coordinates into `CompilerContext`. | ROADMAP.md, docs/topics/compiler-spine/test-plan.md |
| AUTHFACTS-REQ-003 | implemented | Loaded profile write-class allowances and effect write classes participate in compiler profile/effect compatibility checks. | issue #54 |
| AUTHFACTS-REQ-004 | implemented | Malformed, non-digest-locked, invalid, or conflicting authority facts reject with stable `AuthorityFactsLoadFailureKind` categories before a compiler context is returned. | ROADMAP.md |
| AUTHFACTS-REQ-005 | gap | Trusted lawpack and target-profile authorship governance is not implemented by this loader. | docs/design/authority-fact-governance.md |

## Fixtures

| Fixture | Purpose | Oracle |
| --- | --- | --- |
| crates/edict-syntax/tests/authority_facts.rs | Runtime-created authority-facts JSON files for compiler and rejection paths. | Tests pass explicit file paths and assert compiler behavior or stable failure kinds. |

## Test Cases

| ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- |
| AUTHFACTS-TP-001 | implemented | Golden path | AUTHFACTS-REQ-001, AUTHFACTS-REQ-002 | A target-profile fact file plus lawpack fact file produce a `CompilerContext` that compiles `bounded-hello` with the expected profile and budget. | file_backed_authority_facts_compile_bounded_hello | crates/edict-syntax/tests/authority_facts.rs | Uses runtime-created files and asserts software behavior. |
| AUTHFACTS-TP-002 | implemented | Boundary guard | AUTHFACTS-REQ-001, AUTHFACTS-REQ-003 | Loaded read-only profile and replace-effect facts cause an effectful source body to reject with `ProfileEffectMismatch`. | file_backed_authority_facts_reject_write_effect_profile_mismatch | crates/edict-syntax/tests/authority_facts.rs | Proves the compiler consumes loaded facts, not hand-built context. |
| AUTHFACTS-TP-003 | implemented | Error handling | AUTHFACTS-REQ-004 | Malformed JSON rejects with `InvalidJson`. | malformed_authority_facts_file_rejects_with_stable_kind | crates/edict-syntax/tests/authority_facts.rs | Stable failure kind, not parser prose. |
| AUTHFACTS-TP-004 | implemented | Error handling | AUTHFACTS-REQ-004 | A source without a SHA-256 digest rejects with `NonDigestLockedSource`. | nondigest_authority_fact_source_rejects_with_stable_kind | crates/edict-syntax/tests/authority_facts.rs | Source identity must be digest-bound. |
| AUTHFACTS-TP-005 | implemented | Error handling | AUTHFACTS-REQ-004 | Conflicting repeated facts reject with `ConflictingFact`. | conflicting_file_backed_authority_facts_reject_before_context | crates/edict-syntax/tests/authority_facts.rs | Prevents ambiguous compiler facts. |
| AUTHFACTS-TP-006 | implemented | Error handling | AUTHFACTS-REQ-004 | Repeated authority sources with the same kind and coordinate but different digests reject with `ConflictingFact`. | mixed_authority_source_digests_reject_before_context | crates/edict-syntax/tests/authority_facts.rs | Prevents merging facts from multiple reviewed source revisions. |
| AUTHFACTS-TP-007 | gap | Governance | AUTHFACTS-REQ-005 | No author/reviewer trust workflow is claimed by this loader. | - | docs/design/authority-fact-governance.md | Planned for `v0.13.0-alpha.1`. |

## Determinism Obligations

- Tests create explicit temporary files and pass exact paths to the loader.
- Tests do not inspect stdout, stderr, diagnostic prose, filesystem ordering,
  network state, registry state, environment configuration, random values, or
  wall-clock time.
- Loader tests assert compiler behavior and stable failure kinds.
- The loader must not fetch packages, discover directories, or mutate
  dependency state.

## Open Gaps

- No trusted lawpack or target-profile authorship workflow exists.
- Full lawpack and target-profile manifest instance loading remains future
  work.
- Intrinsic, obstruction, obligation, adapter, footprint, and cost corpora are
  not loaded by this first authority-facts slice.
