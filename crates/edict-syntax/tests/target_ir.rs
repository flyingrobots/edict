//! Target IR generation tests for the first Echo lowering slice.
//!
//! These tests assert public behavior: structured Target IR artifacts and
//! stable failure kinds. They do not inspect diagnostic prose, repository
//! layout, or implementation-private lowering helpers.

use std::collections::BTreeMap;

use edict_syntax::{
    check_lowerability, compile_to_core, decode_canonical_cbor, digest_target_ir_artifact,
    encode_target_ir_artifact, lower_to_target_ir, AtomicityRequirement, CanonicalErrorKind,
    CompareOp, CompilerContext, CoreBlock, CoreBound, CoreBudget, CoreExpr,
    CoreExternalActionBudget, CoreImport, CoreImportKind, CoreNode, CorePredicate, CoreType,
    CoreValue, GuardKind, InputConstraint, InputConstraintSource, LocalRef, LowerabilityStatus,
    LoweringRequirements, NativeEffectSupport, ResourceRef, SemanticEffectRequirement,
    TargetEffectLowering, TargetIrArtifact, TargetIrExternalActionRequest, TargetIrLoweringFacts,
    TargetIrRequireFailure, TargetIrStep, TargetLoweringFailureKind, TargetLoweringStatus,
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
const ECHO_CONTINUE_OBSTRUCTED_REQUIRE: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      require true else continue obstructed {\n\
        reason: jim.EditObstruction.StaleBase,\n\
        provided: input.id,\n\
      };\n\
      return { id: input.id };\n\
    }";
const ECHO_TERMINAL_REQUIRE: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      require true else jim.EditObstruction.StaleBase({ reason: input.id });\n\
      return { id: input.id };\n\
    }";
const ECHO_EFFECT_OUTPUT_DEPENDENT_REQUIRE: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      let receipt: Receipt = target.replace(input.id)\n\
        else { rejected(reason) => domain.WriteRejected };\n\
      require receipt.id != \"\" else continue obstructed {\n\
        reason: jim.EditObstruction.StaleBase,\n\
        provided: receipt.id,\n\
      };\n\
      return { id: receipt.id };\n\
    }";
const ECHO_POST_STEP_INPUT_REQUIRE: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      let receipt: Receipt = target.replace(input.id)\n\
        else { rejected(reason) => domain.WriteRejected };\n\
      require input.id != \"\" else continue obstructed {\n\
        reason: jim.EditObstruction.StaleBase,\n\
        provided: input.id,\n\
      };\n\
      return { id: receipt.id };\n\
    }";
const GITWARP_CONTINUE_OBSTRUCTED_REQUIRE: &str = "package a.git@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.gitwarp\n\
      basis none\n\
      budget <= p.tiny {\n\
      require true else continue obstructed {\n\
        reason: jim.EditObstruction.StaleBase,\n\
        provided: input.id,\n\
      };\n\
      return { id: input.id };\n\
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

fn pure_target_facts() -> TargetIrLoweringFacts {
    let mut facts = echo_facts();
    facts
        .operation_profiles
        .push("continuum.profile.read-only/v1".to_owned());
    facts
}

fn pure_artifact() -> TargetIrArtifact {
    lower_to_target_ir(&pure_core(), &pure_target_facts())
        .artifact
        .expect("pure source lowers to Target IR")
}

fn gitwarp_core() -> edict_syntax::CoreModule {
    let module = edict_syntax::parse_module(GITWARP_APPEND_EVENT).expect("git-warp source parses");
    compile_to_core(&module, &gitwarp_context()).expect("git-warp source compiles to Core")
}

fn gitwarp_obstruction_core() -> edict_syntax::CoreModule {
    let module = edict_syntax::parse_module(GITWARP_CONTINUE_OBSTRUCTED_REQUIRE)
        .expect("git-warp obstruction source parses");
    compile_to_core(&module, &gitwarp_context())
        .expect("git-warp obstruction source compiles to Core")
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
            failure_mappings: BTreeMap::new(),
        }],
        pure_functions: Vec::new(),
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
            failure_mappings: BTreeMap::new(),
        }],
        pure_functions: Vec::new(),
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
fn echo_target_ir_contains_obstruction_requirement_payload() {
    let artifact = effectful_artifact(ECHO_CONTINUE_OBSTRUCTED_REQUIRE);
    let intent = artifact.intents.get("t").expect("lowered intent t");

    assert!(intent.steps.is_empty());
    assert_eq!(intent.requirements.len(), 1);
    let requirement = &intent.requirements[0];
    assert_eq!(requirement.id, "t.require.0");
    assert_eq!(requirement.predicate, CorePredicate::True);

    let TargetIrRequireFailure::ContinueObstructed { reason } = &requirement.on_failure else {
        panic!("continue obstructed require remains preserved in Target IR");
    };
    assert_eq!(reason.kind, "jim.EditObstruction.StaleBase");
    assert_eq!(reason.payload.keys().collect::<Vec<_>>(), vec!["provided"]);
    assert!(matches!(
        &reason.payload["provided"],
        CoreExpr::Field { field, .. } if field == "id"
    ));
}

