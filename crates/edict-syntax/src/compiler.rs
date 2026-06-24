//! Executable compiler-spine stages from source AST to in-memory Core IR.
//!
//! This module deliberately stops before canonical encoding, hashing, target
//! lowering, and admission. Those are later stages with separate evidence.

use std::collections::BTreeMap;

use crate::ast::{
    BinOp, BoundRef, Decl, Expr, FieldDecl, Import, ImportKind, IntentClause, IntentDecl, Module,
    RecordEntry, ScalarRefine, Stmt, TypeDecl, TypeExpr, TypeRef,
};
use crate::core_ir::{
    CompareOp, CoreBlock, CoreBudget, CoreExpr, CoreImport, CoreImportKind, CoreIntent, CoreModule,
    CoreNode, CorePredicate, CoreType, CoreValue, InputConstraint, InputConstraintSource, LocalRef,
    ResourceRef, CORE_API_VERSION,
};
use crate::semantic::validate_surface;
use crate::token::Span;

/// Compiler stage that reported an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerStage {
    SurfaceValidation,
    Resolve,
    TypeCheck,
    LowerCore,
}

/// Stable compiler-spine error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerErrorKind {
    SurfaceValidation,
    MissingContextFact,
    UnsupportedSourceShape,
    UnresolvedType,
    UnknownField,
    TypeMismatch,
    ExpectedPredicate,
}

/// A compiler-spine failure with stable stage/kind identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerError {
    pub stage: CompilerStage,
    pub kind: CompilerErrorKind,
    pub message: String,
    pub span: Span,
}

/// Deterministic facts supplied to the compiler spine by the caller.
///
/// The resolver does not invent target/lawpack facts. Source profile and budget
/// coordinates must be bound here before they can lower into Core.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompilerContext {
    operation_profiles: BTreeMap<String, String>,
    budgets: BTreeMap<String, CoreBudget>,
}

impl CompilerContext {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_operation_profile(
        mut self,
        source_coordinate: impl Into<String>,
        core_profile: impl Into<String>,
    ) -> Self {
        self.operation_profiles
            .insert(source_coordinate.into(), core_profile.into());
        self
    }

    #[must_use]
    pub fn with_budget(mut self, source_coordinate: impl Into<String>, budget: CoreBudget) -> Self {
        self.budgets.insert(source_coordinate.into(), budget);
        self
    }
}

/// Module after resolution of names and explicit context facts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModule {
    pub coordinate: String,
    pub imports: Vec<CoreImport>,
    pub types: Vec<ResolvedTypeDecl>,
    pub intents: Vec<ResolvedIntent>,
}

/// Resolved type declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTypeDecl {
    pub name: String,
    pub source: TypeDecl,
}

/// Resolved intent declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedIntent {
    pub name: String,
    pub profile: String,
    pub budget: CoreBudget,
    pub source: IntentDecl,
}

/// Module after type checking and Core-ready local normalization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedModule {
    pub coordinate: String,
    pub imports: Vec<CoreImport>,
    pub types: BTreeMap<String, CoreType>,
    pub intents: Vec<TypedIntent>,
}

/// Typed intent boundary consumed by `lower_core`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedIntent {
    pub name: String,
    pub input: String,
    pub output: String,
    pub profile: String,
    pub budget: CoreBudget,
    pub input_binding: LocalRef,
    pub input_constraints: Vec<InputConstraint>,
    pub body: CoreBlock,
}

/// Run the currently executable compiler spine.
///
/// # Errors
/// Returns structured compiler errors from the first failing stage.
pub fn compile_to_core(
    module: &Module,
    context: &CompilerContext,
) -> Result<CoreModule, Vec<CompilerError>> {
    validate_surface(module).map_err(|errors| {
        errors
            .into_iter()
            .map(|err| CompilerError {
                stage: CompilerStage::SurfaceValidation,
                kind: CompilerErrorKind::SurfaceValidation,
                message: err.message,
                span: err.span,
            })
            .collect::<Vec<_>>()
    })?;
    let resolved = resolve_module(module, context)?;
    let typed = type_check(&resolved)?;
    lower_core(&typed)
}

