use edict_syntax::{
    author_lawpack, decode_lawpack_adapter, decode_lawpack_bundle, LawpackArtifactKind,
    LawpackAuthoringDefinition, LawpackAuthoringFailureKind,
};

const PIN: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";

fn minimal_definition() -> LawpackAuthoringDefinition {
    serde_json::from_value(serde_json::json!({
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
            "ruleset": {"id": "example.text.verifier/v1", "digest": PIN}
        },
        "compatibility": {"id": "example.text.compatibility/v1", "digest": PIN},
        "conformanceFixtureCorpus": {"id": "example.text.fixtures/v1", "digest": PIN},
        "localResources": []
    }))
    .expect("minimal typed authoring definition")
}

#[allow(
    clippy::too_many_lines,
    reason = "one review fixture keeps the complete v1 authoring surface visible"
)]
fn full_definition() -> LawpackAuthoringDefinition {
    serde_json::from_value(serde_json::json!({
        "schema": "edict.lawpack-authoring/v1",
        "id": "example.cell",
        "version": "1",
        "acceptedCoreAbi": ["edict.core/v1"],
        "dependencies": [],
        "exportsCoordinate": "example.cell.exports/v1",
        "exports": {
            "types": [{
                "coordinate": "example.cell@1.CellInput",
                "definition": "Bytes<max=256>"
            }],
            "constants": [{
                "coordinate": "example.cell@1.maxValueBytes",
                "type": "U64",
                "value": 256
            }],
            "pureFunctions": [{
                "source": "component",
                "coordinate": "example.cell@1.keyDigest",
                "typeParameters": [],
                "parameterTypes": ["Bytes<max=256>"],
                "returnType": "Bytes<max=32>",
                "costTemplate": "example.cell@1.smallBudget",
                "determinismClass": "total",
                "implementation": {
                    "component": {"id": "example.cell.key-digest.wasm/v1", "digest": PIN},
                    "sandbox": {"id": "edict.component-sandbox/v1", "digest": PIN},
                    "fuelModel": {"id": "edict.component-fuel/v1", "digest": PIN}
                }
            }],
            "effects": [{
                "coordinate": "example.cell@1.create",
                "typeParameters": [],
                "inputType": "example.cell@1.CellInput",
                "outputType": "example.cell@1.CellInput",
                "executionClass": "runtime",
                "effectKindHint": "create",
                "footprintObligation": "example.cell@1.oneCell",
                "costObligation": "example.cell@1.smallBudget",
                "effectFailures": {
                    "alreadyExists": {
                        "authorityClass": "domainMappable",
                        "payloadType": "example.cell@1.CellInput"
                    }
                },
                "guardSupport": true
            }],
            "obstructions": [{
                "coordinate": "example.cell@1.AlreadyExists",
                "authorityClass": "domainMappable",
                "payloadSchema": "example.cell@1.CellInput"
            }],
            "operationProfiles": {
                "example.cell@1.create": {
                    "opticTemplate": {
                        "opticKind": "affectReintegration",
                        "boundaryKind": "affect",
                        "supportPolicy": "example.cell@1.directSupport",
                        "lossDisposition": "example.cell@1.lossless",
                        "basisTemplate": "example.cell@1.basis",
                        "apertureRequirement": {
                            "kind": "abstractFootprintObligation",
                            "reference": "example.cell@1.oneCell"
                        }
                    },
                    "effectPredicate": "example.cell@1.createEffect"
                }
            }
        },
        "targetAdapters": [{
            "coordinate": "example.cell.adapter.echo/v1",
            "output": "adapters/echo.cbor",
            "acceptedTargetProfile": {"id": "echo.dpo@1", "digest": PIN},
            "acceptedTargetIr": {"id": "echo.span-ir/v1", "digest": PIN},
            "operationProfiles": {
                "example.cell@1.create": {
                    "core": "continuum.profile.create/v1",
                    "semanticEffects": ["example.cell@1.create"],
                    "budgetObligation": "example.cell@1.smallBudget",
                    "targetConfiguration": {"local": "echo-config"}
                }
            },
            "effectImplementations": {
                "example.cell@1.create": {
                    "targetIntrinsic": "echo.dpo@1.generic-create",
                    "targetConfiguration": {"local": "echo-config"},
                    "writeClass": "create",
                    "footprintObligation": "example.cell@1.oneCell",
                    "costObligation": "example.cell@1.smallBudget",
                    "failureMappings": {
                        "alreadyExists": "echo.precondition-mismatch/v1"
                    }
                }
            },
            "budgets": {
                "example.cell@1.smallBudget": {
                    "maxSteps": 16,
                    "maxAllocatedBytes": 2048,
                    "maxOutputBytes": 512
                }
            }
        }],
        "helperComponent": {
            "component": {"id": "example.cell.helper.wasm/v1", "digest": PIN},
            "sandbox": {"id": "edict.component-sandbox/v1", "digest": PIN},
            "fuelModel": {"id": "edict.component-fuel/v1", "digest": PIN}
        },
        "verifier": {"class": "declarative", "ruleset": {"local": "rules"}},
        "compatibility": {"id": "example.cell.compatibility/v1", "digest": PIN},
        "conformanceFixtureCorpus": {"id": "example.cell.fixtures/v1", "digest": PIN},
        "localResources": [{
            "name": "echo-config",
            "coordinate": "example.cell.echo-config/v1",
            "output": "resources/echo-config.cbor",
            "value": {"apiVersion": "example.cell.echo-config/v1", "limit": 256}
        }, {
            "name": "rules",
            "coordinate": "example.cell.verifier-rules/v1",
            "output": "resources/verifier-rules.cbor",
            "value": {"opcodes": {"$edictBytes": "00ff"}}
        }]
    }))
    .expect("full typed authoring definition")
}