#[test]
fn terminal_and_preserved_requirements_are_target_ir_distinct() {
    let terminal = effectful_artifact(ECHO_TERMINAL_REQUIRE);
    let preserved_source = replace_required(
        ECHO_CONTINUE_OBSTRUCTED_REQUIRE,
        "provided: input.id,\n",
        "",
    );
    let preserved = effectful_artifact(&preserved_source);

    let terminal_requirement = &terminal.intents.get("t").expect("intent t").requirements[0];
    let preserved_requirement = &preserved.intents.get("t").expect("intent t").requirements[0];
    assert!(matches!(
        terminal_requirement.on_failure,
        TargetIrRequireFailure::Terminal { .. }
    ));
    assert!(matches!(
        preserved_requirement.on_failure,
        TargetIrRequireFailure::ContinueObstructed { .. }
    ));
    assert_ne!(
        digest_target_ir_artifact(&terminal).expect("terminal Target IR digests"),
        digest_target_ir_artifact(&preserved).expect("preserved Target IR digests")
    );
}

#[test]
fn target_ir_requirement_mutations_move_digest() {
    let baseline = effectful_artifact(ECHO_CONTINUE_OBSTRUCTED_REQUIRE);
    assert_target_ir_digest_changes(&baseline, "require predicate", |artifact| {
        requirement_mut(artifact).predicate = CorePredicate::False;
    });
    assert_target_ir_digest_changes(&baseline, "require reason kind", |artifact| {
        let TargetIrRequireFailure::ContinueObstructed { reason } =
            &mut requirement_mut(artifact).on_failure
        else {
            panic!("baseline requirement stays preserved");
        };
        reason.kind = "jim.EditObstruction.Other".to_owned();
    });
    assert_target_ir_digest_changes(&baseline, "require reason payload value", |artifact| {
        let TargetIrRequireFailure::ContinueObstructed { reason } =
            &mut requirement_mut(artifact).on_failure
        else {
            panic!("baseline requirement stays preserved");
        };
        reason.payload.insert(
            "provided".to_owned(),
            CoreExpr::Const(CoreValue::String("changed".to_owned())),
        );
    });
    assert_target_ir_digest_changes(&baseline, "require failure disposition", |artifact| {
        let TargetIrRequireFailure::ContinueObstructed { reason } =
            requirement_mut(artifact).on_failure.clone()
        else {
            panic!("baseline requirement stays preserved");
        };
        requirement_mut(artifact).on_failure = TargetIrRequireFailure::Terminal { reason };
    });
}

#[test]
fn targets_without_obstruction_requirement_support_reject_with_stable_feature_kind() {
    let report = lower_to_target_ir(&gitwarp_obstruction_core(), &gitwarp_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedTargetFeature]
    );
    assert_eq!(report.failures[0].detail, "obstruction_requirement");
}

#[test]
fn requirement_that_reads_step_output_rejects_with_stable_feature_kind() {
    let module = edict_syntax::parse_module(ECHO_EFFECT_OUTPUT_DEPENDENT_REQUIRE)
        .expect("effect output dependent require source parses");
    let core = compile_to_core(&module, &effectful_context())
        .expect("effect output dependent require source compiles to Core");

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedTargetFeature]
    );
    assert_eq!(
        report.failures[0].detail,
        "obstruction_requirement_step_output_dependency"
    );
}

