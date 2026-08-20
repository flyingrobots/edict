//! Executable compiler-spine stages from source AST to in-memory Core IR.
//!
//! This module deliberately stops before canonical encoding, hashing, target
//! lowering, and admission. Those are later stages with separate evidence.

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{
    BinOp, Block, BoundRef, Decl, DigestLockedPackageRef, ElseClause, Expr, FieldDecl, Import,
    ImportKind, IntentClause, IntentDecl, Module, ObstructionArm, ObstructionHandler,
    ObstructionTarget, RecordEntry, RequireElseArm, ScalarRefine, Stmt, TypeDecl, TypeExpr,
    TypeRef, UnOp, YieldBlock,
};
use crate::core_ir::{
    is_lowercase_sha256_review_digest, parse_core_integer, CompareOp, CoreBlock, CoreBound,
    CoreBudget, CoreExpr, CoreExternalActionBudget, CoreImport, CoreImportKind, CoreIntent,
    CoreModule, CoreNode, CoreObstructionArm, CoreObstructionReason, CorePredicate,
    CoreRequireFailureArm, CoreType, CoreValue, InputConstraint, InputConstraintSource, LocalRef,
    ResourceRef, CORE_API_VERSION, CORE_APPLICATION_INPUT_LOCAL_ID,
};
use crate::lowerability::WriteClass;
use crate::semantic::validate_surface;
use crate::token::{IntSuffix, Span};

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
    UnresolvedFunction,
    InvalidBound,
    UnknownField,
    TypeMismatch,
    ExpectedPredicate,
    ProfileEffectMismatch,
    UnrequestableExternalOperation,
    DuplicateObstructionFailure,
    DuplicateObstructionPayloadField,
}

/// Deterministic signature and canonical identity for one imported pure helper.
///
/// The owning digest-bound lawpack remains the source of executable helper
/// semantics. This fact only makes its reviewed signature and canonical export
/// coordinate available to source resolution and type checking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PureFunctionFact {
    pub lawpack: ResourceRef,
    pub coordinate: String,
    pub type_parameters: Vec<String>,
    pub parameter_types: Vec<String>,
    pub return_type: String,
    pub cost_template: String,
}

/// Canonical identity and Core definition for one imported lawpack type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeShapeFact {
    pub lawpack: ResourceRef,
    pub coordinate: String,
    pub definition: String,
}

