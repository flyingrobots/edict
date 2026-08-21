use edict_syntax::{
    author_lawpack, decode_lawpack_adapter, decode_lawpack_bundle, CanonicalValue,
    LawpackArtifactKind, LawpackAuthoringApertureRequirement, LawpackAuthoringDefinition,
    LawpackAuthoringDependency, LawpackAuthoringFailureCause, LawpackAuthoringFailureKind,
    LawpackAuthoringPureFunction, LawpackAuthoringVerifier, LawpackEffectKind,
    LawpackExecutionClass, LawpackPureFunctionImplementation, LawpackValidationFailureKind,
    LawpackVerifier, MAX_CANONICAL_NESTING_DEPTH,
};

const PIN_RULESET: &str = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const PIN_COMPATIBILITY: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";
const PIN_FIXTURES: &str =
    "sha256:3333333333333333333333333333333333333333333333333333333333333333";
const PIN_COMPONENT: &str =
    "sha256:4444444444444444444444444444444444444444444444444444444444444444";
const PIN_SANDBOX: &str = "sha256:5555555555555555555555555555555555555555555555555555555555555555";
const PIN_FUEL: &str = "sha256:6666666666666666666666666666666666666666666666666666666666666666";
const PIN_TARGET_PROFILE: &str =
    "sha256:7777777777777777777777777777777777777777777777777777777777777777";
const PIN_TARGET_IR: &str =
    "sha256:8888888888888888888888888888888888888888888888888888888888888888";
const PIN_HELPER_COMPONENT: &str =
    "sha256:9999999999999999999999999999999999999999999999999999999999999999";
const MINIMAL_MANIFEST_DIGEST: &str =
    "sha256:e72b2ec555f3da82e2379a87f5ee9de4bef79c6452391079befb6f927eed4c19";
const MINIMAL_EXPORTS_DIGEST: &str =
    "sha256:4b21d047723d605dda46a9f08c528248b19facc91be87b68de43d7a63aadec9b";

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
            "ruleset": {"id": "example.text.verifier/v1", "digest": PIN_RULESET}
        },
        "compatibility": {"id": "example.text.compatibility/v1", "digest": PIN_COMPATIBILITY},
        "conformanceFixtureCorpus": {"id": "example.text.fixtures/v1", "digest": PIN_FIXTURES},
        "localResources": []
    }))
    .expect("minimal typed authoring definition")
}

