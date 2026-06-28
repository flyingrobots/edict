//! Target IR generation tests for the first Echo lowering slice.
//!
//! These tests assert public behavior: structured Target IR artifacts and
//! stable failure kinds. They do not inspect diagnostic prose, repository
//! layout, or implementation-private lowering helpers.

use edict_syntax::{
    check_lowerability, compile_to_core, lower_to_target_ir, AtomicityRequirement, CompilerContext,
    CoreBudget, CoreExpr, GuardKind, LowerabilityStatus, LoweringRequirements, NativeEffectSupport,
    ResourceRef, SemanticEffectRequirement, TargetEffectLowering, TargetIrLoweringFacts,
    TargetLoweringFailureKind, TargetLoweringStatus, TargetProfileFacts, WriteClass,
    ECHO_DPO_TARGET_PROFILE, ECHO_SPAN_IR_DOMAIN,
};

const EFFECTFUL_REPLACE: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      let receipt: Receipt = target.replace(input.id)\n\
        else { rejected(reason) => domain.WriteRejected };\n\
      return { id: input.id };\n\
    }";
const CHAINED_EFFECT_RESULTS: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      let first: Receipt = target.replace(input.id)\n\
        else { rejected(reason) => domain.WriteRejected };\n\
      let second: Receipt = target.replace(first.id)\n\
        else { rejected(reason) => domain.WriteRejected };\n\
      return { id: second.id };\n\
    }";

const PURE_LOCAL_RECORD: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");
const ECHO_PROFILE_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";

fn effectful_core() -> edict_syntax::CoreModule {
    let module = edict_syntax::parse_module(EFFECTFUL_REPLACE).expect("effectful source parses");
    compile_to_core(&module, &effectful_context()).expect("effectful source compiles to Core")
}

fn effectful_artifact(source: &str) -> edict_syntax::TargetIrArtifact {
    let module = edict_syntax::parse_module(source).expect("effectful source parses");
    let core =
        compile_to_core(&module, &effectful_context()).expect("effectful source compiles to Core");
    lower_to_target_ir(&core, &echo_facts())
        .artifact
        .expect("supported source lowers to Target IR")
}

fn pure_core() -> edict_syntax::CoreModule {
    let module = edict_syntax::parse_module(PURE_LOCAL_RECORD).expect("pure source parses");
    compile_to_core(&module, &pure_context()).expect("pure source compiles to Core")
}

fn effectful_context() -> CompilerContext {
    effectful_context_with_profile("continuum.profile.write/v1")
}

fn effectful_context_with_profile(profile: &str) -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("p.effectful", profile)
        .with_operation_profile_write_classes("p.effectful", [WriteClass::Replace])
        .with_effect_write_class("target.replace", WriteClass::Replace)
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 8,
                max_allocated_bytes: 1024,
                max_output_bytes: 256,
            },
        )
}

fn pure_context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("hello.readOnly", "continuum.profile.read-only/v1")
        .with_budget(
            "hello.tinyBudget",
            CoreBudget {
                max_steps: 64,
                max_allocated_bytes: 4096,
                max_output_bytes: 1024,
            },
        )
}

fn echo_facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: ECHO_DPO_TARGET_PROFILE.to_owned(),
            digest: Some(echo_profile_digest()),
        },
        target_ir_domain: ECHO_SPAN_IR_DOMAIN.to_owned(),
        operation_profiles: vec!["continuum.profile.write/v1".to_owned()],
        obstruction_coordinates: vec!["rejected".to_owned()],
        effect_lowerings: vec![TargetEffectLowering {
            effect: "target.replace".to_owned(),
            target_intrinsic: "echo.dpo@1.replace".to_owned(),
        }],
    }
}

fn echo_profile_digest() -> String {
    ECHO_PROFILE_DIGEST.to_owned()
}

fn echo_profile_facts() -> TargetProfileFacts {
    TargetProfileFacts {
        coordinate: ECHO_DPO_TARGET_PROFILE.to_owned(),
        operation_profiles: vec!["continuum.profile.write/v1".to_owned()],
        native_effects: vec![NativeEffectSupport {
            coordinate: "target.replace".to_owned(),
            target_intrinsic: "echo.dpo@1.replace".to_owned(),
            write_class: WriteClass::Replace,
            guard_kinds: vec![GuardKind::PrecommitAtomic],
        }],
        direct_adapters: Vec::new(),
        write_classes: vec![WriteClass::Replace],
        guard_kinds: vec![GuardKind::PrecommitAtomic],
        atomicity: vec![AtomicityRequirement::Atomic],
        postcondition_support: true,
        obstruction_coordinates: vec!["rejected".to_owned()],
        footprint_obligations: vec!["target.replace.footprint".to_owned()],
        cost_obligations: vec!["target.replace.cost".to_owned()],
        optic_contracts: vec!["replace-point".to_owned()],
    }
}