/// Resolve source names and explicit profile/budget facts.
///
/// # Errors
/// Returns resolver-stage errors when required deterministic context facts are
/// missing or a source construct cannot enter the initial compiler spine.
pub fn resolve_module(
    module: &Module,
    context: &CompilerContext,
) -> Result<ResolvedModule, Vec<CompilerError>> {
    let mut errors = Vec::new();
    let coordinate = package_coordinate(&module.package.path, &module.package.version);
    let imports = resolve_imports(&module.imports, &mut errors);
    let mut types = Vec::new();
    let mut intents = Vec::new();

    for decl in &module.decls {
        match decl {
            Decl::Type(decl) => types.push(ResolvedTypeDecl {
                name: decl.name.clone(),
                source: decl.clone(),
            }),
            Decl::Enum(decl) => errors.push(error(
                CompilerStage::Resolve,
                CompilerErrorKind::UnsupportedSourceShape,
                "enum declarations are not in the initial lowerable subset",
                decl.span,
            )),
            Decl::Intent(intent) => {
                if let Some(resolved) = resolve_intent(intent, context, &mut errors) {
                    intents.push(resolved);
                }
            }
        }
    }

    finish(
        errors,
        ResolvedModule {
            coordinate,
            imports,
            types,
            intents,
        },
    )
}

/// Type-check a resolved module into the Core-ready typed boundary.
///
/// # Errors
/// Returns type-check-stage errors for unresolved types, incompatible values, or
/// source constructs outside the initial lowerable subset.
pub fn type_check(resolved: &ResolvedModule) -> Result<TypedModule, Vec<CompilerError>> {
    TypeChecker::new(resolved).check()
}

/// Lower a typed module into in-memory Core IR.
///
/// # Errors
/// Returns lower-core-stage errors. The current typed boundary is already
/// Core-ready, so this stage is expected to be infallible for valid
/// [`TypedModule`] values.
pub fn lower_core(typed: &TypedModule) -> Result<CoreModule, Vec<CompilerError>> {
    let intents = typed
        .intents
        .iter()
        .map(|intent| {
            (
                intent.name.clone(),
                CoreIntent {
                    input: intent.input.clone(),
                    output: intent.output.clone(),
                    required_operation_profile: intent.profile.clone(),
                    input_constraints: intent.input_constraints.clone(),
                    core_evaluation_budget: intent.budget.clone(),
                    body: intent.body.clone(),
                },
            )
        })
        .collect();
    Ok(CoreModule {
        api_version: CORE_API_VERSION.to_owned(),
        coordinate: typed.coordinate.clone(),
        imports: typed.imports.clone(),
        types: typed.types.clone(),
        intents,
        required_core_capabilities: Vec::new(),
    })
}

fn resolve_imports(imports: &[Import], errors: &mut Vec<CompilerError>) -> Vec<CoreImport> {
    let mut out = Vec::new();
    for import in imports {
        let Some(kind) = core_import_kind(import.kind) else {
            if import.kind == ImportKind::Capability {
                errors.push(error(
                    CompilerStage::Resolve,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "capability imports are not supported by the v1 compiler spine",
                    import.span,
                ));
            }
            continue;
        };
        let Some(package) = &import.package else {
            continue;
        };
        out.push(CoreImport {
            kind,
            resource: ResourceRef {
                coordinate: package_coordinate(&package.path, &package.version),
                digest: import.digest.clone(),
            },
            alias: Some(import.alias.clone()),
        });
    }
    out
}

fn core_import_kind(kind: ImportKind) -> Option<CoreImportKind> {
    match kind {
        ImportKind::Lawpack => Some(CoreImportKind::Lawpack),
        ImportKind::Target => Some(CoreImportKind::Target),
        ImportKind::Core => Some(CoreImportKind::Core),
        ImportKind::Shape | ImportKind::Capability => None,
    }
}