/// Canonical identity plus numeric value for one imported static bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundFact {
    pub lawpack: ResourceRef,
    pub coordinate: String,
    pub value: u64,
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
    operation_profile_write_classes: BTreeMap<String, BTreeSet<WriteClass>>,
    operation_profile_budgets: BTreeMap<String, String>,
    effect_write_classes: BTreeMap<String, WriteClass>,
    pure_functions: BTreeMap<String, PureFunctionFact>,
    type_shapes: BTreeMap<String, TypeShapeFact>,
    bounds: BTreeMap<String, BoundFact>,
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
    pub fn with_operation_profile_write_classes<I>(
        mut self,
        source_coordinate: impl Into<String>,
        write_classes: I,
    ) -> Self
    where
        I: IntoIterator<Item = WriteClass>,
    {
        self.operation_profile_write_classes.insert(
            source_coordinate.into(),
            write_classes.into_iter().collect(),
        );
        self
    }

    #[must_use]
    pub fn with_operation_profile_budget(
        mut self,
        source_profile: impl Into<String>,
        source_budget: impl Into<String>,
    ) -> Self {
        self.operation_profile_budgets
            .insert(source_profile.into(), source_budget.into());
        self
    }

    #[must_use]
    pub fn with_effect_write_class(
        mut self,
        source_effect_coordinate: impl Into<String>,
        write_class: WriteClass,
    ) -> Self {
        self.effect_write_classes
            .insert(source_effect_coordinate.into(), write_class);
        self
    }

    #[must_use]
    pub fn with_pure_function(
        mut self,
        source_coordinate: impl Into<String>,
        fact: PureFunctionFact,
    ) -> Self {
        self.pure_functions.insert(source_coordinate.into(), fact);
        self
    }

    #[must_use]
    pub fn with_type_shape(mut self, fact: TypeShapeFact) -> Self {
        self.type_shapes.insert(fact.coordinate.clone(), fact);
        self
    }

    #[must_use]
    pub fn with_bound(mut self, source_coordinate: impl Into<String>, fact: BoundFact) -> Self {
        self.bounds.insert(source_coordinate.into(), fact);
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
    pub effect_write_classes: BTreeMap<String, WriteClass>,
    pub pure_functions: BTreeMap<String, PureFunctionFact>,
    pub type_shapes: BTreeMap<String, TypeShapeFact>,
    pub bounds: BTreeMap<String, BoundFact>,
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
    pub allowed_write_classes: Option<BTreeSet<WriteClass>>,
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
    pub basis: Option<CoreExpr>,
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
    let imports = resolve_imports(&module.imports);
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
            effect_write_classes: context.effect_write_classes.clone(),
            pure_functions: context.pure_functions.clone(),
            type_shapes: context.type_shapes.clone(),
            bounds: context.bounds.clone(),
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
                    basis: intent.basis.clone(),
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

fn resolve_imports(imports: &[Import]) -> Vec<CoreImport> {
    let mut out = Vec::new();
    for import in imports {
        let Some(kind) = core_import_kind(import.kind) else {
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
        ImportKind::Capability => Some(CoreImportKind::Capability),
        ImportKind::Shape => None,
    }
}

fn resolve_intent(
    intent: &IntentDecl,
    context: &CompilerContext,
    errors: &mut Vec<CompilerError>,
) -> Option<ResolvedIntent> {
    let mut profile = None;
    let mut profile_source = None;
    let mut allowed_write_classes = None;
    let mut budget = None;
    let mut budget_source = None;
    for clause in &intent.clauses {
        match clause {
            IntentClause::Profile(path) => {
                let key = path_key(path);
                match context.operation_profiles.get(&key) {
                    Some(value) => {
                        profile = Some(value.clone());
                        profile_source = Some(key.clone());
                        allowed_write_classes =
                            context.operation_profile_write_classes.get(&key).cloned();
                    }
                    None => errors.push(missing_context_fact(
                        format!("operation profile `{key}` has no compiler context fact"),
                        intent.span,
                    )),
                }
            }
            IntentClause::Budget(path) => {
                let key = path_key(path);
                match context.budgets.get(&key) {
                    Some(value) => {
                        budget = Some(value.clone());
                        budget_source = Some(key);
                    }
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

    if let (Some(profile_source), Some(budget_source)) = (&profile_source, &budget_source) {
        if let Some(required_budget) = context.operation_profile_budgets.get(profile_source) {
            if required_budget != budget_source {
                errors.push(missing_context_fact(
                    format!(
                        "profile `{profile_source}` requires budget `{required_budget}`, got `{budget_source}`"
                    ),
                    intent.span,
                ));
                return None;
            }
        }
    }

    Some(ResolvedIntent {
        name: intent.name.clone(),
        profile: profile?,
        allowed_write_classes,
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
    Bytes { max: u64 },
    List { item: Box<TypeShape>, max: u64 },
    Record(BTreeMap<String, TypeShape>),
    ExternalActionRequest { settlement: Box<TypeShape> },
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
            TypeKind::Bytes { max } => CoreType::Bytes { max: *max },
            TypeKind::List { item, max } => CoreType::List {
                item: item.value_type_coord(),
                max: *max,
            },
            TypeKind::Record(fields) => CoreType::Record {
                fields: fields
                    .iter()
                    .map(|(name, shape)| (name.clone(), shape.coord.clone()))
                    .collect(),
            },
            TypeKind::ExternalActionRequest { settlement } => CoreType::ExternalActionRequest {
                settlement: settlement.coord.clone(),
            },
        }
    }

    fn value_type_coord(&self) -> String {
        match &self.kind {
            TypeKind::Bool => "Bool".to_owned(),
            TypeKind::Int { width } => width.clone(),
            TypeKind::String { max, canonical } => string_type_coord(*max, canonical),
            TypeKind::Bytes { max } => bytes_type_coord(*max),
            TypeKind::List { item, max } => list_type_coord(&item.value_type_coord(), *max),
            TypeKind::Record(_) | TypeKind::ExternalActionRequest { .. } => self.coord.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct TypedValue {
    expr: CoreExpr,
    ty: TypeShape,
}

#[derive(Debug)]
struct BodyState {
    nodes: Vec<CoreNode>,
    result: Option<CoreExpr>,
    local_index: usize,
    obstruction_index: usize,
    step_factor: u64,
    accumulated_steps: u64,
}

impl Default for BodyState {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            result: None,
            local_index: 0,
            obstruction_index: 0,
            step_factor: 1,
            accumulated_steps: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReasonPayloadMode {
    PreserveReasonField,
    SkipReasonField,
}

#[derive(Debug, Clone, Copy)]
struct LetStatement<'a> {
    name: &'a str,
    ty: Option<&'a TypeRef>,
    value: &'a Expr,
    handler: Option<&'a ObstructionHandler>,
    span: Span,
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
                if let Some(width) = builtin_integer_width(name) {
                    Some(integer_shape(width))
                } else if let Some(shape) = self.named_types.get(name) {
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
            TypeRef::Named { path, args }
                if path.as_slice() == ["ExternalActionRequest"] && args.len() == 1 =>
            {
                let settlement = self.type_ref_shape(&args[0], span, None)?;
                Some(TypeShape {
                    coord: format!("edict.external-action.request/v1<{}>", settlement.coord),
                    kind: TypeKind::ExternalActionRequest {
                        settlement: Box::new(settlement),
                    },
                })
            }
            TypeRef::StringTy(Some(refine)) => self.string_shape(refine, span, coord_hint),
            TypeRef::BytesTy(Some(bound)) => self.bytes_shape(bound, span, coord_hint),
            TypeRef::List { elem, max } => {
                let BoundRef::Int { value, .. } = max else {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::UnsupportedSourceShape,
                        "coordinate list bounds require bound proof in a later stage",
                        span,
                    ));
                    return None;
                };
                let item = self.type_ref_shape(elem, span, None)?;
                Some(TypeShape {
                    coord: coord_hint
                        .unwrap_or_else(|| list_type_coord(&item.value_type_coord(), *value)),
                    kind: TypeKind::List {
                        item: Box::new(item),
                        max: *value,
                    },
                })
            }
            TypeRef::BytesTy(None)
            | TypeRef::Option(_)
            | TypeRef::CapabilityRef(_)
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

    fn bytes_shape(
        &mut self,
        bound: &BoundRef,
        span: Span,
        coord_hint: Option<String>,
    ) -> Option<TypeShape> {
        let BoundRef::Int { value, .. } = bound else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "coordinate byte bounds require bound proof in a later stage",
                span,
            ));
            return None;
        };
        Some(TypeShape {
            coord: coord_hint.unwrap_or_else(|| bytes_type_coord(*value)),
            kind: TypeKind::Bytes { max: *value },
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
        let param = &source.params[0];
        let input_shape = self.type_ref_shape(&param.ty, param.span, None)?;
        let output_shape = self.type_ref_shape(&source.returns, source.span, None)?;
        let input_binding = LocalRef {
            id: CORE_APPLICATION_INPUT_LOCAL_ID.to_owned(),
            alpha_name: "$arg0".to_owned(),
            ty: input_shape.coord.clone(),
        };
        let mut locals = vec![input_binding.clone()];
        let mut env = BTreeMap::from([(param.name.clone(), (input_binding.clone(), input_shape))]);
        let mut basis_clauses = source.clauses.iter().filter_map(|clause| match clause {
            IntentClause::Basis(basis) => Some(basis),
            _ => None,
        });
        let Some(basis) = basis_clauses.next() else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "intent is missing the required basis clause",
                source.span,
            ));
            return None;
        };
        if basis_clauses.next().is_some() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "intent contains duplicate `basis` clause",
                source.span,
            ));
            return None;
        }
        let basis = match basis {
            Some(basis) => Some(self.check_expr(basis, &env)?.expr),
            None => None,
        };
        let input_constraints = self.input_constraints(source, &env);
        let body = self.check_body(intent, &output_shape, &mut env, &mut locals)?;

        Some(TypedIntent {
            name: intent.name.clone(),
            input: input_binding.ty.clone(),
            output: output_shape.coord,
            profile: intent.profile.clone(),
            budget: intent.budget.clone(),
            input_binding,
            basis,
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
        intent: &ResolvedIntent,
        output_shape: &TypeShape,
        env: &mut BTreeMap<String, (LocalRef, TypeShape)>,
        locals: &mut Vec<LocalRef>,
    ) -> Option<CoreBlock> {
        let source = &intent.source;
        let mut state = BodyState::default();

        for stmt in &source.body.stmts {
            self.check_body_stmt(intent, output_shape, stmt, env, locals, &mut state);
        }

        if let Some(result) = state.result {
            Some(CoreBlock {
                locals: locals.clone(),
                nodes: state.nodes,
                result,
            })
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "intent body must return a value",
                source.span,
            ));
            None
        }
    }

    #[allow(clippy::too_many_lines)]
    fn check_body_stmt(
        &mut self,
        intent: &ResolvedIntent,
        output_shape: &TypeShape,
        stmt: &Stmt,
        env: &mut BTreeMap<String, (LocalRef, TypeShape)>,
        locals: &mut Vec<LocalRef>,
        state: &mut BodyState,
    ) {
        match stmt {
            Stmt::Let {
                name,
                ty,
                value,
                els,
                span,
            } => self.check_let_stmt(
                intent,
                LetStatement {
                    name,
                    ty: ty.as_ref(),
                    value,
                    handler: els.as_ref(),
                    span: *span,
                },
                env,
                locals,
                state,
            ),
            Stmt::Return { value, .. } => {
                let Some(value) = self.check_expr_with_expected(value, env, Some(output_shape))
                else {
                    return;
                };
                if compatible(output_shape, &value.ty) {
                    state.result = Some(value.expr);
                } else {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::TypeMismatch,
                        "return value does not match declared output type",
                        intent.source.span,
                    ));
                }
            }
            Stmt::Effect { call, span, .. } => {
                if self.check_effect_profile(intent, call, *span) {
                    self.unsupported_stmt(*span, "effect statement");
                }
            }
            Stmt::ExternalActionRequest {
                name,
                request_type,
                operation,
                input_schema,
                settlement_schema,
                authority_scope,
                basis,
                max_settlement_bytes,
                max_attempts,
                reconciliation_law,
                span,
            } => self.check_external_action_request(
                name,
                request_type,
                operation,
                input_schema,
                settlement_schema,
                authority_scope,
                basis,
                max_settlement_bytes,
                max_attempts,
                reconciliation_law,
                *span,
                env,
                locals,
                state,
            ),
            Stmt::Require { predicate, arm, .. } => {
                self.check_require_stmt(predicate, arm, env, state);
            }
            Stmt::For {
                var,
                iter,
                bound,
                body,
                span,
            } => self.check_for_stmt(
                intent,
                output_shape,
                var,
                iter,
                bound,
                body,
                *span,
                env,
                state,
            ),
            Stmt::If {
                cond,
                then_block,
                els,
                span,
            } => self.check_if_stmt(
                intent,
                output_shape,
                cond,
                then_block,
                els.as_deref(),
                *span,
                env,
                state,
            ),
            Stmt::Guarantee { span, .. } | Stmt::Assert { span, .. } => {
                self.unsupported_stmt(*span, "statement");
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_if_stmt(
        &mut self,
        intent: &ResolvedIntent,
        output_shape: &TypeShape,
        cond: &Expr,
        then_block: &Block,
        els: Option<&ElseClause>,
        span: Span,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        state: &mut BodyState,
    ) {
        let Some(predicate) = self.check_predicate(cond, env) else {
            return;
        };
        let baseline_steps = state.accumulated_steps;
        let then_block =
            self.check_isolated_statement_block(intent, output_shape, then_block, env, state);
        let then_steps = state.accumulated_steps;
        state.accumulated_steps = baseline_steps;
        let else_block = match els {
            Some(ElseClause::Block(block)) => {
                self.check_isolated_statement_block(intent, output_shape, block, env, state)
            }
            Some(ElseClause::If(_)) => {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "else-if chains require explicit branch-join lowering",
                    span,
                ));
                empty_core_block()
            }
            None => empty_core_block(),
        };
        state.accumulated_steps = then_steps.max(state.accumulated_steps);
        state.nodes.push(CoreNode::Branch {
            predicate,
            then_block,
            else_block,
        });
    }

    fn check_isolated_statement_block(
        &mut self,
        intent: &ResolvedIntent,
        output_shape: &TypeShape,
        block: &Block,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        state: &mut BodyState,
    ) -> CoreBlock {
        let mut nested_env = env.clone();
        let mut nested_locals = Vec::new();
        let mut nested_state = BodyState {
            local_index: state.local_index,
            obstruction_index: state.obstruction_index,
            step_factor: state.step_factor,
            accumulated_steps: state.accumulated_steps,
            ..BodyState::default()
        };
        for stmt in &block.stmts {
            if let Stmt::Return { span, .. } = stmt {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "return is not legal inside an isolated statement branch",
                    *span,
                ));
                continue;
            }
            self.check_body_stmt(
                intent,
                output_shape,
                stmt,
                &mut nested_env,
                &mut nested_locals,
                &mut nested_state,
            );
        }
        state.local_index = nested_state.local_index;
        state.obstruction_index = nested_state.obstruction_index;
        state.accumulated_steps = nested_state.accumulated_steps;
        CoreBlock {
            locals: nested_locals,
            nodes: nested_state.nodes,
            result: CoreExpr::Const(CoreValue::Null),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn check_for_stmt(
        &mut self,
        intent: &ResolvedIntent,
        output_shape: &TypeShape,
        var: &str,
        iter: &Expr,
        bound: &BoundRef,
        body: &Block,
        span: Span,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        state: &mut BodyState,
    ) {
        let Some(iter_value) = self.check_expr(iter, env) else {
            return;
        };
        let TypeKind::List { item, max } = &iter_value.ty.kind else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "bounded for requires a statically bounded list",
                span,
            ));
            return;
        };
        let (value, core_bound) = match bound {
            BoundRef::Int { value, .. } => (*value, CoreBound::Literal(*value)),
            BoundRef::Coord(path) => {
                let source_coordinate = path.join(".");
                let Some(fact) = self.resolved.bounds.get(&source_coordinate) else {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::MissingContextFact,
                        format!("loop bound `{source_coordinate}` has no compiler context fact"),
                        span,
                    ));
                    return;
                };
                if !self.lawpack_is_imported(&fact.lawpack)
                    || !coordinate_is_below_lawpack(&fact.coordinate, &fact.lawpack)
                {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::MissingContextFact,
                        format!(
                            "loop bound `{source_coordinate}` is not owned by an exact imported lawpack"
                        ),
                        span,
                    ));
                    return;
                }
                (fact.value, CoreBound::Coordinate(fact.coordinate.clone()))
            }
        };
        if value < *max {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::InvalidBound,
                format!("loop bound {value} is below iterable maximum {max}"),
                span,
            ));
            return;
        }
        let Some((loop_steps, accumulated_steps)) =
            self.charge_loop_work(intent, state, value, span)
        else {
            return;
        };

        let binder_shape = item.as_ref().clone();
        let binder = next_local(&mut state.local_index, binder_shape.value_type_coord());
        let mut nested_env = env.clone();
        nested_env.insert(var.to_owned(), (binder.clone(), binder_shape));
        let mut nested_locals = vec![binder.clone()];
        let mut nested_state = BodyState {
            local_index: state.local_index,
            obstruction_index: state.obstruction_index,
            step_factor: loop_steps,
            accumulated_steps,
            ..BodyState::default()
        };
        for stmt in &body.stmts {
            if let Stmt::Return { span, .. } = stmt {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "return is not legal inside a bounded for body",
                    *span,
                ));
                continue;
            }
            self.check_body_stmt(
                intent,
                output_shape,
                stmt,
                &mut nested_env,
                &mut nested_locals,
                &mut nested_state,
            );
        }
        state.local_index = nested_state.local_index;
        state.obstruction_index = nested_state.obstruction_index;
        state.accumulated_steps = nested_state.accumulated_steps;
        state.nodes.push(CoreNode::For {
            binder,
            iter: iter_value.expr,
            bound: core_bound,
            body: CoreBlock {
                locals: nested_locals,
                nodes: nested_state.nodes,
                result: CoreExpr::Const(CoreValue::Null),
            },
        });
    }

    fn charge_loop_work(
        &mut self,
        intent: &ResolvedIntent,
        state: &BodyState,
        bound: u64,
        span: Span,
    ) -> Option<(u64, u64)> {
        let loop_steps = state.step_factor.checked_mul(bound);
        let accumulated_steps =
            loop_steps.and_then(|steps| state.accumulated_steps.checked_add(steps));
        let Some((loop_steps, accumulated_steps)) = loop_steps.zip(accumulated_steps) else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::InvalidBound,
                "cumulative loop step accounting overflows u64",
                span,
            ));
            return None;
        };
        if accumulated_steps > intent.budget.max_steps {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::InvalidBound,
                format!(
                    "cumulative loop work {accumulated_steps} exceeds operation step budget {}",
                    intent.budget.max_steps
                ),
                span,
            ));
            return None;
        }
        Some((loop_steps, accumulated_steps))
    }

    #[allow(clippy::too_many_arguments)]
    fn check_external_action_request(
        &mut self,
        name: &str,
        request_type: &TypeRef,
        operation: &Expr,
        input_schema: &DigestLockedPackageRef,
        settlement_schema: &DigestLockedPackageRef,
        authority_scope: &Expr,
        basis: &Expr,
        max_settlement_bytes: &Expr,
        max_attempts: &Expr,
        reconciliation_law: &DigestLockedPackageRef,
        span: Span,
        env: &mut BTreeMap<String, (LocalRef, TypeShape)>,
        locals: &mut Vec<LocalRef>,
        state: &mut BodyState,
    ) {
        let Some(request_shape) = self.type_ref_shape(request_type, span, None) else {
            return;
        };
        let TypeKind::ExternalActionRequest { settlement } = &request_shape.kind else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "request binding type must be ExternalActionRequest<Settlement>",
                span,
            ));
            return;
        };
        let settlement_type = settlement.coord.clone();
        let Some((operation_resource, input)) =
            self.check_external_action_operation(operation, env, span)
        else {
            return;
        };
        if !requestable_operation_family(&operation_resource.coordinate) {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnrequestableExternalOperation,
                format!(
                    "external operation family `{}` is not requestable",
                    operation_resource.coordinate
                ),
                span,
            ));
            return;
        }
        let Some(authority_scope) = self.check_expr(authority_scope, env) else {
            return;
        };
        let Some(basis) = self.check_expr(basis, env) else {
            return;
        };
        let settlement_budget_shape = integer_shape("U64");
        let Some(max_settlement_bytes) = self.check_expr_with_expected(
            max_settlement_bytes,
            env,
            Some(&settlement_budget_shape),
        ) else {
            return;
        };
        let attempt_budget_shape = integer_shape("U32");
        let Some(max_attempts) =
            self.check_expr_with_expected(max_attempts, env, Some(&attempt_budget_shape))
        else {
            return;
        };
        let Some(input_schema) = self.request_resource(input_schema) else {
            return;
        };
        let Some(settlement_schema) = self.request_resource(settlement_schema) else {
            return;
        };
        let Some(reconciliation_law) = self.request_resource(reconciliation_law) else {
            return;
        };
        let local = next_local(&mut state.local_index, request_shape.coord.clone());
        state.nodes.push(CoreNode::ExternalActionRequest {
            binding: local.clone(),
            operation: operation_resource,
            input_type: input.ty.value_type_coord(),
            settlement_type,
            input_schema,
            settlement_schema,
            input: input.expr,
            authority_scope: Box::new(authority_scope.expr),
            basis: Box::new(basis.expr),
            budget: Box::new(CoreExternalActionBudget {
                max_settlement_bytes: max_settlement_bytes.expr,
                max_attempts: max_attempts.expr,
            }),
            reconciliation_law,
        });
        locals.push(local.clone());
        env.insert(name.to_owned(), (local, request_shape));
    }

    fn check_external_action_operation(
        &mut self,
        operation: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        span: Span,
    ) -> Option<(ResourceRef, TypedValue)> {
        let Expr::Call {
            callee,
            type_args,
            args,
            ..
        } = operation
        else {
            self.unsupported_stmt(span, "external-action request operation");
            return None;
        };
        if !type_args.is_empty() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "external-action request operation does not accept type arguments",
                span,
            ));
            return None;
        }
        let Expr::Ident { name: alias, .. } = callee.as_ref() else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "external-action request operation must name one capability import alias",
                span,
            ));
            return None;
        };
        let Some(operation_resource) = self
            .resolved
            .imports
            .iter()
            .find(|import| {
                import.kind == CoreImportKind::Capability
                    && import.alias.as_deref() == Some(alias.as_str())
                    && import
                        .resource
                        .digest
                        .as_deref()
                        .is_some_and(is_lowercase_sha256_review_digest)
            })
            .map(|import| import.resource.clone())
        else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::MissingContextFact,
                format!(
                    "external-action operation `{alias}` has no digest-locked capability import"
                ),
                span,
            ));
            return None;
        };
        let [input] = args.as_slice() else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "external-action request operation accepts exactly one input value",
                span,
            ));
            return None;
        };
        let input = self.check_expr(input, env)?;
        Some((operation_resource, input))
    }

    fn request_resource(&mut self, resource: &DigestLockedPackageRef) -> Option<ResourceRef> {
        if !is_lowercase_sha256_review_digest(&resource.digest) {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "external-action resource digest must use lowercase sha256 review rendering",
                resource.span,
            ));
            return None;
        }
        Some(ResourceRef {
            coordinate: package_coordinate(&resource.package.path, &resource.package.version),
            digest: Some(resource.digest.clone()),
        })
    }

    fn check_require_stmt(
        &mut self,
        predicate: &Expr,
        arm: &RequireElseArm,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        state: &mut BodyState,
    ) {
        let Some(predicate) = self.check_predicate(predicate, env) else {
            return;
        };
        let Some(arm) = self.check_require_else_arm(arm, env) else {
            return;
        };
        state.nodes.push(CoreNode::Require { predicate, arm });
    }

    fn check_require_else_arm(
        &mut self,
        arm: &RequireElseArm,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
    ) -> Option<CoreRequireFailureArm> {
        match arm {
            RequireElseArm::Terminal(target) => {
                let reason = self.check_terminal_require_reason(target, env)?;
                Some(CoreRequireFailureArm::Terminal { reason })
            }
            RequireElseArm::ContinueObstructed(obstruction) => {
                let reason_kind =
                    self.check_reason_kind(&obstruction.reason, env, obstruction.span)?;
                let payload = self.check_reason_payload(
                    &obstruction.payload,
                    env,
                    ReasonPayloadMode::SkipReasonField,
                )?;
                Some(CoreRequireFailureArm::ContinueObstructed {
                    reason: CoreObstructionReason {
                        kind: reason_kind,
                        payload,
                    },
                })
            }
        }
    }

    fn check_terminal_require_reason(
        &mut self,
        target: &ObstructionTarget,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
    ) -> Option<CoreObstructionReason> {
        if self.reject_capability_obstruction_root(&target.coordinate, target.span) {
            return None;
        }
        let payload = match &target.payload {
            Some(Expr::Record { entries, .. }) => {
                self.check_reason_payload(entries, env, ReasonPayloadMode::PreserveReasonField)?
            }
            Some(_) => {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "terminal require obstruction payloads must be record values",
                    target.span,
                ));
                return None;
            }
            None => BTreeMap::new(),
        };
        Some(CoreObstructionReason {
            kind: path_key(&target.coordinate),
            payload,
        })
    }

    fn check_reason_kind(
        &mut self,
        reason: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        span: Span,
    ) -> Option<String> {
        let Some(kind) = plain_path_coordinate(reason) else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "obstruction reason must be a stable coordinate",
                span,
            ));
            return None;
        };
        if let Some(root) = plain_path_root(reason) {
            if self.capability_import_alias(root) {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "capability imports are legal only as request operations",
                    span,
                ));
                return None;
            }
            if env.contains_key(root) {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnsupportedSourceShape,
                    "obstruction reason must be a stable coordinate, not a local expression",
                    span,
                ));
                return None;
            }
        }
        Some(kind)
    }

    fn check_reason_payload(
        &mut self,
        entries: &[RecordEntry],
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        mode: ReasonPayloadMode,
    ) -> Option<BTreeMap<String, CoreExpr>> {
        let mut fields = BTreeMap::new();
        let mut accepted = true;
        for entry in entries {
            match entry {
                RecordEntry::Field { name, .. } | RecordEntry::Shorthand { name, .. }
                    if mode == ReasonPayloadMode::SkipReasonField && name == "reason" => {}
                RecordEntry::Field { name, value } => {
                    if fields.contains_key(name) {
                        self.errors.push(error(
                            CompilerStage::TypeCheck,
                            CompilerErrorKind::DuplicateObstructionPayloadField,
                            format!("obstruction reason payload repeats field `{name}`"),
                            expr_span(value),
                        ));
                        accepted = false;
                        continue;
                    }
                    if let Some(value) = self.check_expr(value, env) {
                        fields.insert(name.clone(), value.expr);
                    } else {
                        accepted = false;
                    }
                }
                RecordEntry::Shorthand { name, span } => {
                    if fields.contains_key(name) {
                        self.errors.push(error(
                            CompilerStage::TypeCheck,
                            CompilerErrorKind::DuplicateObstructionPayloadField,
                            format!("obstruction reason payload repeats field `{name}`"),
                            *span,
                        ));
                        accepted = false;
                        continue;
                    }
                    let Some((local, _)) = env.get(name) else {
                        self.errors.push(error(
                            CompilerStage::TypeCheck,
                            CompilerErrorKind::UnresolvedType,
                            format!("obstruction reason payload field `{name}` has no binding"),
                            *span,
                        ));
                        accepted = false;
                        continue;
                    };
                    fields.insert(
                        name.clone(),
                        CoreExpr::Local {
                            reference: local.clone(),
                        },
                    );
                }
                RecordEntry::Spread(value) => {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::UnsupportedSourceShape,
                        "obstruction reason payload spreads are outside the Core subset",
                        expr_span(value),
                    ));
                    accepted = false;
                }
            }
        }
        if accepted {
            Some(fields)
        } else {
            None
        }
    }

    fn check_let_stmt(
        &mut self,
        intent: &ResolvedIntent,
        stmt: LetStatement<'_>,
        env: &mut BTreeMap<String, (LocalRef, TypeShape)>,
        locals: &mut Vec<LocalRef>,
        state: &mut BodyState,
    ) {
        if let Some(handler) = stmt.handler {
            self.check_effectful_let(intent, stmt, handler, env, locals, state);
        } else {
            self.check_pure_let(intent, stmt, env, locals, state);
        }
    }

    fn check_pure_let(
        &mut self,
        intent: &ResolvedIntent,
        stmt: LetStatement<'_>,
        env: &mut BTreeMap<String, (LocalRef, TypeShape)>,
        locals: &mut Vec<LocalRef>,
        state: &mut BodyState,
    ) {
        if !self.check_known_effect_profiles(intent, stmt.value) {
            return;
        }
        let annotation_shape = match stmt.ty {
            Some(annotation) => match self.type_ref_shape(annotation, stmt.span, None) {
                Some(shape) => Some(shape),
                None => return,
            },
            None => None,
        };
        let Some(value) = self.check_expr_with_expected(stmt.value, env, annotation_shape.as_ref())
        else {
            return;
        };
        let Some(binding_shape) = self.pure_let_binding_shape(&stmt, &value, annotation_shape)
        else {
            return;
        };
        let local = next_local(&mut state.local_index, binding_shape.coord.clone());
        state.nodes.push(CoreNode::Let {
            binding: local.clone(),
            value: value.expr,
        });
        locals.push(local.clone());
        env.insert(stmt.name.to_owned(), (local, binding_shape));
    }

    fn pure_let_binding_shape(
        &mut self,
        stmt: &LetStatement<'_>,
        value: &TypedValue,
        annotation_shape: Option<TypeShape>,
    ) -> Option<TypeShape> {
        let Some(annotation_shape) = annotation_shape else {
            return Some(value.ty.clone());
        };
        if compatible(&annotation_shape, &value.ty) {
            Some(annotation_shape)
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                format!("`{}` initializer does not match annotation", stmt.name),
                stmt.span,
            ));
            None
        }
    }

    fn check_effectful_let(
        &mut self,
        intent: &ResolvedIntent,
        stmt: LetStatement<'_>,
        handler: &ObstructionHandler,
        env: &mut BTreeMap<String, (LocalRef, TypeShape)>,
        locals: &mut Vec<LocalRef>,
        state: &mut BodyState,
    ) {
        if !self.check_effect_profile(intent, stmt.value, stmt.span) {
            return;
        }
        let Some(binding_shape) = self.effect_binding_shape(stmt.ty, stmt.span) else {
            return;
        };
        let Some((effect, input)) = self.check_effect_call(stmt.value, env, stmt.span) else {
            return;
        };
        let local = next_local(&mut state.local_index, binding_shape.coord.clone());
        locals.push(local.clone());
        let Some(obstruction_map) = self.check_obstruction_handler(
            handler,
            &effect,
            &mut state.obstruction_index,
            locals,
            stmt.span,
        ) else {
            return;
        };
        state.nodes.push(CoreNode::Effect {
            binding: local.clone(),
            effect,
            input,
            obstruction_map,
        });
        env.insert(stmt.name.to_owned(), (local, binding_shape));
    }

    fn check_effect_profile(&mut self, intent: &ResolvedIntent, call: &Expr, span: Span) -> bool {
        if let Some(alias) = self.capability_import_alias_for_call(call) {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                format!(
                    "external-action capability `{alias}` is request data, not a semantic effect"
                ),
                span,
            ));
            return false;
        }
        let Some(effect) = effect_coordinate(call) else {
            self.unsupported_stmt(span, "effect call");
            return false;
        };
        let Some(write_class) = self.resolved.effect_write_classes.get(&effect) else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::MissingContextFact,
                format!("effect `{effect}` has no compiler context fact"),
                span,
            ));
            return false;
        };
        let Some(allowed_write_classes) = &intent.allowed_write_classes else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::MissingContextFact,
                format!(
                    "operation profile `{}` has no write-class compiler context fact",
                    intent.profile
                ),
                span,
            ));
            return false;
        };
        if allowed_write_classes.contains(write_class) {
            true
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::ProfileEffectMismatch,
                format!(
                    "effect `{effect}` requires write class {write_class:?}, which profile `{}` does not allow",
                    intent.profile
                ),
                span,
            ));
            false
        }
    }

    fn capability_import_alias_for_call(&self, call: &Expr) -> Option<String> {
        let Expr::Call { callee, .. } = call else {
            return None;
        };
        let alias = plain_path_root(callee)?;
        self.resolved
            .imports
            .iter()
            .any(|import| {
                import.kind == CoreImportKind::Capability && import.alias.as_deref() == Some(alias)
            })
            .then(|| alias.to_owned())
    }

    fn effect_binding_shape(&mut self, ty: Option<&TypeRef>, span: Span) -> Option<TypeShape> {
        let Some(ty) = ty else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "effectful `let ... else` requires an explicit result type annotation",
                span,
            ));
            return None;
        };
        self.type_ref_shape(ty, span, None)
    }

    fn check_effect_call(
        &mut self,
        call: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        span: Span,
    ) -> Option<(String, CoreExpr)> {
        let effect = effect_coordinate(call)?;
        let Expr::Call {
            args, type_args, ..
        } = call
        else {
            self.unsupported_stmt(span, "effect call");
            return None;
        };
        if !type_args.is_empty() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "effectful `let ... else` does not support effect type arguments",
                span,
            ));
            return None;
        }
        let [arg] = args.as_slice() else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "effectful `let ... else` supports exactly one effect argument",
                span,
            ));
            return None;
        };
        let input = self.check_expr(arg, env)?;
        Some((effect, input.expr))
    }

    fn check_obstruction_handler(
        &mut self,
        handler: &ObstructionHandler,
        effect: &str,
        obstruction_index: &mut usize,
        locals: &mut Vec<LocalRef>,
        span: Span,
    ) -> Option<BTreeMap<String, CoreObstructionArm>> {
        let ObstructionHandler::Map(arms) = handler else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "effectful `let ... else` supports obstruction maps only",
                span,
            ));
            return None;
        };
        let mut arms_by_failure = BTreeMap::new();
        let mut accepted = true;
        for arm in arms {
            if arms_by_failure.insert(arm.failure.as_str(), arm).is_some() {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::DuplicateObstructionFailure,
                    format!(
                        "obstruction map repeats failure key `{}` in effect `{effect}`",
                        arm.failure
                    ),
                    arm.span,
                ));
                accepted = false;
            }
        }
        if !accepted {
            return None;
        }

        let mut out = BTreeMap::new();
        for arm in arms_by_failure.values() {
            let Some((failure, obstruction_arm)) =
                self.check_obstruction_arm(effect, arm, *obstruction_index)
            else {
                continue;
            };
            *obstruction_index += 1;
            locals.push(obstruction_arm.binder.clone());
            out.insert(failure, obstruction_arm);
        }
        if out.len() == arms.len() {
            Some(out)
        } else {
            None
        }
    }

    fn check_obstruction_arm(
        &mut self,
        effect: &str,
        arm: &ObstructionArm,
        obstruction_index: usize,
    ) -> Option<(String, CoreObstructionArm)> {
        if arm.binder.is_none() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "effect obstruction map arms require binders in the first lowerable subset",
                arm.span,
            ));
            return None;
        }
        let binder = LocalRef {
            id: format!("obstruction.{obstruction_index}"),
            alpha_name: format!("$obstruction{obstruction_index}"),
            ty: format!("{effect}.{}", arm.failure),
        };
        let value = self.check_obstruction_target(&arm.target)?;
        Some((arm.failure.clone(), CoreObstructionArm { binder, value }))
    }

    fn check_obstruction_target(&mut self, target: &ObstructionTarget) -> Option<CoreExpr> {
        if self.reject_capability_obstruction_root(&target.coordinate, target.span) {
            return None;
        }
        if target.payload.is_some() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "obstruction payload expressions are outside the first effectful Core subset",
                target.span,
            ));
            return None;
        }
        Some(CoreExpr::Call {
            callee: path_key(&target.coordinate),
            type_args: Vec::new(),
            args: Vec::new(),
        })
    }

    fn reject_capability_obstruction_root(&mut self, coordinate: &[String], span: Span) -> bool {
        if coordinate
            .first()
            .is_some_and(|root| self.capability_import_alias(root))
        {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "capability imports are legal only as request operations",
                span,
            ));
            true
        } else {
            false
        }
    }

    fn capability_import_alias(&self, alias: &str) -> bool {
        self.resolved.imports.iter().any(|import| {
            import.kind == CoreImportKind::Capability && import.alias.as_deref() == Some(alias)
        })
    }

    fn check_known_effect_profiles(&mut self, intent: &ResolvedIntent, expr: &Expr) -> bool {
        let mut accepted = true;
        if let Some(effect) = effect_coordinate(expr) {
            if self.resolved.effect_write_classes.contains_key(&effect) {
                accepted &= self.check_effect_profile(intent, expr, expr_span(expr));
            }
        }
        match expr {
            Expr::Ident { .. }
            | Expr::Int { .. }
            | Expr::Str { .. }
            | Expr::Bool { .. }
            | Expr::Digest { .. } => {}
            Expr::Field { base, .. } => {
                accepted &= self.check_known_effect_profiles(intent, base);
            }
            Expr::Call { callee, args, .. } => {
                accepted &= self.check_known_effect_profiles(intent, callee);
                for arg in args {
                    accepted &= self.check_known_effect_profiles(intent, arg);
                }
            }
            Expr::Unary { operand, .. } => {
                accepted &= self.check_known_effect_profiles(intent, operand);
            }
            Expr::Binary { lhs, rhs, .. } => {
                accepted &= self.check_known_effect_profiles(intent, lhs);
                accepted &= self.check_known_effect_profiles(intent, rhs);
            }
            Expr::Record { entries, .. } => {
                for entry in entries {
                    accepted &= self.check_known_effect_profiles_in_record_entry(intent, entry);
                }
            }
            Expr::If {
                cond, then, els, ..
            } => {
                accepted &= self.check_known_effect_profiles(intent, cond);
                accepted &= self.check_known_effect_profiles(intent, then);
                accepted &= self.check_known_effect_profiles(intent, els);
            }
            Expr::IfYield {
                pred,
                then_block,
                else_block,
                ..
            } => {
                accepted &= self.check_known_effect_profiles(intent, pred);
                accepted &= self.check_known_effect_profiles_in_yield_block(intent, then_block);
                accepted &= self.check_known_effect_profiles_in_yield_block(intent, else_block);
            }
            Expr::VariantLit { payload, .. } => {
                if let Some(payload) = payload {
                    accepted &= self.check_known_effect_profiles(intent, payload);
                }
            }
            Expr::Match {
                scrutinee, arms, ..
            } => {
                accepted &= self.check_known_effect_profiles(intent, scrutinee);
                for arm in arms {
                    accepted &= self.check_known_effect_profiles(intent, &arm.body);
                }
            }
        }
        accepted
    }

    fn check_known_effect_profiles_in_record_entry(
        &mut self,
        intent: &ResolvedIntent,
        entry: &RecordEntry,
    ) -> bool {
        match entry {
            RecordEntry::Field { value, .. } | RecordEntry::Spread(value) => {
                self.check_known_effect_profiles(intent, value)
            }
            RecordEntry::Shorthand { .. } => true,
        }
    }

    fn check_known_effect_profiles_in_yield_block(
        &mut self,
        intent: &ResolvedIntent,
        block: &YieldBlock,
    ) -> bool {
        let mut accepted = true;
        for stmt in &block.stmts {
            accepted &= self.check_known_effect_profiles_in_stmt(intent, stmt);
        }
        accepted &= self.check_known_effect_profiles(intent, &block.value);
        accepted
    }

    fn check_known_effect_profiles_in_block(
        &mut self,
        intent: &ResolvedIntent,
        block: &Block,
    ) -> bool {
        let mut accepted = true;
        for stmt in &block.stmts {
            accepted &= self.check_known_effect_profiles_in_stmt(intent, stmt);
        }
        accepted
    }

    fn check_known_effect_profiles_in_stmt(
        &mut self,
        intent: &ResolvedIntent,
        stmt: &Stmt,
    ) -> bool {
        match stmt {
            Stmt::Let {
                value, els, span, ..
            } => {
                if els.is_some() {
                    self.check_effect_profile(intent, value, *span)
                } else {
                    self.check_known_effect_profiles(intent, value)
                }
            }
            Stmt::Effect { call, span, .. } => self.check_effect_profile(intent, call, *span),
            Stmt::ExternalActionRequest {
                operation,
                authority_scope,
                basis,
                max_settlement_bytes,
                max_attempts,
                ..
            } => {
                let mut accepted = self.check_known_effect_profiles(intent, operation);
                accepted &= self.check_known_effect_profiles(intent, authority_scope);
                accepted &= self.check_known_effect_profiles(intent, basis);
                accepted &= self.check_known_effect_profiles(intent, max_settlement_bytes);
                accepted &= self.check_known_effect_profiles(intent, max_attempts);
                accepted
            }
            Stmt::Require { predicate, .. }
            | Stmt::Guarantee { predicate, .. }
            | Stmt::Assert { predicate, .. } => self.check_known_effect_profiles(intent, predicate),
            Stmt::If {
                cond,
                then_block,
                els,
                ..
            } => {
                let mut accepted = self.check_known_effect_profiles(intent, cond);
                accepted &= self.check_known_effect_profiles_in_block(intent, then_block);
                if let Some(els) = els {
                    accepted &= self.check_known_effect_profiles_in_else(intent, els);
                }
                accepted
            }
            Stmt::For { iter, body, .. } => {
                let mut accepted = self.check_known_effect_profiles(intent, iter);
                accepted &= self.check_known_effect_profiles_in_block(intent, body);
                accepted
            }
            Stmt::Return { value, .. } => self.check_known_effect_profiles(intent, value),
        }
    }

    fn check_known_effect_profiles_in_else(
        &mut self,
        intent: &ResolvedIntent,
        els: &ElseClause,
    ) -> bool {
        match els {
            ElseClause::Block(block) => self.check_known_effect_profiles_in_block(intent, block),
            ElseClause::If(stmt) => self.check_known_effect_profiles_in_stmt(intent, stmt),
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
        let (left, right) = match (is_bare_integer_literal(lhs), is_bare_integer_literal(rhs)) {
            (true, false) => {
                let right = self.check_expr(rhs, env)?;
                let left = self.check_expr_with_expected(lhs, env, Some(&right.ty))?;
                (left, right)
            }
            (false, true) => {
                let left = self.check_expr(lhs, env)?;
                let right = self.check_expr_with_expected(rhs, env, Some(&left.ty))?;
                (left, right)
            }
            _ => (self.check_expr(lhs, env)?, self.check_expr(rhs, env)?),
        };
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
        self.check_expr_with_expected(expr, env, None)
    }

    fn check_expr_with_expected(
        &mut self,
        expr: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        expected: Option<&TypeShape>,
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
            Expr::Int {
                value,
                suffix,
                span,
            } => self.check_integer_literal(value, *suffix, expected, *span),
            Expr::Unary {
                op: UnOp::Neg,
                operand,
                span,
            } => {
                let Expr::Int { value, suffix, .. } = operand.as_ref() else {
                    self.errors.push(error(
                        CompilerStage::TypeCheck,
                        CompilerErrorKind::UnsupportedSourceShape,
                        "negated expression is outside the initial lowerable subset",
                        *span,
                    ));
                    return None;
                };
                self.check_integer_literal(&format!("-{value}"), *suffix, expected, *span)
            }
            Expr::Field { base, field, span } => self.check_field(base, field, *span, env),
            Expr::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
                span,
            } => self.check_string_concat(lhs, rhs, env, *span),
            Expr::Record { entries, span } => self.check_record(entries, env, expected, *span),
            Expr::Call {
                callee,
                type_args,
                args,
                span,
            } => self.check_pure_call(callee, type_args, args, env, expected, *span),
            Expr::If {
                cond,
                then,
                els,
                span,
            } => self.check_pure_conditional(cond, then, els, env, expected, *span),
            Expr::Binary { .. }
            | Expr::Digest { .. }
            | Expr::Unary { .. }
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

    fn check_pure_call(
        &mut self,
        callee: &Expr,
        type_args: &[TypeRef],
        args: &[Expr],
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        expected: Option<&TypeShape>,
        span: Span,
    ) -> Option<TypedValue> {
        let (source_coordinate, fact) = self.resolve_pure_function(callee, span)?;
        if !type_args.is_empty() || !fact.type_parameters.is_empty() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "generic pure-helper calls are outside the initial lowerable subset",
                span,
            ));
            return None;
        }
        if args.len() != fact.parameter_types.len() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                format!(
                    "pure helper `{source_coordinate}` expects {} arguments, got {}",
                    fact.parameter_types.len(),
                    args.len()
                ),
                span,
            ));
            return None;
        }

        let mut core_args = Vec::with_capacity(args.len());
        for (arg, parameter_type) in args.iter().zip(&fact.parameter_types) {
            let Some(parameter_shape) =
                self.shape_for_helper_coordinate(parameter_type, &fact.lawpack)
            else {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::UnresolvedType,
                    format!(
                        "pure helper `{source_coordinate}` parameter type `{parameter_type}` is not available in the module type closure"
                    ),
                    span,
                ));
                return None;
            };
            let value = self.check_expr_with_expected(arg, env, Some(&parameter_shape))?;
            if !compatible(&parameter_shape, &value.ty) {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::TypeMismatch,
                    format!("argument does not match pure helper `{source_coordinate}` signature"),
                    expr_span(arg),
                ));
                return None;
            }
            core_args.push(value.expr);
        }

        let Some(return_shape) = self.shape_for_helper_coordinate(&fact.return_type, &fact.lawpack)
        else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnresolvedType,
                format!(
                    "pure helper `{source_coordinate}` return type `{}` is not available in the module type closure",
                    fact.return_type
                ),
                span,
            ));
            return None;
        };
        if expected.is_some_and(|expected| !compatible(expected, &return_shape)) {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                format!("pure helper `{source_coordinate}` return type does not match its use"),
                span,
            ));
            return None;
        }

        Some(TypedValue {
            expr: CoreExpr::Call {
                callee: fact.coordinate,
                type_args: Vec::new(),
                args: core_args,
            },
            ty: return_shape,
        })
    }

    fn helper_is_owned(&self, fact: &PureFunctionFact) -> bool {
        self.lawpack_is_imported(&fact.lawpack)
            && coordinate_is_below_lawpack(&fact.coordinate, &fact.lawpack)
    }

    fn lawpack_is_imported(&self, lawpack: &ResourceRef) -> bool {
        self.resolved
            .imports
            .iter()
            .any(|import| import.kind == CoreImportKind::Lawpack && &import.resource == lawpack)
    }

    fn resolve_pure_function(
        &mut self,
        callee: &Expr,
        span: Span,
    ) -> Option<(String, PureFunctionFact)> {
        let Some(source_coordinate) = plain_callee_coordinate(callee) else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnresolvedFunction,
                "pure-helper callee is not a stable imported coordinate",
                span,
            ));
            return None;
        };
        let Some(fact) = self
            .resolved
            .pure_functions
            .get(&source_coordinate)
            .cloned()
        else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnresolvedFunction,
                format!("pure helper `{source_coordinate}` has no compiler context fact"),
                span,
            ));
            return None;
        };
        if !self.helper_is_owned(&fact) {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnresolvedFunction,
                format!(
                    "pure helper `{source_coordinate}` is not owned by an exact imported lawpack"
                ),
                span,
            ));
            return None;
        }
        Some((source_coordinate, fact))
    }

    fn shape_for_coordinate(&self, coordinate: &str) -> Option<TypeShape> {
        if coordinate == "Bool" {
            return Some(TypeShape {
                coord: "Bool".to_owned(),
                kind: TypeKind::Bool,
            });
        }
        if let Some(width) = builtin_integer_width(coordinate) {
            return Some(integer_shape(width));
        }
        self.named_types
            .values()
            .find(|shape| shape.coord == coordinate)
            .cloned()
    }

    fn shape_for_helper_coordinate(
        &self,
        coordinate: &str,
        lawpack: &ResourceRef,
    ) -> Option<TypeShape> {
        if coordinate == "Bool" || builtin_integer_width(coordinate).is_some() {
            return self.shape_for_coordinate(coordinate);
        }
        self.resolved
            .type_shapes
            .get(coordinate)
            .filter(|fact| {
                &fact.lawpack == lawpack
                    && coordinate_is_below_lawpack(&fact.coordinate, &fact.lawpack)
            })
            .and_then(|fact| self.shape_from_imported_type(fact))
    }

    fn shape_from_imported_type(&self, fact: &TypeShapeFact) -> Option<TypeShape> {
        let mut resolving = BTreeSet::from([fact.coordinate.clone()]);
        let mut shape = imported_type_definition_shape(
            &fact.definition,
            &self.resolved.type_shapes,
            &fact.lawpack,
            &mut resolving,
        )?;
        shape.coord.clone_from(&fact.coordinate);
        Some(shape)
    }

    fn check_pure_conditional(
        &mut self,
        cond: &Expr,
        then: &Expr,
        els: &Expr,
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        expected: Option<&TypeShape>,
        span: Span,
    ) -> Option<TypedValue> {
        let predicate = self.check_predicate(cond, env)?;
        let (then_value, else_value) = if expected.is_none() && is_bare_integer_literal(then) {
            let else_value = self.check_expr_with_expected(els, env, None)?;
            let then_value = self.check_expr_with_expected(then, env, Some(&else_value.ty))?;
            (then_value, else_value)
        } else {
            let then_value = self.check_expr_with_expected(then, env, expected)?;
            let branch_expectation = expected.unwrap_or(&then_value.ty);
            let else_value = self.check_expr_with_expected(els, env, Some(branch_expectation))?;
            (then_value, else_value)
        };

        let ty = if let Some(expected) = expected {
            if !compatible(expected, &then_value.ty) || !compatible(expected, &else_value.ty) {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::TypeMismatch,
                    "conditional branches do not match the expected type",
                    span,
                ));
                return None;
            }
            expected.clone()
        } else if compatible(&then_value.ty, &else_value.ty) {
            then_value.ty.clone()
        } else if compatible(&else_value.ty, &then_value.ty) {
            else_value.ty.clone()
        } else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "conditional branches do not have a compatible bounded type",
                span,
            ));
            return None;
        };

        Some(TypedValue {
            expr: CoreExpr::If {
                predicate: Box::new(predicate),
                then_value: Box::new(then_value.expr),
                else_value: Box::new(else_value.expr),
            },
            ty,
        })
    }

    fn check_integer_literal(
        &mut self,
        value: &str,
        suffix: Option<IntSuffix>,
        expected: Option<&TypeShape>,
        span: Span,
    ) -> Option<TypedValue> {
        let width = if let Some(suffix) = suffix {
            integer_suffix_width(suffix)
        } else {
            let Some(TypeShape {
                kind: TypeKind::Int { width },
                ..
            }) = expected
            else {
                self.errors.push(error(
                    CompilerStage::TypeCheck,
                    CompilerErrorKind::TypeMismatch,
                    "bare integer literal has no unambiguous expected width",
                    span,
                ));
                return None;
            };
            width
        };
        if parse_core_integer(width, value).is_none() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                format!("integer literal is outside the exact `{width}` domain"),
                span,
            ));
            return None;
        }
        Some(TypedValue {
            expr: CoreExpr::Const(CoreValue::Int {
                width: width.to_owned(),
                value: value.to_owned(),
            }),
            ty: integer_shape(width),
        })
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
        expected: Option<&TypeShape>,
        span: Span,
    ) -> Option<TypedValue> {
        let expected_fields = expected.and_then(|shape| match &shape.kind {
            TypeKind::Record(fields) => Some(fields),
            _ => None,
        });
        let mut fields = BTreeMap::new();
        let mut field_types = BTreeMap::new();
        for entry in entries {
            match entry {
                RecordEntry::Field { name, value } => {
                    let field_expected = expected_fields.and_then(|fields| fields.get(name));
                    let value = self.check_expr_with_expected(value, env, field_expected)?;
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

fn is_bare_integer_literal(expr: &Expr) -> bool {
    match expr {
        Expr::Int { suffix: None, .. } => true,
        Expr::Unary {
            op: UnOp::Neg,
            operand,
            ..
        } => matches!(operand.as_ref(), Expr::Int { suffix: None, .. }),
        _ => false,
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

fn next_local(index: &mut usize, ty: String) -> LocalRef {
    let current = *index;
    *index += 1;
    LocalRef {
        id: format!("local.{current}"),
        alpha_name: format!("$local{current}"),
        ty,
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
        (TypeKind::Bool, TypeKind::Bool) => true,
        (TypeKind::Int { width: expected }, TypeKind::Int { width: actual }) => expected == actual,
        (TypeKind::Bytes { max: expected }, TypeKind::Bytes { max: actual }) => actual <= expected,
        (
            TypeKind::List {
                item: expected_item,
                max: expected_max,
            },
            TypeKind::List {
                item: actual_item,
                max: actual_max,
            },
        ) => actual_max <= expected_max && compatible(expected_item, actual_item),
        (
            TypeKind::ExternalActionRequest {
                settlement: expected,
            },
            TypeKind::ExternalActionRequest { settlement: actual },
        ) => compatible(expected, actual),
        _ => false,
    }
}

fn requestable_operation_family(coordinate: &str) -> bool {
    coordinate
        .split(['.', '@'])
        .next()
        .is_some_and(|root| root.eq_ignore_ascii_case("workspace"))
}

fn coordinate_is_below_lawpack(coordinate: &str, lawpack: &ResourceRef) -> bool {
    let prefix = format!("{}.", lawpack.coordinate);
    coordinate
        .strip_prefix(&prefix)
        .is_some_and(|suffix| !suffix.is_empty())
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

fn imported_type_definition_shape(
    definition: &str,
    type_shapes: &BTreeMap<String, TypeShapeFact>,
    lawpack: &ResourceRef,
    resolving: &mut BTreeSet<String>,
) -> Option<TypeShape> {
    if definition == "Bool" {
        return Some(TypeShape {
            coord: definition.to_owned(),
            kind: TypeKind::Bool,
        });
    }
    if let Some(width) = builtin_integer_width(definition) {
        return Some(integer_shape(width));
    }
    if let Some(inner) = definition
        .strip_prefix("String<max=")
        .and_then(|value| value.strip_suffix('>'))
    {
        let (max, canonical) = inner.split_once(",canonical=")?;
        if !matches!(canonical, "raw-utf8" | "unicode-scalar-nfc") {
            return None;
        }
        let max = max.parse().ok()?;
        return Some(TypeShape {
            coord: definition.to_owned(),
            kind: TypeKind::String {
                max,
                canonical: canonical.to_owned(),
            },
        });
    }
    if let Some(max) = definition
        .strip_prefix("Bytes<max=")
        .and_then(|value| value.strip_suffix('>'))
        .and_then(|value| value.parse().ok())
    {
        return Some(TypeShape {
            coord: definition.to_owned(),
            kind: TypeKind::Bytes { max },
        });
    }
    let inner = definition
        .strip_prefix("List<")
        .and_then(|value| value.strip_suffix('>'));
    if let Some(inner) = inner {
        let (item, max) = inner.rsplit_once(",max=")?;
        return Some(TypeShape {
            coord: definition.to_owned(),
            kind: TypeKind::List {
                item: Box::new(imported_type_definition_shape(
                    item,
                    type_shapes,
                    lawpack,
                    resolving,
                )?),
                max: max.parse().ok()?,
            },
        });
    }
    let fact = type_shapes
        .get(definition)
        .filter(|fact| &fact.lawpack == lawpack)?;
    if !resolving.insert(definition.to_owned()) {
        return None;
    }
    let mut shape =
        imported_type_definition_shape(&fact.definition, type_shapes, lawpack, resolving)?;
    resolving.remove(definition);
    shape.coord.clone_from(&fact.coordinate);
    Some(shape)
}

fn bytes_type_coord(max: u64) -> String {
    format!("Bytes<max={max}>")
}

fn list_type_coord(item: &str, max: u64) -> String {
    format!("List<{item},max={max}>")
}

fn empty_core_block() -> CoreBlock {
    CoreBlock {
        locals: Vec::new(),
        nodes: Vec::new(),
        result: CoreExpr::Const(CoreValue::Null),
    }
}

fn builtin_integer_width(name: &str) -> Option<&'static str> {
    match name {
        "I32" => Some("I32"),
        "I64" => Some("I64"),
        "U32" => Some("U32"),
        "U64" => Some("U64"),
        _ => None,
    }
}

const fn integer_suffix_width(suffix: IntSuffix) -> &'static str {
    match suffix {
        IntSuffix::I32 => "I32",
        IntSuffix::I64 => "I64",
        IntSuffix::U32 => "U32",
        IntSuffix::U64 => "U64",
    }
}

fn integer_shape(width: &str) -> TypeShape {
    TypeShape {
        coord: width.to_owned(),
        kind: TypeKind::Int {
            width: width.to_owned(),
        },
    }
}

fn path_key(path: &[String]) -> String {
    path.join(".")
}

fn effect_coordinate(call: &Expr) -> Option<String> {
    if let Expr::Call { callee, .. } = call {
        plain_callee_coordinate(callee)
    } else {
        None
    }
}

fn plain_callee_coordinate(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Ident { name, .. } => Some(name.clone()),
        Expr::Field { base, field, .. } => {
            Some(format!("{}.{}", plain_callee_coordinate(base)?, field))
        }
        Expr::Call { .. }
        | Expr::Int { .. }
        | Expr::Str { .. }
        | Expr::Bool { .. }
        | Expr::Digest { .. }
        | Expr::Unary { .. }
        | Expr::Binary { .. }
        | Expr::Record { .. }
        | Expr::If { .. }
        | Expr::IfYield { .. }
        | Expr::VariantLit { .. }
        | Expr::Match { .. } => None,
    }
}

fn plain_path_coordinate(expr: &Expr) -> Option<String> {
    plain_callee_coordinate(expr)
}

fn plain_path_root(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Ident { name, .. } => Some(name),
        Expr::Field { base, .. } => plain_path_root(base),
        Expr::Call { .. }
        | Expr::Int { .. }
        | Expr::Str { .. }
        | Expr::Bool { .. }
        | Expr::Digest { .. }
        | Expr::Unary { .. }
        | Expr::Binary { .. }
        | Expr::Record { .. }
        | Expr::If { .. }
        | Expr::IfYield { .. }
        | Expr::VariantLit { .. }
        | Expr::Match { .. } => None,
    }
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
