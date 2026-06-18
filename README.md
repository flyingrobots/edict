# Edict

Edict is a restricted deterministic language for lawful operations. It compiles
source into Edict Core IR, lowers through explicit target profiles, and binds
the resulting artifacts into participant-neutral contract bundles.

The language exists to make autonomous operation boring in the right places:
explicit law, bounded effects, target-owned verification, canonical artifacts,
and participant admission.

## Current Status

This repository is in Phase 0. It currently holds the design baseline and
specification split for:

- Edict language syntax, type system, effects, and Core IR.
- Target profile ABI and atomic application semantics.
- Continuum contract bundle identity.
- Continuum admission artifacts.
- Assurance and transparency guidance.

## Specifications

- [SPEC - Edict Language v1](./docs/SPEC_edict-language-v1.md)
- [SPEC - Edict Target Profile ABI v1](./docs/SPEC_edict-target-profile-abi-v1.md)
- [SPEC - Continuum Contract Bundle v1](./docs/SPEC_continuum-contract-bundle-v1.md)
- [SPEC - Continuum Admission v1](./docs/SPEC_continuum-admission-v1.md)
- [GUIDE - Edict Assurance and Transparency](./docs/GUIDE_edict-assurance-transparency.md)
- [Design Baseline](./docs/DESIGN_runtime-neutral-edict-sha-lock-assurance.md)

## Repository Boundary

Edict owns the language, Core IR, canonicalization rules, conformance fixtures,
and target-profile ABI surface.

Wesley owns GraphQL and `weslaw` source-profile adapters. Continuum owns
participant protocol and admission. Echo owns `echo.dpo@1` target semantics.

## License

Apache-2.0. See [LICENSE](./LICENSE).