fn resolve_intent(
    intent: &IntentDecl,
    context: &CompilerContext,
    errors: &mut Vec<CompilerError>,
) -> Option<ResolvedIntent> {
    let mut profile = None;
    let mut budget = None;
    for clause in &intent.clauses {
        match clause {
            IntentClause::Profile(path) => {
                let key = path_key(path);
                match context.operation_profiles.get(&key) {
                    Some(value) => profile = Some(value.clone()),
                    None => errors.push(missing_context_fact(
                        format!("operation profile `{key}` has no compiler context fact"),
                        intent.span,
                    )),
                }
            }
            IntentClause::Budget(path) => {
                let key = path_key(path);
                match context.budgets.get(&key) {
                    Some(value) => budget = Some(value.clone()),
                    None => errors.push(missing_context_fact(
                        format!("budget `{key}` has no compiler context fact"),
                        intent.span,
                    )),
                }
            }
            IntentClause::Implements(_) => errors.push(error(
                CompilerStage::Resolve,
                CompilerErrorKind::UnsupportedSourceShape,
                "`implements` intents are outside the initial lowerable subset",
                intent.span,
            )),
            IntentClause::Basis(_) | IntentClause::Footprint(_) | IntentClause::Where(_) => {}
        }
    }

    Some(ResolvedIntent {
        name: intent.name.clone(),
        profile: profile?,
        budget: budget?,
        source: intent.clone(),
    })
}

