//! Source-AST semantic validation for checks that do not require Core IR.

use crate::ast::{
    Block, Decl, ElseClause, Expr, IntentClause, IntentDecl, Module, ObstructionHandler,
    ObstructionTarget, RecordEntry, Stmt, TypeExpr, TypeRef, YieldBlock,
};
use crate::token::Span;

/// Stable semantic validation error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticErrorKind {
    UnboundedScalar,
    MissingOperationMode,
    MissingBudget,
    MissingBasis,
    DuplicateIntentClause,
}

/// A semantic validation failure with a stable kind and source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "semantic error at {}..{}: {}",
            self.span.start, self.span.end, self.message
        )
    }
}

impl std::error::Error for SemanticError {}

/// Validate source-level semantic constraints that are independent of import
/// resolution and Core lowering.
///
/// # Errors
/// Returns all semantic errors found by a deterministic source-AST traversal.
/// Exact ordering is not a public contract for this first validation slice.
pub fn validate_module(module: &Module) -> Result<(), Vec<SemanticError>> {
    let mut errors = Vec::new();
    for decl in &module.decls {
        match decl {
            Decl::Type(decl) => validate_type_expr(&decl.body, decl.span, &mut errors),
            Decl::Enum(_) => {}
            Decl::Intent(intent) => validate_intent(intent, &mut errors),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_intent(intent: &IntentDecl, errors: &mut Vec<SemanticError>) {
    let mut profile = None;
    let mut implements = None;
    let mut basis = None;
    let mut footprint = None;
    let mut budget = None;

    for clause in &intent.clauses {
        match clause {
            IntentClause::Profile(_) => {
                record_singleton("profile", intent.span, &mut profile, errors);
            }
            IntentClause::Implements(_) => {
                record_singleton("implements", intent.span, &mut implements, errors);
            }
            IntentClause::Basis(expr) => {
                record_singleton("basis", intent.span, &mut basis, errors);
                if let Some(expr) = expr {
                    validate_expr(expr, errors);
                }
            }
            IntentClause::Footprint(_) => {
                record_singleton("footprint", intent.span, &mut footprint, errors);
            }
            IntentClause::Budget(_) => record_singleton("budget", intent.span, &mut budget, errors),
            IntentClause::Where(predicates) => {
                for predicate in predicates {
                    validate_expr(predicate, errors);
                }
            }
        }
    }

    if profile.is_none() && implements.is_none() {
        errors.push(error(
            SemanticErrorKind::MissingOperationMode,
            "intent must declare at least one of `profile` or `implements`",
            intent.span,
        ));
    }
    if budget.is_none() {
        errors.push(error(
            SemanticErrorKind::MissingBudget,
            "intent must declare a `budget` clause",
            intent.span,
        ));
    }
    if basis.is_none() {
        errors.push(error(
            SemanticErrorKind::MissingBasis,
            "intent must declare a `basis` clause",
            intent.span,
        ));
    }

    for param in &intent.params {
        validate_type_ref(&param.ty, param.span, errors);
    }
    validate_type_ref(&intent.returns, intent.span, errors);
    validate_block(&intent.body, errors);
}

fn record_singleton(
    name: &str,
    span: Span,
    slot: &mut Option<()>,
    errors: &mut Vec<SemanticError>,
) {
    if slot.replace(()).is_some() {
        // Intent clauses do not currently preserve their own spans, so duplicate
        // clause diagnostics report at intent granularity.
        errors.push(error(
            SemanticErrorKind::DuplicateIntentClause,
            format!("intent contains duplicate `{name}` clause"),
            span,
        ));
    }
}

fn validate_type_expr(expr: &TypeExpr, span: Span, errors: &mut Vec<SemanticError>) {
    match expr {
        TypeExpr::Record(fields) => {
            for field in fields {
                validate_type_ref(&field.ty, field.span, errors);
            }
        }
        TypeExpr::Variant(cases) => {
            for case in cases {
                if let Some(payload) = &case.payload {
                    validate_type_ref(payload, case.span, errors);
                }
            }
        }
        TypeExpr::Ref(ty) => validate_type_ref(ty, span, errors),
    }
}

fn validate_type_ref(ty: &TypeRef, span: Span, errors: &mut Vec<SemanticError>) {
    match ty {
        TypeRef::Named { args, .. } => {
            for arg in args {
                validate_type_ref(arg, span, errors);
            }
        }
        TypeRef::StringTy(None) => errors.push(error(
            SemanticErrorKind::UnboundedScalar,
            "runtime `String` type must carry a `max=` bound",
            span,
        )),
        TypeRef::BytesTy(None) => errors.push(error(
            SemanticErrorKind::UnboundedScalar,
            "runtime `Bytes` type must carry a `max=` bound",
            span,
        )),
        TypeRef::StringTy(Some(_)) | TypeRef::BytesTy(Some(_)) => {}
        TypeRef::Option(inner) | TypeRef::CapabilityRef(inner) => {
            validate_type_ref(inner, span, errors);
        }
        TypeRef::List { elem, .. } => validate_type_ref(elem, span, errors),
        TypeRef::Map { key, value, .. } => {
            validate_type_ref(key, span, errors);
            validate_type_ref(value, span, errors);
        }
    }
}

fn validate_block(block: &Block, errors: &mut Vec<SemanticError>) {
    for stmt in &block.stmts {
        validate_stmt(stmt, errors);
    }
}

fn validate_stmt(stmt: &Stmt, errors: &mut Vec<SemanticError>) {
    match stmt {
        Stmt::Let {
            ty,
            value,
            els,
            span,
            ..
        } => {
            if let Some(ty) = ty {
                validate_type_ref(ty, *span, errors);
            }
            validate_expr(value, errors);
            if let Some(els) = els {
                validate_obstruction_handler(els, errors);
            }
        }
        Stmt::Effect { call, els, .. } => {
            validate_expr(call, errors);
            if let Some(els) = els {
                validate_obstruction_handler(els, errors);
            }
        }
        Stmt::Require {
            predicate,
            obstruction,
            ..
        } => {
            validate_expr(predicate, errors);
            validate_obstruction_target(obstruction, errors);
        }
        Stmt::Guarantee {
            predicate,
            obstruction,
            ..
        } => {
            validate_expr(predicate, errors);
            if let Some(obstruction) = obstruction {
                validate_obstruction_target(obstruction, errors);
            }
        }
        Stmt::Assert { predicate, .. }
        | Stmt::Return {
            value: predicate, ..
        } => {
            validate_expr(predicate, errors);
        }
        Stmt::If {
            cond,
            then_block,
            els,
            ..
        } => {
            validate_expr(cond, errors);
            validate_block(then_block, errors);
            if let Some(els) = els.as_deref() {
                match els {
                    ElseClause::Block(block) => validate_block(block, errors),
                    ElseClause::If(stmt) => validate_stmt(stmt, errors),
                }
            }
        }
        Stmt::For { iter, body, .. } => {
            validate_expr(iter, errors);
            validate_block(body, errors);
        }
    }
}

fn validate_obstruction_handler(handler: &ObstructionHandler, errors: &mut Vec<SemanticError>) {
    match handler {
        ObstructionHandler::Single(target) => validate_obstruction_target(target, errors),
        ObstructionHandler::Map(arms) => {
            for arm in arms {
                validate_obstruction_target(&arm.target, errors);
            }
        }
    }
}

fn validate_obstruction_target(target: &ObstructionTarget, errors: &mut Vec<SemanticError>) {
    if let Some(payload) = &target.payload {
        validate_expr(payload, errors);
    }
}

fn validate_expr(expr: &Expr, errors: &mut Vec<SemanticError>) {
    match expr {
        Expr::Ident { .. }
        | Expr::Int { .. }
        | Expr::Str { .. }
        | Expr::Bool { .. }
        | Expr::Digest { .. } => {}
        Expr::Field { base, .. } | Expr::Unary { operand: base, .. } => validate_expr(base, errors),
        Expr::Call {
            callee,
            type_args,
            args,
            span,
        } => {
            validate_expr(callee, errors);
            for ty in type_args {
                validate_type_ref(ty, *span, errors);
            }
            for arg in args {
                validate_expr(arg, errors);
            }
        }
        Expr::Binary { lhs, rhs, .. } => {
            validate_expr(lhs, errors);
            validate_expr(rhs, errors);
        }
        Expr::Record { entries, .. } => {
            for entry in entries {
                match entry {
                    RecordEntry::Field { value, .. } | RecordEntry::Spread(value) => {
                        validate_expr(value, errors);
                    }
                    RecordEntry::Shorthand { .. } => {}
                }
            }
        }
        Expr::If {
            cond, then, els, ..
        } => {
            validate_expr(cond, errors);
            validate_expr(then, errors);
            validate_expr(els, errors);
        }
        Expr::IfYield {
            pred,
            then_block,
            else_block,
            ..
        } => {
            validate_expr(pred, errors);
            validate_yield_block(then_block, errors);
            validate_yield_block(else_block, errors);
        }
        Expr::VariantLit { payload, .. } => {
            if let Some(payload) = payload {
                validate_expr(payload, errors);
            }
        }
        Expr::Match {
            scrutinee, arms, ..
        } => {
            validate_expr(scrutinee, errors);
            for arm in arms {
                validate_expr(&arm.body, errors);
            }
        }
    }
}

fn validate_yield_block(block: &YieldBlock, errors: &mut Vec<SemanticError>) {
    for stmt in &block.stmts {
        validate_stmt(stmt, errors);
    }
    validate_expr(&block.value, errors);
}

fn error(kind: SemanticErrorKind, message: impl Into<String>, span: Span) -> SemanticError {
    SemanticError {
        kind,
        message: message.into(),
        span,
    }
}
