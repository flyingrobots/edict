# Edict

Edict is a restricted deterministic language for lawful operations. It compiles
optic-shaped intent source into Edict Core IR, lowers through explicit target
profiles, and binds the resulting artifacts into participant-neutral contract
bundles.

Edict is intended as a contribution toward
[Continuum](https://github.com/flyingrobots/continuum), the protocol suite for
lawful causal interoperability over witnessed causal history. Continuum is not a
runtime, database, compiler, debugger, filesystem, service registry, app
framework, or universal graph. It is the shared protocol vocabulary by which
heterogeneous participants say what happened, from which basis, under which law,
with which witness, and with what outcome.

## Why Edict?

Continuum's core distinction is that history is the territory. State is a
policy-relative materialized view, the graph is a coordinate chart, files are
readings, and admission is witnessed. A message, edit, import, or generated
artifact arriving at a host does not become causal truth until the relevant
runtime or authority admits it against a bounded basis under explicit law.

That creates a gap between three useful layers:

- [GraphQL](https://spec.graphql.org/) can describe contract-family shape: the
  fields, operations, values, and callable surfaces that cross boundaries.
- [Wesley](https://github.com/flyingrobots/wesley) can compile those shapes,
  `weslaw` facts, codecs, validators, manifests, and generated access artifacts
  into deterministic evidence.
- Continuum can name the shared protocol envelopes, evidence posture,
  witnesses, readings, suffix exchange, admission outcomes, and compatibility
  truth.

Those layers still do not provide a portable language for the lawful operation
itself. They can say what the callable surface is, what evidence was generated,
and how an admission should be witnessed. They do not, by themselves, give an
agent or tool a deterministic way to say:

- what aperture over causal history it is allowed to inspect or affect;
- what basis, frontier, and bounds the operation depends on;
- which effects are proof-only and which materialize at runtime;
- how target failures become domain obstructions;
- what cost and footprint must be checked before execution;
- what support obligations and witness debt the result carries;
- which target-owned atomic application unit will verify the result.

Edict is a proposed answer to that missing layer. The better category-theory
object is not a plain function or unconstrained morphism. It is an
Observer-Geometry-shaped optic: a focused, bounded, evidence-bearing operation
over witnessed causal history.

A read intent is a revelation optic. It projects a bounded aperture into a
reading without authoring history. A write intent is an affect/reintegration
optic. It proposes effects against a basis and carries the guards, support
obligations, and obstruction vocabulary needed for a participant to decide
whether the result can enter admitted history. A semantic lawpack intent is a
portable optic candidate that can be interpreted into different target profiles
without pretending those targets share a storage substrate.

Edict Core is the normalized form of that optic. A target profile is then a
structure-preserving interpretation into a runtime-owned execution category,
such as [Echo](https://github.com/flyingrobots/echo) DPO, a KV/CAS transaction
profile, or another participant-owned target. A valid lowering must preserve the
Observer Geometry structure: basis, aperture, projection or affect boundary,
footprint independence, support ledger posture, atomic guards, cost budgets,
obstruction classes, and canonical artifact identity.

This framing is practical, not decorative. It separates source derivation
honesty from destination admission lawfulness. An Edict bundle can be honestly
compiled from its source and still be obstructed, pluralized, conflicted, or
rejected by a destination participant because Continuum admission remains
runtime-owned and basis-relative. That is the point: Edict should make proposed
operations inspectable and reproducible without pretending to be the runtime,
the protocol, or the final admission authority.

The goal is maximum autonomy only after maximum explicitness: no ambient host
authority, no hidden storage model, no unchecked filesystem or network access,
and no trust-me callbacks. Edict operations should either compile into
SHA-locked, target-verified artifacts or fail with structured reasons that
humans and agents can repair.

## Related Projects

- [Continuum](https://github.com/flyingrobots/continuum): shared causal
  vocabulary, protocol profiles, evidence posture, admission and outcome
  vocabulary, conformance posture, and compatibility truth.
- [Wesley](https://github.com/flyingrobots/wesley): GraphQL and `weslaw`
  compiler substrate for contract-family shape, law facts, manifests, codecs,
  validators, and generated artifacts.
- [Echo](https://github.com/flyingrobots/echo): sibling Continuum runtime
  implementation for hot/interactive admitted causal history.
- [Graft](https://github.com/flyingrobots/graft): structural observer and
  review engine that consumes observer plans, reading envelopes, and evidence
  posture.
- [jedit](https://github.com/flyingrobots/jedit): product-pressure editing app
  used to test whether the language handles real intents, readings, stale bases,
  checkpoints, and structural history.
- [WARP TTD](https://github.com/flyingrobots/warp-ttd): debugger/operator
  surface over Continuum profiles.
- [WARP DRIVE](https://github.com/flyingrobots/warp-drive): POSIX-shaped
  membrane over readings and intents.
- [Think](https://github.com/flyingrobots/think): agent/session/history app
  candidate for observation, counterfactual, and evidence-ledger profiles.
- [Bijou](https://github.com/flyingrobots/bijou): rendered UI/TUI candidate for
  reading envelopes and intent-producing interfaces.
- [AION](https://github.com/flyingrobots/aion): theory source and conceptual
  background for causal worlds, optics, and braid theory.

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