fn echo_requirements() -> LoweringRequirements {
    LoweringRequirements {
        operation_profile: "continuum.profile.write/v1".to_owned(),
        semantic_effects: vec![SemanticEffectRequirement {
            coordinate: "target.replace".to_owned(),
            write_class: WriteClass::Replace,
            guard_kinds: vec![GuardKind::PrecommitAtomic],
            obstruction_coordinates: vec!["rejected".to_owned()],
            footprint_obligations: vec!["target.replace.footprint".to_owned()],
            cost_obligations: vec!["target.replace.cost".to_owned()],
        }],
        required_write_classes: vec![WriteClass::Replace],
        guard_kinds: vec![GuardKind::PrecommitAtomic],
        atomicity: AtomicityRequirement::Atomic,
        postcondition_support: true,
        obstruction_coordinates: vec!["rejected".to_owned()],
        footprint_obligations: vec!["target.replace.footprint".to_owned()],
        cost_obligations: vec!["target.replace.cost".to_owned()],
        optic_contract: "replace-point".to_owned(),
    }
}

fn failure_kinds(report: &edict_syntax::TargetLoweringReport) -> Vec<TargetLoweringFailureKind> {
    report.failures.iter().map(|failure| failure.kind).collect()
}

#[test]
fn supported_effectful_core_lowers_to_echo_span_ir() {
    let core = effectful_core();
    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());

    let artifact = report.artifact.expect("supported Core emits Target IR");
    assert_eq!(artifact.domain, ECHO_SPAN_IR_DOMAIN);
    assert_eq!(artifact.target_profile.coordinate, ECHO_DPO_TARGET_PROFILE);
    assert_eq!(artifact.source_core_coordinate, "a.b@1");
    assert_eq!(artifact.intents.len(), 1);

    let intent = artifact.intents.get("t").expect("lowered intent t");
    assert_eq!(intent.operation_profile, "continuum.profile.write/v1");
    assert_eq!(intent.steps.len(), 1);

    let step = &intent.steps[0];
    assert_eq!(step.id, "t.step.0");
    assert_eq!(step.effect, "target.replace");
    assert_eq!(step.target_intrinsic, "echo.dpo@1.replace");
    assert_eq!(step.obstruction_failures, vec!["rejected".to_owned()]);

    let CoreExpr::Field { field, .. } = &step.input else {
        panic!("effect input is preserved structurally");
    };
    assert_eq!(field, "id");
}

#[test]
fn lowerability_native_support_feeds_echo_target_lowering() {
    let profile_facts = echo_profile_facts();
    let lowerability = check_lowerability(&echo_requirements(), &profile_facts);
    assert_eq!(lowerability.status, LowerabilityStatus::Native);
    assert!(lowerability.failures.is_empty());

    let target_facts = TargetIrLoweringFacts::from_lowerability_report(
        Some(echo_profile_digest()),
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    )
    .expect("native lowerability builds target facts");
    let report = lower_to_target_ir(&effectful_core(), &target_facts);

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    let artifact = report
        .artifact
        .expect("native lowerability feeds target IR");
    let step = &artifact.intents.get("t").expect("intent t").steps[0];
    assert_eq!(step.effect, "target.replace");
    assert_eq!(step.target_intrinsic, "echo.dpo@1.replace");
}

#[test]
fn lowerability_bridge_carries_only_selected_native_effect() {
    let mut profile_facts = echo_profile_facts();
    profile_facts.native_effects.push(NativeEffectSupport {
        coordinate: "target.replace".to_owned(),
        target_intrinsic: "echo.dpo@1.replace.unselected".to_owned(),
        write_class: WriteClass::Replace,
        guard_kinds: Vec::new(),
    });
    let lowerability = check_lowerability(&echo_requirements(), &profile_facts);
    assert_eq!(lowerability.status, LowerabilityStatus::Native);
    assert!(lowerability.failures.is_empty());

    let target_facts = TargetIrLoweringFacts::from_lowerability_report(
        Some(echo_profile_digest()),
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    )
    .expect("native lowerability builds target facts");
    let report = lower_to_target_ir(&effectful_core(), &target_facts);

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    let artifact = report
        .artifact
        .expect("unselected native support does not make target lowering ambiguous");
    let step = &artifact.intents.get("t").expect("intent t").steps[0];
    assert_eq!(step.target_intrinsic, "echo.dpo@1.replace");
}