#[test]
fn requirement_after_target_step_rejects_with_stable_feature_kind() {
    let module = edict_syntax::parse_module(ECHO_POST_STEP_INPUT_REQUIRE)
        .expect("post-step input require source parses");
    let core = compile_to_core(&module, &effectful_context())
        .expect("post-step input require source compiles to Core");

    let report = lower_to_target_ir(&core, &echo_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedTargetFeature]
    );
    assert_eq!(
        report.failures[0].detail,
        "obstruction_requirement_after_target_step"
    );
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
        failure_mappings: BTreeMap::new(),
    });
    facts.effect_lowerings.push(TargetEffectLowering {
        effect: "target.archive".to_owned(),
        target_intrinsic: "echo.dpo@1.archive.v2".to_owned(),
        failure_mappings: BTreeMap::new(),
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
        failure_mappings: BTreeMap::new(),
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
fn every_copied_target_expression_requires_closed_helper_authority() {
    let unbound_call = || CoreExpr::Call {
        callee: "hello.echo@1.notExported".to_owned(),
        type_args: Vec::new(),
        args: Vec::new(),
    };
    let require_core = || {
        let module = edict_syntax::parse_module(ECHO_CONTINUE_OBSTRUCTED_REQUIRE)
            .expect("require source parses");
        compile_to_core(&module, &effectful_context()).expect("require source compiles")
    };

    let mut basis = require_core();
    basis.intents.get_mut("t").expect("intent t").basis = Some(unbound_call());

    let mut input_constraint = require_core();
    input_constraint
        .intents
        .get_mut("t")
        .expect("intent t")
        .input_constraints
        .push(InputConstraint {
            coordinate: "review.constraint".to_owned(),
            source: InputConstraintSource::Compiler,
            predicate: CorePredicate::Compare {
                op: CompareOp::Eq,
                left: unbound_call(),
                right: CoreExpr::Const(CoreValue::Bool(true)),
            },
        });

    let mut require_predicate = require_core();
    let CoreNode::Require { predicate, .. } = &mut require_predicate
        .intents
        .get_mut("t")
        .expect("intent t")
        .body
        .nodes[0]
    else {
        panic!("fixture begins with a requirement");
    };
    *predicate = CorePredicate::Compare {
        op: CompareOp::Eq,
        left: unbound_call(),
        right: CoreExpr::Const(CoreValue::Bool(true)),
    };

    let mut require_payload = require_core();
    let CoreNode::Require { arm, .. } = &mut require_payload
        .intents
        .get_mut("t")
        .expect("intent t")
        .body
        .nodes[0]
    else {
        panic!("fixture begins with a requirement");
    };
    let reason = match arm {
        edict_syntax::CoreRequireFailureArm::Terminal { reason }
        | edict_syntax::CoreRequireFailureArm::ContinueObstructed { reason } => reason,
    };
    reason.payload.insert("forged".to_owned(), unbound_call());

    let mut effect_input = effectful_core();
    let CoreNode::Effect { input, .. } = &mut effect_input
        .intents
        .get_mut("t")
        .expect("intent t")
        .body
        .nodes[0]
    else {
        panic!("fixture begins with an effect");
    };
    *input = unbound_call();

    let mut obstruction_value = effectful_core();
    let CoreNode::Effect {
        obstruction_map, ..
    } = &mut obstruction_value
        .intents
        .get_mut("t")
        .expect("intent t")
        .body
        .nodes[0]
    else {
        panic!("fixture begins with an effect");
    };
    let Some(arm) = obstruction_map.values_mut().next() else {
        panic!("fixture carries an obstruction arm");
    };
    arm.value = CoreExpr::Call {
        callee: "hello.echo@1.notExported".to_owned(),
        type_args: Vec::new(),
        args: vec![CoreExpr::Const(CoreValue::Bool(true))],
    };

    assert_invalid_copied_expression_cases([
        ("intent basis", basis),
        ("input constraint", input_constraint),
        ("require predicate", require_predicate),
        ("require payload", require_payload),
        ("effect input", effect_input),
        ("obstruction value", obstruction_value),
    ]);
}

fn assert_invalid_copied_expression_cases(
    cases: impl IntoIterator<Item = (&'static str, edict_syntax::CoreModule)>,
) {
    for (case, core) in cases {
        let report = lower_to_target_ir(&core, &echo_facts());

        assert_eq!(report.status, TargetLoweringStatus::Unsupported, "{case}");
        assert!(report.artifact.is_none(), "{case}");
        let [failure] = report.failures.as_slice() else {
            panic!("{case} must reject with one structured failure");
        };
        assert_eq!(
            failure.kind,
            TargetLoweringFailureKind::InvalidCoreIdentity,
            "{case}"
        );
    }
}

#[test]
fn colliding_target_obstruction_mappings_reject_without_artifact() {
    let mut core = effectful_core();
    let intent = core.intents.get_mut("t").expect("intent t");
    let mut alternate_arm = match &intent.body.nodes[0] {
        CoreNode::Effect {
            obstruction_map, ..
        } => obstruction_map
            .get("rejected")
            .expect("fixture declares rejected obstruction")
            .clone(),
        _ => panic!("fixture begins with an effect node"),
    };
    alternate_arm.binder = LocalRef {
        id: "obstruction.1".to_owned(),
        alpha_name: "$obstruction1".to_owned(),
        ty: "target.replace.alternate".to_owned(),
    };
    intent.body.locals.push(alternate_arm.binder.clone());
    let CoreNode::Effect {
        obstruction_map, ..
    } = &mut intent.body.nodes[0]
    else {
        panic!("fixture begins with an effect node");
    };
    obstruction_map.insert("alternate".to_owned(), alternate_arm);
    let mut facts = echo_facts();
    facts.effect_lowerings[0]
        .failure_mappings
        .insert("alternate".to_owned(), "rejected".to_owned());

    let report = lower_to_target_ir(&core, &facts);

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::AmbiguousObstructionMapping]
    );
    assert_eq!(report.failures[0].detail, "rejected");
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
fn pure_core_bindings_lower_as_generic_target_program() {
    let core = pure_core();
    let mut facts = echo_facts();
    facts
        .operation_profiles
        .push("continuum.profile.read-only/v1".to_owned());
    let report = lower_to_target_ir(&core, &facts);

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    let artifact = report.artifact.as_ref().expect("pure Target IR");
    let intent = artifact.intents.get("sayHello").expect("pure intent");
    assert_eq!(intent.pure_bindings.len(), 1);
    assert_eq!(intent.pure_bindings[0].id, "sayHello.binding.0");
    assert!(matches!(
        intent.pure_bindings[0].value,
        CoreExpr::Call { ref callee, .. } if callee == "core.string.concat"
    ));
    assert!(
        report.result_projections.contains_key("sayHello"),
        "projection failures: {:?}",
        report.result_projection_failures
    );
}

#[test]
fn unsupported_core_nodes_reject_without_artifact() {
    let mut core = pure_core();
    core.intents
        .get_mut("sayHello")
        .expect("pure intent")
        .body
        .nodes = vec![CoreNode::For {
        binder: LocalRef {
            id: "local.1".to_owned(),
            alpha_name: "$local1".to_owned(),
            ty: "U64".to_owned(),
        },
        iter: CoreExpr::Const(CoreValue::Null),
        bound: CoreBound::Literal(1),
        body: CoreBlock {
            locals: Vec::new(),
            nodes: Vec::new(),
            result: CoreExpr::Const(CoreValue::Null),
        },
    }];

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::UnsupportedCoreNode]
    );
}