fn nested_arrays(container_count: usize) -> serde_json::Value {
    let mut value = serde_json::json!(0);
    for _ in 0..container_count {
        value = serde_json::json!([value]);
    }
    value
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
                    "component": {"id": "example.cell.key-digest.wasm/v1", "digest": PIN_COMPONENT},
                    "sandbox": {"id": "edict.component-sandbox/v1", "digest": PIN_SANDBOX},
                    "fuelModel": {"id": "edict.component-fuel/v1", "digest": PIN_FUEL}
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
            "acceptedTargetProfile": {"id": "echo.dpo@1", "digest": PIN_TARGET_PROFILE},
            "acceptedTargetIr": {"id": "echo.span-ir/v1", "digest": PIN_TARGET_IR},
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
            "component": {"id": "example.cell.helper.wasm/v1", "digest": PIN_HELPER_COMPONENT},
            "sandbox": {"id": "edict.component-sandbox/v1", "digest": PIN_SANDBOX},
            "fuelModel": {"id": "edict.component-fuel/v1", "digest": PIN_FUEL}
        },
        "verifier": {"class": "declarative", "ruleset": {"local": "rules"}},
        "compatibility": {"id": "example.cell.compatibility/v1", "digest": PIN_COMPATIBILITY},
        "conformanceFixtureCorpus": {"id": "example.cell.fixtures/v1", "digest": PIN_FIXTURES},
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

    assert_eq!(manifest.digest(), MINIMAL_MANIFEST_DIGEST);
    assert_eq!(exports.digest(), MINIMAL_EXPORTS_DIGEST);

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
    let constant = &bundle.exports().constants[0];
    assert_eq!(constant.coordinate, "example.cell@1.maxValueBytes");
    assert_eq!(constant.ty, "U64");
    assert_eq!(constant.value, CanonicalValue::Integer(256));
    let effect = &bundle.exports().effects[0];
    assert_eq!(effect.coordinate, "example.cell@1.create");
    assert_eq!(effect.input_type, "example.cell@1.CellInput");
    assert_eq!(effect.output_type, "example.cell@1.CellInput");
    assert_eq!(effect.execution_class, LawpackExecutionClass::Runtime);
    assert_eq!(effect.effect_kind_hint, LawpackEffectKind::Create);
    assert_eq!(
        adapter.target_profile().digest_review_string(),
        PIN_TARGET_PROFILE
    );
    assert_eq!(adapter.target_ir().digest_review_string(), PIN_TARGET_IR);
    let LawpackPureFunctionImplementation::Component { implementation } =
        &bundle.exports().pure_functions[0].implementation
    else {
        panic!("component-backed helper");
    };
    assert_eq!(
        implementation.component.digest_review_string(),
        PIN_COMPONENT
    );
    assert_eq!(implementation.sandbox.digest_review_string(), PIN_SANDBOX);
    assert_eq!(implementation.fuel_model.digest_review_string(), PIN_FUEL);
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
        "component": {"id": "example.cell.verifier.wasm/v1", "digest": PIN_COMPONENT},
        "sandbox": {"id": "edict.component-sandbox/v1", "digest": PIN_SANDBOX},
        "fuelModel": {"id": "edict.component-fuel/v1", "digest": PIN_FUEL}
    }))
    .expect("typed executable verifier");
    let executable =
        author_lawpack(&executable_verifier, &[]).expect("executable verifier round trips");
    let executable_bundle = decode_authored(&executable);
    let LawpackVerifier::Executable { executable } = &executable_bundle.manifest().verifier else {
        panic!("executable verifier class");
    };
    assert_eq!(executable.component.digest_review_string(), PIN_COMPONENT);
    assert_eq!(executable.sandbox.digest_review_string(), PIN_SANDBOX);
    assert_eq!(executable.fuel_model.digest_review_string(), PIN_FUEL);
}

#[test]
fn malformed_inputs_fail_with_stable_categories() {
    let mut invalid_digest = full_definition();
    invalid_digest.target_adapters[0]
        .accepted_target_profile
        .digest = "sha256:ABC".to_owned();
    let failures = author_lawpack(&invalid_digest, &[]).expect_err("invalid digest rejects");
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].kind, LawpackAuthoringFailureKind::InvalidDigest);

    let mut missing_resource = full_definition();
    missing_resource.target_adapters[0]
        .effect_implementations
        .get_mut("example.cell@1.create")
        .expect("effect")
        .target_configuration =
        serde_json::from_value(serde_json::json!({"local": "missing"})).expect("local reference");
    let failures = author_lawpack(&missing_resource, &[]).expect_err("missing resource rejects");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::MissingLocalResource
    );

    let mut escaping_path = full_definition();
    escaping_path.local_resources[0].output = "../escape.cbor".to_owned();
    let failures = author_lawpack(&escaping_path, &[]).expect_err("path escape rejects");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidOutputPath
    );

    let mut invalid_number = full_definition();
    invalid_number.exports.constants[0].value = serde_json::json!(1.5);
    let failures = author_lawpack(&invalid_number, &[]).expect_err("float rejects");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidCanonicalValue
    );

    let mut invalid_bytes = full_definition();
    invalid_bytes.local_resources[1].value = serde_json::json!({"$edictBytes": "GG"});
    let failures = author_lawpack(&invalid_bytes, &[]).expect_err("invalid bytes reject");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidCanonicalValue
    );

    let mut duplicate_path = full_definition();
    duplicate_path.local_resources[1].output = duplicate_path.local_resources[0].output.clone();
    let failures = author_lawpack(&duplicate_path, &[]).expect_err("duplicate path rejects");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::DuplicateIdentity
    );

    let mut duplicate_coordinate = full_definition();
    duplicate_coordinate.local_resources[0].coordinate =
        duplicate_coordinate.exports_coordinate.clone();
    let failures =
        author_lawpack(&duplicate_coordinate, &[]).expect_err("duplicate coordinate rejects");
    assert_eq!(failures.len(), 1);
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
    assert_eq!(failures.len(), 1);
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
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidLawpack
    );
    let Some(LawpackAuthoringFailureCause::Lawpack(cause)) = &failures[0].cause else {
        panic!("malformed pure body retains its typed lawpack cause");
    };
    assert_eq!(
        cause.kind,
        LawpackValidationFailureKind::InvalidPureFunctionBody
    );
}

