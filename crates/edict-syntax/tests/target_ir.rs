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

const PURE_LOCAL_RECORD: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");

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
    CompilerContext::new()
        .with_operation_profile("p.effectful", "continuum.profile.write/v1")
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
            digest: Some(
                "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                    .to_owned(),
            ),
        },
        target_ir_domain: ECHO_SPAN_IR_DOMAIN.to_owned(),
        effect_lowerings: vec![TargetEffectLowering {
            effect: "target.replace".to_owned(),
            target_intrinsic: "echo.dpo@1.replace".to_owned(),
        }],
    }
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
        ResourceRef {
            coordinate: profile_facts.coordinate.clone(),
            digest: Some(
                "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                    .to_owned(),
            ),
        },
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    );
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
        ResourceRef {
            coordinate: profile_facts.coordinate.clone(),
            digest: Some(
                "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                    .to_owned(),
            ),
        },
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    );
    let report = lower_to_target_ir(&effectful_core(), &target_facts);

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    let artifact = report
        .artifact
        .expect("unselected native support does not make target lowering ambiguous");
    let step = &artifact.intents.get("t").expect("intent t").steps[0];
    assert_eq!(step.target_intrinsic, "echo.dpo@1.replace");
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
fn unsupported_core_nodes_reject_without_artifact() {
    let core = pure_core();
    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedCoreNode]
    );
}