#[test]
fn minimal_authoring_emits_a_deterministic_valid_bundle() {
    let definition = minimal_definition();

    let first = author_lawpack(&definition, &[]).expect("first authoring succeeds");
    let second = author_lawpack(&definition, &[]).expect("second authoring succeeds");

    assert_eq!(
        first, second,
        "fixed semantic inputs reproduce exact artifacts"
    );
    assert_eq!(first.artifacts().len(), 4);

    let manifest = first
        .artifact(LawpackArtifactKind::Manifest)
        .expect("manifest artifact");
    let exports = first
        .artifact(LawpackArtifactKind::Exports)
        .expect("exports artifact");
    let bundle = decode_lawpack_bundle(manifest.bytes(), exports.bytes())
        .expect("authored bytes pass the public lawpack decoder");

    assert_eq!(bundle.manifest().id, "example.text");
    assert_eq!(bundle.manifest().version, "1");
    assert_eq!(bundle.exports().types[0].coordinate, "example.text@1.Key");
    assert_eq!(manifest.digest(), bundle.manifest_digest_review_string());
    assert_eq!(
        first
            .artifact(LawpackArtifactKind::ManifestDigest)
            .expect("manifest digest sidecar")
            .bytes(),
        format!("{}\n", manifest.digest()).as_bytes()
    );
}

#[test]
fn exported_semantic_mutation_moves_exports_and_manifest_identity() {
    let original = author_lawpack(&minimal_definition(), &[]).expect("original authoring");
    let mut changed = minimal_definition();
    changed.exports.types[0].definition = "String<max=65>".to_owned();
    let changed = author_lawpack(&changed, &[]).expect("changed authoring");

    assert_ne!(
        original
            .artifact(LawpackArtifactKind::Exports)
            .expect("original exports")
            .digest(),
        changed
            .artifact(LawpackArtifactKind::Exports)
            .expect("changed exports")
            .digest()
    );
    assert_ne!(
        original
            .artifact(LawpackArtifactKind::Manifest)
            .expect("original manifest")
            .digest(),
        changed
            .artifact(LawpackArtifactKind::Manifest)
            .expect("changed manifest")
            .digest()
    );
}

#[test]
fn full_surface_round_trips_through_existing_decoders() {
    let authored = author_lawpack(&full_definition(), &[]).expect("full authoring succeeds");
    let manifest = authored
        .artifact(LawpackArtifactKind::Manifest)
        .expect("manifest artifact");
    let exports = authored
        .artifact(LawpackArtifactKind::Exports)
        .expect("exports artifact");
    let adapter = authored
        .artifact(LawpackArtifactKind::Adapter)
        .expect("adapter artifact");
    let bundle = decode_lawpack_bundle(manifest.bytes(), exports.bytes())
        .expect("full authored bundle passes public decoder");
    let adapter = decode_lawpack_adapter(&bundle, "echo.dpo@1", adapter.bytes())
        .expect("full authored adapter passes public decoder");

    assert_eq!(bundle.exports().constants.len(), 1);
    assert_eq!(bundle.exports().pure_functions.len(), 1);
    assert_eq!(bundle.exports().effects.len(), 1);
    assert_eq!(bundle.exports().obstructions.len(), 1);
    assert_eq!(bundle.exports().operation_profiles.len(), 1);
    assert_eq!(adapter.operation_profiles().len(), 1);
    assert_eq!(adapter.effects().len(), 1);
    assert_eq!(adapter.budgets().len(), 1);
    assert_eq!(
        authored
            .artifacts()
            .iter()
            .filter(|artifact| artifact.kind() == LawpackArtifactKind::LocalResource)
            .count(),
        2
    );
}

#[test]
fn malformed_inputs_fail_with_stable_categories() {
    let mut invalid_digest = full_definition();
    invalid_digest.target_adapters[0]
        .accepted_target_profile
        .digest = "sha256:ABC".to_owned();
    let failures = author_lawpack(&invalid_digest, &[]).expect_err("invalid digest rejects");
    assert_eq!(failures[0].kind, LawpackAuthoringFailureKind::InvalidDigest);

    let mut missing_resource = full_definition();
    missing_resource.target_adapters[0]
        .effect_implementations
        .get_mut("example.cell@1.create")
        .expect("effect")
        .target_configuration =
        serde_json::from_value(serde_json::json!({"local": "missing"})).expect("local reference");
    let failures = author_lawpack(&missing_resource, &[]).expect_err("missing resource rejects");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::MissingLocalResource
    );

    let mut escaping_path = full_definition();
    escaping_path.local_resources[0].output = "../escape.cbor".to_owned();
    let failures = author_lawpack(&escaping_path, &[]).expect_err("path escape rejects");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidOutputPath
    );
}