#[test]
fn lowerability_bridge_deduplicates_identical_native_effect_selection() {
    let mut requirements = echo_requirements();
    requirements
        .semantic_effects
        .push(requirements.semantic_effects[0].clone());
    let lowerability = check_lowerability(&requirements, &echo_profile_facts());
    assert_eq!(lowerability.status, LowerabilityStatus::Native);
    assert_eq!(lowerability.effect_results.len(), 2);

    let target_facts = TargetIrLoweringFacts::from_lowerability_report(
        Some(echo_profile_digest()),
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    )
    .expect("native lowerability builds target facts");
    let report = lower_to_target_ir(&effectful_core(), &target_facts);

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    let artifact = report
        .artifact
        .expect("duplicate selected effect still lowers once");
    let step = &artifact.intents.get("t").expect("intent t").steps[0];
    assert_eq!(step.target_intrinsic, "echo.dpo@1.replace");
}

#[test]
fn unused_duplicate_effect_lowerings_do_not_reject_supported_effect() {
    let mut facts = echo_facts();
    facts.effect_lowerings.push(TargetEffectLowering {
        effect: "target.archive".to_owned(),
        target_intrinsic: "echo.dpo@1.archive".to_owned(),
    });
    facts.effect_lowerings.push(TargetEffectLowering {
        effect: "target.archive".to_owned(),
        target_intrinsic: "echo.dpo@1.archive.v2".to_owned(),
    });

    let report = lower_to_target_ir(&effectful_core(), &facts);

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    let artifact = report
        .artifact
        .expect("unused duplicate lowerings do not block supported effect");
    let step = &artifact.intents.get("t").expect("intent t").steps[0];
    assert_eq!(step.effect, "target.replace");
    assert_eq!(step.target_intrinsic, "echo.dpo@1.replace");
}

#[test]
fn unsupported_lowerability_report_does_not_build_target_ir_facts() {
    let mut profile_facts = echo_profile_facts();
    profile_facts.operation_profiles.clear();
    let lowerability = check_lowerability(&echo_requirements(), &profile_facts);
    assert_eq!(lowerability.status, LowerabilityStatus::Unsupported);

    let error = TargetIrLoweringFacts::from_lowerability_report(
        Some(echo_profile_digest()),
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    )
    .expect_err("unsupported lowerability cannot build target facts");

    assert_eq!(
        error.kind,
        TargetLoweringFailureKind::UnsupportedLowerabilityReport
    );
}

#[test]
fn lowerability_bridge_uses_report_target_profile_identity() {
    let mut profile_facts = echo_profile_facts();
    profile_facts.coordinate = "gitwarp.ref_crdt@1".to_owned();
    let lowerability = check_lowerability(&echo_requirements(), &profile_facts);
    assert_eq!(lowerability.status, LowerabilityStatus::Native);

    let target_facts = TargetIrLoweringFacts::from_lowerability_report(
        Some(echo_profile_digest()),
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    )
    .expect("native lowerability builds target facts");
    let report = lower_to_target_ir(&effectful_core(), &target_facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedTargetProfile]
    );
}

#[test]
fn lowerability_bridge_uses_report_operation_profile_identity() {
    let lowerability = check_lowerability(&echo_requirements(), &echo_profile_facts());
    assert_eq!(lowerability.status, LowerabilityStatus::Native);

    let target_facts = TargetIrLoweringFacts::from_lowerability_report(
        Some(echo_profile_digest()),
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    )
    .expect("native lowerability builds target facts");
    let module = edict_syntax::parse_module(EFFECTFUL_REPLACE).expect("effectful source parses");
    let core = compile_to_core(
        &module,
        &effectful_context_with_profile("continuum.profile.unreviewed/v1"),
    )
    .expect("effectful source compiles to Core with caller-supplied profile");
    let report = lower_to_target_ir(&core, &target_facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::MissingOperationProfile]
    );
}

#[test]
fn obstruction_arm_values_are_preserved_in_echo_span_ir() {
    let base = effectful_artifact(EFFECTFUL_REPLACE);
    let changed = effectful_artifact(
        &EFFECTFUL_REPLACE.replace("domain.WriteRejected", "domain.WriteDifferentlyRejected"),
    );

    assert_ne!(base, changed);
}

