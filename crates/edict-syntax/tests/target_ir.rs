//! Target IR generation tests for the first Echo lowering slice.
//!
//! These tests assert public behavior: structured Target IR artifacts and
//! stable failure kinds. They do not inspect diagnostic prose, repository
//! layout, or implementation-private lowering helpers.

use std::collections::BTreeMap;

use edict_syntax::{
    check_lowerability, compile_to_core, decode_canonical_cbor, digest_target_ir_artifact,
    encode_target_ir_artifact, lower_to_target_ir, AtomicityRequirement, CanonicalErrorKind,
    CompilerContext, CoreBudget, CoreExpr, CoreImport, CoreImportKind, CorePredicate, CoreValue,
    GuardKind, InputConstraint, InputConstraintSource, LowerabilityStatus, LoweringRequirements,
    NativeEffectSupport, ResourceRef, SemanticEffectRequirement, TargetEffectLowering,
    TargetIrArtifact, TargetIrLoweringFacts, TargetLoweringFailureKind, TargetLoweringStatus,
    TargetProfileFacts, WriteClass, ECHO_DPO_TARGET_PROFILE, ECHO_SPAN_IR_DOMAIN,
    GITWARP_COMMIT_REDUCER_IR_DOMAIN, GITWARP_REF_CRDT_TARGET_PROFILE,
    TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
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
const GITWARP_APPEND_EVENT: &str = "package a.git@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.gitwarp\n\
      basis none\n\
      budget <= p.tiny\n\
      where input.id != \"\" {\n\
      let receipt: Receipt = gitwarp.appendEvent(input.id)\n\
        else { conflict(reason) => domain.MergeConflict };\n\
      return { id: receipt.id };\n\
    }";

const PURE_LOCAL_RECORD: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");
const ECHO_PROFILE_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const GITWARP_PROFILE_DIGEST: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";

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

fn gitwarp_artifact() -> edict_syntax::TargetIrArtifact {
    lower_to_target_ir(&gitwarp_core(), &gitwarp_facts())
        .artifact
        .expect("supported git-warp source lowers to Target IR")
}

fn pure_core() -> edict_syntax::CoreModule {
    let module = edict_syntax::parse_module(PURE_LOCAL_RECORD).expect("pure source parses");
    compile_to_core(&module, &pure_context()).expect("pure source compiles to Core")
}

fn gitwarp_core() -> edict_syntax::CoreModule {
    let module = edict_syntax::parse_module(GITWARP_APPEND_EVENT).expect("git-warp source parses");
    compile_to_core(&module, &gitwarp_context()).expect("git-warp source compiles to Core")
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

fn gitwarp_context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("p.gitwarp", "continuum.profile.append/v1")
        .with_operation_profile_write_classes("p.gitwarp", [WriteClass::Append])
        .with_effect_write_class("gitwarp.appendEvent", WriteClass::Append)
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 13,
                max_allocated_bytes: 2048,
                max_output_bytes: 512,
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

fn gitwarp_facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: GITWARP_REF_CRDT_TARGET_PROFILE.to_owned(),
            digest: Some(gitwarp_profile_digest()),
        },
        target_ir_domain: GITWARP_COMMIT_REDUCER_IR_DOMAIN.to_owned(),
        operation_profiles: vec!["continuum.profile.append/v1".to_owned()],
        obstruction_coordinates: vec!["conflict".to_owned()],
        effect_lowerings: vec![TargetEffectLowering {
            effect: "gitwarp.appendEvent".to_owned(),
            target_intrinsic: "gitwarp.ref_crdt@1.appendEvent".to_owned(),
        }],
    }
}

fn echo_profile_digest() -> String {
    ECHO_PROFILE_DIGEST.to_owned()
}

fn gitwarp_profile_digest() -> String {
    GITWARP_PROFILE_DIGEST.to_owned()
}

fn echo_profile_ref() -> ResourceRef {
    ResourceRef {
        coordinate: ECHO_DPO_TARGET_PROFILE.to_owned(),
        digest: Some(echo_profile_digest()),
    }
}

