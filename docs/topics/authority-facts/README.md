# Authority Facts Topic

Status: current HEAD contract.

This shelf describes Edict's file-backed authority-fact loading boundary. An
authority-facts document is a deterministic, digest-bound input that can provide
the first compiler facts already modeled by `CompilerContext`: operation
profiles, profile write-class allowances, effect write classes, and budgets.

This is not trusted authorship governance. The loader proves that facts came
from explicit files with digest-locked source identity and stable validation
behavior. It does not decide whether a participant trusts the author, reviewer,
identity system, or policy behind those files.

## Public Surface

The public authority-facts surface lives in `edict_syntax`:

- `load_authority_facts_file` parses and validates one JSON authority-facts
  document. [AUTHFACTS-REQ-001]
- `load_compiler_context_from_authority_fact_files` loads one or more files and
  merges their facts into a deterministic `CompilerContext`.
  [AUTHFACTS-REQ-002]
- `AuthorityFactsLoadFailureKind` gives stable failure categories for malformed
  files, non-digest-locked sources, invalid fact coordinates, invalid write
  classes, and conflicting facts. [AUTHFACTS-REQ-004]

Authority-facts documents identify their source as either `lawpack` or
`targetProfile`, with a coordinate and `sha256:<64 hex>` digest. The source
identity is evidence binding for the loaded facts; it is not participant trust
policy.

## Current Contract

- File-backed profile and budget facts can resolve the `bounded-hello` compiler
  fixture without caller-constructed in-memory context facts.
  [AUTHFACTS-REQ-002]
- File-backed profile write-class allowances and effect write classes
  participate in compiler profile/effect compatibility checks. A write-class
  effect under a read-only loaded profile rejects with
  `ProfileEffectMismatch`. [AUTHFACTS-REQ-003]
- Authority-facts loading is deterministic. The loader consumes the exact file
  paths provided by the caller and does not discover directories, fetch
  registries, read environment configuration, or mutate dependency state.
  [AUTHFACTS-REQ-001]
- Conflicting facts reject before a `CompilerContext` is returned. A repeated
  coordinate with identical content is harmless; a repeated coordinate with
  different content is a conflict. [AUTHFACTS-REQ-004]

## Deferred

The following are not implemented:

- full `edict.lawpack/v1` manifest instance validation;
- full `edict.target-profile/v1` file-backed manifest loading;
- intrinsic, obstruction, obligation, adapter, footprint, and cost corpus
  validation beyond the first compiler-context facts;
- author/reviewer provenance validation;
- signatures, trust roots, revocation, registry selection, or Continuum
  participant acceptance policy.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