#[test]
fn deeply_nested_canonical_values_reject_before_stack_exhaustion() {
    let mut definition = full_definition();
    let at_limit = nested_arrays(MAX_CANONICAL_NESTING_DEPTH);
    definition.local_resources[0].value = at_limit.clone();
    author_lawpack(&definition, &[]).expect("terminal scalar at canonical depth limit succeeds");

    definition.local_resources[0].value = serde_json::json!([at_limit]);

    let failures = author_lawpack(&definition, &[]).expect_err("deep value rejects");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidCanonicalValue
    );
}

#[test]
fn export_values_account_for_their_enclosing_canonical_containers() {
    const EXPORT_ENCLOSING_CONTAINERS: usize = 3;
    let at_limit = nested_arrays(MAX_CANONICAL_NESTING_DEPTH - EXPORT_ENCLOSING_CONTAINERS);
    let over_limit = serde_json::json!([at_limit.clone()]);

    let mut constant = full_definition();
    constant.exports.constants[0].value = at_limit.clone();
    author_lawpack(&constant, &[]).expect("constant at complete artifact depth limit succeeds");
    constant.exports.constants[0].value = over_limit.clone();
    let failures = author_lawpack(&constant, &[]).expect_err("over-deep constant rejects early");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidCanonicalValue
    );

    let edict_helper = |body| {
        serde_json::from_value::<LawpackAuthoringPureFunction>(serde_json::json!({
            "source": "edict",
            "coordinate": "example.cell@1.depthHelper",
            "typeParameters": [],
            "parameterTypes": [],
            "returnType": "U64",
            "costTemplate": "example.cell@1.smallBudget",
            "determinismClass": "total",
            "body": body
        }))
        .expect("typed Edict helper")
    };
    let mut pure = full_definition();
    pure.exports.pure_functions = vec![edict_helper(over_limit)];
    let failures = author_lawpack(&pure, &[]).expect_err("over-deep pure body rejects early");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidCanonicalValue
    );
}

#[test]
fn large_unique_artifact_path_set_authors_successfully() {
    let mut definition = full_definition();
    for index in 0..1_000 {
        definition.local_resources.push(
            serde_json::from_value(serde_json::json!({
                "name": format!("extra-{index:04}"),
                "coordinate": format!("example.cell.extra-{index:04}/v1"),
                "output": format!("resources/extra-{index:04}.cbor"),
                "value": index
            }))
            .expect("typed extra local resource"),
        );
    }

    let authored = author_lawpack(&definition, &[]).expect("large unique path set authors");
    assert_eq!(authored.artifacts().len(), 2_010);
}

