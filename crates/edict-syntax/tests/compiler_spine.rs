//! Compiler-spine tests for the first executable source-to-Core path.
//!
//! These tests assert public stage boundaries and structured values. They do
//! not inspect stdout, stderr, diagnostic prose, canonical bytes, or hashes.

use std::fs;
use std::path::{Path, PathBuf};

use edict_syntax::{
    compile_to_core, load_compiler_context_from_authority_fact_files, lower_core, parse_module,
    resolve_module, type_check, CompilerContext, CompilerErrorKind, CompilerStage, CoreBudget,
    CoreExpr, CoreNode, CorePredicate, CoreType, WriteClass,
};

const BOUNDED_HELLO: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");
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
const EFFECTFUL_BRANCH_YIELD: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      let id = if true {\n\
        target.replace(input.id) else { rejected(reason) => domain.WriteRejected };\n\
        yield input.id;\n\
      } else {\n\
        yield input.id;\n\
      };\n\
      return { id };\n\
    }";
const DUPLICATE_OBSTRUCTION_FAILURE: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      let receipt: Receipt = target.replace(input.id)\n\
        else {\n\
          rejected(reason) => domain.WriteRejected,\n\
          rejected(other) => domain.WriteRejectedAgain,\n\
        };\n\
      return { id: input.id };\n\
    }";

fn hello_context() -> CompilerContext {
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

#[test]
fn bounded_hello_compiles_to_initial_core() {
    let module = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let core = compile_to_core(&module, &hello_context()).expect("fixture compiles to Core");

    assert_eq!(core.api_version, "edict.core/v1");
    assert_eq!(core.coordinate, "examples.hello@1");
    assert_eq!(core.imports.len(), 1);
    assert_eq!(core.imports[0].resource.coordinate, "hello.optics@1");
    assert_eq!(
        core.imports[0].resource.digest.as_deref(),
        Some("sha256:0000000000000000000000000000000000000000000000000000000000000000")
    );

    let input = core.types.get("HelloInput").expect("HelloInput type");
    assert_eq!(
        input,
        &CoreType::Record {
            fields: [(
                "name".to_owned(),
                "examples.hello@1.HelloInput.name".to_owned()
            )]
            .into()
        }
    );

    let name_ty = core
        .types
        .get("HelloInput.name")
        .expect("lowered field type");
    assert_eq!(
        name_ty,
        &CoreType::String {
            max: 256,
            canonical: "raw-utf8".to_owned(),
        }
    );

    let intent = core.intents.get("sayHello").expect("sayHello intent");
    assert_eq!(
        intent.required_operation_profile,
        "continuum.profile.read-only/v1"
    );
    assert_eq!(intent.core_evaluation_budget.max_steps, 64);
    assert_eq!(intent.input_constraints.len(), 1);

    assert!(matches!(
        &intent.input_constraints[0].predicate,
        CorePredicate::Compare { .. }
    ));
    assert_eq!(intent.body.locals.len(), 2);
    assert_eq!(intent.body.locals[0].id, "arg.0");
    assert_eq!(intent.body.locals[0].alpha_name, "$arg0");
    assert_eq!(intent.body.locals[1].id, "local.0");
    assert_eq!(intent.body.locals[1].alpha_name, "$local0");
    assert_eq!(intent.body.nodes.len(), 1);
    assert!(matches!(intent.body.nodes[0], CoreNode::Let { .. }));
    assert!(matches!(intent.body.result, CoreExpr::Record { .. }));
}

#[test]
fn compiler_spine_exposes_distinct_stage_boundaries() {
    let module = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let resolved = resolve_module(&module, &hello_context()).expect("resolve stage");
    assert_eq!(resolved.coordinate, "examples.hello@1");
    assert_eq!(
        resolved.intents[0].profile,
        "continuum.profile.read-only/v1"
    );

    let typed = type_check(&resolved).expect("type-check stage");
    assert_eq!(
        typed.intents[0].input_binding.ty,
        "examples.hello@1.HelloInput"
    );

    let core = lower_core(&typed).expect("lower Core stage");
    assert!(core.intents.contains_key("sayHello"));
}

#[test]
fn missing_context_facts_reject_in_resolve_stage() {
    let module = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let errors =
        resolve_module(&module, &CompilerContext::new()).expect_err("missing context facts reject");

    assert!(errors.iter().all(|err| err.stage == CompilerStage::Resolve));
    assert_eq!(
        errors
            .iter()
            .map(|err| err.kind)
            .collect::<Vec<CompilerErrorKind>>(),
        vec![
            CompilerErrorKind::MissingContextFact,
            CompilerErrorKind::MissingContextFact,
        ]
    );
}

#[test]
fn unresolved_local_types_reject_in_type_check_stage() {
    let module = parse_module(
        "package a.b@1;\n\
         intent t(input: MissingInput) returns MissingOutput\n\
           profile p.read\n\
           basis none\n\
           budget <= p.tiny {\n\
           return { input };\n\
         }",
    )
    .expect("source parses");
    let context = CompilerContext::new()
        .with_operation_profile("p.read", "continuum.profile.read-only/v1")
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 1,
                max_allocated_bytes: 1,
                max_output_bytes: 1,
            },
        );

    let resolved = resolve_module(&module, &context).expect("resolve accepts source coordinates");
    let errors = type_check(&resolved).expect_err("unknown types reject in type-check");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert!(errors
        .iter()
        .any(|err| err.kind == CompilerErrorKind::UnresolvedType));
}

