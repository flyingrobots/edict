//! Edict abstract syntax tree (minimal-v1 subset).
//!
//! This is source AST, not Core IR: it keeps source order and surface spelling.
//! Canonicalization, alpha-normalization, and coordinate resolution happen
//! later when lowering to Core IR (Phase 3).

use crate::token::{IntSuffix, Span};

/// A package coordinate such as `examples.hello@1`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRef {
    pub path: Vec<String>,
    pub version: String,
    pub span: Span,
}

/// A source module: one package, then imports, then declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub package: PackageRef,
    pub imports: Vec<Import>,
    pub decls: Vec<Decl>,
}

/// What kind of artifact an import binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportKind {
    Shape,
    Lawpack,
    Target,
    Core,
    /// `use capability ... as ...` — present in product sketches; rejected by v1.
    Capability,
}

/// An `use <kind> ... [digest "..."] as <alias>;` import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    pub kind: ImportKind,
    /// Package ref for lawpack/target/core/capability imports.
    pub package: Option<PackageRef>,
    /// Quoted locator for `use shape "path" ...`.
    pub shape_path: Option<String>,
    /// Optional `digest "..."` clause (required for a locked bundle).
    pub digest: Option<String>,
    pub alias: String,
    pub span: Span,
}

/// A top-level declaration.
#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Type(TypeDecl),
    Intent(IntentDecl),
}

/// `type Name = <type-expr>;`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDecl {
    pub name: String,
    pub params: Vec<String>,
    pub body: TypeExpr,
    pub span: Span,
}

/// The right-hand side of a `type` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeExpr {
    Record(Vec<FieldDecl>),
    Ref(TypeRef),
}

/// A record field with optional field-level constraints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldDecl {
    pub name: String,
    pub ty: TypeRef,
    pub constraints: Vec<FieldConstraint>,
    pub span: Span,
}

/// A `max=`, `min=`, `pattern=`, or `canonical=` field constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldConstraint {
    Max(BoundRef),
    Min(BoundRef),
    Pattern(String),
    Canonical(String),
}

/// A literal-or-digest-locked bound (`max=128` or `max=rope.maxLeaves`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoundRef {
    Int(u64),
    Coord(Vec<String>),
}

/// A type reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRef {
    /// `qual.ident` with optional `<type-args>`.
    Named { path: Vec<String>, args: Vec<TypeRef> },
    /// `String` / `String<max=N[, canonical=x]>`.
    StringTy(Option<ScalarRefine>),
    /// `Bytes` / `Bytes<max=N>` (max-only; no canonicalization).
    BytesTy(Option<u64>),
    Option(Box<TypeRef>),
    CapabilityRef(Box<TypeRef>),
    List { elem: Box<TypeRef>, max: BoundRef },
    Map { key: Box<TypeRef>, value: Box<TypeRef>, max: BoundRef },
}

/// A refined `String` bound: `max=` plus optional `canonical=`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScalarRefine {
    pub max: BoundRef,
    pub canonical: Option<String>,
}

/// `intent name(params) returns Ty <clauses> { body }`
#[derive(Debug, Clone, PartialEq)]
pub struct IntentDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub returns: TypeRef,
    pub clauses: Vec<IntentClause>,
    pub body: Block,
    pub span: Span,
}

/// A `name: Type` parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: TypeRef,
    pub span: Span,
}

/// An intent clause (order-independent).
#[derive(Debug, Clone, PartialEq)]
pub enum IntentClause {
    Profile(Vec<String>),
    Implements(Vec<String>),
    /// `basis none` or `basis <expr>`.
    Basis(Option<Expr>),
    Footprint(Vec<String>),
    Budget(Vec<String>),
    Where(Vec<Expr>),
}

/// A `{ ... }` statement block.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub span: Span,
}

/// A statement inside an intent body (minimal-v1 subset).
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Let { name: String, ty: Option<TypeRef>, value: Expr, span: Span },
    Return { value: Expr, span: Span },
}

/// A binary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Or,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
}

/// A unary operator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnOp {
    Not,
    Neg,
}

/// An expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// A single bare identifier (local/parameter).
    Ident { name: String, span: Span },
    Int { value: String, suffix: Option<IntSuffix>, span: Span },
    Str { value: String, span: Span },
    /// `expr.field` member access.
    Field { base: Box<Expr>, field: String, span: Span },
    Unary { op: UnOp, operand: Box<Expr>, span: Span },
    Binary { op: BinOp, lhs: Box<Expr>, rhs: Box<Expr>, span: Span },
    /// A record literal `{ a: x, b, ...spread }`.
    Record { entries: Vec<RecordEntry>, span: Span },
}

/// One entry in a record literal.
#[derive(Debug, Clone, PartialEq)]
pub enum RecordEntry {
    /// `name: value`
    Field { name: String, value: Expr },
    /// `name` (shorthand for `name: name`).
    Shorthand { name: String, span: Span },
    /// `...expr`
    Spread(Expr),
}