fn missing_context_fact(message: String, span: Span) -> CompilerError {
    error(
        CompilerStage::Resolve,
        CompilerErrorKind::MissingContextFact,
        message,
        span,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TypeShape {
    coord: String,
    kind: TypeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TypeKind {
    Bool,
    Int { width: String },
    String { max: u64, canonical: String },
    Record(BTreeMap<String, TypeShape>),
}

impl TypeShape {
    fn core_type(&self) -> CoreType {
        match &self.kind {
            TypeKind::Bool => CoreType::Bool,
            TypeKind::Int { width } => CoreType::Int {
                width: width.clone(),
            },
            TypeKind::String { max, canonical } => CoreType::String {
                max: *max,
                canonical: canonical.clone(),
            },
            TypeKind::Record(fields) => CoreType::Record {
                fields: fields
                    .iter()
                    .map(|(name, shape)| (name.clone(), shape.coord.clone()))
                    .collect(),
            },
        }
    }
}

#[derive(Debug, Clone)]
struct TypedValue {
    expr: CoreExpr,
    ty: TypeShape,
}

#[derive(Debug)]
struct TypeChecker<'a> {
    resolved: &'a ResolvedModule,
    errors: Vec<CompilerError>,
    named_types: BTreeMap<String, TypeShape>,
    core_types: BTreeMap<String, CoreType>,
}

impl<'a> TypeChecker<'a> {
    fn new(resolved: &'a ResolvedModule) -> Self {
        Self {
            resolved,
            errors: Vec::new(),
            named_types: BTreeMap::new(),
            core_types: BTreeMap::new(),
        }
    }

    fn check(mut self) -> Result<TypedModule, Vec<CompilerError>> {
        self.check_types();
        let mut intents = Vec::new();
        for intent in &self.resolved.intents {
            if let Some(typed) = self.check_intent(intent) {
                intents.push(typed);
            }
        }
        let module = TypedModule {
            coordinate: self.resolved.coordinate.clone(),
            imports: self.resolved.imports.clone(),
            types: self.core_types,
            intents,
        };
        finish(self.errors, module)
    }

    fn check_types(&mut self) {
        for decl in &self.resolved.types {
            let Some(shape) = self.type_decl_shape(&decl.source) else {
                continue;
            };
            self.core_types.insert(decl.name.clone(), shape.core_type());
            self.named_types.insert(decl.name.clone(), shape);
        }
    }

    fn type_decl_shape(&mut self, decl: &TypeDecl) -> Option<TypeShape> {
        let coord = format!("{}.{}", self.resolved.coordinate, decl.name);
        match &decl.body {
            TypeExpr::Record(fields) => self.record_shape(&coord, &decl.name, fields, decl.span),
            TypeExpr::Variant(_) | TypeExpr::Ref(_) => {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "only record type declarations are in the initial lowerable subset",
                    decl.span,
                ));
                None
            }
        }
    }

    fn record_shape(
        &mut self,
        coord: &str,
        type_name: &str,
        fields: &[FieldDecl],
        span: Span,
    ) -> Option<TypeShape> {
        let mut out = BTreeMap::new();
        for field in fields {
            let field_key = format!("{type_name}.{}", field.name);
            let field_coord = format!("{coord}.{}", field.name);
            let Some(shape) = self.type_ref_shape(&field.ty, field.span, Some(field_coord)) else {
                continue;
            };
            self.core_types.insert(field_key, shape.core_type());
            out.insert(field.name.clone(), shape);
        }
        if out.len() == fields.len() {
            Some(TypeShape {
                coord: coord.to_owned(),
                kind: TypeKind::Record(out),
            })
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "record type contains unsupported field types",
                span,
            ));
            None
        }
    }

    fn type_ref_shape(
        &mut self,
        ty: &TypeRef,
        span: Span,
        coord_hint: Option<String>,
    ) -> Option<TypeShape> {
        match ty {
            TypeRef::Named { path, args } if args.is_empty() && path.len() == 1 => {
                let name = &path[0];
                if let Some(shape) = self.named_types.get(name) {
                    Some(shape.clone())
                } else {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::UnresolvedType,
                        format!("type `{name}` is not declared in this module"),
                        span,
                    ));
                    None
                }
            }
            TypeRef::StringTy(Some(refine)) => self.string_shape(refine, span, coord_hint),
            TypeRef::BytesTy(_)
            | TypeRef::Option(_)
            | TypeRef::CapabilityRef(_)
            | TypeRef::List { .. }
            | TypeRef::Map { .. }
            | TypeRef::Named { .. }
            | TypeRef::StringTy(None) => {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "type is outside the initial lowerable subset",
                    span,
                ));
                None
            }
        }
    }

    fn string_shape(
        &mut self,
        refine: &ScalarRefine,
        span: Span,
        coord_hint: Option<String>,
    ) -> Option<TypeShape> {
        let BoundRef::Int { value, .. } = refine.max else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "coordinate string bounds require bound proof in a later stage",
                span,
            ));
            return None;
        };
        let canonical = refine
            .canonical
            .clone()
            .unwrap_or_else(|| "raw-utf8".to_owned());
        Some(TypeShape {
            coord: coord_hint.unwrap_or_else(|| string_type_coord(value, &canonical)),
            kind: TypeKind::String {
                max: value,
                canonical,
            },
        })
    }

    fn check_intent(&mut self, intent: &ResolvedIntent) -> Option<TypedIntent> {
        let source = &intent.source;
        if source.params.len() != 1 {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "initial Core lowering supports exactly one intent parameter",
                source.span,
            ));
            return None;
        }
        if !source
            .clauses
            .iter()
            .any(|clause| matches!(clause, IntentClause::Basis(None)))
        {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "initial Core lowering supports only `basis none`",
                source.span,
            ));
            return None;
        }

        let param = &source.params[0];
        let input_shape = self.type_ref_shape(&param.ty, param.span, None)?;
        let output_shape = self.type_ref_shape(&source.returns, source.span, None)?;
        let input_binding = LocalRef {
            id: "arg.0".to_owned(),
            alpha_name: "$arg0".to_owned(),
            ty: input_shape.coord.clone(),
        };
        let mut locals = vec![input_binding.clone()];
        let mut env = BTreeMap::from([(param.name.clone(), (input_binding.clone(), input_shape))]);
        let input_constraints = self.input_constraints(source, &env);
        let body = self.check_body(source, &output_shape, &mut env, &mut locals)?;

        Some(TypedIntent {
            name: intent.name.clone(),
            input: input_binding.ty.clone(),
            output: output_shape.coord,
            profile: intent.profile.clone(),
            budget: intent.budget.clone(),
            input_binding,
            input_constraints,
            body,
        })
    }

    fn input_constraints(
        &mut self,
        intent: &IntentDecl,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
    ) -> Vec<InputConstraint> {
        let mut out = Vec::new();
        let mut index = 0usize;
        for clause in &intent.clauses {
            if let IntentClause::Where(predicates) = clause {
                for predicate in predicates {
                    if let Some(predicate) = self.check_predicate(predicate, env) {
                        out.push(InputConstraint {
                            coordinate: format!("where.{index}"),
                            source: InputConstraintSource::Where,
                            predicate,
                        });
                    }
                    index += 1;
                }
            }
        }
        out
    }

    fn check_body(
        &mut self,
        intent: &IntentDecl,
        output_shape: &TypeShape,
        env: &mut BTreeMap<String, (LocalRef, TypeShape)>,
        locals: &mut Vec<LocalRef>,
    ) -> Option<CoreBlock> {
        let mut nodes = Vec::new();
        let mut result = None;
        let mut local_index = 0usize;

        for stmt in &intent.body.stmts {
            match stmt {
                Stmt::Let {
                    name,
                    ty,
                    value,
                    els,
                    span,
                } => {
                    if els.is_some() {
                        self.unsupported_stmt(*span, "effectful `let ... else`");
                        continue;
                    }
                    let Some(value) = self.check_expr(value, env) else {
                        continue;
                    };
                    let binding_shape = if let Some(annotation) = ty {
                        let Some(annotation_shape) = self.type_ref_shape(annotation, *span, None)
                        else {
                            continue;
                        };
                        if !compatible(&annotation_shape, &value.ty) {
                            self.errors.push(error(
                                CompilerStage::TypeCheck,
                                CompilerErrorKind::TypeMismatch,
                                format!("`{name}` initializer does not match annotation"),
                                *span,
                            ));
                            continue;
                        }
                        annotation_shape
                    } else {
                        value.ty.clone()
                    };
                    let local = LocalRef {
                        id: format!("local.{local_index}"),
                        alpha_name: format!("$local{local_index}"),
                        ty: binding_shape.coord.clone(),
                    };
                    local_index += 1;
                    nodes.push(CoreNode::Let {
                        binding: local.clone(),
                        value: value.expr,
                    });
                    locals.push(local.clone());
                    env.insert(name.clone(), (local, binding_shape));
                }
                Stmt::Return { value, .. } => {
                    let Some(value) = self.check_expr(value, env) else {
                        continue;
                    };
                    if compatible(output_shape, &value.ty) {
                        result = Some(value.expr);
                    } else {
                        self.errors.push(error(
                            CompilerStage::TypeCheck,
                            CompilerErrorKind::TypeMismatch,
                            "return value does not match declared output type",
                            intent.span,
                        ));
                    }
                }
                Stmt::Effect { span, .. }
                | Stmt::Require { span, .. }
                | Stmt::Guarantee { span, .. }
                | Stmt::Assert { span, .. }
                | Stmt::If { span, .. }
                | Stmt::For { span, .. } => {
                    self.unsupported_stmt(*span, "statement");
                }
            }
        }

        if let Some(result) = result {
            Some(CoreBlock {
                locals: locals.clone(),
                nodes,
                result,
            })
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "intent body must return a value",
                intent.span,
            ));
            None
        }
    }

    fn unsupported_stmt(&mut self, span: Span, what: &str) {
        self.errors.push(error(
            CompilerStage::TypeCheck,
            CompilerErrorKind::UnsupportedSourceShape,
            format!("{what} is outside the initial lowerable subset"),
            span,
        ));
    }

    fn check_predicate(
        &mut self,
        expr: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
    ) -> Option<CorePredicate> {
        match expr {
            Expr::Bool { value, .. } => Some(if *value {
                CorePredicate::True
            } else {
                CorePredicate::False
            }),
            Expr::Unary {
                op: crate::ast::UnOp::Not,
                operand,
                ..
            } => self
                .check_predicate(operand, env)
                .map(|value| CorePredicate::Not(Box::new(value))),
            Expr::Binary { op, lhs, rhs, .. } => {
                if let Some(op) = compare_op(*op) {
                    self.check_compare_predicate(op, lhs, rhs, env, expr_span(expr))
                } else {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::ExpectedPredicate,
                        "where expression is not a predicate",
                        expr_span(expr),
                    ));
                    None
                }
            }
            _ => {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::ExpectedPredicate,
                    "where expression is not a predicate",
                    expr_span(expr),
                ));
                None
            }
        }
    }

    fn check_compare_predicate(
        &mut self,
        op: CompareOp,
        lhs: &Expr,
        rhs: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        span: Span,
    ) -> Option<CorePredicate> {
        let left = self.check_expr(lhs, env)?;
        let right = self.check_expr(rhs, env)?;
        if comparable(&left.ty, &right.ty) {
            Some(CorePredicate::Compare {
                op,
                left: left.expr,
                right: right.expr,
            })
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "comparison operands are not compatible",
                span,
            ));
            None
        }
    }

    fn check_expr(
        &mut self,
        expr: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
    ) -> Option<TypedValue> {
        match expr {
            Expr::Ident { name, span } => self.check_ident(name, *span, env),
            Expr::Str { value, .. } => Some(string_value(value)),
            Expr::Bool { value, .. } => Some(TypedValue {
                expr: CoreExpr::Const(CoreValue::Bool(*value)),
                ty: TypeShape {
                    coord: "Bool".to_owned(),
                    kind: TypeKind::Bool,
                },
            }),
            Expr::Int { value, .. } => Some(TypedValue {
                expr: CoreExpr::Const(CoreValue::Int {
                    width: "I64".to_owned(),
                    value: value.clone(),
                }),
                ty: TypeShape {
                    coord: "I64".to_owned(),
                    kind: TypeKind::Int {
                        width: "I64".to_owned(),
                    },
                },
            }),
            Expr::Field { base, field, span } => self.check_field(base, field, *span, env),
            Expr::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
                span,
            } => self.check_string_concat(lhs, rhs, env, *span),
            Expr::Record { entries, span } => self.check_record(entries, env, *span),
            Expr::Binary { .. }
            | Expr::Digest { .. }
            | Expr::Call { .. }
            | Expr::Unary { .. }
            | Expr::If { .. }
            | Expr::IfYield { .. }
            | Expr::VariantLit { .. }
            | Expr::Match { .. } => {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "expression is outside the initial lowerable subset",
                    expr_span(expr),
                ));
                None
            }
        }
    }

    fn check_ident(
        &mut self,
        name: &str,
        span: Span,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
    ) -> Option<TypedValue> {
        if let Some((local, ty)) = env.get(name) {
            Some(TypedValue {
                expr: CoreExpr::Local {
                    reference: local.clone(),
                },
                ty: ty.clone(),
            })
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnresolvedType,
                format!("identifier `{name}` has no typed binding"),
                span,
            ));
            None
        }
    }

    fn check_field(
        &mut self,
        base: &Expr,
        field: &str,
        span: Span,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
    ) -> Option<TypedValue> {
        let base = self.check_expr(base, env)?;
        let TypeKind::Record(fields) = &base.ty.kind else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "field access requires a record base",
                span,
            ));
            return None;
        };
        if let Some(field_ty) = fields.get(field) {
            Some(TypedValue {
                expr: CoreExpr::Field {
                    base: Box::new(base.expr),
                    field: field.to_owned(),
                },
                ty: field_ty.clone(),
            })
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnknownField,
                format!("record type has no field `{field}`"),
                span,
            ));
            None
        }
    }

    fn check_string_concat(
        &mut self,
        lhs: &Expr,
        rhs: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        span: Span,
    ) -> Option<TypedValue> {
        let left = self.check_expr(lhs, env)?;
        let right = self.check_expr(rhs, env)?;
        let (TypeKind::String { max: lmax, .. }, TypeKind::String { max: rmax, .. }) =
            (&left.ty.kind, &right.ty.kind)
        else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "string concatenation requires string operands",
                span,
            ));
            return None;
        };
        let max = lmax + rmax;
        let canonical = "raw-utf8".to_owned();
        Some(TypedValue {
            expr: CoreExpr::Call {
                callee: "core.string.concat".to_owned(),
                type_args: Vec::new(),
                args: vec![left.expr, right.expr],
            },
            ty: TypeShape {
                coord: string_type_coord(max, &canonical),
                kind: TypeKind::String { max, canonical },
            },
        })
    }

    fn check_record(
        &mut self,
        entries: &[RecordEntry],
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        span: Span,
    ) -> Option<TypedValue> {
        let mut fields = BTreeMap::new();
        let mut field_types = BTreeMap::new();
        for entry in entries {
            match entry {
                RecordEntry::Field { name, value } => {
                    let value = self.check_expr(value, env)?;
                    fields.insert(name.clone(), value.expr);
                    field_types.insert(name.clone(), value.ty);
                }
                RecordEntry::Shorthand { name, span } => {
                    let Some((local, ty)) = env.get(name) else {
                        self.errors.push(error(
                            CompilerStage::TypeCheck,
                            CompilerErrorKind::UnresolvedType,
                            format!("record shorthand `{name}` has no typed binding"),
                            *span,
                        ));
                        continue;
                    };
                    fields.insert(
                        name.clone(),
                        CoreExpr::Local {
                            reference: local.clone(),
                        },
                    );
                    field_types.insert(name.clone(), ty.clone());
                }
                RecordEntry::Spread(_) => {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::UnsupportedSourceShape,
                        "record spreads are outside the initial lowerable subset",
                        span,
                    ));
                }
            }
        }
        Some(TypedValue {
            expr: CoreExpr::Record { fields },
            ty: TypeShape {
                coord: "anonymous.record".to_owned(),
                kind: TypeKind::Record(field_types),
            },
        })
    }
}

