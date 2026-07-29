# External-Action Requests

Status: current HEAD contract.

Edict can construct a typed request for one external operation without
performing that operation. The request is deterministic compiler data. Echo
later admits and records it, an authorized adapter performs the boundary
crossing, and a schema-validated settlement may resume the program. This topic
owns request construction only.

## Source Surface

A requestable operation family enters source through a digest-locked
`capability` import:

```edict
use capability workspace.snapshot.observe@1
  digest "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  as snapshot;
```

The alias is callable only in a `request` statement:

```edict
request pending: ExternalActionRequest<Bytes<max=65536>> =
  snapshot(input.payload)
  input schema workspace.snapshot.input@1 digest "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  settlement schema workspace.snapshot.settlement@1 digest "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
  authority input.scope
  basis input.basis
  budget maxSettlementBytes input.maxSettlementBytes maxAttempts input.maxAttempts
  reconcile workspace.snapshot.reconcile@1 digest "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
```

The operation takes one typed input. Scope, basis, and both budgets are ordinary
typed expressions, so their runtime values survive compilation for Echo
admission. Schemas and the reconciliation law are exact digest-locked
resources. [EXTREQ-REQ-001] [EXTREQ-REQ-006]

## Core And Target IR

Core emits `ExternalActionRequest` as a distinct node containing:

- compiler-owned binding identity;
- exact operation resource;
- input and settlement type coordinates;
- exact input and settlement schemas;
- input, authority-scope, and basis expressions;
- maximum settlement bytes and attempt expressions;
- exact reconciliation law;
- fixed `awaitingSettlement` state and `schemaRequired` settlement posture.

The operation resource must equal one `capability` import. Canonical Core
encoding fails closed when that closure is missing. [EXTREQ-REQ-002]
[EXTREQ-REQ-004]

Target lowering copies the node into `externalActionRequests`. It never turns
the operation into a target step or `targetIntrinsic`. The Target IR semantic
closure binds the exact capability resource alongside the source Core and any
lawpacks; canonical Target IR encoding refuses a request whose operation is
absent from that closure. [EXTREQ-REQ-003] [EXTREQ-REQ-004]

Equivalent source and compiler facts produce byte-identical Core and Target IR.
Every operation, schema, input, scope, basis, budget, or reconciliation change
participates in canonical identity. [EXTREQ-REQ-005]

## Authority Boundary

Request authority is not performance authority:

- a capability alias cannot be invoked as an ordinary semantic effect;
- the current request-family allowlist contains only the domain-specific
  `workspace` root used by `workspace.snapshot.observe@1`;
- raw `filesystem`, `process`, `network`, Git, GitHub, `model`, and `shell`
  operation families, case variants, abbreviations, and unregistered roots are
  rejected with `UnrequestableExternalOperation`;
- compiling and lowering perform no I/O;
- the compiler/provider component interface gains no callable import;
- dynamic path, ref, basis, budget, adapter, and settlement constraints remain
  Echo admission obligations.

The first admitted family is domain-specific
`workspace.snapshot.observe@1`, not ambient filesystem access.
[EXTREQ-REQ-003] [EXTREQ-REQ-006] [EXTREQ-REQ-007]

## Waiting And Settlement

The request binding and Target IR request id identify explicit
`awaitingSettlement` program state. No native stack, continuation, callback, or
`async` host frame is serialized. This Edict slice does not define settlement
execution or resumption; Echo owns durable request, claim, settlement, recovery,
and replay semantics.

## Deferred

- Echo admission and durable settlement of the compiler-emitted request;
- bounded workspace-observation adapter execution;
- settlement-driven deterministic resumption;
- basis-bound validated patch application;
- Git, GitHub, process, network, timer, and model adapters;
- the autonomous delivery loop.

The verification matrix, fixed seed, and stress bound are in
[test-plan.md](./test-plan.md).
