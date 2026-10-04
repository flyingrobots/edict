# Jim state-read boundary witness

This Docker-only integration witness builds the current Edict source and
fetches two exact public inputs: Jim's authored state-read probe at
`19edb6fba94a8fea2dea63aa2f05cffc3e084f97` and Echo's existing provider at
`49e9efb68001dfd78563d18bac9359a87671e431`.

From the repository root:

```sh
docker build -f scripts/consumer-witnesses/jedit-state-read.Dockerfile \
  -t edict-ordered-jim-witness .
docker run --rm edict-ordered-jim-witness
```

No host repository is mounted. Docker compiles Edict and runs both public JSONL
lawpack/application builds in temporary container storage. The first build
retains Jim's original v1 adapter and reproduces `TargetLoweringFailed`. The
second explicitly selects `echo.span-ir/v2` in a disposable lawpack copy,
binds its contract reference to the current CDDL bytes, republishes the lawpack,
and updates only the source's lawpack import digest.

The authored read/guard body is unchanged. It passes Core and Target lowering,
canonical encoding, and result-projection verification, then reaches the old
provider's schema gate: `InvalidProviderInvocation`. Neither attempt publishes
application output. Successful verification prints
`ORDERED_PUBLIC_BUILD_REACHED_OLD_PROVIDER_GATE`.

The v2 contract reference is experimental; this is not evidence of a released
Echo v2 provider. No executable artifact is handwritten, no native edit planner
is called, and no runtime state read occurs. Original Jim application locks and
producer pins are not edited. Echo #684 owns subsequent provider/interpreter
support; Jim #296 owns complete ReplaceRange behavior.