#[test]
fn intent_result_is_preserved_in_echo_span_ir() {
    let base = effectful_artifact(EFFECTFUL_REPLACE);
    let changed = effectful_artifact(
        &EFFECTFUL_REPLACE.replace("return { id: input.id };", "return { id: receipt.id };"),
    );

    assert_ne!(base, changed);
}

#[test]
fn effect_result_bindings_are_preserved_in_echo_span_ir() {
    let artifact = effectful_artifact(CHAINED_EFFECT_RESULTS);
    let intent = artifact.intents.get("t").expect("intent t");

    assert_eq!(intent.steps.len(), 2);
    assert_eq!(intent.steps[0].binding.id, "local.0");
    assert_eq!(intent.steps[1].binding.id, "local.1");

    let CoreExpr::Field { base, field } = &intent.steps[1].input else {
        panic!("second effect input reads from first effect result");
    };
    assert_eq!(field, "id");
    assert!(matches!(base.as_ref(), CoreExpr::Local { reference } if reference.id == "local.0"));
}

#[test]
fn non_echo_target_profile_rejects_without_artifact() {
    let core = effectful_core();
    let mut facts = echo_facts();
    facts.target_profile.coordinate = "gitwarp.ref_crdt@1".to_owned();
    facts.target_ir_domain = "gitwarp.commit-reducer-ir/v1".to_owned();

    let report = lower_to_target_ir(&core, &facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedTargetProfile]
    );
}

#[test]
fn undigested_target_profile_rejects_without_artifact() {
    for digest in [None, Some("sha256:not-a-review-digest".to_owned())] {
        let mut facts = echo_facts();
        facts.target_profile.digest = digest;

        let report = lower_to_target_ir(&effectful_core(), &facts);

        assert_eq!(report.status, TargetLoweringStatus::Unsupported);
        assert!(report.artifact.is_none());
        assert_eq!(
            failure_kinds(&report),
            vec![TargetLoweringFailureKind::UndigestedTargetProfile]
        );
    }
}

#[test]
fn unsupported_operation_profile_rejects_without_artifact() {
    let module = edict_syntax::parse_module(EFFECTFUL_REPLACE).expect("effectful source parses");
    let core = compile_to_core(
        &module,
        &effectful_context_with_profile("continuum.profile.unreviewed/v1"),
    )
    .expect("effectful source compiles to Core with caller-supplied profile");

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::MissingOperationProfile]
    );
}

#[test]
fn foreign_target_intrinsic_rejects_without_artifact() {
    let mut facts = echo_facts();
    facts.effect_lowerings[0].target_intrinsic = "kv.transactional@1.get".to_owned();

    let report = lower_to_target_ir(&effectful_core(), &facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedTargetIntrinsic]
    );
}

#[test]
fn unsupported_obstruction_key_rejects_without_artifact() {
    let module = edict_syntax::parse_module(
        &EFFECTFUL_REPLACE.replace("rejected(reason) =>", "unexpected(reason) =>"),
    )
    .expect("effectful source parses");
    let core =
        compile_to_core(&module, &effectful_context()).expect("effectful source compiles to Core");

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::MissingObstruction]
    );
}

#[test]
fn empty_target_step_intents_reject_without_artifact() {
    let mut core = effectful_core();
    core.intents
        .get_mut("t")
        .expect("intent t")
        .body
        .nodes
        .clear();

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::NoTargetSteps]
    );
}

#[test]
fn empty_core_modules_reject_without_artifact() {
    let mut core = effectful_core();
    core.intents.clear();

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::NoTargetSteps]
    );
}

#[test]
fn unsupported_core_nodes_reject_without_artifact() {
    let core = pure_core();
    let mut facts = echo_facts();
    facts
        .operation_profiles
        .push("continuum.profile.read-only/v1".to_owned());
    let report = lower_to_target_ir(&core, &facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedCoreNode]
    );
}

#[test]
fn unsupported_core_abi_rejects_without_artifact() {
    let mut core = effectful_core();
    core.api_version = "edict.core/v2".to_owned();

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedCoreAbi]
    );
}

#[test]
fn unsupported_core_capability_rejects_without_artifact() {
    let mut core = effectful_core();
    core.required_core_capabilities
        .push("edict.core.capability.variant-map/v1".to_owned());

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedCoreCapability]
    );
}
