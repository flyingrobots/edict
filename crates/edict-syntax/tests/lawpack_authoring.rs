use edict_syntax::{
    author_lawpack, decode_lawpack_adapter, decode_lawpack_bundle, LawpackArtifactKind,
    LawpackAuthoringDefinition, LawpackAuthoringDependency, LawpackAuthoringFailureKind,
    LawpackAuthoringPureFunction,
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

    let mut executable_verifier = full_definition();
    executable_verifier.verifier = serde_json::from_value(serde_json::json!({
        "class": "executable",
        "component": {"id": "example.cell.verifier.wasm/v1", "digest": PIN},
        "sandbox": {"id": "edict.component-sandbox/v1", "digest": PIN},
        "fuelModel": {"id": "edict.component-fuel/v1", "digest": PIN}
    }))
    .expect("typed executable verifier");
    author_lawpack(&executable_verifier, &[]).expect("executable verifier round trips");
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

    let mut invalid_number = full_definition();
    invalid_number.exports.constants[0].value = serde_json::json!(1.5);
    let failures = author_lawpack(&invalid_number, &[]).expect_err("float rejects");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidCanonicalValue
    );

    let mut invalid_bytes = full_definition();
    invalid_bytes.local_resources[1].value = serde_json::json!({"$edictBytes": "GG"});
    let failures = author_lawpack(&invalid_bytes, &[]).expect_err("invalid bytes reject");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidCanonicalValue
    );

    let mut duplicate_path = full_definition();
    duplicate_path.local_resources[1].output = duplicate_path.local_resources[0].output.clone();
    let failures = author_lawpack(&duplicate_path, &[]).expect_err("duplicate path rejects");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::DuplicateIdentity
    );

    let mut duplicate_coordinate = full_definition();
    duplicate_coordinate.local_resources[0].coordinate =
        duplicate_coordinate.exports_coordinate.clone();
    let failures =
        author_lawpack(&duplicate_coordinate, &[]).expect_err("duplicate coordinate rejects");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::DuplicateIdentity
    );

    let mut incomplete_adapter = full_definition();
    incomplete_adapter.target_adapters[0]
        .effect_implementations
        .clear();
    let failures =
        author_lawpack(&incomplete_adapter, &[]).expect_err("incomplete adapter rejects");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidAdapter
    );

    let mut malformed_pure = full_definition();
    malformed_pure.exports.pure_functions = vec![serde_json::from_value::<
        LawpackAuthoringPureFunction,
    >(serde_json::json!({
        "source": "edict",
        "coordinate": "example.cell@1.badPure",
        "typeParameters": [],
        "parameterTypes": [],
        "returnType": "U64",
        "costTemplate": "example.cell@1.smallBudget",
        "determinismClass": "total",
        "body": {"not": "core-fn-body"}
    }))
    .expect("typed malformed pure definition")];
    let failures = author_lawpack(&malformed_pure, &[]).expect_err("malformed pure rejects");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidLawpack
    );
}

#[test]
fn exact_dependency_closure_is_required_and_corroborated() {
    let mut dependency_definition = minimal_definition();
    dependency_definition.id = "example.base".to_owned();
    dependency_definition.exports_coordinate = "example.base.exports/v1".to_owned();
    dependency_definition.exports.types[0].coordinate = "example.base@1.Key".to_owned();
    let dependency_artifacts =
        author_lawpack(&dependency_definition, &[]).expect("author dependency");
    let dependency = decode_lawpack_bundle(
        dependency_artifacts
            .artifact(LawpackArtifactKind::Manifest)
            .expect("dependency manifest")
            .bytes(),
        dependency_artifacts
            .artifact(LawpackArtifactKind::Exports)
            .expect("dependency exports")
            .bytes(),
    )
    .expect("decode dependency");

    let mut root = minimal_definition();
    root.dependencies =
        vec![
            serde_json::from_value::<LawpackAuthoringDependency>(serde_json::json!({
                "id": "example.base",
                "version": "1",
                "digest": dependency.manifest_digest_review_string()
            }))
            .expect("typed dependency edge"),
        ];
    author_lawpack(&root, std::slice::from_ref(&dependency)).expect("exact closure succeeds");

    let missing = author_lawpack(&root, &[]).expect_err("missing dependency rejects");
    assert_eq!(
        missing[0].kind,
        LawpackAuthoringFailureKind::MissingDependency
    );

    root.dependencies[0].digest = PIN.to_owned();
    let substituted = author_lawpack(&root, std::slice::from_ref(&dependency))
        .expect_err("substituted dependency rejects");
    assert_eq!(
        substituted[0].kind,
        LawpackAuthoringFailureKind::DependencyDigestMismatch
    );

    let disconnected = author_lawpack(&minimal_definition(), &[dependency])
        .expect_err("disconnected dependency rejects");
    assert_eq!(
        disconnected[0].kind,
        LawpackAuthoringFailureKind::InvalidDependencyClosure
    );
}