#[test]
fn unresolved_record_field_types_reject_in_type_check_stage() {
    let module = parse_module(
        "package a.b@1;\n\
         type Box = { value: MissingValue, };\n\
         intent t(input: Box) returns Box\n\
           profile p.read\n\
           basis none\n\
           budget <= p.tiny {\n\
           return { input };\n\
         }",
    )
    .expect("source parses");
    let context = CompilerContext::new()
        .with_operation_profile("p.read", "continuum.profile.read-only/v1")
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 1,
                max_allocated_bytes: 1,
                max_output_bytes: 1,
            },
        );

    let resolved = resolve_module(&module, &context).expect("resolve accepts source coordinates");
    let errors = type_check(&resolved).expect_err("unknown field type rejects in type-check");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert!(errors
        .iter()
        .any(|err| err.kind == CompilerErrorKind::UnresolvedType));
}

#[test]
fn record_return_shape_mismatch_rejects_in_type_check_stage() {
    let source = BOUNDED_HELLO.replace("return { message };", "return { wrong: message };");
    let module = parse_module(&source).expect("source parses");
    let resolved = resolve_module(&module, &hello_context()).expect("resolve stage");
    let errors = type_check(&resolved).expect_err("return shape rejects");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert!(errors
        .iter()
        .any(|err| err.kind == CompilerErrorKind::TypeMismatch));
}

#[test]
fn missing_return_rejects_in_type_check_stage() {
    let source = BOUNDED_HELLO.replace("  return { message };\n", "");
    let module = parse_module(&source).expect("source parses");
    let resolved = resolve_module(&module, &hello_context()).expect("resolve stage");
    let errors = type_check(&resolved).expect_err("missing return rejects");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert!(errors
        .iter()
        .any(|err| err.kind == CompilerErrorKind::TypeMismatch));
}

#[test]
fn read_only_profile_rejects_write_effect_body() {
    let module = parse_module(
        "package a.b@1;\n\
         type Input = { id: String<max=16>, };\n\
         type Output = { id: String<max=16>, };\n\
         intent t(input: Input) returns Output\n\
           profile p.readOnly\n\
           basis none\n\
           budget <= p.tiny {\n\
           target.replace(input.id) else domain.WriteRejected;\n\
           return { id: input.id };\n\
         }",
    )
    .expect("source parses");
    let context = CompilerContext::new()
        .with_operation_profile("p.readOnly", "continuum.profile.read-only/v1")
        .with_operation_profile_write_classes("p.readOnly", [WriteClass::Read])
        .with_effect_write_class("target.replace", WriteClass::Replace)
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 1,
                max_allocated_bytes: 1,
                max_output_bytes: 1,
            },
        );

    let errors = compile_to_core(&module, &context)
        .expect_err("write effect rejects under read-only profile");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert_eq!(
        errors
            .iter()
            .map(|err| err.kind)
            .collect::<Vec<CompilerErrorKind>>(),
        vec![CompilerErrorKind::ProfileEffectMismatch]
    );
}

