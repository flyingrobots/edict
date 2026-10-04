# Jim state-read boundary witness

This Docker-only integration witness builds the current Edict source and
fetches two exact public inputs: Jim's authored state-read probe at
`19edb6fba94a8fea2dea63aa2f05cffc3e084f97` and Echo's existing provider at
`49e9efb68001dfd78563d18bac9359a87671e431`.

Reuse a compatible COPY-based worker and its existing Cargo cache first. Copy
this revision and the pinned consumer/provider source into that worker, then
run `sh scripts/consumer-witnesses/run-jedit-state-read.sh` from the copied
repository root under its validation lock. `CARGO_TARGET_DIR` must be absolute;
the entrypoint selects the binary from that directory. Do not mount a host
repository, Git directory, or recovery checkout.

If no compatible worker exists, the following standalone invocation builds a
source/toolchain image and compiles only at container runtime. Its read-only
root and size-limited temporary filesystems cap generated build storage at
8 GiB and scratch data at 256 MiB. These caches disappear when the container
exits; use the existing worker for repeated audits. Check aggregate project
cache usage against the 20 GiB budget and host free space against the 50 GiB
floor before running.

From the repository root:

```sh
docker build -f scripts/consumer-witnesses/jedit-state-read.Dockerfile \
  -t edict-consumer-witness .
docker run --rm --read-only --cpus=4 --memory=12g --pids-limit=512 \
  --tmpfs /build-cache:rw,size=8g --tmpfs /tmp:rw,size=256m \
  edict-consumer-witness
```

No Cargo output is baked into the image. Docker compiles Edict at runtime and
runs both public JSONL
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
