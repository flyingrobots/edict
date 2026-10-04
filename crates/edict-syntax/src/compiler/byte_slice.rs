//! Proven half-open slicing over bounded raw bytes.
use super::{
    error, integer_shape, unsigned_subtraction::proven_order, BTreeMap, CompilerErrorKind,
    CompilerStage, CoreExpr, Expr, LocalRef, Span, TypeChecker, TypeKind, TypeRef, TypeShape,
    TypedValue,
};

impl TypeChecker<'_> {
    pub(super) fn check_byte_slice(
        &mut self,
        type_args: &[TypeRef],
        args: &[Expr],
        env: &BTreeMap<String, (LocalRef, TypeShape)>,
        span: Span,
    ) -> Option<TypedValue> {
        let [bytes, start, end] = args else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "slice requires bounded bytes and two U64 endpoints",
                span,
            ));
            return None;
        };
        if !type_args.is_empty() {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "slice does not accept authored type arguments",
                span,
            ));
            return None;
        }
        let bytes = self.check_expr(bytes, env)?;
        let TypeKind::Bytes { max, .. } = bytes.ty.kind else {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "slice requires bounded structural Bytes",
                span,
            ));
            return None;
        };
        let index_type = integer_shape("U64");
        let start = self.check_expr_with_expected(start, env, Some(&index_type))?;
        let end = self.check_expr_with_expected(end, env, Some(&index_type))?;
        if start.ty != index_type || end.ty != index_type {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::TypeMismatch,
                "slice endpoints must have type U64",
                span,
            ));
            return None;
        }
        let coordinate = bytes.ty.value_type_coord();
        let length = CoreExpr::Call {
            callee: "core.bytes.length".into(),
            type_args: vec![coordinate.clone()],
            args: vec![bytes.expr.clone()],
        };
        if !proven_order(&end.expr, &start.expr, &self.input_proof_constraints)
            || !proven_order(&length, &end.expr, &self.input_proof_constraints)
        {
            self.errors.push(error(
                CompilerStage::TypeCheck,
                CompilerErrorKind::UnsupportedSourceShape,
                "slice requires proof of start <= end <= len(bytes) before evaluation",
                span,
            ));
            return None;
        }
        Some(TypedValue {
            expr: CoreExpr::Call {
                callee: "core.bytes.slice".into(),
                type_args: vec![coordinate],
                args: vec![bytes.expr, start.expr, end.expr],
            },
            ty: TypeShape::canonical_structural(TypeKind::Bytes { min: None, max })?,
        })
    }
}