fn string_value(value: &str) -> TypedValue {
    let max = value.len() as u64;
    let canonical = "raw-utf8".to_owned();
    TypedValue {
        expr: CoreExpr::Const(CoreValue::String(value.to_owned())),
        ty: TypeShape {
            coord: string_type_coord(max, &canonical),
            kind: TypeKind::String { max, canonical },
        },
    }
}

fn compatible(expected: &TypeShape, actual: &TypeShape) -> bool {
    if expected.coord == actual.coord {
        return true;
    }
    match (&expected.kind, &actual.kind) {
        (
            TypeKind::String {
                max: expected_max,
                canonical: expected_canonical,
            },
            TypeKind::String {
                max: actual_max,
                canonical: actual_canonical,
            },
        ) => actual_max <= expected_max && actual_canonical == expected_canonical,
        (TypeKind::Record(expected), TypeKind::Record(actual)) => {
            expected.len() == actual.len()
                && expected.iter().all(|(name, expected_ty)| {
                    actual
                        .get(name)
                        .is_some_and(|actual_ty| compatible(expected_ty, actual_ty))
                })
        }
        (TypeKind::Bool, TypeKind::Bool) | (TypeKind::Int { .. }, TypeKind::Int { .. }) => true,
        _ => false,
    }
}

fn comparable(left: &TypeShape, right: &TypeShape) -> bool {
    compatible(left, right) || compatible(right, left)
}

