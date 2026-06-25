# Edict Specifications

The current specification set is:

- [Topic Shelves](./topics/README.md): current reference chapters and
  verification plans for landed subsystems. A topic shelf is a contract graph,
  not a proposal: `README.md` says what is true in HEAD, `test-plan.md` says how
  it is verified and where gaps remain, and optional `architecture.md` /
  `rationale.md` files explain machinery and still-relevant tradeoffs.
- [Testing Workflow Topic](./topics/tests/): RED/GREEN development discipline,
  fixture reuse, and local verification workflow.
- [Documentation Standards Topic](./topics/documentation/): reader-task page
  types, documentation coverage, examples, and docs-impact rules.
- [Release Roadmap](../ROADMAP.md): scheduled alpha milestones, release gates,
  and the GitHub issue/milestone map.
- [v0.3 Release Notes](./releases/v0.3.0-alpha.1.md): publish-ready notes for
  the compiler-spine, canonical Core encoder, reviewed golden bytes, and exact
  digest alpha.
- [v0.2 Release Notes](./releases/v0.2.0-alpha.1.md): published notes for the
  Core semantic model and normative schema alpha.
- [v0.1 Release Notes](./releases/v0.1.0-alpha.1.md): published notes for the
  first front-end alpha milestone.
- [Release Process Topic](./topics/release-process/): tag-triggered GitHub
  Release publication contract and verification matrix.
- [Contract Bundles Topic](./topics/contract-bundles/): typed v1
  participant-neutral bundle and assurance evidence manifest validation.
- [Lowerability Topic](./topics/lowerability/): typed v1 lowering
  requirements, target-profile facts, and direct-only support classification.
- [Target Profiles Topic](./topics/target-profiles/): typed v1 target-profile
  manifest conformance and runtime-neutral profile validation.
- [Compiler Spine Topic](./topics/compiler-spine/): executable
  source-AST-to-in-memory-Core stage contract for the initial lowerable subset.
- [Core IR Topic](./topics/core-ir/): current contract and verification matrix
  for the `edict.core/v1` semantic model and CDDL schema.
- [Syntax Topic](./topics/syntax/): current contract and verification matrix for
  the Phase 1 `edict-syntax` lexer/parser.
- [SPEC - Edict Language v1](./SPEC_edict-language-v1.md): source syntax, type
  system, effect rules, Core IR, and language-level canonical value semantics.
- [SPEC - Edict Lawpack ABI v1](./SPEC_edict-lawpack-abi-v1.md): lawpack
  manifest and dependency graph, exported types/constants, pure helper and
  semantic effect signatures, proof-only vs runtime-materialized classification,
  typed obstruction payloads, footprint/cost obligations, and target adapters.
- [SPEC - Edict Target Profile ABI v1](./SPEC_edict-target-profile-abi-v1.md):
  intrinsic signatures, effect signatures, target lowering, application model,
  verifier ABI, footprint algebra, and cost algebra. Canonical schemas live in
  [`abi/`](./abi/) (`edict-target-profile.cddl`, `edict-target-lowerer.wit`).
- [SPEC - Continuum Contract Bundle v1](./SPEC_continuum-contract-bundle-v1.md):
  participant-neutral contract bundle identity, artifact graph, provenance
  references, canonical CBOR/hash framing, and attestation roles.
- [SPEC - Continuum Admission v1](./SPEC_continuum-admission-v1.md):
  participant descriptors, policy epochs, admission requests, admission
  receipts, capability receipts, and participant-specific decisions.
- [GUIDE - Edict Assurance and Transparency](./GUIDE_edict-assurance-transparency.md):
  HOLMES, Watson, Moriarty, transparency logs, nutrition labels, profile diffs,
  relapse fuzzing, the hash ladder, the Aperture Ledger, and the two-lowerer
  trial.
- [REQUIREMENTS - Fixture Constitution](./REQUIREMENTS.md): every normative
  requirement ID, its owner spec, and its positive/negative/golden fixtures.
- [Design Baseline](./DESIGN_runtime-neutral-edict-sha-lock-assurance.md):
  original runtime-neutral Edict/SHA-lock design packet retained as
  non-normative context.

Machine-readable ABIs live in [`abi/`](./abi/) and are the single source of
truth for the artifacts they describe; prose JSON in the specs is generated from
them.
