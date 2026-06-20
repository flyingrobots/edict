//! Semantic validation tests for source-AST checks that do not require Core IR.

use edict_syntax::{parse_module, validate_module, SemanticErrorKind};

const BOUNDED_HELLO: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");
const CONDITIONAL_BLOB: &str =
    include_str!("../../../fixtures/lang/effects/conditional-blob.edict");
const READ_GREETING: &str = include_str!("../../../fixtures/lang/effects/read-greeting.edict");

fn semantic_kinds(src: &str) -> Vec<SemanticErrorKind> {
    let module = parse_module(src).expect("source parses");
    let mut kinds = validate_module(&module)
        .expect_err("source must fail semantic validation")
        .into_iter()
        .map(|err| err.kind)
        .collect::<Vec<_>>();
    kinds.sort_by_key(|kind| format!("{kind:?}"));
    kinds
}

#[test]
fn phase1_fixtures_validate_semantically() {
    for src in [BOUNDED_HELLO, CONDITIONAL_BLOB, READ_GREETING] {
        let module = parse_module(src).expect("fixture parses");
        validate_module(&module).expect("fixture is semantically valid");
    }
}

#[test]
fn unbounded_runtime_scalars_are_rejected_recursively() {
    let kinds = semantic_kinds(
        "package a.b@1;\n\
         type T = { name: String, blob: Option<Bytes>, items: List<String, max=3>, };\n",
    );
    assert_eq!(
        kinds,
        vec![
            SemanticErrorKind::UnboundedScalar,
            SemanticErrorKind::UnboundedScalar,
            SemanticErrorKind::UnboundedScalar,
        ]
    );
}

#[test]
fn intent_required_clauses_are_validated() {
    let kinds = semantic_kinds(
        "package a.b@1;\n\
         intent t(input: shape.In) returns shape.Out {\n\
           return { input };\n\
         }",
    );
    assert_eq!(
        kinds,
        vec![
            SemanticErrorKind::MissingBasis,
            SemanticErrorKind::MissingBudget,
            SemanticErrorKind::MissingOperationMode,
        ]
    );
}

#[test]
fn profile_or_implements_satisfies_operation_mode() {
    for clause in ["profile shape.readOnly", "implements shape.reader"] {
        let src = format!(
            "package a.b@1;\n\
             intent t(input: shape.In) returns shape.Out\n\
               {clause}\n\
               basis none\n\
               budget <= shape.tinyBudget {{\n\
               return {{ input }};\n\
             }}"
        );
        let module = parse_module(&src).expect("source parses");
        validate_module(&module).expect("operation mode is present");
    }
}

#[test]
fn duplicate_singleton_intent_clauses_are_rejected() {
    let kinds = semantic_kinds(
        "package a.b@1;\n\
         intent t(input: shape.In) returns shape.Out\n\
           profile shape.readOnly\n\
           profile shape.readWrite\n\
           basis none\n\
           basis input.id\n\
           budget <= shape.tinyBudget\n\
           budget <= shape.largeBudget {\n\
           return { input };\n\
         }",
    );
    assert_eq!(
        kinds,
        vec![
            SemanticErrorKind::DuplicateIntentClause,
            SemanticErrorKind::DuplicateIntentClause,
            SemanticErrorKind::DuplicateIntentClause,
        ]
    );
}