#[allow(
    clippy::too_many_lines,
    reason = "one mutation matrix keeps all identity-bearing v1 surfaces comparable"
)]
#[test]
fn semantic_surface_mutations_move_their_owning_identities() {
    let original = author_lawpack(&full_definition(), &[]).expect("original full authoring");
    let original_manifest = digest(&original, LawpackArtifactKind::Manifest);
    let original_exports = digest(&original, LawpackArtifactKind::Exports);
    let original_adapter = digest(&original, LawpackArtifactKind::Adapter);
    let original_resource = digest(&original, LawpackArtifactKind::LocalResource);

    let mut helper = full_definition();
    let LawpackAuthoringPureFunction::Component { implementation, .. } =
        &mut helper.exports.pure_functions[0]
    else {
        panic!("component helper fixture");
    };
    let edict_syntax::LawpackAuthoringResourceRef::External(component) =
        &mut implementation.component
    else {
        panic!("external component fixture");
    };
    component.digest =
        "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned();
    let helper = author_lawpack(&helper, &[]).expect("helper mutation");
    assert_ne!(
        digest(&helper, LawpackArtifactKind::Exports),
        original_exports
    );
    assert_ne!(
        digest(&helper, LawpackArtifactKind::Manifest),
        original_manifest
    );

    let mut constant = full_definition();
    constant.exports.constants[0].value = serde_json::json!(255);
    let constant = author_lawpack(&constant, &[]).expect("constant mutation");
    assert_ne!(
        digest(&constant, LawpackArtifactKind::Exports),
        original_exports
    );
    assert_ne!(
        digest(&constant, LawpackArtifactKind::Manifest),
        original_manifest
    );

    let mut effect = full_definition();
    effect.exports.effects[0].footprint_obligation = "example.cell@1.twoCells".to_owned();
    effect.target_adapters[0]
        .effect_implementations
        .get_mut("example.cell@1.create")
        .expect("effect implementation")
        .footprint_obligation = "example.cell@1.twoCells".to_owned();
    let effect = author_lawpack(&effect, &[]).expect("effect mutation");
    assert_ne!(
        digest(&effect, LawpackArtifactKind::Exports),
        original_exports
    );
    assert_ne!(
        digest(&effect, LawpackArtifactKind::Adapter),
        original_adapter
    );
    assert_ne!(
        digest(&effect, LawpackArtifactKind::Manifest),
        original_manifest
    );

    let mut profile = full_definition();
    profile
        .exports
        .operation_profiles
        .get_mut("example.cell@1.create")
        .expect("operation profile")
        .optic_template
        .support_policy = "example.cell@1.alternateSupport".to_owned();
    let profile = author_lawpack(&profile, &[]).expect("profile mutation");
    assert_ne!(
        digest(&profile, LawpackArtifactKind::Exports),
        original_exports
    );
    assert_ne!(
        digest(&profile, LawpackArtifactKind::Manifest),
        original_manifest
    );

    let mut adapter = full_definition();
    adapter.target_adapters[0]
        .budgets
        .get_mut("example.cell@1.smallBudget")
        .expect("budget")
        .max_steps = 17;
    let adapter = author_lawpack(&adapter, &[]).expect("adapter mutation");
    assert_ne!(
        digest(&adapter, LawpackArtifactKind::Adapter),
        original_adapter
    );
    assert_ne!(
        digest(&adapter, LawpackArtifactKind::Manifest),
        original_manifest
    );

    let mut resource = full_definition();
    resource.local_resources[0].value["limit"] = serde_json::json!(257);
    let resource = author_lawpack(&resource, &[]).expect("resource mutation");
    assert_ne!(
        digest(&resource, LawpackArtifactKind::LocalResource),
        original_resource
    );
    assert_ne!(
        digest(&resource, LawpackArtifactKind::Adapter),
        original_adapter
    );
    assert_ne!(
        digest(&resource, LawpackArtifactKind::Manifest),
        original_manifest
    );
}

fn digest(
    artifacts: &edict_syntax::LawpackAuthoredArtifactSet,
    kind: LawpackArtifactKind,
) -> String {
    artifacts
        .artifact(kind)
        .expect("artifact by kind")
        .digest()
        .to_owned()
}