#[test]
fn malformed_pure_binding_graphs_reject_before_target_artifact() {
    let cases = [
        (
            "dangling local",
            LocalRef {
                id: "local.999".to_owned(),
                alpha_name: "$local999".to_owned(),
                ty: "String<max=512,canonical=raw-utf8>".to_owned(),
            },
        ),
        (
            "self reference",
            LocalRef {
                id: "local.0".to_owned(),
                alpha_name: "$local0".to_owned(),
                ty: "String<max=512,canonical=raw-utf8>".to_owned(),
            },
        ),
        (
            "conflicting local type",
            LocalRef {
                id: "arg.0".to_owned(),
                alpha_name: "$arg0".to_owned(),
                ty: "Bytes<max=32>".to_owned(),
            },
        ),
    ];

    for (case, reference) in cases {
        let mut core = pure_core();
        let intent = core.intents.get_mut("sayHello").expect("pure intent");
        let CoreNode::Let { value, .. } = &mut intent.body.nodes[0] else {
            panic!("pure fixture starts with a let");
        };
        *value = CoreExpr::Local { reference };

        let report = lower_to_target_ir(&core, &pure_target_facts());

        assert_eq!(report.status, TargetLoweringStatus::Unsupported, "{case}");
        assert!(report.artifact.is_none(), "{case}");
        assert_eq!(
            failure_kinds(&report),
            vec![TargetLoweringFailureKind::InvalidCoreIdentity],
            "{case}"
        );
    }

    let mut duplicate = pure_core();
    let intent = duplicate.intents.get_mut("sayHello").expect("pure intent");
    intent.body.nodes.push(intent.body.nodes[0].clone());
    let report = lower_to_target_ir(&duplicate, &pure_target_facts());
    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::InvalidCoreIdentity]
    );

    let mut duplicate_local = pure_core();
    let intent = duplicate_local
        .intents
        .get_mut("sayHello")
        .expect("pure intent");
    intent.body.locals.push(intent.body.locals[1].clone());
    let report = lower_to_target_ir(&duplicate_local, &pure_target_facts());
    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::InvalidCoreIdentity]
    );
}

#[test]
fn dangling_pure_result_rejects_before_target_artifact() {
    let mut core = pure_core();
    core.intents
        .get_mut("sayHello")
        .expect("pure intent")
        .body
        .result = CoreExpr::Local {
        reference: LocalRef {
            id: "local.999".to_owned(),
            alpha_name: "$local999".to_owned(),
            ty: "examples.hello@1.HelloReading".to_owned(),
        },
    };

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    let [failure] = report.failures.as_slice() else {
        panic!("a dangling result rejects with exactly one failure");
    };
    assert_eq!(failure.kind, TargetLoweringFailureKind::InvalidCoreIdentity);
    assert_eq!(failure.intent.as_deref(), Some("sayHello"));
    assert_eq!(failure.node_index, None);
}

#[test]
fn type_incompatible_pure_binding_rejects_before_target_artifact() {
    let mut core = pure_core();
    let intent = core.intents.get_mut("sayHello").expect("pure intent");
    let CoreNode::Let { value, .. } = &mut intent.body.nodes[0] else {
        panic!("pure fixture starts with a let");
    };
    *value = CoreExpr::Const(CoreValue::Bool(true));

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    let [failure] = report.failures.as_slice() else {
        panic!("an incompatible binding rejects with exactly one failure");
    };
    assert_eq!(failure.kind, TargetLoweringFailureKind::InvalidCoreIdentity);
    assert_eq!(failure.intent.as_deref(), Some("sayHello"));
    assert_eq!(failure.node_index, Some(0));
}

#[test]
fn narrow_core_integer_widths_lower_as_builtin_types() {
    for (index, (width, value)) in [
        ("I8", "-128"),
        ("I16", "-32768"),
        ("U8", "255"),
        ("U16", "65535"),
    ]
    .into_iter()
    .enumerate()
    {
        let mut core = pure_core();
        assert!(
            !core.types.contains_key(width),
            "built-in widths must not require redundant Core type entries"
        );
        let intent = core.intents.get_mut("sayHello").expect("pure intent");
        let binding = LocalRef {
            id: format!("local.narrow.{index}"),
            alpha_name: format!("$narrow{index}"),
            ty: width.to_owned(),
        };
        intent.body.locals.push(binding.clone());
        intent.body.nodes.insert(
            0,
            CoreNode::Let {
                binding: binding.clone(),
                value: CoreExpr::Const(CoreValue::Int {
                    width: width.to_owned(),
                    value: value.to_owned(),
                }),
            },
        );

        let report = lower_to_target_ir(&core, &pure_target_facts());

        assert_eq!(
            report.status,
            TargetLoweringStatus::Lowered,
            "{width} should lower without a redundant type entry: {:?}",
            report.failures
        );
        let artifact = report.artifact.expect("narrow integer Target IR");
        let lowered = artifact
            .intents
            .get("sayHello")
            .expect("pure intent")
            .pure_bindings
            .iter()
            .find(|candidate| candidate.binding.id == binding.id)
            .expect("narrow integer binding");
        assert_eq!(lowered.binding.ty, width);
        assert_eq!(
            lowered.value,
            CoreExpr::Const(CoreValue::Int {
                width: width.to_owned(),
                value: value.to_owned(),
            })
        );
    }
}

