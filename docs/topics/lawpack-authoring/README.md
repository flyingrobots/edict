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
closure validation, then compares the complete output tree byte for byte under
the publication lock. It never repairs or changes the owned artifact tree; it
may create the persistent sibling lock file used only for coordination.
Missing, changed, or extra output is `LawpackOutputDrift`.

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
strings; floating-point JSON numbers are rejected. Canonical JSON permits at
most 128 containing arrays or objects around a terminal value; another
container is rejected before recursive conversion. Constant values and Edict
pure-function bodies reserve three of those levels for the exports map, member
array, and member map that enclose them in the canonical artifact.

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
output cannot be nested beneath an ancestor containing another lawpack output
index. Edict acquires every proper ancestor output lock in top-down order,
rechecks ancestor ownership while holding those locks, and retains them through
check or publication. Parent and child builds therefore cannot use different
locks to race across overlapping replacement trees. A blocking common
publication coordinator is acquired before output inspection and remains held
through completion, so the same rule covers parent and child outputs whose
build documents have different roots. This deliberately serializes lawpack
write and check operations across those roots.

An existing non-empty directory without a valid `edict.lawpack-output/v1`
index is refused rather than deleted. Output paths and input dependency paths
must be confined relative paths, and dependency inputs must remain outside the
owned output tree. Existing symlink traversal is rejected. Authored files may
not collide by using another file as a parent, and the ownership-index path is
reserved for Edict. Edict indexes every emitted file path and checks each
proper ancestor against that set, so duplicate and file/descendant collisions
remain fail-closed without pairwise growth as application-owned resource sets
expand. Artifact paths containing a filesystem NUL byte are preflighted
immediately after the build document is decoded, before output inspection,
coordination, or dependency filesystem I/O.

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
creating that parent or the sibling publication lock.

A persistent hidden lock file is kept beside the output directory so concurrent
processes cannot acquire locks on different inodes while the directory itself
is replaced; it is coordination state, not a canonical lawpack artifact, and
should remain untracked. A second persistent lock in the operating-system
temporary directory is the common cross-root publication coordinator; it is
also non-canonical coordination state. Activation is the publication commit point. Failure to
remove the hidden previous-tree backup after activation does not turn a
committed replacement into a failed command; that backup is best-effort cleanup
state and may be removed by the operator.

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