#[test]
fn read_only_profile_rejects_write_effect_let_without_else() {
    let module = parse_module(
        "package a.b@1;\n\
         type Input = { id: String<max=16>, };\n\
         type Output = { id: String<max=16>, };\n\
         intent t(input: Input) returns Output\n\
           profile p.readOnly\n\
           basis none\n\
           budget <= p.tiny {\n\
           let _receipt = target.replace(input.id);\n\
           return { id: input.id };\n\
         }",
    )
    .expect("source parses");
    let context = CompilerContext::new()
        .with_operation_profile("p.readOnly", "continuum.profile.read-only/v1")
        .with_operation_profile_write_classes("p.readOnly", [WriteClass::Read])
        .with_effect_write_class("target.replace", WriteClass::Replace)
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 1,
                max_allocated_bytes: 1,
                max_output_bytes: 1,
            },
        );

    let errors = compile_to_core(&module, &context)
        .expect_err("write effect rejects under read-only profile");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert_eq!(
        errors
            .iter()
            .map(|err| err.kind)
            .collect::<Vec<CompilerErrorKind>>(),
        vec![CompilerErrorKind::ProfileEffectMismatch]
    );
}

#[test]
fn effectful_write_intent_lowers_to_typed_core_from_file_backed_facts() {
    let dir = temp_case_dir("effectful-write");
    let target = write_json(
        &dir,
        "target-profile-facts.json",
        effectful_target_profile_facts(),
    );
    let lawpack = write_json(&dir, "lawpack-facts.json", effectful_lawpack_facts());
    let context =
        load_compiler_context_from_authority_fact_files([target.as_path(), lawpack.as_path()])
            .expect("authority facts load");
    let module = parse_module(EFFECTFUL_REPLACE).expect("effectful source parses");
    let core = compile_to_core(&module, &context).expect("effectful source compiles to Core");
    let intent = core.intents.get("t").expect("compiled effectful intent");

    assert_eq!(
        intent.required_operation_profile,
        "continuum.profile.write/v1"
    );
    assert_eq!(intent.body.nodes.len(), 1);

    let CoreNode::Effect {
        binding,
        effect,
        input,
        obstruction_map,
    } = &intent.body.nodes[0]
    else {
        panic!("effectful source lowers to a semantic effect node");
    };

    assert_eq!(binding.id, "local.0");
    assert_eq!(binding.alpha_name, "$local0");
    assert_eq!(binding.ty, "a.b@1.Receipt");
    assert_eq!(effect, "target.replace");

    let CoreExpr::Field { base, field } = input else {
        panic!("effect input preserves the source argument expression");
    };
    assert_eq!(field, "id");
    assert!(matches!(base.as_ref(), CoreExpr::Local { reference } if reference.id == "arg.0"));

    let arm = obstruction_map
        .get("rejected")
        .expect("failure arm is keyed by low-level failure coordinate");
    assert_eq!(arm.binder.id, "obstruction.0");
    assert_eq!(arm.binder.alpha_name, "$obstruction0");
    assert_eq!(arm.binder.ty, "target.replace.rejected");
    assert!(matches!(
        &arm.value,
        CoreExpr::Call { callee, args, .. } if callee == "domain.WriteRejected" && args.is_empty()
    ));
}

#[test]
fn unsupported_effectful_branch_yield_rejects_before_core_lowering() {
    let dir = temp_case_dir("effectful-branch-yield");
    let target = write_json(
        &dir,
        "target-profile-facts.json",
        effectful_target_profile_facts(),
    );
    let lawpack = write_json(&dir, "lawpack-facts.json", effectful_lawpack_facts());
    let context =
        load_compiler_context_from_authority_fact_files([target.as_path(), lawpack.as_path()])
            .expect("authority facts load");
    let module = parse_module(EFFECTFUL_BRANCH_YIELD).expect("unsupported effectful source parses");

    let errors =
        compile_to_core(&module, &context).expect_err("unsupported effectful shape rejects");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert!(errors
        .iter()
        .any(|err| err.kind == CompilerErrorKind::UnsupportedSourceShape));
    assert!(!errors
        .iter()
        .any(|err| err.kind == CompilerErrorKind::ProfileEffectMismatch));
}

