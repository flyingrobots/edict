# Contract Bundles Topic

Status: current HEAD contract.

This chapter describes the v1 participant-neutral contract bundle manifest
validation and assembly contract that exists today. It validates typed bundle
values after Core and target lowering have produced hash-addressed artifacts,
and it can assemble a `ContractBundleManifest` from a real `CoreModule` plus
supplied digest-locked references. It does not load bundle files, canonicalize
Target IR bytes, run lowerers or verifiers, validate CDDL instances, or make an
admission claim.

## Public Surface

The `edict_syntax` crate exposes `validate_contract_bundle_manifest` and typed
contract-bundle data structures for:

- `ContractBundleManifest`, the typed `edict.contract-bundle/v1` manifest value;
- `assemble_contract_bundle`, `ContractBundleAssemblyInput`, and typed supplied
  digest/resource wrappers for constructing a manifest and recomputing the
  semantic and release bundle digests;
- `ContractBundleValidationReport`, including `Valid` and `Invalid`
  classifications;
- stable `ContractBundleValidationFailureKind` categories;
- `BundleSubject`, `BundleSubjectKind`, `AssuranceEvidenceRef`, and
  `AssuranceRole` for participant-neutral HOLMES, Watson, and Moriarty evidence
  references when those external assurance artifacts are bundled.
  [BUNDLE-REQ-001]

The canonical artifact identity rules are named in
[`docs/SPEC_continuum-contract-bundle-v1.md`](../../SPEC_continuum-contract-bundle-v1.md).
[BUNDLE-REQ-001]

## Current Contract

- `ContractBundleManifest` records semantic and release bundle digests plus
  source, source-profile facts, Core IR, target-profile, target-IR, lawpack,
  generated artifact, compiler, lowerer, verifier, semantic and nonsemantic
  compile-option, build-provenance, canonicalization-profile,
  conformance-corpus, verifier-report, compile-explanation, and assurance
  evidence references. [BUNDLE-REQ-001]
- Validation is runtime-neutral. Echo-shaped and KV-shaped bundles are checked
  by the same obligations; the checker does not require graph, database,
  event-log, repository, or storage runtime nouns. [BUNDLE-REQ-002]
- Every hash-significant artifact reference must carry a non-empty coordinate and
  valid lowercase `sha256:<64 lowercase hex>` digest review rendering.
  [BUNDLE-REQ-003]
- Source artifact paths must be logical package-relative paths. Absolute paths,
  drive-letter paths, backslashes, empty paths, current-directory segments, and
  parent-directory segments are rejected by both manifest validation and the
  bundle digest preimage helper. [BUNDLE-REQ-004]
- HOLMES, Watson, and Moriarty evidence is optional in the typed bundle. When an
  evidence entry is present, it must bind to the manifest's selected bundle
  subject digest, target profile digest, and target IR digest. [BUNDLE-REQ-005]
- Admission artifacts remain outside the participant-neutral contract bundle
  manifest. Non-empty admission references are rejected. [BUNDLE-REQ-006]
- The canonicalization profile coordinate is pinned to `edict.canonical-cbor/v1`.
  [BUNDLE-REQ-007]
- `assemble_contract_bundle` computes `coreIrDigest` from the supplied
  `CoreModule` with `digest_core_module`; callers cannot supply an alternate
  Core digest. The assembler validates its generated manifest before returning
  it and rejects inputs that would produce an invalid required bundle structure.
  [BUNDLE-REQ-008]
- `targetIrDigest` remains a supplied digest-locked Target IR reference for this
  slice. The same typed Target IR resource supplies both
  `manifest.target_ir.digest` and the semantic bundle digest preimage. Canonical
  Target IR bytes are tracked separately by issue #105. [BUNDLE-REQ-008]
- `semanticBundleDigest` is recomputed from the exact ordered semantic preimage:
  computed Core digest, target profile digest, supplied Target IR digest,
  lawpack digests, source-profile semantic facts digest, generated artifact
  digests, canonicalization profile digest, semantic compile options digest,
  conformance fixture corpus digests, and verifier report digest.
  [BUNDLE-REQ-008]
- `releaseBundleDigest` is recomputed from the semantic bundle digest plus
  structured release-only provenance: source logical paths and artifact
  references, compiler/lowerer/verifier coordinates and digests, nonsemantic
  compile options, build provenance, and compile explanation. Coordinate and
  logical-path changes move the release digest even when digest bytes stay the
  same. [BUNDLE-REQ-008]
- Optional assurance evidence is generated with subject, target-profile, and
  target-IR bindings when supplied, but it is not a top-level semantic or
  release digest preimage component. Admission artifacts remain rejected and are
  not emitted by the assembler. [BUNDLE-REQ-006] [BUNDLE-REQ-008]
- The v0.11 bundle digest golden in
  `fixtures/bundle/assembly/bounded-hello.bundle-digests.txt` freezes the
  semantic/release bundle preimage shape and resulting digest review strings.
  `cargo xtask bundle-goldens --check` regenerates it from executable assembly.
  [BUNDLE-REQ-008]

## Deferred

The following are not implemented by this contract-bundle slice:

- canonical-CBOR encode/decode helpers for `ContractBundleManifest`;
- canonical Target IR bytes or computing `targetIrDigest` from Target IR bytes
  (#105);
- file-backed bundle loading;
- full CDDL instance validation;
- target lowerer or verifier execution;
- admission request, receipt, policy, catalog, participant descriptor, and
  signature validation;
- distribution envelope validation.

The verification matrix is tracked in [test-plan.md](./test-plan.md).
