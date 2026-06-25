# Contract Bundles Topic

Status: current HEAD contract.

This chapter describes the v1 participant-neutral contract bundle manifest
validation contract that exists today. It validates typed bundle values after
Core and target lowering have produced hash-addressed artifacts. It does not
load bundle files, recompute semantic or release bundle digests, run lowerers or
verifiers, validate CDDL instances, or make an admission claim.

## Public Surface

The `edict_syntax` crate exposes `validate_contract_bundle_manifest` and typed
contract-bundle data structures for:

- `ContractBundleManifest`, the typed `edict.contract-bundle/v1` manifest value;
- `ContractBundleValidationReport`, including `Valid` and `Invalid`
  classifications;
- stable `ContractBundleValidationFailureKind` categories;
- `BundleSubject`, `BundleSubjectKind`, `AssuranceEvidenceRef`, and
  `AssuranceRole` for participant-neutral HOLMES, Watson, and Moriarty evidence
  references. [BUNDLE-REQ-001]

The canonical artifact identity rules are named in
[`docs/SPEC_continuum-contract-bundle-v1.md`](../../SPEC_continuum-contract-bundle-v1.md).
[BUNDLE-REQ-001]

## Current Contract

- `ContractBundleManifest` records semantic and release bundle digests plus
  source, source-profile facts, Core IR, target-profile, target-IR, lawpack,
  generated artifact, compiler, lowerer, verifier, compile-option,
  canonicalization-profile, conformance-corpus, verifier-report,
  compile-explanation, and assurance evidence references. [BUNDLE-REQ-001]
- Validation is runtime-neutral. Echo-shaped and KV-shaped bundles are checked
  by the same obligations; the checker does not require graph, database,
  event-log, repository, or storage runtime nouns. [BUNDLE-REQ-002]
- Every hash-significant artifact reference must carry a non-empty coordinate and
  valid `sha256:<64 hex>` digest review rendering. [BUNDLE-REQ-003]
- Source artifact paths must be logical package-relative paths. Absolute paths,
  drive-letter paths, backslashes, empty paths, current-directory segments, and
  parent-directory segments are rejected. [BUNDLE-REQ-004]
- HOLMES, Watson, and Moriarty evidence must be present and must bind to the
  manifest's selected bundle subject digest, target profile digest, and target
  IR digest. [BUNDLE-REQ-005]
- Admission artifacts remain outside the participant-neutral contract bundle
  manifest. Non-empty admission references are rejected. [BUNDLE-REQ-006]

## Deferred

The following are not implemented by this contract-bundle slice:

- canonical-CBOR encode/decode helpers for `ContractBundleManifest`;
- digest recomputation from canonical semantic and release preimages;
- file-backed bundle loading;
- full CDDL instance validation;
- target lowerer or verifier execution;
- admission request, receipt, policy, catalog, participant descriptor, and
  signature validation;
- distribution envelope validation.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