#[test]
fn artifact_paths_reject_file_ancestors_and_the_ownership_index_namespace() {
    let mut prefix_collision = full_definition();
    prefix_collision.local_resources[0].output = "r/x.cbor".to_owned();
    prefix_collision.local_resources[1].output = "r/x.cbor-1.cbor".to_owned();
    prefix_collision.local_resources.push(
        serde_json::from_value(serde_json::json!({
            "name": "descendant",
            "coordinate": "example.cell.descendant/v1",
            "output": "r/x.cbor/y.cbor",
            "value": 3
        }))
        .expect("typed descendant resource"),
    );
    let failures = author_lawpack(&prefix_collision, &[])
        .expect_err("an emitted file cannot be another artifact's ancestor");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::DuplicateIdentity
    );

    let mut reserved_namespace = full_definition();
    reserved_namespace.local_resources[0].output =
        "edict.lawpack-output.json/child.cbor".to_owned();
    let failures = author_lawpack(&reserved_namespace, &[])
        .expect_err("the generated ownership index remains a file namespace");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidOutputPath
    );

    let mut nul_path = full_definition();
    nul_path.local_resources[0].output = "resources/bad\0.cbor".to_owned();
    let failures =
        author_lawpack(&nul_path, &[]).expect_err("filesystem NUL path rejects during authoring");
    assert_eq!(
        failures[0].kind,
        LawpackAuthoringFailureKind::InvalidOutputPath
    );

    for output in [
        "resources/Config.cbor",
        "resources/bad:name.cbor",
        "con.cbor",
    ] {
        let mut nonportable = full_definition();
        nonportable.local_resources[0].output = output.to_owned();
        let failures = author_lawpack(&nonportable, &[])
            .expect_err("nonportable filesystem path rejects during authoring");
        assert_eq!(
            failures[0].kind,
            LawpackAuthoringFailureKind::InvalidOutputPath,
            "{output}"
        );
    }
}

