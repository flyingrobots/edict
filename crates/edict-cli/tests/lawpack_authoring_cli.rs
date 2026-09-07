use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use edict_syntax::{TARGET_PROVIDER_ABI, TARGET_PROVIDER_MANIFEST_API_VERSION};
use serde_json::{json, Value};

static COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn external_application_authors_vendors_and_builds_its_own_lawpack() {
    let root = temp_tree("consumer");
    let caller = temp_tree("caller");
    write_external_consumer(&root);

    let lawpack_document = root.join("edict.lawpack.json");
    let authored = run_edict(
        &caller,
        &jsonl(&json!({
            "schema": "edict.compiler.settings/v1",
            "type": "compilerSettings",
            "operation": "build",
            "lawpack": lawpack_document,
        })),
    );
    assert_success(&authored, "lawpack authoring");

    let generated = root.join("vendor/generated-workspace-snapshot");
    for file in [
        "manifest.cbor",
        "exports.cbor",
        "adapter.cbor",
        "request-profile-configuration.cbor",
    ] {
        assert_eq!(
            fs::read(generated.join(file)).unwrap_or_else(|error| {
                panic!("read generated {file}: {error}");
            }),
            fs::read(
                fixture_root()
                    .join("fixtures/lawpack/workspace-snapshot")
                    .join(file)
            )
            .unwrap_or_else(|error| panic!("read reviewed {file}: {error}")),
            "public authoring must reproduce the existing reviewed artifact `{file}`"
        );
    }

    let application = run_edict(
        &caller,
        &jsonl(&json!({
            "schema": "edict.compiler.settings/v1",
            "type": "compilerSettings",
            "operation": "build",
            "application": root.join("edict.application.json"),
        })),
    );
    assert_success(&application, "application build");

    for (actual, reviewed) in [
        ("core.cbor", "observe-workspace.core.cbor"),
        ("target-ir.cbor", "observe-workspace.target-ir.cbor"),
    ] {
        assert_eq!(
            fs::read(root.join(".build/application").join(actual))
                .unwrap_or_else(|error| panic!("read built {actual}: {error}")),
            fs::read(
                fixture_root()
                    .join("fixtures/lawpack/workspace-snapshot")
                    .join(reviewed)
            )
            .unwrap_or_else(|error| panic!("read reviewed {reviewed}: {error}")),
            "application build must consume the exact publicly authored closure"
        );
    }

    fs::remove_dir_all(root).expect("remove consumer tree");
    fs::remove_dir_all(caller).expect("remove caller tree");
}

#[allow(
    clippy::too_many_lines,
    reason = "the standalone witness keeps its complete external file closure visible"
)]
fn write_external_consumer(root: &Path) {
    let reviewed = fixture_root().join("fixtures/lawpack/workspace-snapshot");
    let source = root.join("src");
    let vendor = root.join("vendor/workspace-snapshot");
    let provider = root.join("provider/generated/primary");
    for directory in [&source, &vendor, &provider] {
        fs::create_dir_all(directory).expect("create external consumer directory");
    }
    fs::copy(
        reviewed.join("observe-workspace.edict"),
        source.join("observe-workspace.edict"),
    )
    .expect("copy application source");
    for file in [
        "input-schema.cbor",
        "settlement-schema.cbor",
        "reconciliation-law.cbor",
    ] {
        fs::copy(reviewed.join(file), vendor.join(file))
            .unwrap_or_else(|error| panic!("copy {file}: {error}"));
    }
    fs::copy(
        fixture_root().join(
            "fixtures/providers/echo-target-profile/generated/primary/target-profile.echo-dpo.cbor",
        ),
        provider.join("target-profile.echo-dpo.cbor"),
    )
    .expect("copy target profile");

    let lawpack = workspace_snapshot_lawpack_document();
    fs::write(
        root.join("edict.lawpack.json"),
        serde_json::to_vec_pretty(&lawpack).expect("encode lawpack document"),
    )
    .expect("write lawpack document");

    let provider_manifest = json!({
        "apiVersion": TARGET_PROVIDER_MANIFEST_API_VERSION,
        "providerAbi": TARGET_PROVIDER_ABI,
        "provider": resource_json("echo.edict-provider@1", '1'),
        "artifacts": [
            {
                "role": "target-profile.echo-dpo",
                "artifactKind": "targetProfile",
                "resource": {
                    "coordinate": "echo.dpo@1",
                    "digest": "sha256:2e2494121aecf5e6a2d920f5fb85408825d394765fad41484c416397c920fb04"
                },
                "source": {
                    "kind": "generated",
                    "semanticSource": resource_json("echo.semantic-schema@1", '2'),
                    "generator": resource_json("echo-wesley-gen.provider-artifact-generator@1", '3')
                }
            },
            {
                "role": "schema.echo-provider-artifacts",
                "artifactKind": "artifactSchema",
                "resource": resource_json("echo.provider-artifacts.cddl@1", '4'),
                "source": {
                    "kind": "generated",
                    "semanticSource": resource_json("echo.semantic-schema@1", '2'),
                    "generator": resource_json("echo-wesley-gen.provider-artifact-generator@1", '3')
                }
            }
        ],
        "schemaBindings": [{
            "domain": "echo.generated-artifact/v1",
            "schemaRole": "schema.echo-provider-artifacts",
            "format": "selfContainedCddlV1",
            "rootRule": "generated-artifact"
        }]
    });
    fs::write(
        root.join("provider/provider-manifest.echo.json"),
        serde_json::to_vec_pretty(&provider_manifest).expect("encode provider manifest"),
    )
    .expect("write provider manifest");

    let application = json!({
        "schema": "edict.application/v1",
        "buildKind": "externalAction",
        "coordinate": "examples.workspace_observer@1",
        "sources": ["src/observe-workspace.edict"],
        "lawpacks": [{
            "manifest": "vendor/generated-workspace-snapshot/manifest.cbor",
            "exports": "vendor/generated-workspace-snapshot/exports.cbor",
            "adapter": "vendor/generated-workspace-snapshot/adapter.cbor",
            "targetConfiguration": "vendor/generated-workspace-snapshot/request-profile-configuration.cbor"
        }],
        "externalActionResources": [
            {"artifact": "vendor/workspace-snapshot/input-schema.cbor"},
            {"artifact": "vendor/workspace-snapshot/settlement-schema.cbor"},
            {"artifact": "vendor/workspace-snapshot/reconciliation-law.cbor"}
        ],
        "target": {"profile": "echo.dpo@1", "providerPackage": "provider"},
        "outputDirectory": ".build/application"
    });
    fs::write(
        root.join("edict.application.json"),
        serde_json::to_vec_pretty(&application).expect("encode application manifest"),
    )
    .expect("write application manifest");
}

