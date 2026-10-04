//! Independently validate a byte-length primitive at the untrusted Core boundary.
use super::{
    expression_has_closed_authority, expression_type_coordinate, resolved_core_type, BTreeMap,
    CoreExpr, CoreModule, CoreType, LocalRef, TargetPureFunctionFact,
};

pub(super) const OPERATION: &str = "core.bytes.length";

pub(super) fn type_coordinate(
    core: &CoreModule,
    functions: &[TargetPureFunctionFact],
    type_args: &[String],
    args: &[CoreExpr],
    available: &BTreeMap<&str, &LocalRef>,
) -> Option<String> {
    let ([coordinate], [operand]) = (type_args, args) else {
        return None;
    };
    if !matches!(
        resolved_core_type(core, coordinate),
        Some(CoreType::Bytes { .. })
    ) || !expression_has_closed_authority(core, functions, operand, available)
        || expression_type_coordinate(core, functions, operand, available).as_ref()
            != Some(coordinate)
    {
        return None;
    }
    Some("U64".into())
}