fn gitwarp_profile_ref() -> ResourceRef {
    ResourceRef {
        coordinate: GITWARP_REF_CRDT_TARGET_PROFILE.to_owned(),
        digest: Some(gitwarp_profile_digest()),
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

fn gitwarp_profile_facts() -> TargetProfileFacts {
    TargetProfileFacts {
        coordinate: GITWARP_REF_CRDT_TARGET_PROFILE.to_owned(),
        operation_profiles: vec!["continuum.profile.append/v1".to_owned()],
        native_effects: vec![NativeEffectSupport {
            coordinate: "gitwarp.appendEvent".to_owned(),
            target_intrinsic: "gitwarp.ref_crdt@1.appendEvent".to_owned(),
            write_class: WriteClass::Append,
            guard_kinds: vec![GuardKind::PrecommitAtomic],
        }],
        direct_adapters: Vec::new(),
        write_classes: vec![WriteClass::Append],
        guard_kinds: vec![GuardKind::PrecommitAtomic],
        atomicity: vec![AtomicityRequirement::Atomic],
        postcondition_support: true,
        obstruction_coordinates: vec!["conflict".to_owned()],
        footprint_obligations: vec!["gitwarp.appendEvent.footprint".to_owned()],
        cost_obligations: vec!["gitwarp.appendEvent.cost".to_owned()],
        optic_contracts: vec!["append-event".to_owned()],
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

fn gitwarp_requirements() -> LoweringRequirements {
    LoweringRequirements {
        operation_profile: "continuum.profile.append/v1".to_owned(),
        semantic_effects: vec![SemanticEffectRequirement {
            coordinate: "gitwarp.appendEvent".to_owned(),
            write_class: WriteClass::Append,
            guard_kinds: vec![GuardKind::PrecommitAtomic],
            obstruction_coordinates: vec!["conflict".to_owned()],
            footprint_obligations: vec!["gitwarp.appendEvent.footprint".to_owned()],
            cost_obligations: vec!["gitwarp.appendEvent.cost".to_owned()],
        }],
        required_write_classes: vec![WriteClass::Append],
        guard_kinds: vec![GuardKind::PrecommitAtomic],
        atomicity: AtomicityRequirement::Atomic,
        postcondition_support: true,
        obstruction_coordinates: vec!["conflict".to_owned()],
        footprint_obligations: vec!["gitwarp.appendEvent.footprint".to_owned()],
        cost_obligations: vec!["gitwarp.appendEvent.cost".to_owned()],
        optic_contract: "append-event".to_owned(),
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
        echo_profile_ref(),
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
fn supported_gitwarp_core_lowers_to_commit_reducer_ir() {
    let core = gitwarp_core();
    let report = lower_to_target_ir(&core, &gitwarp_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());

    let artifact = report.artifact.expect("supported Core emits Target IR");
    assert_eq!(artifact.domain, GITWARP_COMMIT_REDUCER_IR_DOMAIN);
    assert_eq!(
        artifact.target_profile.coordinate,
        GITWARP_REF_CRDT_TARGET_PROFILE
    );
    assert_eq!(artifact.source_core_coordinate, "a.git@1");

    let intent = artifact.intents.get("t").expect("lowered intent t");
    assert_eq!(intent.operation_profile, "continuum.profile.append/v1");
    assert_eq!(
        intent.core_evaluation_budget,
        CoreBudget {
            max_steps: 13,
            max_allocated_bytes: 2048,
            max_output_bytes: 512,
        }
    );
    assert_eq!(intent.input_constraints.len(), 1);
    assert!(matches!(
        intent.input_constraints[0].predicate,
        CorePredicate::Compare { .. }
    ));
    assert_eq!(intent.steps.len(), 1);

    let step = &intent.steps[0];
    assert_eq!(step.effect, "gitwarp.appendEvent");
    assert_eq!(step.target_intrinsic, "gitwarp.ref_crdt@1.appendEvent");
    assert_eq!(step.obstruction_failures, vec!["conflict".to_owned()]);
    assert!(step.obstruction_arms.contains_key("conflict"));

    let CoreExpr::Field { field, .. } = &step.input else {
        panic!("git-warp effect input is preserved structurally");
    };
    assert_eq!(field, "id");

    let CoreExpr::Record { fields } = &intent.result else {
        panic!("git-warp intent result is preserved structurally");
    };
    assert!(fields.contains_key("id"));
}

#[test]
fn lowerability_native_support_feeds_gitwarp_target_lowering() {
    let lowerability = check_lowerability(&gitwarp_requirements(), &gitwarp_profile_facts());
    assert_eq!(lowerability.status, LowerabilityStatus::Native);
    assert!(lowerability.failures.is_empty());

    let target_facts = TargetIrLoweringFacts::from_lowerability_report(
        gitwarp_profile_ref(),
        GITWARP_COMMIT_REDUCER_IR_DOMAIN,
        &lowerability,
    )
    .expect("native git-warp lowerability builds target facts");
    let report = lower_to_target_ir(&gitwarp_core(), &target_facts);

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    let artifact = report
        .artifact
        .expect("native git-warp lowerability feeds target IR");
    let step = &artifact.intents.get("t").expect("intent t").steps[0];
    assert_eq!(step.effect, "gitwarp.appendEvent");
    assert_eq!(step.target_intrinsic, "gitwarp.ref_crdt@1.appendEvent");
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
        echo_profile_ref(),
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
        echo_profile_ref(),
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
        echo_profile_ref(),
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

    let error = TargetIrLoweringFacts::from_lowerability_report(
        echo_profile_ref(),
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    )
    .expect_err("target profile reference must match lowerability report");

    assert_eq!(
        error.kind,
        TargetLoweringFailureKind::UnsupportedTargetProfile
    );
}

#[test]
fn lowerability_bridge_uses_report_operation_profile_identity() {
    let lowerability = check_lowerability(&echo_requirements(), &echo_profile_facts());
    assert_eq!(lowerability.status, LowerabilityStatus::Native);

    let target_facts = TargetIrLoweringFacts::from_lowerability_report(
        echo_profile_ref(),
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
fn lowerability_bridge_requires_matching_target_profile_reference() {
    let lowerability = check_lowerability(&echo_requirements(), &echo_profile_facts());
    assert_eq!(lowerability.status, LowerabilityStatus::Native);

    let error = TargetIrLoweringFacts::from_lowerability_report(
        ResourceRef {
            coordinate: "gitwarp.ref_crdt@1".to_owned(),
            digest: Some(echo_profile_digest()),
        },
        ECHO_SPAN_IR_DOMAIN,
        &lowerability,
    )
    .expect_err("target profile reference must match lowerability report");

    assert_eq!(
        error.kind,
        TargetLoweringFailureKind::UnsupportedTargetProfile
    );
}

#[test]
fn obstruction_arm_values_are_preserved_in_echo_span_ir() {
    let artifact = effectful_artifact(
        &EFFECTFUL_REPLACE.replace("domain.WriteRejected", "domain.WriteDifferentlyRejected"),
    );
    let arm = &artifact.intents.get("t").expect("intent t").steps[0].obstruction_arms["rejected"];

    let CoreExpr::Call {
        callee,
        type_args,
        args,
    } = &arm.value
    else {
        panic!("obstruction arm value is preserved as a call expression");
    };
    assert_eq!(callee, "domain.WriteDifferentlyRejected");
    assert!(type_args.is_empty());
    assert!(args.is_empty());
}

#[test]
fn intent_result_is_preserved_in_echo_span_ir() {
    let artifact = effectful_artifact(
        &EFFECTFUL_REPLACE.replace("return { id: input.id };", "return { id: receipt.id };"),
    );
    let result = &artifact.intents.get("t").expect("intent t").result;

    let CoreExpr::Record { fields } = result else {
        panic!("intent result is preserved as a record expression");
    };
    let CoreExpr::Field { base, field } = &fields["id"] else {
        panic!("result id field is preserved as a field expression");
    };
    assert_eq!(field, "id");
    assert!(matches!(base.as_ref(), CoreExpr::Local { reference } if reference.id == "local.0"));
}

#[test]
fn intent_constraints_and_budget_are_preserved_in_echo_span_ir() {
    let constrained_source = EFFECTFUL_REPLACE.replace(
        "budget <= p.tiny {",
        "budget <= p.tiny\n      where input.id != \"\" {",
    );
    let artifact = effectful_artifact(&constrained_source);
    let intent = artifact.intents.get("t").expect("intent t");

    assert_eq!(
        intent.core_evaluation_budget,
        CoreBudget {
            max_steps: 8,
            max_allocated_bytes: 1024,
            max_output_bytes: 256,
        }
    );
    assert_eq!(intent.input_constraints.len(), 1);
    assert_eq!(intent.input_constraints[0].coordinate, "where.0");
    assert!(matches!(
        intent.input_constraints[0].predicate,
        CorePredicate::Compare { .. }
    ));
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
fn unsupported_target_profile_rejects_without_artifact() {
    let core = effectful_core();
    let mut facts = echo_facts();
    facts.target_profile.coordinate = "kv.transactional@1".to_owned();
    facts.target_ir_domain = "kv.transaction-ir/v1".to_owned();

    let report = lower_to_target_ir(&core, &facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedTargetProfile]
    );
}

#[test]
fn unsupported_target_ir_domain_rejects_without_artifact() {
    let mut facts = echo_facts();
    facts.target_ir_domain = "echo.span-ir/v2".to_owned();

    let report = lower_to_target_ir(&effectful_core(), &facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedTargetIrDomain]
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
fn missing_effect_lowering_rejects_without_artifact() {
    let mut facts = echo_facts();
    facts.effect_lowerings.clear();

    let report = lower_to_target_ir(&effectful_core(), &facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::MissingEffectLowering]
    );
}

#[test]
fn ambiguous_effect_lowering_rejects_without_artifact() {
    let mut facts = echo_facts();
    facts.effect_lowerings.push(TargetEffectLowering {
        effect: "target.replace".to_owned(),
        target_intrinsic: "echo.dpo@1.replace.alternate".to_owned(),
    });

    let report = lower_to_target_ir(&effectful_core(), &facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::AmbiguousEffectLowering]
    );
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

#[test]
fn undigested_core_import_rejects_without_artifact() {
    let mut core = effectful_core();
    core.imports.push(CoreImport {
        kind: CoreImportKind::Lawpack,
        resource: ResourceRef {
            coordinate: "hello.optics@1".to_owned(),
            digest: None,
        },
        alias: Some("hello".to_owned()),
    });

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UndigestedCoreImport]
    );
}

#[test]
fn target_ir_artifact_bytes_and_digests_are_deterministic() {
    assert_eq!(
        TARGET_IR_ARTIFACT_DIGEST_DOMAIN,
        "edict.target-ir.artifact/v1"
    );

    let echo = effectful_artifact(EFFECTFUL_REPLACE);
    let gitwarp = gitwarp_artifact();

    let echo_bytes = encode_target_ir_artifact(&echo).expect("Echo Target IR encodes");
    let echo_bytes_again = encode_target_ir_artifact(&echo).expect("Echo Target IR re-encodes");
    assert_eq!(echo_bytes, echo_bytes_again);
    decode_canonical_cbor(&echo_bytes).expect("Echo Target IR bytes are canonical CBOR");

    let gitwarp_bytes = encode_target_ir_artifact(&gitwarp).expect("git-warp Target IR encodes");
    let gitwarp_bytes_again =
        encode_target_ir_artifact(&gitwarp).expect("git-warp Target IR re-encodes");
    assert_eq!(gitwarp_bytes, gitwarp_bytes_again);
    decode_canonical_cbor(&gitwarp_bytes).expect("git-warp Target IR bytes are canonical CBOR");

    assert_ne!(echo_bytes, gitwarp_bytes);

    let echo_digest = digest_target_ir_artifact(&echo).expect("Echo Target IR digests");
    let echo_digest_again = digest_target_ir_artifact(&echo).expect("Echo Target IR re-digests");
    assert_eq!(echo_digest, echo_digest_again);
    assert!(echo_digest.to_review_string().starts_with("sha256:"));
    assert_eq!(echo_digest.to_review_string().len(), "sha256:".len() + 64);

    let gitwarp_digest = digest_target_ir_artifact(&gitwarp).expect("git-warp Target IR digests");
    assert_ne!(echo_digest, gitwarp_digest);
}

#[test]
fn target_ir_artifact_canonicalization_ignores_equivalent_construction_order() {
    let mut left = gitwarp_artifact();
    let mut right = left.clone();

    let extra_constraint = InputConstraint {
        coordinate: "compiler.0".to_owned(),
        source: InputConstraintSource::Compiler,
        predicate: CorePredicate::True,
    };

    let left_intent = left.intents.get_mut("t").expect("intent t");
    left_intent.input_constraints.push(extra_constraint.clone());
    let left_step = left_intent.steps.get_mut(0).expect("step 0");
    let conflict_arm = left_step
        .obstruction_arms
        .get("conflict")
        .expect("conflict arm")
        .clone();
    left_step
        .obstruction_arms
        .insert("retry".to_owned(), conflict_arm.clone());
    left_step.obstruction_failures = vec!["retry".to_owned(), "conflict".to_owned()];

    let right_intent = right.intents.get_mut("t").expect("intent t");
    right_intent.input_constraints.insert(0, extra_constraint);
    let right_step = right_intent.steps.get_mut(0).expect("step 0");
    let mut rebuilt_arms = BTreeMap::new();
    rebuilt_arms.insert("retry".to_owned(), conflict_arm);
    rebuilt_arms.insert(
        "conflict".to_owned(),
        right_step
            .obstruction_arms
            .get("conflict")
            .expect("conflict arm")
            .clone(),
    );
    right_step.obstruction_arms = rebuilt_arms;
    right_step.obstruction_failures = vec!["conflict".to_owned(), "retry".to_owned()];

    assert_eq!(
        encode_target_ir_artifact(&left).expect("left Target IR encodes"),
        encode_target_ir_artifact(&right).expect("right Target IR encodes")
    );
    assert_eq!(
        digest_target_ir_artifact(&left).expect("left Target IR digests"),
        digest_target_ir_artifact(&right).expect("right Target IR digests")
    );
}

#[test]
fn target_ir_digest_moves_for_artifact_semantic_mutations() {
    let baseline = effectful_artifact(EFFECTFUL_REPLACE);
    assert_target_ir_digest_changes(&baseline, "target profile digest", |artifact| {
        artifact.target_profile.digest = Some(digest_text('3'));
    });
    assert_target_ir_digest_changes(&baseline, "source Core coordinate", |artifact| {
        artifact.source_core_coordinate = "a.changed@1".to_owned();
    });
    assert_target_ir_digest_changes(&baseline, "intent name", |artifact| {
        let intent = artifact.intents.remove("t").expect("intent t");
        artifact.intents.insert("renamed".to_owned(), intent);
    });
    assert_target_ir_digest_changes(&baseline, "effect coordinate", |artifact| {
        target_step_mut(artifact).effect = "target.replace.changed".to_owned();
    });
    assert_target_ir_digest_changes(&baseline, "selected target intrinsic", |artifact| {
        target_step_mut(artifact).target_intrinsic = "echo.dpo@1.replace.changed".to_owned();
    });
    assert_target_ir_digest_changes(&baseline, "input expression", |artifact| {
        target_step_mut(artifact).input = CoreExpr::Const(CoreValue::String("changed".to_owned()));
    });
    assert_target_ir_digest_changes(&baseline, "obstruction failure", |artifact| {
        target_step_mut(artifact)
            .obstruction_failures
            .push("timeout".to_owned());
    });
    assert_target_ir_digest_changes(&baseline, "input constraint", |artifact| {
        artifact
            .intents
            .get_mut("t")
            .expect("intent t")
            .input_constraints
            .push(InputConstraint {
                coordinate: "compiler.0".to_owned(),
                source: InputConstraintSource::Compiler,
                predicate: CorePredicate::True,
            });
    });
    assert_target_ir_digest_changes(&baseline, "Core evaluation budget", |artifact| {
        artifact
            .intents
            .get_mut("t")
            .expect("intent t")
            .core_evaluation_budget
            .max_steps += 1;
    });
    assert_target_ir_digest_changes(&baseline, "result expression", |artifact| {
        artifact.intents.get_mut("t").expect("intent t").result =
            CoreExpr::Const(CoreValue::String("changed".to_owned()));
    });
}

#[test]
fn target_ir_encoder_rejects_unlocked_or_uppercase_target_profile_digest() {
    let mut missing = effectful_artifact(EFFECTFUL_REPLACE);
    missing.target_profile.digest = None;
    assert_eq!(
        encode_target_ir_artifact(&missing)
            .expect_err("missing target profile digest rejects before hashing")
            .kind(),
        CanonicalErrorKind::UnresolvedDigest
    );
    assert_eq!(
        digest_target_ir_artifact(&missing)
            .expect_err("missing target profile digest rejects during digest")
            .kind(),
        CanonicalErrorKind::UnresolvedDigest
    );

    let mut uppercase = effectful_artifact(EFFECTFUL_REPLACE);
    uppercase.target_profile.digest =
        Some("sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_owned());
    assert_eq!(
        encode_target_ir_artifact(&uppercase)
            .expect_err("uppercase target profile digest rejects before hashing")
            .kind(),
        CanonicalErrorKind::InvalidDigest
    );
}

fn assert_target_ir_digest_changes(
    baseline: &TargetIrArtifact,
    case: &str,
    mutate: impl FnOnce(&mut TargetIrArtifact),
) {
    let baseline_digest =
        digest_target_ir_artifact(baseline).expect("baseline Target IR artifact digests");
    let mut mutated = baseline.clone();
    mutate(&mut mutated);
    let mutated_digest =
        digest_target_ir_artifact(&mutated).expect("mutated Target IR artifact digests");
    assert_ne!(baseline_digest, mutated_digest, "{case} must move digest");
}

fn target_step_mut(artifact: &mut TargetIrArtifact) -> &mut edict_syntax::TargetIrStep {
    artifact
        .intents
        .get_mut("t")
        .expect("intent t")
        .steps
        .get_mut(0)
        .expect("step 0")
}

fn digest_text(hex: char) -> String {
    format!("sha256:{}", hex.to_string().repeat(64))
}
