//! Compiler-spine tests for the first executable source-to-Core path.
//!
//! These tests assert public stage boundaries and structured values. They do
//! not inspect stdout, stderr, diagnostic prose, canonical bytes, or hashes.

use edict_syntax::{
    compile_to_core, lower_core, parse_module, resolve_module, type_check, CompilerContext,
    CompilerErrorKind, CompilerStage, CoreBudget, CoreExpr, CoreNode, CorePredicate, CoreType,
    WriteClass,
};

const BOUNDED_HELLO: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");

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