#[test]
fn duplicate_obstruction_failures_reject_before_core_lowering() {
    let dir = temp_case_dir("duplicate-obstruction-failure");
    let target = write_json(
        &dir,
        "target-profile-facts.json",
        effectful_target_profile_facts(),
    );
    let lawpack = write_json(&dir, "lawpack-facts.json", effectful_lawpack_facts());
    let context =
        load_compiler_context_from_authority_fact_files([target.as_path(), lawpack.as_path()])
            .expect("authority facts load");
    let module =
        parse_module(DUPLICATE_OBSTRUCTION_FAILURE).expect("duplicate obstruction source parses");

    let errors =
        compile_to_core(&module, &context).expect_err("duplicate obstruction failure keys reject");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert_eq!(
        errors
            .iter()
            .map(|err| err.kind)
            .collect::<Vec<CompilerErrorKind>>(),
        vec![CompilerErrorKind::DuplicateObstructionFailure]
    );
}

#[test]
fn chained_effect_calls_reject_before_core_lowering() {
    let source = EFFECTFUL_REPLACE.replace(
        "target.replace(input.id)",
        "target.replace(input.id)(input.id)",
    );
    let dir = temp_case_dir("chained-effect-call");
    let target = write_json(
        &dir,
        "target-profile-facts.json",
        effectful_target_profile_facts(),
    );
    let lawpack = write_json(&dir, "lawpack-facts.json", effectful_lawpack_facts());
    let context =
        load_compiler_context_from_authority_fact_files([target.as_path(), lawpack.as_path()])
            .expect("authority facts load");
    let module = parse_module(&source).expect("chained effect-call source parses");

    let errors = compile_to_core(&module, &context).expect_err("chained effect call rejects");

    assert!(errors
        .iter()
        .all(|err| err.stage == CompilerStage::TypeCheck));
    assert!(errors
        .iter()
        .any(|err| err.kind == CompilerErrorKind::UnsupportedSourceShape));
}

#[test]
fn initial_core_lowering_makes_no_canonical_or_target_claim() {
    let module = parse_module(BOUNDED_HELLO).expect("fixture parses");
    let core = compile_to_core(&module, &hello_context()).expect("fixture compiles to Core");

    assert!(core.required_core_capabilities.is_empty());
    assert!(core
        .imports
        .iter()
        .all(|import| import.kind.as_str() != "target"));
    assert_eq!(core.api_version, "edict.core/v1");
}

fn temp_case_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "edict-compiler-spine-{name}-{}",
        std::process::id()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("remove stale temp compiler-spine directory");
    }
    fs::create_dir_all(&dir).expect("create temp compiler-spine directory");
    dir
}

fn write_json(dir: &Path, name: &str, contents: impl AsRef<str>) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, contents.as_ref()).expect("write authority-facts JSON");
    path
}

fn effectful_target_profile_facts() -> &'static str {
    r#"{
      "apiVersion": "edict.authority-facts/v1",
      "source": {
        "kind": "targetProfile",
        "coordinate": "echo.dpo@1",
        "digest": "sha256:1111111111111111111111111111111111111111111111111111111111111111"
      },
      "operationProfiles": [
        {
          "source": "p.effectful",
          "core": "continuum.profile.write/v1",
          "allowedWriteClasses": ["replace"]
        }
      ],
      "effectWriteClasses": [],
      "budgets": []
    }"#
}

fn effectful_lawpack_facts() -> &'static str {
    r#"{
      "apiVersion": "edict.authority-facts/v1",
      "source": {
        "kind": "lawpack",
        "coordinate": "hello.optics@1",
        "digest": "sha256:2222222222222222222222222222222222222222222222222222222222222222"
      },
      "operationProfiles": [],
      "effectWriteClasses": [
        {
          "effect": "target.replace",
          "writeClass": "replace"
        }
      ],
      "budgets": [
        {
          "source": "p.tiny",
          "maxSteps": 8,
          "maxAllocatedBytes": 1024,
          "maxOutputBytes": 256
        }
      ]
    }"#
}