#[test]
fn ranged_core_byte_coordinates_lower_as_builtin_types() {
    let mut core = pure_core();
    core.types.insert(
        "HelloReading.message".to_owned(),
        CoreType::Bytes {
            min: Some(2),
            max: 4,
        },
    );
    let intent = core.intents.get_mut("sayHello").expect("pure intent");
    let CoreNode::Let { binding, value } = &mut intent.body.nodes[0] else {
        panic!("pure fixture starts with a let");
    };
    let binding_id = binding.id.clone();
    binding.ty = "Bytes<min=2,max=4>".to_owned();
    *value = CoreExpr::Const(CoreValue::Bytes(vec![0x01, 0x02, 0x03]));
    let local = intent
        .body
        .locals
        .iter_mut()
        .find(|local| local.id == binding_id)
        .expect("pure binding local");
    local.ty.clone_from(&binding.ty);
    let CoreExpr::Record { fields } = &mut intent.body.result else {
        panic!("pure fixture returns a record");
    };
    let Some(CoreExpr::Local { reference }) = fields.get_mut("message") else {
        panic!("pure fixture returns the binding as message");
    };
    reference.ty.clone_from(&binding.ty);

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    assert!(report.artifact.is_some());

    let CoreNode::Let { value, .. } = &mut core
        .intents
        .get_mut("sayHello")
        .expect("pure intent")
        .body
        .nodes[0]
    else {
        panic!("pure fixture starts with a let");
    };
    *value = CoreExpr::Const(CoreValue::Bytes(vec![0x01]));

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::InvalidCoreIdentity]
    );
}

#[test]
fn pure_conditional_predicates_require_compatible_bounded_operands() {
    let cases = [
        (
            "incompatible operand types",
            CoreExpr::Const(CoreValue::Bool(true)),
            CoreExpr::Const(CoreValue::Int {
                width: "U64".to_owned(),
                value: "1".to_owned(),
            }),
        ),
        (
            "constant outside its declared width",
            CoreExpr::Const(CoreValue::Int {
                width: "U8".to_owned(),
                value: "0".to_owned(),
            }),
            CoreExpr::Const(CoreValue::Int {
                width: "U8".to_owned(),
                value: "256".to_owned(),
            }),
        ),
    ];

    for (case, left, right) in cases {
        let mut core = pure_core();
        let intent = core.intents.get_mut("sayHello").expect("pure intent");
        let CoreNode::Let { value, .. } = &mut intent.body.nodes[0] else {
            panic!("pure fixture starts with a let");
        };
        let valid_branch = value.clone();
        *value = CoreExpr::If {
            predicate: Box::new(CorePredicate::Compare {
                op: CompareOp::Eq,
                left,
                right,
            }),
            then_value: Box::new(valid_branch.clone()),
            else_value: Box::new(valid_branch),
        };

        let report = lower_to_target_ir(&core, &pure_target_facts());

        assert_eq!(report.status, TargetLoweringStatus::Unsupported, "{case}");
        assert!(report.artifact.is_none(), "{case}");
        let [failure] = report.failures.as_slice() else {
            panic!("{case} must reject with exactly one structured failure");
        };
        assert_eq!(failure.kind, TargetLoweringFailureKind::InvalidCoreIdentity);
        assert_eq!(failure.intent.as_deref(), Some("sayHello"));
        assert_eq!(failure.node_index, Some(0));
    }
}

#[test]
fn pure_conditional_comparison_accepts_supported_call_operands() {
    let mut core = pure_core();
    let intent = core.intents.get_mut("sayHello").expect("pure intent");
    let CoreNode::Let { value, .. } = &mut intent.body.nodes[0] else {
        panic!("pure fixture starts with a let");
    };
    let valid_call = value.clone();
    *value = CoreExpr::If {
        predicate: Box::new(CorePredicate::Compare {
            op: CompareOp::Eq,
            left: valid_call.clone(),
            right: valid_call.clone(),
        }),
        then_value: Box::new(valid_call.clone()),
        else_value: Box::new(valid_call),
    };

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    assert!(report.artifact.is_some());
}

#[test]
fn compiler_produced_conditional_comparisons_infer_compatible_branch_types() {
    let source = "package a.b@1;\n\
        type Input = { value: String<max=32>, };\n\
        type Output = { value: String<max=32>, };\n\
        intent t(input: Input) returns Output\n\
          profile hello.readOnly\n\
          basis none\n\
          budget <= hello.tinyBudget {\n\
          let value = if (if true then \"a\" else \"bb\")\n\
            == (if false then \"ccc\" else \"dddd\")\n\
            then input.value else input.value;\n\
          return { value };\n\
        }";
    let module = edict_syntax::parse_module(source).expect("conditional comparison parses");
    let core = compile_to_core(&module, &pure_context())
        .expect("compatible conditional comparison compiles to Core");
    let CoreNode::Let {
        value:
            CoreExpr::If {
                predicate,
                then_value: _,
                else_value: _,
            },
        ..
    } = &core.intents.get("t").expect("intent t").body.nodes[0]
    else {
        panic!("compiler preserves the outer conditional expression");
    };
    let CorePredicate::Compare { left, right, .. } = predicate.as_ref() else {
        panic!("compiler preserves the comparison predicate");
    };
    assert!(matches!(left, CoreExpr::If { .. }));
    assert!(matches!(right, CoreExpr::If { .. }));

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    assert!(report.artifact.is_some());
}

