# Result Projections

Status: current HEAD contract.

This shelf describes Edict's compiler-owned representation of an application
operation result. A result projection preserves how the authored result is
assembled from declared application input and capability-step results without
executing the operation or introducing host-authored application semantics.
[RESULT-PROJ-REQ-001] [RESULT-PROJ-REQ-007]

## Current Contract

`edict_syntax::emit_result_projection` accepts one exact Core module, its
matching Target IR artifact, and an intent name. The emitter requires:

- the intent to exist in both artifacts;
- the Target IR semantic closure to bind the exact Core coordinate and
  canonical Core digest;
- the Core and Target IR result expressions, operation profile, and evaluation
  budget to agree;
- the application input to be the declared `arg.0` local with the intent input
  type; and
- every capability-result source to identify exactly one Target IR step whose
  binding, effect coordinate, and input expression match the corresponding
  Core effect node.

No projection is emitted after any of these checks fails.
[RESULT-PROJ-REQ-001] [RESULT-PROJ-REQ-005]

The closed `edict.result-projection/v1` expression language contains only:

- records with canonically ordered field names;
- the declared application input;
- capability results addressed by Target IR step identity; and
- bounded field paths rooted at one of those declared sources.

Constants, calls, undeclared locals, unbound capability steps, and other Core
expression forms are outside this version and fail with stable
`ResultProjectionFailureKind` values. Output records must reconstruct the
declared Core output type exactly; missing, extra, or type-incompatible fields
reject before artifact identity exists. [RESULT-PROJ-REQ-002]
[RESULT-PROJ-REQ-006]

Each projection carries:

- schema identity `edict.result-projection/v1`;
- the Core module and intent operation coordinate;
- the declared Core output type;
- the authored `maxOutputBytes` evaluation bound; and
- the closed projection expression.

The representation limits one artifact to 256 expression nodes, 32 path
segments per source, 1,024 UTF-8 bytes per coordinate or field string, and
65,536 canonical bytes. A zero output bound and every one-over-limit value
reject before admission. [RESULT-PROJ-REQ-004]

`encode_result_projection` emits `edict.canonical-cbor/v1` bytes.
`digest_result_projection` frames those exact bytes under
`edict.result-projection.artifact/v1`. The generated provider contract pack
publishes the `result-projection` CDDL root and the matching artifact-domain
binding. The reviewed Hello Echo bytes and digest under
`fixtures/lawpack/hello-echo/` are reproduced only by
`cargo xtask lawpack-goldens`. [RESULT-PROJ-REQ-003]

## Independent Verification

`verify_result_projection` does not call the emitter or trust the claimed
digest. It:

1. decodes the claimed bytes through the canonical CBOR decoder;
2. validates the closed projection shape and representation bounds;
3. reproduces the exact canonical bytes;
4. recomputes the domain-framed identity;
5. independently rebuilds the Core-to-Target step/source correspondence;
6. reconstructs a Core result expression from the projection; and
7. requires that reconstruction to equal both the authored Core result and the
   matching Target IR result.

Only then does the API return `VerifiedResultProjection`, whose fields are
available through read-only accessors. [RESULT-PROJ-REQ-005]
[RESULT-PROJ-REQ-006]

```text
Edict source + lawpack closure
    -> Core result
    -> matching Target IR result
    -> bounded canonical projection
    -> independent reverse reconstruction
    -> verified projection
```

## Authority Boundary

The projection is data, not an execution callback. Emission and verification
perform no filesystem, network, process, model, runtime, or application-host
operation. The representation names application and capability data already
declared by Core and Target IR; it does not grant authority to obtain those
values. [RESULT-PROJ-REQ-007]

Echo #698 owns generic runtime evaluation, durable result binding, recovery,
and exposure of the projected value. Hello Echo #18 owns the external
application proof. Neither responsibility is implemented in this Edict
contract.

## Deferred

- Runtime evaluation of a verified projection.
- Durable binding of evaluated result bytes to an Echo Action, Tick, outcome,
  Receipt, or recovery record.
- Additional expression forms beyond records and bounded source field paths.
- External effects, model invocation, native callbacks, or application-specific
  reconstruction.

The executable verification matrix is tracked in
[test-plan.md](./test-plan.md).
