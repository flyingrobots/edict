# Author an Application-Owned Lawpack

This guide shows an application repository how to turn one reviewable JSON
definition into the canonical lawpack artifacts consumed by `edict application
build`. The application owns the vocabulary and declarations. Edict owns schema
checking, canonical CBOR, digest framing, closure validation, and publication.
No provider component or runtime participates in authoring.

## Authoring Workflow

### 1. Create the Review Document

Create an `edict.lawpack-build/v1` document in the application repository. Its
paths are relative to the document, not the process working directory:

```json
{
  "schema": "edict.lawpack-build/v1",
  "outputDirectory": "generated/example-text",
  "lawpack": {
    "schema": "edict.lawpack-authoring/v1",
    "id": "example.text",
    "version": "1",
    "acceptedCoreAbi": ["edict.core/v1"],
    "dependencies": [],
    "exportsCoordinate": "example.text.exports/v1",
    "exports": {
      "types": [{
        "coordinate": "example.text@1.Key",
        "definition": "String<max=64>"
      }],
      "constants": [],
      "pureFunctions": [],
      "effects": [],
      "obstructions": [],
      "operationProfiles": {}
    },
    "targetAdapters": [],
    "verifier": {
      "class": "declarative",
      "ruleset": {
        "id": "example.text.verifier/v1",
        "digest": "sha256:1111111111111111111111111111111111111111111111111111111111111111"
      }
    },
    "compatibility": {
      "id": "example.text.compatibility/v1",
      "digest": "sha256:2222222222222222222222222222222222222222222222222222222222222222"
    },
    "conformanceFixtureCorpus": {
      "id": "example.text.fixtures/v1",
      "digest": "sha256:3333333333333333333333333333333333333333333333333333333333333333"
    },
    "localResources": []
  },
  "dependencyBundles": []
}
```

### 2. Submit the Build Request

Submit the document through the normal JSONL CLI:

```json
{"schema":"edict.compiler.settings/v1","type":"compilerSettings","operation":"build","lawpack":"edict.lawpack.json"}
```

### 3. Inspect the Authored Artifacts

On success, `generated/example-text/` contains:

- `manifest.cbor` and `manifest.sha256`;
- `exports.cbor` and `exports.sha256`;
- each declared adapter and its `.sha256` sidecar;
- each declared local resource, including target configuration, and its
  `.sha256` sidecar;
- `edict.lawpack-output.json`, the ownership and review index for the generated
  directory.

The `.sha256` files are lowercase review strings ending in one newline. The
canonical artifacts carry typed digest bytes internally; review strings are not
their wire representation.

### 4. Check Generated Output

Use `checkOnly` in CI or before committing vendored output:

```json
{"schema":"edict.compiler.settings/v1","type":"compilerSettings","operation":"build","lawpack":"edict.lawpack.json","checkOnly":true}
```

Check-only mode performs the same bounded reads, construction, decoding, and
closure validation, then compares the complete output tree byte for byte through
one pinned directory handle. Before returning success it verifies that the
requested output name still denotes that same filesystem identity and ownership
basis. It never repairs or changes the owned artifact tree and creates no parent
directories or lock files.
Missing, changed, or extra output is `LawpackOutputDrift`.
Missing output ancestors also report `LawpackOutputDrift`; non-directory or
symlinked ancestors report `LawpackOutputOwnershipFailed`. Check-only never
reports `LawpackOutputWriteFailed` because it performs no publication. Its
parent-chain gate rejects every component that is not a normal relative name.

## Resource And Dependency Identity

An external resource uses an application-supplied exact identity:

```json
{"id":"echo.dpo@1","digest":"sha256:7777777777777777777777777777777777777777777777777777777777777777"}
```

A local resource places reviewable JSON in the authoring document:

```json
{
  "name": "target-config",
  "coordinate": "example.text.target-config/v1",
  "output": "resources/target-config.cbor",
  "value": {"maxBytes": 4096}
}
```

Other declarations refer to it as `{"local":"target-config"}`. Edict converts
the value to canonical CBOR and derives its coordinate-framed identity. JSON
objects of the exact form `{"$edictBytes":"00ff"}` represent canonical byte
strings; floating-point JSON numbers are rejected. Lawpack authoring permits at
most 48 containing arrays or objects around a terminal value; another
container is rejected before recursive conversion. Constant values and Edict
pure-function bodies reserve three of those levels for the exports map, member
array, and member map that enclose them in the canonical artifact. This
authoring-specific limit leaves normal-thread stack headroom while the general
canonical-CBOR profile retains its independent 128-container limit.

Every `dependencies` edge names an id, version, and exact manifest digest. The
matching canonical manifest and exports paths appear in `dependencyBundles`.
The caller must supply the complete transitive closure and nothing disconnected
from the authored root. Edict decodes each dependency, corroborates every pin,
and runs the same complete-graph validator used by application builds.

### 5. Apply Publication Policy