#[test]
fn compiler_produced_conditional_record_comparisons_preserve_structural_compatibility() {
    let source = "package a.b@1;\n\
        type Input = { value: String<max=32>, };\n\
        type Output = { value: String<max=32>, };\n\
        intent t(input: Input) returns Output\n\
          profile hello.readOnly\n\
          basis none\n\
          budget <= hello.tinyBudget {\n\
          let value = if (if true then { value: \"a\" } else { value: \"bb\" })\n\
            == { value: \"ccc\" }\n\
            then input.value else input.value;\n\
          return { value };\n\
        }";
    let module = edict_syntax::parse_module(source).expect("record comparison parses");
    let core = compile_to_core(&module, &pure_context())
        .expect("compatible anonymous record comparison compiles to Core");

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    assert!(report.artifact.is_some());

    let mut incompatible = core;
    let CoreNode::Let {
        value: CoreExpr::If { predicate, .. },
        ..
    } = &mut incompatible
        .intents
        .get_mut("t")
        .expect("intent t")
        .body
        .nodes[0]
    else {
        panic!("compiler preserves the outer conditional expression");
    };
    let CorePredicate::Compare { right, .. } = predicate.as_mut() else {
        panic!("compiler preserves the record comparison predicate");
    };
    let CoreExpr::Record { fields } = right else {
        panic!("comparison keeps the right anonymous record");
    };
    let value = fields.remove("value").expect("record value field");
    fields.insert("other".to_owned(), value);

    let report = lower_to_target_ir(&incompatible, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    assert_eq!(
        failure_kinds(&report),
        vec![TargetLoweringFailureKind::InvalidCoreIdentity]
    );
}

#[test]
fn conditional_byte_interval_comparisons_use_component_wise_join() {
    let mut core = pure_core();
    let intent = core.intents.get_mut("sayHello").expect("pure intent");
    let CoreNode::Let { value, .. } = &mut intent.body.nodes[0] else {
        panic!("pure fixture starts with a let");
    };
    let valid_branch = value.clone();
    let byte_interval = |first: usize, second: usize| CoreExpr::If {
        predicate: Box::new(CorePredicate::True),
        then_value: Box::new(CoreExpr::Const(CoreValue::Bytes(vec![0x11; first]))),
        else_value: Box::new(CoreExpr::Const(CoreValue::Bytes(vec![0x22; second]))),
    };
    *value = CoreExpr::If {
        predicate: Box::new(CorePredicate::Compare {
            op: CompareOp::Eq,
            left: byte_interval(1, 3),
            right: byte_interval(2, 4),
        }),
        then_value: Box::new(valid_branch.clone()),
        else_value: Box::new(valid_branch),
    };

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    assert!(report.artifact.is_some());
}

#[test]
fn target_lowering_accepts_compiler_type_depth_boundary() {
    const TYPE_DEPTH: usize = 128;

    let mut core = pure_core();
    let root_type = format!("{}.Deep0", core.coordinate);
    for index in 0..TYPE_DEPTH {
        let coordinate = format!("{}.Deep{index}", core.coordinate);
        let next = if index + 1 == TYPE_DEPTH {
            "U64".to_owned()
        } else {
            format!("{}.Deep{}", core.coordinate, index + 1)
        };
        core.types.insert(
            coordinate,
            CoreType::Record {
                fields: BTreeMap::from([("next".to_owned(), next)]),
            },
        );
    }

    let intent = core.intents.get_mut("sayHello").expect("pure intent");
    intent.input.clone_from(&root_type);
    intent.output.clone_from(&root_type);
    intent.input_constraints.clear();
    let input = intent
        .body
        .locals
        .iter()
        .find(|local| local.id == "arg.0")
        .expect("compiler-owned input local")
        .clone();
    let input = LocalRef {
        ty: root_type.clone(),
        ..input
    };
    let binding = LocalRef {
        id: "local.deep".to_owned(),
        alpha_name: "$deep".to_owned(),
        ty: root_type,
    };
    intent.body.locals = vec![input.clone(), binding.clone()];
    intent.body.nodes = vec![CoreNode::Let {
        binding: binding.clone(),
        value: CoreExpr::Local { reference: input },
    }];
    intent.body.result = CoreExpr::Local { reference: binding };

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(report.failures.is_empty());
    assert!(report.artifact.is_some());
}

#[test]
fn nested_string_conditional_predicates_require_compatible_bounded_operands() {
    let cases = [
        (
            "incompatible operand types",
            CoreExpr::Const(CoreValue::Bool(true)),
            CoreExpr::Const(CoreValue::Int {
                width: "U64".to_owned(),
                value: "1".to_owned(),
            }),
        ),
        (
            "constant outside its declared width",
            CoreExpr::Const(CoreValue::Int {
                width: "U8".to_owned(),
                value: "0".to_owned(),
            }),
            CoreExpr::Const(CoreValue::Int {
                width: "U8".to_owned(),
                value: "256".to_owned(),
            }),
        ),
    ];

    for (case, left, right) in cases {
        let mut core = pure_core();
        let intent = core.intents.get_mut("sayHello").expect("pure intent");
        let CoreNode::Let {
            value: CoreExpr::Call { callee, args, .. },
            ..
        } = &mut intent.body.nodes[0]
        else {
            panic!("pure fixture starts with a string concatenation binding");
        };
        assert_eq!(callee, "core.string.concat");
        args[0] = CoreExpr::If {
            predicate: Box::new(CorePredicate::Compare {
                op: CompareOp::Eq,
                left,
                right,
            }),
            then_value: Box::new(CoreExpr::Const(CoreValue::String("hello".to_owned()))),
            else_value: Box::new(CoreExpr::Const(CoreValue::String("hi".to_owned()))),
        };

        let report = lower_to_target_ir(&core, &pure_target_facts());

        assert_eq!(report.status, TargetLoweringStatus::Unsupported, "{case}");
        assert!(report.artifact.is_none(), "{case}");
        let [failure] = report.failures.as_slice() else {
            panic!("{case} must reject with exactly one structured failure");
        };
        assert_eq!(failure.kind, TargetLoweringFailureKind::InvalidCoreIdentity);
        assert_eq!(failure.intent.as_deref(), Some("sayHello"));
        assert_eq!(failure.node_index, Some(0));
    }
}

#[test]
fn non_raw_string_constant_rejects_before_target_artifact() {
    let mut core = pure_core();
    let intent = core.intents.get_mut("sayHello").expect("pure intent");
    let CoreNode::Let { binding, value } = &mut intent.body.nodes[0] else {
        panic!("pure fixture starts with a let");
    };
    binding.ty = "String<max=263,canonical=unicode-scalar-nfc>".to_owned();
    intent.body.locals[1].ty.clone_from(&binding.ty);
    *value = CoreExpr::Const(CoreValue::String("already normalized".to_owned()));

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Unsupported);
    assert!(report.artifact.is_none());
    let [failure] = report.failures.as_slice() else {
        panic!("a raw string constant under a non-raw contract rejects once");
    };
    assert_eq!(failure.kind, TargetLoweringFailureKind::InvalidCoreIdentity);
    assert_eq!(failure.intent.as_deref(), Some("sayHello"));
    assert_eq!(failure.node_index, Some(0));
}