fn compare_op(op: BinOp) -> Option<CompareOp> {
    match op {
        BinOp::Eq => Some(CompareOp::Eq),
        BinOp::Ne => Some(CompareOp::Ne),
        BinOp::Lt => Some(CompareOp::Lt),
        BinOp::Le => Some(CompareOp::Le),
        BinOp::Gt => Some(CompareOp::Gt),
        BinOp::Ge => Some(CompareOp::Ge),
        BinOp::Or | BinOp::And | BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem => {
            None
        }
    }
}

fn string_type_coord(max: u64, canonical: &str) -> String {
    format!("String<max={max},canonical={canonical}>")
}

fn path_key(path: &[String]) -> String {
    path.join(".")
}

fn package_coordinate(path: &[String], version: &str) -> String {
    format!("{}@{version}", path.join("."))
}

fn expr_span(expr: &Expr) -> Span {
    match expr {
        Expr::Ident { span, .. }
        | Expr::Int { span, .. }
        | Expr::Str { span, .. }
        | Expr::Bool { span, .. }
        | Expr::Digest { span, .. }
        | Expr::Field { span, .. }
        | Expr::Call { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Record { span, .. }
        | Expr::If { span, .. }
        | Expr::IfYield { span, .. }
        | Expr::VariantLit { span, .. }
        | Expr::Match { span, .. } => *span,
    }
}

fn error(
    stage: CompilerStage,
    kind: CompilerErrorKind,
    message: impl Into<String>,
    span: Span,
) -> CompilerError {
    CompilerError {
        stage,
        kind,
        message: message.into(),
        span,
    }
}

fn finish<T>(errors: Vec<CompilerError>, value: T) -> Result<T, Vec<CompilerError>> {
    if errors.is_empty() {
        Ok(value)
    } else {
        Err(errors)
    }
}