`outputDirectory` is exclusively owned by this build after its output index is
present. A write build stages a complete sibling directory, preserves the old
directory, activates the replacement, and restores the old directory if
activation fails. A later successful build replaces the whole owned directory,
so artifacts removed from the definition cannot survive as stale output. An
internal transaction or backup name is fixed-length and independent of the
user-selected output component, so every accepted output name remains
publishable within portable component limits. Output-directory components that
could alias Edict's `.edict-lawpack-...` transaction names or hidden
`.*.edict-lawpack-build.lock` coordination files are reserved case-insensitively
and reject before filesystem access. Raw output-directory paths use `/` as their
only separator, reject empty and platform-reserved components, and cap each
component at 229 bytes so its derived lock name remains portable. An output
cannot be nested beneath an ancestor containing another lawpack output index.
Within one document-root publication namespace, Edict acquires shared
intent locks for proper output ancestors in top-down order and an exclusive lock
for the output itself. Sibling output footprints can publish concurrently;
identical and parent/child footprints conflict. The real-directory ancestor
chain is rechecked after intent acquisition. The publication root and output
parent are then pinned as capability directories. Write mode stages the complete
replacement, captures the currently named output, and authorizes that exact
captured directory through a retained capability handle. Staging, activation,
rollback, and cleanup cannot be redirected by a later ambient-path replacement,
and rollback refuses to delete an output that appeared concurrently.
Check-only performs no locking or filesystem mutation. Concurrent overlapping
publication from different document roots is outside one namespace and must be
avoided by the caller.

An existing non-empty directory without a valid
`edict.lawpack-output.json` ownership index (schema
`edict.lawpack-output/v1`) is refused rather than deleted. Output paths and
input dependency paths must be confined relative paths, and dependency inputs
must remain outside the owned output tree. Existing symlink traversal is
rejected. Authored files may not collide by using another file as a parent, and
the ownership-index path is reserved for Edict. Edict indexes every emitted
file path and checks each proper ancestor against that set, so duplicate and
file/descendant collisions
remain fail-closed without pairwise growth as application-owned resource sets
expand. Pure preflight derives the fixed manifest and exports, every authored
artifact, every digest sidecar, and the reserved ownership-index namespace, then
rejects duplicates and file/descendant collisions immediately after the build
document is decoded and before output inspection, coordination, or dependency
filesystem I/O. Primary `.cbor` paths reserve the two bytes added when their
extension becomes `.sha256`; derived sidecars are separately validated at the
255-byte component and 1024-byte relative-artifact ceilings. Output components
use bounded lowercase ASCII letters, digits, `.`, `_`, and `-`, and reject
Windows device names, filesystem NUL, case aliases, trailing-dot aliases,
overlong components or relative paths, and
platform-specific forbidden punctuation. Only `/` separates non-empty raw path
components; backslashes and repeated separators reject before host path parsing.

The index identity must match the lawpack being authored. Edict refuses to
replace or check a tree owned by a different lawpack id or version even when
that tree otherwise carries a structurally valid ownership index.

Definition, dependency, generated-index, and published-artifact reads are
bounded to 1 MiB per file, generated artifacts are refused before publication
if they exceed that same read boundary, and the supplied dependency closure is
bounded to 192 bundles. Raw authoring JSON rejects duplicate object keys before
typed deserialization, so no hash-significant value can silently replace an
earlier review value. Check-only traversal accepts only directories needed by
expected artifacts and rejects an unexpected file before reading its contents,
so drift cannot force accumulation of an arbitrary output tree in memory. A
missing nested output parent is reported as `LawpackOutputDrift` without
creating that parent or a sibling publication lock. A successful check reopens
the output after traversal and reports drift if either its directory identity or
ownership basis changed.

A persistent hidden lock file is kept beside each write output and its proper
ancestors. These files are footprint-coordination state, not canonical lawpack
artifacts, and should remain untracked. There is no process-wide, host-wide, or
filesystem-wide publication coordinator. Activation is the publication commit
point. Failure to remove the hidden previous-tree backup after activation does
not turn a committed replacement into a failed command; that backup is
best-effort cleanup state and may be removed by the operator.

## The Five Artifacts That Must Stay Distinct

| Artifact | Owner | Meaning |
| --- | --- | --- |
| Lawpack authoring JSON | Application | Reviewable semantic declarations and exact external pins. |
| Generated lawpack artifacts | Edict encoder | Canonical manifest, exports, adapters, configurations, and derived identities. |
| Application `.edict` source | Application | Executable application law that imports the exact generated manifest digest. |
| Provider package | Target provider | Target profile plus bounded lowering and verification components; unused during authoring. |
| Runtime receipt | Runtime | Evidence about one admitted execution; neither input to nor output from authoring. |

The independent application witness
`external_application_authors_vendors_and_builds_its_own_lawpack` runs outside
the Edict checkout. It authors the workspace-snapshot closure through the
public binary, reproduces the four reviewed canonical artifacts byte for byte,
and then feeds those generated bytes to the public application build. The
fixture proves authoring and compilation, not runtime execution.

## Failure Boundary

Malformed definitions fail before artifact return. Emitted manifest, exports,
and adapter bytes are passed back through `decode_lawpack_bundle`,
`decode_lawpack_adapter`, and `validate_lawpack_dependency_graph` before any
publication. Stable CLI kinds distinguish invalid definitions or digests,
unresolved local resources, invalid canonical values, incomplete adapters,
dependency substitution, path escape or prefix collision, unknown tagged
fields, output ownership, output size, output drift, and publication failure.

Edict does not discover application semantics from schemas or fixtures, invoke
a provider, build an executable package, admit an operation, or create a runtime
receipt on this path.