#[test]
fn pure_program_without_imports_or_basis_still_binds_source_core_identity() {
    let mut core = pure_core();
    core.imports.clear();
    core.intents.get_mut("sayHello").expect("pure intent").basis = None;

    let report = lower_to_target_ir(&core, &pure_target_facts());

    assert_eq!(report.status, TargetLoweringStatus::Lowered);
    assert!(
        report
            .artifact
            .as_ref()
            .expect("pure Target IR")
            .semantic_closure
            .is_some(),
        "pure executable expressions require an exact source-Core closure"
    );
    assert!(report.result_projections.contains_key("sayHello"));
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
fn target_ir_step_order_changes_digest() {
    let baseline = effectful_artifact(CHAINED_EFFECT_RESULTS);
    let mut reordered = baseline.clone();
    reordered
        .intents
        .get_mut("t")
        .expect("intent t")
        .steps
        .reverse();

    assert_ne!(
        digest_target_ir_artifact(&baseline).expect("baseline Target IR digests"),
        digest_target_ir_artifact(&reordered).expect("reordered Target IR digests")
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
fn target_ir_obstruction_arm_value_mutation_moves_digest() {
    let baseline = effectful_artifact(EFFECTFUL_REPLACE);
    assert_target_ir_digest_changes(&baseline, "obstruction arm value", |artifact| {
        target_step_mut(artifact)
            .obstruction_arms
            .get_mut("rejected")
            .expect("rejected arm")
            .value = CoreExpr::Const(CoreValue::String("changed".to_owned()));
    });
}

#[test]
fn pure_binding_semantic_mutations_move_target_ir_identity() {
    let baseline = pure_artifact();
    assert_target_ir_digest_changes(&baseline, "pure binding id", |artifact| {
        pure_binding_mut(artifact).id = "sayHello.binding.changed".to_owned();
    });
    assert_target_ir_digest_changes(&baseline, "pure binding local", |artifact| {
        pure_binding_mut(artifact).binding.alpha_name = "$changed".to_owned();
    });
    assert_target_ir_digest_changes(&baseline, "pure helper identity", |artifact| {
        let CoreExpr::Call { callee, .. } = &mut pure_binding_mut(artifact).value else {
            panic!("pure fixture binding is a helper call");
        };
        *callee = "core.string.other".to_owned();
    });
    assert_target_ir_digest_changes(
        &baseline,
        "pure helper implementation closure",
        |artifact| {
            artifact
                .semantic_closure
                .as_mut()
                .expect("pure artifact is closed")
                .lawpacks[0]
                .digest = Some(format!("sha256:{}", "f".repeat(64)));
        },
    );
    assert_target_ir_digest_changes(&baseline, "pure conditional arm", |artifact| {
        pure_binding_mut(artifact).value = CoreExpr::If {
            predicate: Box::new(CorePredicate::True),
            then_value: Box::new(CoreExpr::Const(CoreValue::String("then".to_owned()))),
            else_value: Box::new(CoreExpr::Const(CoreValue::String("else".to_owned()))),
        };
    });
    assert_target_ir_digest_changes(&baseline, "pure result dependency", |artifact| {
        artifact
            .intents
            .get_mut("sayHello")
            .expect("pure intent")
            .result = CoreExpr::Const(CoreValue::Null);
    });

    let mut two_bindings = baseline;
    let first = pure_binding_mut(&mut two_bindings).clone();
    let mut second = first;
    second.id = "sayHello.binding.1".to_owned();
    second.binding.id = "local.1".to_owned();
    second.binding.alpha_name = "$local1".to_owned();
    two_bindings
        .intents
        .get_mut("sayHello")
        .expect("pure intent")
        .pure_bindings
        .push(second);
    assert_target_ir_digest_changes(&two_bindings, "pure binding order", |artifact| {
        artifact
            .intents
            .get_mut("sayHello")
            .expect("pure intent")
            .pure_bindings
            .swap(0, 1);
    });
}

#[test]
fn pure_binding_encoder_rejects_duplicate_identity_or_missing_closure() {
    let mut duplicate = pure_artifact();
    let binding = pure_binding_mut(&mut duplicate).clone();
    duplicate
        .intents
        .get_mut("sayHello")
        .expect("pure intent")
        .pure_bindings
        .push(binding);
    assert_eq!(
        encode_target_ir_artifact(&duplicate)
            .expect_err("duplicate pure binding id must reject")
            .kind(),
        CanonicalErrorKind::UnsupportedValue
    );

    let mut duplicate_local = pure_artifact();
    let mut binding = pure_binding_mut(&mut duplicate_local).clone();
    binding.id = "sayHello.binding.1".to_owned();
    duplicate_local
        .intents
        .get_mut("sayHello")
        .expect("pure intent")
        .pure_bindings
        .push(binding);
    assert_eq!(
        encode_target_ir_artifact(&duplicate_local)
            .expect_err("distinct target ids cannot share one compiler-local identity")
            .kind(),
        CanonicalErrorKind::UnsupportedValue
    );

    let mut unclosed = pure_artifact();
    unclosed.semantic_closure = None;
    assert_eq!(
        encode_target_ir_artifact(&unclosed)
            .expect_err("pure binding without semantic closure must reject")
            .kind(),
        CanonicalErrorKind::UnsupportedValue
    );
}

#[test]
fn target_ir_encoder_rejects_local_identity_shared_across_producer_classes() {
    let baseline = pure_artifact();
    let shared_binding = baseline
        .intents
        .get("sayHello")
        .expect("pure intent")
        .pure_bindings[0]
        .binding
        .clone();

    let mut step_collision = baseline.clone();
    step_collision
        .intents
        .get_mut("sayHello")
        .expect("pure intent")
        .steps
        .push(TargetIrStep {
            id: "sayHello.step.0".to_owned(),
            binding: shared_binding.clone(),
            effect: "example.effect".to_owned(),
            target_intrinsic: "echo.dpo@1.example".to_owned(),
            input: CoreExpr::Const(CoreValue::Null),
            obstruction_failures: Vec::new(),
            obstruction_arms: BTreeMap::new(),
        });
    assert_eq!(
        encode_target_ir_artifact(&step_collision)
            .expect_err("a target step cannot reproduce a pure-binding local")
            .kind(),
        CanonicalErrorKind::UnsupportedValue
    );

    let operation = ResourceRef {
        coordinate: "example.observe@1".to_owned(),
        digest: Some(digest_text('4')),
    };
    let mut request_collision = baseline;
    request_collision
        .semantic_closure
        .as_mut()
        .expect("pure artifact closure")
        .capabilities
        .push(operation.clone());
    request_collision
        .intents
        .get_mut("sayHello")
        .expect("pure intent")
        .external_action_requests
        .push(TargetIrExternalActionRequest {
            id: "sayHello.request.0".to_owned(),
            binding: shared_binding,
            operation,
            input_type: "U64".to_owned(),
            settlement_type: "U64".to_owned(),
            input_schema: ResourceRef {
                coordinate: "example.input-schema@1".to_owned(),
                digest: Some(digest_text('5')),
            },
            settlement_schema: ResourceRef {
                coordinate: "example.settlement-schema@1".to_owned(),
                digest: Some(digest_text('6')),
            },
            input: CoreExpr::Const(CoreValue::Int {
                width: "U64".to_owned(),
                value: "0".to_owned(),
            }),
            authority_scope: CoreExpr::Const(CoreValue::Null),
            basis: CoreExpr::Const(CoreValue::Null),
            budget: CoreExternalActionBudget {
                max_settlement_bytes: CoreExpr::Const(CoreValue::Int {
                    width: "U64".to_owned(),
                    value: "64".to_owned(),
                }),
                max_attempts: CoreExpr::Const(CoreValue::Int {
                    width: "U64".to_owned(),
                    value: "1".to_owned(),
                }),
            },
            reconciliation_law: ResourceRef {
                coordinate: "example.reconciliation@1".to_owned(),
                digest: Some(digest_text('7')),
            },
        });
    assert_eq!(
        encode_target_ir_artifact(&request_collision)
            .expect_err("an external request cannot reproduce a pure-binding local")
            .kind(),
        CanonicalErrorKind::UnsupportedValue
    );
}

#[test]
fn target_ir_encoder_reserves_application_input_local_identity() {
    let mut artifact = pure_artifact();
    pure_binding_mut(&mut artifact).binding.id = "arg.0".to_owned();

    assert_eq!(
        encode_target_ir_artifact(&artifact)
            .expect_err("a pure binding cannot reproduce the application input local")
            .kind(),
        CanonicalErrorKind::UnsupportedValue
    );
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

fn pure_binding_mut(artifact: &mut TargetIrArtifact) -> &mut edict_syntax::TargetIrPureBinding {
    artifact
        .intents
        .get_mut("sayHello")
        .expect("pure intent")
        .pure_bindings
        .get_mut(0)
        .expect("pure binding 0")
}

fn requirement_mut(artifact: &mut TargetIrArtifact) -> &mut edict_syntax::TargetIrRequirement {
    artifact
        .intents
        .get_mut("t")
        .expect("intent t")
        .requirements
        .get_mut(0)
        .expect("requirement 0")
}

fn replace_required(source: &str, from: &str, to: &str) -> String {
    assert!(
        source.contains(from),
        "test fixture must contain replacement fragment {from:?}"
    );
    source.replace(from, to)
}

fn digest_text(hex: char) -> String {
    format!("sha256:{}", hex.to_string().repeat(64))
}