fn workspace_snapshot_lawpack_document() -> Value {
    json!({
        "schema": "edict.lawpack-build/v1",
        "outputDirectory": "vendor/generated-workspace-snapshot",
        "lawpack": {
            "schema": "edict.lawpack-authoring/v1",
            "id": "workspace.snapshot",
            "version": "1",
            "acceptedCoreAbi": ["edict.core/v1"],
            "dependencies": [],
            "exportsCoordinate": "workspace.snapshot.exports/v1",
            "exports": {
                "types": [], "constants": [], "pureFunctions": [], "effects": [],
                "obstructions": [],
                "operationProfiles": {
                    "workspace.snapshot@1.observeRequest": {
                        "opticTemplate": {
                            "opticKind": "revelation",
                            "boundaryKind": "projection",
                            "supportPolicy": "workspace.snapshot@1.requestOnly",
                            "lossDisposition": "workspace.snapshot@1.lossless",
                            "apertureRequirement": {
                                "kind": "abstractFootprintObligation",
                                "reference": "workspace.snapshot@1.authorityScope"
                            }
                        },
                        "effectPredicate": "workspace.snapshot@1.externalObservation"
                    }
                }
            },
            "targetAdapters": [{
                "coordinate": "workspace.snapshot.echo-adapter/v1",
                "output": "adapter.cbor",
                "acceptedTargetProfile": {
                    "id": "echo.dpo@1",
                    "digest": "sha256:2e2494121aecf5e6a2d920f5fb85408825d394765fad41484c416397c920fb04"
                },
                "acceptedTargetIr": {
                    "id": "echo.span-ir/v1",
                    "digest": "sha256:0057167e68f50c99dcce087b3e1cd677d17c5d1dc238bdb52d89469e1472fc2f"
                },
                "operationProfiles": {
                    "workspace.snapshot@1.observeRequest": {
                        "core": "continuum.profile.read-only/v1",
                        "semanticEffects": [],
                        "budgetObligation": "workspace.snapshot@1.tinyObservationBudget",
                        "targetConfiguration": {"local": "request-profile"}
                    }
                },
                "effectImplementations": {},
                "budgets": {
                    "workspace.snapshot@1.tinyObservationBudget": {
                        "maxSteps": 512,
                        "maxAllocatedBytes": 262_144,
                        "maxOutputBytes": 131_072
                    }
                }
            }],
            "verifier": {
                "class": "declarative",
                "ruleset": {
                    "id": "workspace.snapshot.verifier-rules/v1",
                    "digest": format!("sha256:{}", "84".repeat(32))
                }
            },
            "compatibility": {
                "id": "workspace.snapshot.compatibility/v1",
                "digest": format!("sha256:{}", "85".repeat(32))
            },
            "conformanceFixtureCorpus": {
                "id": "workspace.snapshot.fixtures/v1",
                "digest": format!("sha256:{}", "86".repeat(32))
            },
            "localResources": [{
                "name": "request-profile",
                "coordinate": "workspace.snapshot.request-profile/v1",
                "output": "request-profile-configuration.cbor",
                "value": {
                    "apiVersion": "workspace.snapshot.request-profile/v1",
                    "operation": "workspace.snapshot.observe@1",
                    "authorityClass": "scoped",
                    "basisClass": "workspace-root"
                }
            }]
        },
        "dependencyBundles": []
    })
}

fn resource_json(coordinate: &str, digest: char) -> Value {
    json!({
        "coordinate": coordinate,
        "digest": format!("sha256:{}", digest.to_string().repeat(64))
    })
}

fn run_edict(directory: &Path, input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_edict"))
        .current_dir(directory)
        .env_remove(edict_cli::MAX_STDIN_BYTES_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn edict");
    child
        .stdin
        .as_mut()
        .expect("stdin pipe")
        .write_all(input.as_bytes())
        .expect("write request");
    child.wait_with_output().expect("collect output")
}

fn assert_success(output: &Output, context: &str) {
    assert_eq!(output.status.code(), Some(0), "{context}: {output:?}");
    assert!(output.stderr.is_empty(), "{context}: {output:?}");
}

fn jsonl(value: &Value) -> String {
    format!(
        "{}\n",
        serde_json::to_string(&value).expect("encode request")
    )
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn temp_tree(name: &str) -> PathBuf {
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "edict-lawpack-authoring-{name}-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("create temp tree");
    path
}