#[test]
fn tagged_authoring_variants_reject_unknown_fields() {
    let mut verifier = serde_json::json!({
        "class": "declarative",
        "ruleset": {"id": "example.rules/v1", "digest": PIN_RULESET},
        "rulesett": {"id": "ignored/v1", "digest": PIN_RULESET}
    });
    let verifier_error = serde_json::from_value::<LawpackAuthoringVerifier>(verifier.clone())
        .expect_err("unknown verifier field rejects");
    assert!(verifier_error.is_data());
    verifier
        .as_object_mut()
        .expect("verifier object")
        .remove("rulesett")
        .expect("unknown verifier field");
    serde_json::from_value::<LawpackAuthoringVerifier>(verifier)
        .expect("removing only the unknown verifier field restores validity");

    let mut pure = serde_json::json!({
        "source": "edict",
        "coordinate": "example.text@1.helper",
        "typeParameters": [],
        "parameterTypes": [],
        "returnType": "U64",
        "costTemplate": "example.text@1.smallBudget",
        "determinismClass": "total",
        "body": {"node": "core-fn-body", "statements": [], "result": {"node": "literal", "value": 1}},
        "boddy": {}
    });
    let pure_error = serde_json::from_value::<LawpackAuthoringPureFunction>(pure.clone())
        .expect_err("unknown pure-function field rejects");
    assert!(pure_error.is_data());
    pure.as_object_mut()
        .expect("pure-function object")
        .remove("boddy")
        .expect("unknown pure-function field");
    serde_json::from_value::<LawpackAuthoringPureFunction>(pure)
        .expect("removing only the unknown pure-function field restores validity");

    let mut aperture = serde_json::json!({
        "kind": "footprintCeiling",
        "reference": "example.text@1.oneRange",
        "referense": "ignored"
    });
    let aperture_error =
        serde_json::from_value::<LawpackAuthoringApertureRequirement>(aperture.clone())
            .expect_err("unknown aperture field rejects");
    assert!(aperture_error.is_data());
    aperture
        .as_object_mut()
        .expect("aperture object")
        .remove("referense")
        .expect("unknown aperture field");
    serde_json::from_value::<LawpackAuthoringApertureRequirement>(aperture)
        .expect("removing only the unknown aperture field restores validity");
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

    root.dependencies[0].digest = PIN_RULESET.to_owned();
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

    let mut leaf_definition = minimal_definition();
    leaf_definition.id = "example.leaf".to_owned();
    leaf_definition.exports_coordinate = "example.leaf.exports/v1".to_owned();
    leaf_definition.exports.types[0].coordinate = "example.leaf@1.Key".to_owned();
    let leaf =
        decode_authored(&author_lawpack(&leaf_definition, &[]).expect("author transitive leaf"));

    let mut middle_definition = minimal_definition();
    middle_definition.id = "example.middle".to_owned();
    middle_definition.exports_coordinate = "example.middle.exports/v1".to_owned();
    middle_definition.exports.types[0].coordinate = "example.middle@1.Key".to_owned();
    middle_definition.dependencies = vec![LawpackAuthoringDependency {
        id: "example.leaf".to_owned(),
        version: "1".to_owned(),
        digest: leaf.manifest_digest_review_string(),
    }];
    let middle = decode_authored(
        &author_lawpack(&middle_definition, std::slice::from_ref(&leaf))
            .expect("author transitive middle"),
    );

    let mut transitive_root = minimal_definition();
    transitive_root.dependencies = vec![LawpackAuthoringDependency {
        id: "example.middle".to_owned(),
        version: "1".to_owned(),
        digest: middle.manifest_digest_review_string(),
    }];
    author_lawpack(&transitive_root, &[middle.clone(), leaf.clone()])
        .expect("complete depth-two closure succeeds");

    let mut unrelated_definition = minimal_definition();
    unrelated_definition.id = "example.unrelated".to_owned();
    unrelated_definition.exports_coordinate = "example.unrelated.exports/v1".to_owned();
    unrelated_definition.exports.types[0].coordinate = "example.unrelated@1.Key".to_owned();
    let unrelated = decode_authored(
        &author_lawpack(&unrelated_definition, &[]).expect("author unrelated bundle"),
    );
    let disconnected = author_lawpack(&transitive_root, &[middle.clone(), leaf, unrelated.clone()])
        .expect_err("depth-two closure rejects an unreachable supplied bundle");
    assert_eq!(disconnected.len(), 1);
    assert_eq!(
        disconnected[0].kind,
        LawpackAuthoringFailureKind::InvalidDependencyClosure
    );

    let failures = author_lawpack(&transitive_root, &[middle, unrelated])
        .expect_err("unreachable bundle cannot offset a missing transitive dependency");
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0].kind,
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
    let original_resource = digest_at(
        &original,
        LawpackArtifactKind::LocalResource,
        "example.cell.echo-config/v1",
    );
    let original_rules = digest_at(
        &original,
        LawpackArtifactKind::LocalResource,
        "example.cell.verifier-rules/v1",
    );

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
    assert_eq!(
        digest(&helper, LawpackArtifactKind::Adapter),
        original_adapter
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
    assert_eq!(
        digest_at(
            &constant,
            LawpackArtifactKind::LocalResource,
            "example.cell.echo-config/v1"
        ),
        original_resource
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
    assert_eq!(
        digest_at(
            &effect,
            LawpackArtifactKind::LocalResource,
            "example.cell.verifier-rules/v1"
        ),
        original_rules
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
    assert_eq!(
        digest(&profile, LawpackArtifactKind::Adapter),
        original_adapter
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
    assert_eq!(
        digest(&adapter, LawpackArtifactKind::Exports),
        original_exports
    );

    let mut resource = full_definition();
    resource.local_resources[0].value["limit"] = serde_json::json!(257);
    let resource = author_lawpack(&resource, &[]).expect("resource mutation");
    assert_ne!(
        digest_at(
            &resource,
            LawpackArtifactKind::LocalResource,
            "example.cell.echo-config/v1"
        ),
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
    assert_eq!(
        digest(&resource, LawpackArtifactKind::Exports),
        original_exports
    );
    assert_eq!(
        digest_at(
            &resource,
            LawpackArtifactKind::LocalResource,
            "example.cell.verifier-rules/v1"
        ),
        original_rules
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

fn digest_at(
    artifacts: &edict_syntax::LawpackAuthoredArtifactSet,
    kind: LawpackArtifactKind,
    coordinate: &str,
) -> String {
    artifacts
        .artifacts()
        .iter()
        .find(|artifact| artifact.kind() == kind && artifact.coordinate() == coordinate)
        .expect("artifact by kind and coordinate")
        .digest()
        .to_owned()
}

fn decode_authored(
    artifacts: &edict_syntax::LawpackAuthoredArtifactSet,
) -> edict_syntax::ValidatedLawpackBundle {
    decode_lawpack_bundle(
        artifacts
            .artifact(LawpackArtifactKind::Manifest)
            .expect("authored manifest")
            .bytes(),
        artifacts
            .artifact(LawpackArtifactKind::Exports)
            .expect("authored exports")
            .bytes(),
    )
    .expect("decode authored bundle")
}
