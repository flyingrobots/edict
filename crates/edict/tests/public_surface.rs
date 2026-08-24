use edict::{
    artifact::{
        digest_core_module, digest_result_projection, digest_target_ir_artifact, CoreDigest,
        ResultProjection, ResultProjectionArtifact, TargetIrArtifact,
    },
    check,
    diagnostic::{
        CompilerErrorKind, ParseErrorKind, ResultProjectionFailureKind, SemanticErrorKind,
        TargetLoweringFailureKind,
    },
    CheckOutcome,
};

#[test]
fn curated_facade_exposes_check_diagnostics_and_artifact_identity() {
    assert_eq!(
        check("package examples.public_surface@1;\n"),
        CheckOutcome::Valid
    );

    assert_eq!(std::mem::size_of_val(&digest_core_module), 0);
    assert_eq!(std::mem::size_of_val(&digest_target_ir_artifact), 0);
    assert_eq!(std::mem::size_of_val(&digest_result_projection), 0);

    let artifact_type_sizes = [
        std::mem::size_of::<CoreDigest>(),
        std::mem::size_of::<edict::artifact::CoreModule>(),
        std::mem::size_of::<TargetIrArtifact>(),
        std::mem::size_of::<ResultProjection>(),
        std::mem::size_of::<ResultProjectionArtifact>(),
    ];
    assert!(artifact_type_sizes.into_iter().all(|size| size > 0));

    let stable_failure_kind_sizes = [
        std::mem::size_of::<ParseErrorKind>(),
        std::mem::size_of::<SemanticErrorKind>(),
        std::mem::size_of::<CompilerErrorKind>(),
        std::mem::size_of::<TargetLoweringFailureKind>(),
        std::mem::size_of::<ResultProjectionFailureKind>(),
    ];
    assert!(stable_failure_kind_sizes.into_iter().all(|size| size > 0));
}

#[test]
fn implementation_modules_are_compile_fail_doctested() {
    let facade = include_str!("../src/lib.rs");
    assert!(facade.contains("```compile_fail\n//! use edict::parser::parse_module;"));
}
