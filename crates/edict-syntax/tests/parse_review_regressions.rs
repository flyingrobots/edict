//! Regression coverage for PR #9 review findings.
//!
//! These cases pin syntax-boundary behavior through the public `parse_module`
//! API and assert stable AST/error identities.

mod common;
use common::{body, intent_of};
use edict_syntax::ast::{BinOp, BoundRef, Decl, Expr, Stmt, TypeExpr, TypeRef};
use edict_syntax::{parse_module, ParseErrorKind};

const ZERO_DIGEST: &str = "sha256:0000000000000000000000000000000000000000000000000000000000000000";

fn reject_kind(src: &str, kind: ParseErrorKind) {
    let err = parse_module(src).expect_err("source must reject");
    assert_eq!(err.kind, kind, "wrong error for source:\n{src}");
}

fn first_let_value(src: &str) -> Expr {
    let m = parse_module(src).expect("module parses");
    let intent = intent_of(&m);
    let Stmt::Let { value, .. } = &intent.body.stmts[0] else {
        panic!("first statement is a let");
    };
    value.clone()
}

#[test]
fn package_versions_preserve_underscores() {
    let m = parse_module("package a.b@1_2.3-beta;").expect("package parses");
    assert_eq!(m.package.version, "1_2.3-beta");
}

#[test]
fn capability_imports_are_rejected_in_v1() {
    reject_kind(
        "package a.b@1;\nuse capability c.d@1 as c;",
        ParseErrorKind::UnsupportedSyntax,
    );
}

#[test]
fn import_digest_literals_are_validated() {
    reject_kind(
        "package a.b@1;\nuse lawpack x.y@1 digest \"not-a-digest\" as x;",
        ParseErrorKind::InvalidDigest,
    );

    let src = format!("package a.b@1;\nuse lawpack x.y@1 digest \"{ZERO_DIGEST}\" as x;");
    let m = parse_module(&src).expect("valid digest import parses");
    assert_eq!(m.imports[0].digest.as_deref(), Some(ZERO_DIGEST));
}

#[test]
fn bytes_accept_coordinate_bounds() {
    let m = parse_module("package a.b@1;\ntype T = { bytes: Bytes<max=limits.maxBytes>, };")
        .expect("coordinate-bounded bytes parse");
    let Decl::Type(t) = &m.decls[0] else {
        panic!("decl 0 is type");
    };
    let TypeExpr::Record(fields) = &t.body else {
        panic!("T is a record");
    };
    assert_eq!(
        fields[0].ty,
        TypeRef::BytesTy(Some(BoundRef::Coord(vec![
            "limits".into(),
            "maxBytes".into()
        ])))
    );
}

#[test]
fn bool_and_digest_literals_are_real_literals() {
    let yes = first_let_value(&body("  let ok = true;\n  return { ok };"));
    assert!(matches!(yes, Expr::Bool { value: true, .. }));

    let no = first_let_value(&body("  let ok = false;\n  return { ok };"));
    assert!(matches!(no, Expr::Bool { value: false, .. }));

    let src = body(&format!(
        "  let h = digest(\"{ZERO_DIGEST}\");\n  return {{ h }};"
    ));
    let digest = first_let_value(&src);
    assert!(matches!(digest, Expr::Digest { value, .. } if value == ZERO_DIGEST));
}

#[test]
fn upper_ident_productions_are_enforced() {
    reject_kind(
        "package a.b@1;\ntype bad = { x: Bytes<max=1>, };",
        ParseErrorKind::InvalidName,
    );
    reject_kind(
        "package a.b@1;\nenum channel { Red }",
        ParseErrorKind::InvalidName,
    );
    reject_kind(
        "package a.b@1;\ntype T = variant { bad };",
        ParseErrorKind::InvalidName,
    );
    reject_kind(
        &body("  let x = shape.Paint::bad;\n  return { x };"),
        ParseErrorKind::InvalidName,
    );
}

#[test]
fn empty_enum_and_empty_obstruction_maps_reject() {
    reject_kind("package a.b@1;\nenum E {}", ParseErrorKind::EmptyEnum);
    reject_kind(
        &body("  target.effect() else {};\n  return { input };"),
        ParseErrorKind::EmptyObstructionMap,
    );
}

#[test]
fn reserved_words_are_rejected_in_all_binder_positions() {
    reject_kind(
        &body("  for match in input.items bounded 1 { }\n  return { input };"),
        ParseErrorKind::ReservedKeyword,
    );
    reject_kind(
        &body("  target.effect() else { else => domain.Oops };\n  return { input };"),
        ParseErrorKind::ReservedKeyword,
    );
    reject_kind(
        &body("  target.effect() else { mismatch(return) => domain.Oops };\n  return { input };"),
        ParseErrorKind::ReservedKeyword,
    );
}

#[test]
fn effect_positions_must_be_calls() {
    reject_kind(
        &body("  input.value;\n  return { input };"),
        ParseErrorKind::NonCallEffect,
    );
    reject_kind(
        &body("  let x = input.value else domain.Oops;\n  return { x };"),
        ParseErrorKind::NonCallEffect,
    );
}

#[test]
fn yield_blocks_reject_return_statements() {
    reject_kind(
        &body(
            "  let x = if input.ok { return input.a; yield input.a; } else { yield input.b; };\n\
             \x20 return { x };",
        ),
        ParseErrorKind::ReturnInYieldBlock,
    );
}

#[test]
fn type_call_suffix_requires_adjacency_to_call_paren() {
    let value = first_let_value(&body("  let x = input.lo<input.hi> (0);\n  return { x };"));
    assert!(
        matches!(value, Expr::Binary { op: BinOp::Gt, .. }),
        "spaced `> (` must parse as relation, not a type-call suffix: {value:?}",
    );
}
