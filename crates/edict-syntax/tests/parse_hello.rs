//! Conformance: the bounded `hello` example parses to the expected AST shape.
//!
//! Fixture: `fixtures/lang/bounds/bounded-hello.edict` (positive fixture for
//! `EDICT-LANG-BOUNDS-001`). This is the first real-world parse target.

use edict_syntax::ast::{
    ActionClause, BoundRef, Decl, Expr, ImportKind, RecordEntry, ScalarRefine, Stmt, TypeExpr,
    TypeRef,
};
use edict_syntax::{parse_module, ParseErrorKind};

const HELLO: &str = include_str!("../../../fixtures/lang/bounds/bounded-hello.edict");

#[test]
fn bounded_hello_parses() {
    let module = parse_module(HELLO).expect("bounded-hello must parse");

    // package examples.hello@1;
    assert_eq!(module.package.path, vec!["examples", "hello"]);
    assert_eq!(module.package.version, "1");

    // use lawpack hello.optics@1 digest "sha256:..." as hello;
    assert_eq!(module.imports.len(), 1);
    let imp = &module.imports[0];
    assert_eq!(imp.kind, ImportKind::Lawpack);
    assert_eq!(imp.alias, "hello");
    assert_eq!(imp.package.as_ref().unwrap().path, vec!["hello", "optics"]);
    assert!(imp.digest.as_ref().unwrap().starts_with("sha256:"));

    // two type decls
    assert_eq!(module.decls.len(), 3);
    let Decl::Type(hello_input) = &module.decls[0] else {
        panic!("decl 0 is a type")
    };
    assert_eq!(hello_input.name, "HelloInput");
    let TypeExpr::Record(fields) = &hello_input.body else {
        panic!("HelloInput is a record")
    };
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "name");
    assert_eq!(
        fields[0].ty,
        TypeRef::StringTy(Some(ScalarRefine {
            max: BoundRef::Int {
                value: 256,
                suffix: None,
            },
            canonical: None
        }))
    );

    let Decl::Type(hello_reading) = &module.decls[1] else {
        panic!("decl 1 is a type")
    };
    assert_eq!(hello_reading.name, "HelloReading");

    // action sayHello(...) ...
    let Decl::Action(action) = &module.decls[2] else {
        panic!("decl 2 is an action")
    };
    assert_eq!(action.name, "sayHello");
    assert_eq!(action.params.len(), 1);
    assert_eq!(action.params[0].name, "input");

    // clauses: profile, basis none, budget, where
    assert!(matches!(&action.clauses[0], ActionClause::Profile(p) if p == &["hello", "readOnly"]));
    assert!(matches!(&action.clauses[1], ActionClause::Basis(None)));
    assert!(matches!(&action.clauses[2], ActionClause::Budget(b) if b == &["hello", "tinyBudget"]));
    assert!(matches!(&action.clauses[3], ActionClause::Where(w) if w.len() == 1));

    // body: let message = "hello, " + input.name;  return { message };
    assert_eq!(action.body.stmts.len(), 2);
    let Stmt::Let { name, value, .. } = &action.body.stmts[0] else {
        panic!("stmt 0 is let")
    };
    assert_eq!(name, "message");
    assert!(
        matches!(value, Expr::Binary { .. }),
        "concat is a binary expr"
    );

    let Stmt::Return { value, .. } = &action.body.stmts[1] else {
        panic!("stmt 1 is return")
    };
    let Expr::Record { entries, .. } = value else {
        panic!("return value is a record")
    };
    assert_eq!(entries.len(), 1);
    assert!(matches!(&entries[0], RecordEntry::Shorthand { name, .. } if name == "message"));
}

#[test]
fn missing_semicolon_is_a_parse_error() {
    let bad = "package examples.hello@1\n";
    let err = parse_module(bad).expect_err("missing package semicolon must reject");
    assert_eq!(err.kind, ParseErrorKind::ExpectedToken);
    assert!(
        err.message.contains("Semi"),
        "diagnostic should identify the missing semicolon: {err}"
    );
}

#[test]
fn multi_part_package_version() {
    let m = parse_module("package a.b@1.2.3-beta;").expect("multi-part version parses");
    assert_eq!(m.package.version, "1.2.3-beta");
}
