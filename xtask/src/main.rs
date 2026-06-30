#![allow(clippy::print_stderr)]
#![allow(clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use edict_syntax::{
    assemble_contract_bundle, compile_to_core, digest_core_module, digest_target_ir_artifact,
    encode_core_module, encode_target_ir_artifact, lower_to_target_ir, parse_module,
    CompilerContext, ContractBundleAssemblyInput, ContractBundleSourceArtifact, CoreBudget,
    DigestLockedResource, ResourceRef, SuppliedTargetIrResource, TargetEffectLowering,
    TargetIrArtifact, TargetIrLoweringFacts, WriteClass, ECHO_DPO_TARGET_PROFILE,
    ECHO_SPAN_IR_DOMAIN, GITWARP_COMMIT_REDUCER_IR_DOMAIN, GITWARP_REF_CRDT_TARGET_PROFILE,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("contract-check") => contract_check(&repo_root()?),
        Some("core-goldens") => {
            let mode = match args.next().as_deref() {
                Some("--write") => CoreGoldenMode::Write,
                Some("--check") | None => CoreGoldenMode::Check,
                Some(flag) => return Err(format!("unknown core-goldens flag `{flag}`")),
            };
            if let Some(extra) = args.next() {
                return Err(format!("unexpected core-goldens argument `{extra}`"));
            }
            core_goldens(&repo_root()?, mode)
        }
        Some("bundle-goldens") => {
            let mode = match args.next().as_deref() {
                Some("--write") => BundleGoldenMode::Write,
                Some("--check") | None => BundleGoldenMode::Check,
                Some(flag) => return Err(format!("unknown bundle-goldens flag `{flag}`")),
            };
            if let Some(extra) = args.next() {
                return Err(format!("unexpected bundle-goldens argument `{extra}`"));
            }
            bundle_goldens(&repo_root()?, mode)
        }
        Some("target-ir-goldens") => {
            let mode = match args.next().as_deref() {
                Some("--write") => TargetIrGoldenMode::Write,
                Some("--check") | None => TargetIrGoldenMode::Check,
                Some(flag) => return Err(format!("unknown target-ir-goldens flag `{flag}`")),
            };
            if let Some(extra) = args.next() {
                return Err(format!("unexpected target-ir-goldens argument `{extra}`"));
            }
            target_ir_goldens(&repo_root()?, mode)
        }
        Some("verify") => verify(&repo_root()?),
        Some(cmd) => Err(format!("unknown xtask command `{cmd}`")),
        None => Err(
            "usage: cargo xtask <verify|contract-check|core-goldens|bundle-goldens|target-ir-goldens>"
                .into(),
        ),
    }
}

fn verify(root: &Path) -> Result<(), String> {
    run_cmd(root, "cargo", ["fmt", "--all", "--check"])?;
    run_cmd(
        root,
        "cargo",
        [
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run_cmd(root, "cargo", ["test", "--workspace", "--all-features"])?;
    run_cmd(
        root,
        "cargo",
        ["test", "--workspace", "--doc", "--all-features"],
    )?;
    core_goldens(root, CoreGoldenMode::Check)?;
    target_ir_goldens(root, TargetIrGoldenMode::Check)?;
    bundle_goldens(root, BundleGoldenMode::Check)?;
    contract_check(root)?;
    let base = diff_check_base(root)?;
    run_cmd(root, "git", ["diff", "--check", &format!("{base}...HEAD")])?;
    Ok(())
}

fn run_cmd<const N: usize>(root: &Path, program: &str, args: [&str; N]) -> Result<(), String> {
    let rendered = args.join(" ");
    println!("$ {program} {rendered}");
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|err| format!("failed to run `{program}`: {err}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("command failed: {program} {rendered}"))
    }
}

fn diff_check_base(root: &Path) -> Result<String, String> {
    choose_diff_check_base(|candidate| git_ref_exists(root, candidate))
}

fn choose_diff_check_base(
    mut exists: impl FnMut(&str) -> Result<bool, String>,
) -> Result<String, String> {
    for candidate in ["origin/main", "main", "HEAD^"] {
        if exists(candidate)? {
            return Ok(candidate.to_owned());
        }
    }
    Err("could not find diff-check base (tried origin/main, main, HEAD^)".into())
}

fn git_ref_exists(root: &Path, candidate: &str) -> Result<bool, String> {
    let status = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", candidate])
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("failed to inspect git ref `{candidate}`: {err}"))?;
    Ok(status.success())
}

fn contract_check(root: &Path) -> Result<(), String> {
    let docs = root.join("docs");
    let topics = docs.join("topics");
    let requirement_registry = read_to_string(&docs.join("REQUIREMENTS.md"))?;
    let mut tests = collect_rust_test_names(&root.join("crates"))?;
    tests.extend(collect_rust_test_names(&root.join("xtask"))?);

    let mut topic_count = 0usize;
    for topic in dirs(&topics)? {
        topic_count += 1;
        check_topic(root, &topic, &tests, &requirement_registry)?;
    }
    if topic_count == 0 {
        return Err("docs/topics contains no topic shelves".into());
    }

    let mut docs_to_link = vec![docs.join("README.md")];
    docs_to_link.extend(markdown_files(&topics)?);
    for doc in docs_to_link {
        check_links(root, &doc)?;
    }

    println!("contract-check: {topic_count} topic shelf(s) validated");
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoreGoldenMode {
    Check,
    Write,
}

#[derive(Debug)]
struct CoreGoldenCase {
    source: &'static str,
    bytes: &'static str,
    digest: &'static str,
}

const CORE_GOLDEN_CASES: &[CoreGoldenCase] = &[CoreGoldenCase {
    source: "fixtures/lang/bounds/bounded-hello.edict",
    bytes: "fixtures/core/canonical/bounded-hello.core.cbor",
    digest: "fixtures/core/canonical/bounded-hello.core.sha256",
}];

fn core_goldens(root: &Path, mode: CoreGoldenMode) -> Result<(), String> {
    for case in CORE_GOLDEN_CASES {
        check_or_write_core_golden(root, case, mode)?;
    }
    println!(
        "core-goldens: {} case(s) {}",
        CORE_GOLDEN_CASES.len(),
        match mode {
            CoreGoldenMode::Check => "checked",
            CoreGoldenMode::Write => "written",
        }
    );
    Ok(())
}

fn check_or_write_core_golden(
    root: &Path,
    case: &CoreGoldenCase,
    mode: CoreGoldenMode,
) -> Result<(), String> {
    let source = read_to_string(&root.join(case.source))?;
    let module = parse_module(&source).map_err(|err| format!("parse {}: {err}", case.source))?;
    let core = compile_to_core(&module, &core_golden_context())
        .map_err(|err| format!("compile {} to Core: {err:?}", case.source))?;
    let bytes = encode_core_module(&core)
        .map_err(|err| format!("encode {} as canonical Core: {err}", case.source))?;
    let digest = digest_core_module(&core)
        .map_err(|err| format!("digest {} as canonical Core: {err}", case.source))?;
    let digest_text = format!("{digest}\n");

    match mode {
        CoreGoldenMode::Check => {
            check_golden_file(root, case.bytes, &bytes)?;
            check_golden_file(root, case.digest, digest_text.as_bytes())?;
        }
        CoreGoldenMode::Write => {
            write_golden_file(&root.join(case.bytes), &bytes)?;
            write_golden_file(&root.join(case.digest), digest_text.as_bytes())?;
        }
    }
    Ok(())
}

fn core_golden_context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("hello.readOnly", "continuum.profile.read-only/v1")
        .with_budget(
            "hello.tinyBudget",
            CoreBudget {
                max_steps: 64,
                max_allocated_bytes: 4096,
                max_output_bytes: 1024,
            },
        )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TargetIrGoldenMode {
    Check,
    Write,
}

#[derive(Debug, Clone, Copy)]
enum TargetIrGoldenKind {
    Echo,
    Gitwarp,
}

#[derive(Debug)]
struct TargetIrGoldenCase {
    kind: TargetIrGoldenKind,
    bytes: &'static str,
    digest: &'static str,
}

const TARGET_IR_GOLDEN_CASES: &[TargetIrGoldenCase] = &[
    TargetIrGoldenCase {
        kind: TargetIrGoldenKind::Echo,
        bytes: "fixtures/target-ir/canonical/echo-effectful.target-ir.cbor",
        digest: "fixtures/target-ir/canonical/echo-effectful.target-ir.sha256",
    },
    TargetIrGoldenCase {
        kind: TargetIrGoldenKind::Gitwarp,
        bytes: "fixtures/target-ir/canonical/gitwarp-append.target-ir.cbor",
        digest: "fixtures/target-ir/canonical/gitwarp-append.target-ir.sha256",
    },
];

const TARGET_IR_ECHO_SOURCE: &str = "package a.b@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.effectful\n\
      basis none\n\
      budget <= p.tiny {\n\
      let receipt: Receipt = target.replace(input.id)\n\
        else { rejected(reason) => domain.WriteRejected };\n\
      return { id: input.id };\n\
    }";

const TARGET_IR_GITWARP_SOURCE: &str = "package a.git@1;\n\
    type Input = { id: String<max=16>, };\n\
    type Receipt = { id: String<max=16>, };\n\
    type Output = { id: String<max=16>, };\n\
    intent t(input: Input) returns Output\n\
      profile p.gitwarp\n\
      basis none\n\
      budget <= p.tiny\n\
      where input.id != \"\" {\n\
      let receipt: Receipt = gitwarp.appendEvent(input.id)\n\
        else { conflict(reason) => domain.MergeConflict };\n\
      return { id: receipt.id };\n\
    }";

fn target_ir_goldens(root: &Path, mode: TargetIrGoldenMode) -> Result<(), String> {
    for case in TARGET_IR_GOLDEN_CASES {
        check_or_write_target_ir_golden(root, case, mode)?;
    }
    println!(
        "target-ir-goldens: {} case(s) {}",
        TARGET_IR_GOLDEN_CASES.len(),
        match mode {
            TargetIrGoldenMode::Check => "checked",
            TargetIrGoldenMode::Write => "written",
        }
    );
    Ok(())
}

fn check_or_write_target_ir_golden(
    root: &Path,
    case: &TargetIrGoldenCase,
    mode: TargetIrGoldenMode,
) -> Result<(), String> {
    let artifact = target_ir_golden_artifact(case)?;
    let bytes = encode_target_ir_artifact(&artifact)
        .map_err(|err| format!("encode {:?} as canonical Target IR: {err}", case.kind))?;
    let digest = digest_target_ir_artifact(&artifact)
        .map_err(|err| format!("digest {:?} as canonical Target IR: {err}", case.kind))?;
    let digest_text = format!("{digest}\n");

    match mode {
        TargetIrGoldenMode::Check => {
            check_golden_file(root, case.bytes, &bytes)?;
            check_golden_file(root, case.digest, digest_text.as_bytes())?;
        }
        TargetIrGoldenMode::Write => {
            write_golden_file(&root.join(case.bytes), &bytes)?;
            write_golden_file(&root.join(case.digest), digest_text.as_bytes())?;
        }
    }
    Ok(())
}

fn target_ir_golden_artifact(case: &TargetIrGoldenCase) -> Result<TargetIrArtifact, String> {
    let (source, context, facts) = match case.kind {
        TargetIrGoldenKind::Echo => (
            TARGET_IR_ECHO_SOURCE,
            target_ir_echo_context(),
            target_ir_echo_facts(),
        ),
        TargetIrGoldenKind::Gitwarp => (
            TARGET_IR_GITWARP_SOURCE,
            target_ir_gitwarp_context(),
            target_ir_gitwarp_facts(),
        ),
    };
    let module = parse_module(source)
        .map_err(|err| format!("parse {:?} Target IR golden source: {err}", case.kind))?;
    let core = compile_to_core(&module, &context)
        .map_err(|err| format!("compile {:?} Target IR golden source: {err:?}", case.kind))?;
    let report = lower_to_target_ir(&core, &facts);
    report.artifact.ok_or_else(|| {
        format!(
            "lower {:?} Target IR golden source failed: {:?}",
            case.kind, report.failures
        )
    })
}

fn target_ir_echo_context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("p.effectful", "continuum.profile.write/v1")
        .with_operation_profile_write_classes("p.effectful", [WriteClass::Replace])
        .with_effect_write_class("target.replace", WriteClass::Replace)
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 8,
                max_allocated_bytes: 1024,
                max_output_bytes: 256,
            },
        )
}

fn target_ir_gitwarp_context() -> CompilerContext {
    CompilerContext::new()
        .with_operation_profile("p.gitwarp", "continuum.profile.append/v1")
        .with_operation_profile_write_classes("p.gitwarp", [WriteClass::Append])
        .with_effect_write_class("gitwarp.appendEvent", WriteClass::Append)
        .with_budget(
            "p.tiny",
            CoreBudget {
                max_steps: 13,
                max_allocated_bytes: 2048,
                max_output_bytes: 512,
            },
        )
}

fn target_ir_echo_facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: ECHO_DPO_TARGET_PROFILE.to_owned(),
            digest: Some(digest_text('1')),
        },
        target_ir_domain: ECHO_SPAN_IR_DOMAIN.to_owned(),
        operation_profiles: vec!["continuum.profile.write/v1".to_owned()],
        obstruction_coordinates: vec!["rejected".to_owned()],
        effect_lowerings: vec![TargetEffectLowering {
            effect: "target.replace".to_owned(),
            target_intrinsic: "echo.dpo@1.replace".to_owned(),
        }],
    }
}

fn target_ir_gitwarp_facts() -> TargetIrLoweringFacts {
    TargetIrLoweringFacts {
        target_profile: ResourceRef {
            coordinate: GITWARP_REF_CRDT_TARGET_PROFILE.to_owned(),
            digest: Some(digest_text('2')),
        },
        target_ir_domain: GITWARP_COMMIT_REDUCER_IR_DOMAIN.to_owned(),
        operation_profiles: vec!["continuum.profile.append/v1".to_owned()],
        obstruction_coordinates: vec!["conflict".to_owned()],
        effect_lowerings: vec![TargetEffectLowering {
            effect: "gitwarp.appendEvent".to_owned(),
            target_intrinsic: "gitwarp.ref_crdt@1.appendEvent".to_owned(),
        }],
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BundleGoldenMode {
    Check,
    Write,
}

#[derive(Debug)]
struct BundleGoldenCase {
    source: &'static str,
    output: &'static str,
}

const BUNDLE_GOLDEN_CASES: &[BundleGoldenCase] = &[BundleGoldenCase {
    source: "fixtures/lang/bounds/bounded-hello.edict",
    output: "fixtures/bundle/assembly/bounded-hello.bundle-digests.txt",
}];

const BUNDLE_CORE_IR_COORDINATE: &str = "edict.core.bounded-hello/v1";
const BUNDLE_SOURCE_LOGICAL_PATH: &str = "contracts/hello.edict";
const BUNDLE_SOURCE_COORDINATE: &str = "source.contracts.hello";
const BUNDLE_SOURCE_DIGEST: char = 'e';
const BUNDLE_SOURCE_PROFILE_COORDINATE: &str = "source-profile.hello/v1";
const BUNDLE_SOURCE_PROFILE_DIGEST: char = 'f';
const BUNDLE_TARGET_PROFILE_COORDINATE: &str = "echo.dpo@1";
const BUNDLE_TARGET_PROFILE_DIGEST: char = 'c';
const BUNDLE_TARGET_IR_COORDINATE: &str = "echo.span-ir/v1";
const BUNDLE_TARGET_IR_DIGEST: char = 'd';
const BUNDLE_LAWPACK_COORDINATE: &str = "hello.optics@1";
const BUNDLE_LAWPACK_DIGEST: char = '2';
const BUNDLE_GENERATED_COORDINATE: &str = "echo.dpo.registration/v1";
const BUNDLE_GENERATED_DIGEST: char = '3';
const BUNDLE_COMPILER_COORDINATE: &str = "edict.compiler/v1";
const BUNDLE_COMPILER_DIGEST: char = '4';
const BUNDLE_LOWERER_COORDINATE: &str = "echo.dpo.lowerer/v1";
const BUNDLE_LOWERER_DIGEST: char = '5';
const BUNDLE_VERIFIER_COORDINATE: &str = "echo.dpo.verifier/v1";
const BUNDLE_VERIFIER_DIGEST: char = '6';
const BUNDLE_SEMANTIC_OPTIONS_COORDINATE: &str = "edict.compile-options.semantic/v1";
const BUNDLE_SEMANTIC_OPTIONS_DIGEST: char = '7';
const BUNDLE_NONSEMANTIC_OPTIONS_COORDINATE: &str = "edict.compile-options.nonsemantic/v1";
const BUNDLE_NONSEMANTIC_OPTIONS_DIGEST: char = '8';
const BUNDLE_BUILD_PROVENANCE_COORDINATE: &str = "edict.build-provenance/v1";
const BUNDLE_BUILD_PROVENANCE_DIGEST: char = '9';
const BUNDLE_CANONICALIZATION_COORDINATE: &str = "edict.canonical-cbor/v1";
const BUNDLE_CANONICALIZATION_DIGEST: char = '8';
const BUNDLE_CONFORMANCE_COORDINATE: &str = "echo.dpo.fixtures/v1";
const BUNDLE_CONFORMANCE_DIGEST: char = '9';
const BUNDLE_VERIFIER_REPORT_COORDINATE: &str = "echo.dpo.verifier-report/v1";
const BUNDLE_VERIFIER_REPORT_DIGEST: char = 'a';
const BUNDLE_COMPILE_EXPLANATION_COORDINATE: &str = "watson.compile-explanation/v1";
const BUNDLE_COMPILE_EXPLANATION_DIGEST: char = 'b';

fn bundle_goldens(root: &Path, mode: BundleGoldenMode) -> Result<(), String> {
    for case in BUNDLE_GOLDEN_CASES {
        check_or_write_bundle_golden(root, case, mode)?;
    }
    println!(
        "bundle-goldens: {} case(s) {}",
        BUNDLE_GOLDEN_CASES.len(),
        match mode {
            BundleGoldenMode::Check => "checked",
            BundleGoldenMode::Write => "written",
        }
    );
    Ok(())
}

fn check_or_write_bundle_golden(
    root: &Path,
    case: &BundleGoldenCase,
    mode: BundleGoldenMode,
) -> Result<(), String> {
    let golden = render_bundle_golden(root, case)?;
    match mode {
        BundleGoldenMode::Check => check_golden_file(root, case.output, golden.as_bytes()),
        BundleGoldenMode::Write => write_golden_file(&root.join(case.output), golden.as_bytes()),
    }
}

fn render_bundle_golden(root: &Path, case: &BundleGoldenCase) -> Result<String, String> {
    let input = bundle_golden_input(root, case)?;
    let manifest = assemble_contract_bundle(input)
        .map_err(|err| format!("assemble {} contract bundle: {err}", case.source))?;
    Ok(format!(
        "\
# Edict contract bundle v0.11 digest golden
# Generated by `cargo xtask bundle-goldens --write`.
# Scope: semantic/release bundle digest preimage shape and digest values only.
# Non-scope: canonical Target IR bytes and ContractBundleManifest bytes.

source_fixture = {source}
core_ir_coordinate = {core_ir_coordinate}
source_artifact = {source_logical_path} {source_coordinate} {source_digest}
source_profile_semantic_facts = {source_profile_coordinate} {source_profile_digest}
target_profile = {target_profile_coordinate} {target_profile_digest}
target_ir = {target_ir_coordinate} {target_ir_digest}
lawpack = {lawpack_coordinate} {lawpack_digest}
generated_artifact = {generated_coordinate} {generated_digest}
compiler = {compiler_coordinate} {compiler_digest}
lowerer = {lowerer_coordinate} {lowerer_digest}
verifier = {verifier_coordinate} {verifier_digest}
semantic_compile_options = {semantic_options_coordinate} {semantic_options_digest}
non_semantic_compile_options = {nonsemantic_options_coordinate} {nonsemantic_options_digest}
build_provenance = {build_provenance_coordinate} {build_provenance_digest}
canonicalization_profile = {canonicalization_coordinate} {canonicalization_digest}
conformance_fixture_corpus = {conformance_coordinate} {conformance_digest}
verifier_report = {verifier_report_coordinate} {verifier_report_digest}
compile_explanation = {compile_explanation_coordinate} {compile_explanation_digest}

semantic_bundle_digest = {semantic_bundle_digest}
release_bundle_digest = {release_bundle_digest}
",
        source = case.source,
        core_ir_coordinate = BUNDLE_CORE_IR_COORDINATE,
        source_logical_path = BUNDLE_SOURCE_LOGICAL_PATH,
        source_coordinate = BUNDLE_SOURCE_COORDINATE,
        source_digest = digest_text(BUNDLE_SOURCE_DIGEST),
        source_profile_coordinate = BUNDLE_SOURCE_PROFILE_COORDINATE,
        source_profile_digest = digest_text(BUNDLE_SOURCE_PROFILE_DIGEST),
        target_profile_coordinate = BUNDLE_TARGET_PROFILE_COORDINATE,
        target_profile_digest = digest_text(BUNDLE_TARGET_PROFILE_DIGEST),
        target_ir_coordinate = BUNDLE_TARGET_IR_COORDINATE,
        target_ir_digest = digest_text(BUNDLE_TARGET_IR_DIGEST),
        lawpack_coordinate = BUNDLE_LAWPACK_COORDINATE,
        lawpack_digest = digest_text(BUNDLE_LAWPACK_DIGEST),
        generated_coordinate = BUNDLE_GENERATED_COORDINATE,
        generated_digest = digest_text(BUNDLE_GENERATED_DIGEST),
        compiler_coordinate = BUNDLE_COMPILER_COORDINATE,
        compiler_digest = digest_text(BUNDLE_COMPILER_DIGEST),
        lowerer_coordinate = BUNDLE_LOWERER_COORDINATE,
        lowerer_digest = digest_text(BUNDLE_LOWERER_DIGEST),
        verifier_coordinate = BUNDLE_VERIFIER_COORDINATE,
        verifier_digest = digest_text(BUNDLE_VERIFIER_DIGEST),
        semantic_options_coordinate = BUNDLE_SEMANTIC_OPTIONS_COORDINATE,
        semantic_options_digest = digest_text(BUNDLE_SEMANTIC_OPTIONS_DIGEST),
        nonsemantic_options_coordinate = BUNDLE_NONSEMANTIC_OPTIONS_COORDINATE,
        nonsemantic_options_digest = digest_text(BUNDLE_NONSEMANTIC_OPTIONS_DIGEST),
        build_provenance_coordinate = BUNDLE_BUILD_PROVENANCE_COORDINATE,
        build_provenance_digest = digest_text(BUNDLE_BUILD_PROVENANCE_DIGEST),
        canonicalization_coordinate = BUNDLE_CANONICALIZATION_COORDINATE,
        canonicalization_digest = digest_text(BUNDLE_CANONICALIZATION_DIGEST),
        conformance_coordinate = BUNDLE_CONFORMANCE_COORDINATE,
        conformance_digest = digest_text(BUNDLE_CONFORMANCE_DIGEST),
        verifier_report_coordinate = BUNDLE_VERIFIER_REPORT_COORDINATE,
        verifier_report_digest = digest_text(BUNDLE_VERIFIER_REPORT_DIGEST),
        compile_explanation_coordinate = BUNDLE_COMPILE_EXPLANATION_COORDINATE,
        compile_explanation_digest = digest_text(BUNDLE_COMPILE_EXPLANATION_DIGEST),
        semantic_bundle_digest = manifest.semantic_bundle_digest,
        release_bundle_digest = manifest.release_bundle_digest,
    ))
}

fn bundle_golden_input(
    root: &Path,
    case: &BundleGoldenCase,
) -> Result<ContractBundleAssemblyInput, String> {
    let source = read_to_string(&root.join(case.source))?;
    let module = parse_module(&source).map_err(|err| format!("parse {}: {err}", case.source))?;
    let core = compile_to_core(&module, &core_golden_context())
        .map_err(|err| format!("compile {} to Core: {err:?}", case.source))?;
    Ok(ContractBundleAssemblyInput {
        core_module: core,
        core_ir_coordinate: BUNDLE_CORE_IR_COORDINATE.to_owned(),
        source_artifacts: vec![bundle_source_artifact(
            BUNDLE_SOURCE_LOGICAL_PATH,
            BUNDLE_SOURCE_COORDINATE,
            BUNDLE_SOURCE_DIGEST,
        )?],
        source_profile_semantic_facts: bundle_resource(
            BUNDLE_SOURCE_PROFILE_COORDINATE,
            BUNDLE_SOURCE_PROFILE_DIGEST,
        )?,
        target_profile: bundle_resource(
            BUNDLE_TARGET_PROFILE_COORDINATE,
            BUNDLE_TARGET_PROFILE_DIGEST,
        )?,
        target_ir: bundle_target_ir(BUNDLE_TARGET_IR_COORDINATE, BUNDLE_TARGET_IR_DIGEST)?,
        lawpacks: vec![bundle_resource(
            BUNDLE_LAWPACK_COORDINATE,
            BUNDLE_LAWPACK_DIGEST,
        )?],
        generated_artifacts: vec![bundle_resource(
            BUNDLE_GENERATED_COORDINATE,
            BUNDLE_GENERATED_DIGEST,
        )?],
        compiler: bundle_resource(BUNDLE_COMPILER_COORDINATE, BUNDLE_COMPILER_DIGEST)?,
        lowerer: bundle_resource(BUNDLE_LOWERER_COORDINATE, BUNDLE_LOWERER_DIGEST)?,
        verifier: bundle_resource(BUNDLE_VERIFIER_COORDINATE, BUNDLE_VERIFIER_DIGEST)?,
        semantic_compile_options: bundle_resource(
            BUNDLE_SEMANTIC_OPTIONS_COORDINATE,
            BUNDLE_SEMANTIC_OPTIONS_DIGEST,
        )?,
        non_semantic_compile_options: bundle_resource(
            BUNDLE_NONSEMANTIC_OPTIONS_COORDINATE,
            BUNDLE_NONSEMANTIC_OPTIONS_DIGEST,
        )?,
        build_provenance: bundle_resource(
            BUNDLE_BUILD_PROVENANCE_COORDINATE,
            BUNDLE_BUILD_PROVENANCE_DIGEST,
        )?,
        canonicalization_profile: bundle_resource(
            BUNDLE_CANONICALIZATION_COORDINATE,
            BUNDLE_CANONICALIZATION_DIGEST,
        )?,
        conformance_fixture_corpora: vec![bundle_resource(
            BUNDLE_CONFORMANCE_COORDINATE,
            BUNDLE_CONFORMANCE_DIGEST,
        )?],
        verifier_report: bundle_resource(
            BUNDLE_VERIFIER_REPORT_COORDINATE,
            BUNDLE_VERIFIER_REPORT_DIGEST,
        )?,
        compile_explanation: bundle_resource(
            BUNDLE_COMPILE_EXPLANATION_COORDINATE,
            BUNDLE_COMPILE_EXPLANATION_DIGEST,
        )?,
        assurance_evidence: Vec::new(),
    })
}

fn bundle_resource(coordinate: &str, hex: char) -> Result<DigestLockedResource, String> {
    DigestLockedResource::new(coordinate, digest_text(hex)).map_err(|err| err.to_string())
}

fn bundle_target_ir(coordinate: &str, hex: char) -> Result<SuppliedTargetIrResource, String> {
    SuppliedTargetIrResource::new(coordinate, digest_text(hex)).map_err(|err| err.to_string())
}

fn bundle_source_artifact(
    logical_path: &str,
    coordinate: &str,
    hex: char,
) -> Result<ContractBundleSourceArtifact, String> {
    ContractBundleSourceArtifact::new(logical_path, coordinate, digest_text(hex))
        .map_err(|err| err.to_string())
}

fn digest_text(hex: char) -> String {
    format!("sha256:{}", hex.to_string().repeat(64))
}

fn check_golden_file(root: &Path, relative: &str, expected: &[u8]) -> Result<(), String> {
    let path = root.join(relative);
    let actual = fs::read(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{} does not match generated golden; run the matching `cargo xtask *-goldens --write` command",
            path.display()
        ))
    }
}

fn write_golden_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|err| format!("write {}: {err}", path.display()))
}

fn check_topic(
    root: &Path,
    topic: &Path,
    tests: &BTreeSet<String>,
    requirement_registry: &str,
) -> Result<(), String> {
    let test_plan = topic.join("test-plan.md");
    let readme = topic.join("README.md");
    let plan = read_to_string(&test_plan)?;
    let chapter = read_to_string(&readme)?;

    let requirements = parse_requirement_rows(&plan)?;
    let cases = parse_case_rows(&plan)?;
    if requirements.is_empty() {
        return Err(format!("{} has no requirement rows", test_plan.display()));
    }
    if cases.is_empty() {
        return Err(format!("{} has no case rows", test_plan.display()));
    }

    let requirement_ids: BTreeSet<&str> = requirements.keys().map(String::as_str).collect();
    let case_ids: BTreeSet<&str> = cases.keys().map(String::as_str).collect();
    let mut covered_requirements = BTreeSet::new();
    let mut implemented_requirements = BTreeSet::new();

    for (id, requirement) in &requirements {
        if !is_known_test_plan_status(&requirement.status) {
            return Err(format!("{id} has invalid status `{}`", requirement.status));
        }
        check_requirement_sources(root, id, &requirement.source, requirement_registry)?;
    }

    for case in cases.values() {
        for requirement in split_cell_list(&case.requirement) {
            if !requirement_ids.contains(requirement) {
                return Err(format!(
                    "{} references unknown requirement {}",
                    case.id, requirement
                ));
            }
            covered_requirements.insert(requirement.to_owned());
        }
        if case.oracle.trim().is_empty() || case.oracle.trim() == "-" {
            return Err(format!("{} is missing a mandatory oracle", case.id));
        }
        match case.status.as_str() {
            "implemented" => {
                if case.evidence.trim().is_empty() || case.evidence.trim() == "-" {
                    return Err(format!("{} is implemented without evidence", case.id));
                }
                for evidence in split_cell_list(&case.evidence) {
                    let test_name = evidence.rsplit("::").next().unwrap_or(evidence);
                    if !tests.contains(test_name) {
                        return Err(format!(
                            "{} evidence `{}` does not match a Rust test function",
                            case.id, evidence
                        ));
                    }
                }
                for requirement in split_cell_list(&case.requirement) {
                    implemented_requirements.insert(requirement.to_owned());
                }
            }
            status if is_known_test_plan_status(status) => {}
            other => return Err(format!("{} has invalid status `{other}`", case.id)),
        }
        for fixture in split_cell_list(&case.fixtures) {
            if fixture != "-" && !root.join(fixture).is_file() {
                return Err(format!("{} fixture `{fixture}` does not exist", case.id));
            }
        }
    }

    for (id, requirement) in &requirements {
        if !covered_requirements.contains(id) {
            return Err(format!("{id} has no case"));
        }
        if requirement.status == "implemented" && !implemented_requirements.contains(id) {
            return Err(format!(
                "implemented requirement {id} has no implemented case"
            ));
        }
    }

    let all_topic_ids = requirement_ids
        .union(&case_ids)
        .copied()
        .collect::<BTreeSet<_>>();
    for id in bracket_ids(&(chapter + "\n" + &plan)) {
        if (is_requirement_id(&id) || is_case_id(&id)) && !all_topic_ids.contains(id.as_str()) {
            return Err(format!("topic references unknown ID `{id}`"));
        }
        if id.starts_with("EDICT-") && !registry_has_id(requirement_registry, &id) {
            return Err(format!("topic references unknown registry ID `{id}`"));
        }
    }

    check_fixture_table(root, &plan)?;
    Ok(())
}

fn is_known_test_plan_status(status: &str) -> bool {
    matches!(status, "implemented" | "planned" | "gap" | "policy")
}

fn parse_requirement_rows(plan: &str) -> Result<BTreeMap<String, RequirementRow>, String> {
    let mut out = BTreeMap::new();
    for line in plan.lines() {
        let cells = table_cells(line);
        if cells.first().is_some_and(|cell| is_requirement_id(cell)) {
            if cells.len() < 4 {
                return Err(format!("malformed requirement row: {line}"));
            }
            let row = RequirementRow {
                id: cells[0].clone(),
                status: cells[1].clone(),
                source: cells[3].clone(),
            };
            if out.insert(row.id.clone(), row).is_some() {
                return Err(format!("duplicate requirement ID {}", cells[0]));
            }
        }
    }
    Ok(out)
}

fn parse_case_rows(plan: &str) -> Result<BTreeMap<String, CaseRow>, String> {
    let mut out = BTreeMap::new();
    for line in plan.lines() {
        let cells = table_cells(line);
        if cells.first().is_some_and(|cell| is_case_id(cell)) {
            if cells.len() < 8 {
                return Err(format!("malformed case row: {line}"));
            }
            let row = CaseRow {
                id: cells[0].clone(),
                status: cells[1].clone(),
                requirement: cells[3].clone(),
                oracle: cells[4].clone(),
                evidence: cells[5].clone(),
                fixtures: cells[6].clone(),
            };
            if out.insert(row.id.clone(), row).is_some() {
                return Err(format!("duplicate case ID {}", cells[0]));
            }
        }
    }
    Ok(out)
}

fn check_requirement_sources(
    root: &Path,
    id: &str,
    source_cell: &str,
    requirement_registry: &str,
) -> Result<(), String> {
    for source in split_cell_list(source_cell) {
        if source == "-" {
            return Err(format!("{id} has no requirement source"));
        }
        if source.starts_with("EDICT-") {
            if !registry_has_id(requirement_registry, source) {
                return Err(format!("{id} unknown registry source ID `{source}`"));
            }
        } else if !is_external_requirement_source(source) && !root.join(source).is_file() {
            return Err(format!("{id} source `{source}` does not resolve"));
        }
    }
    Ok(())
}

fn check_fixture_table(root: &Path, plan: &str) -> Result<(), String> {
    for line in plan.lines() {
        let cells = table_cells(line);
        if cells
            .first()
            .is_some_and(|cell| cell.starts_with("fixtures/"))
            && !root.join(&cells[0]).is_file()
        {
            return Err(format!("fixture table path `{}` does not exist", cells[0]));
        }
    }
    Ok(())
}

fn check_links(root: &Path, doc: &Path) -> Result<(), String> {
    let text = read_to_string(doc)?;
    let base = doc
        .parent()
        .ok_or_else(|| format!("{} has no parent", doc.display()))?;
    let canonical_root =
        fs::canonicalize(root).map_err(|err| format!("canonicalize {}: {err}", root.display()))?;
    let mut rest = text.as_str();
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else {
            break;
        };
        let target = &rest[..end];
        rest = &rest[end + 1..];
        if target.starts_with("http://")
            || target.starts_with("https://")
            || target.starts_with("mailto:")
            || target.starts_with('#')
        {
            continue;
        }
        let path_part = target.split_once('#').map_or(target, |(path, _)| path);
        if path_part.is_empty() {
            continue;
        }
        let path = Path::new(path_part);
        if path.is_absolute() {
            return Err(format!(
                "{} has absolute local link `{target}`",
                doc.display()
            ));
        }
        let resolved = base.join(path);
        if !resolved.exists() {
            return Err(format!(
                "{} has broken link `{target}` resolved to {}",
                doc.display(),
                resolved.display()
            ));
        }
        let canonical_resolved = fs::canonicalize(&resolved)
            .map_err(|err| format!("canonicalize {}: {err}", resolved.display()))?;
        if !canonical_resolved.starts_with(&canonical_root) {
            return Err(format!(
                "{} link `{target}` escapes repository root",
                doc.display()
            ));
        }
    }
    Ok(())
}

fn collect_rust_test_names(root: &Path) -> Result<BTreeSet<String>, String> {
    let mut names = BTreeSet::new();
    for path in rust_files(root)? {
        let text = read_to_string(&path)?;
        let mut previous_was_test = false;
        for raw in text.lines() {
            let line = raw.trim();
            if line == "#[test]" {
                previous_was_test = true;
                continue;
            }
            if previous_was_test {
                if let Some(rest) = line.strip_prefix("fn ") {
                    if let Some((name, _)) = rest.split_once('(') {
                        names.insert(name.to_owned());
                    }
                }
                previous_was_test = false;
            }
        }
    }
    Ok(names)
}

fn bracket_ids(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        rest = &rest[start + 1..];
        let Some(end) = rest.find(']') else {
            break;
        };
        let id = &rest[..end];
        if is_requirement_id(id) || is_case_id(id) || id.starts_with("EDICT-") {
            out.insert(id.to_owned());
        }
        rest = &rest[end + 1..];
    }
    out
}

fn is_requirement_id(s: &str) -> bool {
    s.contains("-REQ-")
        && s.chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-')
}

fn is_case_id(s: &str) -> bool {
    s.contains("-TP-")
        && s.chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-')
}

fn split_cell_list(cell: &str) -> impl Iterator<Item = &str> {
    cell.split(',')
        .map(str::trim)
        .map(|s| s.trim_matches('`'))
        .filter(|s| !s.is_empty())
}

fn registry_has_id(requirement_registry: &str, id: &str) -> bool {
    requirement_registry.lines().any(|line| {
        table_cells(line)
            .first()
            .is_some_and(|cell| cell.as_str() == id)
    })
}

fn is_external_requirement_source(source: &str) -> bool {
    source.starts_with("issue #") || source.starts_with("http://") || source.starts_with("https://")
}

fn table_cells(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') || !trimmed.ends_with('|') {
        return Vec::new();
    }
    trimmed
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .map(ToOwned::to_owned)
        .collect()
}

fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    files_with_extension(root, "md")
}

fn rust_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    files_with_extension(root, "rs")
}

fn files_with_extension(root: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    visit(root, &mut |path| {
        if path.extension() == Some(OsStr::new(extension)) {
            out.push(path.to_owned());
        }
        Ok(())
    })?;
    Ok(out)
}

fn dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(root).map_err(|err| format!("read {}: {err}", root.display()))? {
        let entry = entry.map_err(|err| format!("read {} entry: {err}", root.display()))?;
        let path = entry.path();
        if path.is_dir() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn visit(dir: &Path, f: &mut impl FnMut(&Path) -> Result<(), String>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| format!("read {}: {err}", dir.display()))? {
        let entry = entry.map_err(|err| format!("read {} entry: {err}", dir.display()))?;
        let path = entry.path();
        if path.is_dir() {
            visit(&path, f)?;
        } else {
            f(&path)?;
        }
    }
    Ok(())
}

fn read_to_string(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))
}

fn repo_root() -> Result<PathBuf, String> {
    let mut dir = env::current_dir().map_err(|err| format!("current dir: {err}"))?;
    loop {
        if dir.join("Cargo.toml").is_file() && dir.join("docs").is_dir() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err("could not find repository root".into());
        }
    }
}

#[derive(Debug)]
struct RequirementRow {
    id: String,
    status: String,
    source: String,
}

#[derive(Debug)]
struct CaseRow {
    id: String,
    status: String,
    requirement: String,
    oracle: String,
    evidence: String,
    fixtures: String,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use edict_syntax::parse_module;
    use regex::Regex;
    use serde_json::Value;

    use super::{
        bundle_goldens, check_topic, contract_check, repo_root, target_ir_goldens,
        BundleGoldenMode, TargetIrGoldenMode,
    };

    fn toml_section(document: &str, header: &str) -> String {
        let start = document.find(header).expect("section header");
        let section = &document[start..];
        let end = section
            .lines()
            .enumerate()
            .skip(1)
            .find_map(|(index, line)| line.starts_with('[').then_some(index))
            .unwrap_or_else(|| section.lines().count());
        section.lines().take(end).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn contract_graph_is_valid() {
        contract_check(&repo_root().expect("repo root")).expect("contract graph is valid");
    }

    #[test]
    fn bundle_digest_goldens_match_assembly() {
        bundle_goldens(&repo_root().expect("repo root"), BundleGoldenMode::Check)
            .expect("bundle digest goldens match assembly output");
    }

    #[test]
    fn target_ir_goldens_match_executable_encoder() {
        target_ir_goldens(&repo_root().expect("repo root"), TargetIrGoldenMode::Check)
            .expect("Target IR goldens match executable encoder output");
    }

    #[test]
    fn contract_graph_rejects_unknown_registry_source_ids() {
        let root = temp_root("unknown-registry-source");
        let topic = root.join("docs/topics/example");
        fs::create_dir_all(&topic).expect("topic directory");
        fs::write(topic.join("README.md"), "# Example\n\n[EXAMPLE-REQ-001]\n").expect("chapter");
        fs::write(
            topic.join("test-plan.md"),
            "# Example Test Plan\n\n\
             | ID | Status | Requirement | Source |\n\
             | --- | --- | --- | --- |\n\
             | EXAMPLE-REQ-001 | implemented | Example requirement. | EDICT-NOT-A-REAL-ID |\n\n\
             | ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |\n\
             | --- | --- | --- | --- | --- | --- | --- | --- |\n\
             | EXAMPLE-TP-001 | implemented | Golden path | EXAMPLE-REQ-001 | exact state | evidence_test | - | fixture-free |\n",
        )
        .expect("test plan");

        let tests = BTreeSet::from(["evidence_test".to_owned()]);
        let err = check_topic(&root, &topic, &tests, "EDICT-KNOWN-ID")
            .expect_err("unknown registry Source ID must fail");
        assert!(
            err.contains("unknown registry source ID `EDICT-NOT-A-REAL-ID`"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn contract_graph_rejects_truncated_registry_bracket_ids() {
        let root = temp_root("truncated-registry-bracket-id");
        let topic = root.join("docs/topics/example");
        fs::create_dir_all(&topic).expect("topic directory");
        fs::write(
            topic.join("README.md"),
            "# Example\n\n[EDICT-LANG-RECORD]\n",
        )
        .expect("chapter");
        fs::write(
            topic.join("test-plan.md"),
            "# Example Test Plan\n\n\
             | ID | Status | Requirement | Source |\n\
             | --- | --- | --- | --- |\n\
             | EXAMPLE-REQ-001 | implemented | Example requirement. | docs/REQUIREMENTS.md |\n\n\
             | ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |\n\
             | --- | --- | --- | --- | --- | --- | --- | --- |\n\
             | EXAMPLE-TP-001 | implemented | Golden path | EXAMPLE-REQ-001 | exact state | evidence_test | - | fixture-free |\n",
        )
        .expect("test plan");
        fs::create_dir_all(root.join("docs")).expect("docs directory");
        fs::write(
            root.join("docs/REQUIREMENTS.md"),
            "| ID | Requirement |\n\
             | --- | --- |\n\
             | EDICT-LANG-RECORD-SHORTHAND-001 | exact longer ID |\n",
        )
        .expect("requirements");

        let tests = BTreeSet::from(["evidence_test".to_owned()]);
        let err = check_topic(&root, &topic, &tests, "EDICT-LANG-RECORD-SHORTHAND-001")
            .expect_err("truncated registry ID must fail");
        assert!(
            err.contains("unknown registry ID `EDICT-LANG-RECORD`"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn contract_graph_requires_implemented_evidence_for_implemented_requirements() {
        let root = temp_root("implemented-requirement-without-evidence");
        let topic = root.join("docs/topics/example");
        fs::create_dir_all(root.join("docs")).expect("docs directory");
        fs::create_dir_all(&topic).expect("topic directory");
        fs::write(root.join("docs/REQUIREMENTS.md"), "# Requirements\n").expect("requirements");
        fs::write(topic.join("README.md"), "# Example\n\n[EXAMPLE-REQ-001]\n").expect("chapter");
        fs::write(
            topic.join("test-plan.md"),
            "# Example Test Plan\n\n\
             | ID | Status | Requirement | Source |\n\
             | --- | --- | --- | --- |\n\
             | EXAMPLE-REQ-001 | implemented | Example requirement. | docs/REQUIREMENTS.md |\n\n\
             | ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |\n\
             | --- | --- | --- | --- | --- | --- | --- | --- |\n\
             | EXAMPLE-TP-001 | planned | Golden path | EXAMPLE-REQ-001 | exact state | - | - | planned only |\n",
        )
        .expect("test plan");

        let tests = BTreeSet::new();
        let err = check_topic(&root, &topic, &tests, "EDICT-KNOWN-ID")
            .expect_err("implemented requirement without implemented case must fail");
        assert!(
            err.contains("implemented requirement EXAMPLE-REQ-001 has no implemented case"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn contract_graph_accepts_policy_rows_without_rust_evidence() {
        let root = temp_root("policy-rows-without-rust-evidence");
        let topic = root.join("docs/topics/example");
        fs::create_dir_all(root.join("docs")).expect("docs directory");
        fs::create_dir_all(&topic).expect("topic directory");
        fs::write(root.join("docs/REQUIREMENTS.md"), "# Requirements\n").expect("requirements");
        fs::write(topic.join("README.md"), "# Example\n\n[EXAMPLE-REQ-001]\n").expect("chapter");
        fs::write(
            topic.join("test-plan.md"),
            "# Example Test Plan\n\n\
             | ID | Status | Requirement | Source |\n\
             | --- | --- | --- | --- |\n\
             | EXAMPLE-REQ-001 | policy | Example policy. | docs/REQUIREMENTS.md |\n\n\
             | ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |\n\
             | --- | --- | --- | --- | --- | --- | --- | --- |\n\
             | EXAMPLE-TP-001 | policy | Review policy | EXAMPLE-REQ-001 | human review records the policy decision | - | - | no executable software behavior |\n",
        )
        .expect("test plan");

        let tests = BTreeSet::new();
        check_topic(&root, &topic, &tests, "").expect("policy rows require no Rust evidence");

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn contract_graph_rejects_unknown_requirement_status() {
        let root = temp_root("unknown-requirement-status");
        let topic = root.join("docs/topics/example");
        fs::create_dir_all(root.join("docs")).expect("docs directory");
        fs::create_dir_all(&topic).expect("topic directory");
        fs::write(root.join("docs/REQUIREMENTS.md"), "# Requirements\n").expect("requirements");
        fs::write(topic.join("README.md"), "# Example\n\n[EXAMPLE-REQ-001]\n").expect("chapter");
        fs::write(
            topic.join("test-plan.md"),
            "# Example Test Plan\n\n\
             | ID | Status | Requirement | Source |\n\
             | --- | --- | --- | --- |\n\
             | EXAMPLE-REQ-001 | maybe | Example requirement. | docs/REQUIREMENTS.md |\n\n\
             | ID | Status | Category | Requirement | Oracle | Evidence | Fixtures | Notes |\n\
             | --- | --- | --- | --- | --- | --- | --- | --- |\n\
             | EXAMPLE-TP-001 | planned | Planned path | EXAMPLE-REQ-001 | exact state | - | - | planned only |\n",
        )
        .expect("test plan");

        let tests = BTreeSet::new();
        let err = check_topic(&root, &topic, &tests, "")
            .expect_err("unknown requirement status must reject");
        assert!(
            err.contains("EXAMPLE-REQ-001 has invalid status `maybe`"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn link_check_rejects_local_links_outside_repo() {
        let root = temp_root("repo-escaping-link");
        let outside = temp_root("repo-escaping-link-outside").join("outside.md");
        fs::write(&outside, "# Outside\n").expect("outside file");
        let doc = root.join("docs/topics/example/README.md");
        fs::create_dir_all(doc.parent().expect("doc parent")).expect("doc directory");
        fs::write(&doc, format!("# Example\n\n[bad]({})\n", outside.display())).expect("chapter");

        let err = super::check_links(&root, &doc).expect_err("absolute local links must reject");
        assert!(
            err.contains("absolute local link"),
            "unexpected error: {err}"
        );

        fs::remove_dir_all(root).ok();
        if let Some(parent) = outside.parent() {
            fs::remove_dir_all(parent).ok();
        }
    }

    #[test]
    fn diff_check_base_selection_falls_back_without_origin_main() {
        let refs = BTreeSet::from(["HEAD^"]);
        let base = super::choose_diff_check_base(|candidate| Ok(refs.contains(candidate)))
            .expect("fallback diff base");
        assert_eq!(base, "HEAD^");
    }

    #[test]
    fn diff_check_base_selection_prefers_origin_main() {
        let refs = BTreeSet::from(["origin/main", "main", "HEAD^"]);
        let base = super::choose_diff_check_base(|candidate| Ok(refs.contains(candidate)))
            .expect("preferred diff base");
        assert_eq!(base, "origin/main");
    }

    fn workflow_block<'a>(workflow: &'a str, start: &str, end: &str) -> &'a str {
        workflow
            .split(start)
            .nth(1)
            .and_then(|tail| tail.split(end).next())
            .unwrap_or_else(|| panic!("workflow block missing `{start}`"))
    }

    fn milestone_lookup_consumes_complete_stream(block: &str) -> bool {
        block.contains(
            "gh api --paginate \"repos/${GITHUB_REPOSITORY}/milestones?state=all&per_page=100\"",
        ) && block.contains("jq -s '.[0] // empty'")
            && !block.contains("head -n 1")
    }

    fn tree_sitter_rule_declared(grammar: &str, rule: &str) -> bool {
        grammar.contains(&format!("{rule}: $ =>"))
    }

    fn tree_sitter_literal_declared(grammar: &str, literal: &str) -> bool {
        grammar.contains(&format!("'{literal}'")) || grammar.contains(&format!("\"{literal}\""))
    }

    fn quoted_query_atoms(line: &str) -> Vec<String> {
        let mut atoms = Vec::new();
        let mut chars = line.char_indices();
        while let Some((start, ch)) = chars.next() {
            if ch != '"' {
                continue;
            }
            let atom_start = start + ch.len_utf8();
            for (end, end_ch) in chars.by_ref() {
                if end_ch == '"' {
                    atoms.push(line[atom_start..end].to_owned());
                    break;
                }
            }
        }
        atoms
    }

    fn tree_sitter_query_atoms_for_capture(query: &str, capture: &str) -> BTreeSet<String> {
        let mut atoms = BTreeSet::new();
        let mut list_atoms = Vec::new();
        let mut in_list = false;

        for line in query.lines() {
            let trimmed = line.trim();
            if trimmed == "[" {
                in_list = true;
                list_atoms.clear();
                continue;
            }

            if in_list {
                if trimmed.starts_with(']') {
                    if trimmed.contains(capture) {
                        atoms.extend(list_atoms.drain(..));
                    }
                    in_list = false;
                    list_atoms.clear();
                } else {
                    list_atoms.extend(quoted_query_atoms(trimmed));
                }
                continue;
            }

            if trimmed.contains("#any-of?") && trimmed.contains(capture) {
                atoms.extend(quoted_query_atoms(trimmed));
            }
        }

        atoms
    }

    fn textmate_grammar() -> Value {
        let root = repo_root().expect("repo root");
        let grammar = fs::read_to_string(root.join("grammars/textmate/edict.tmLanguage.json"))
            .expect("TextMate grammar");
        serde_json::from_str(&grammar).expect("TextMate grammar must be valid JSON")
    }

    fn json_str<'a>(value: &'a Value, key: &str) -> &'a str {
        value
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("JSON object missing string field `{key}`"))
    }

    fn textmate_repository_match<'a>(grammar: &'a Value, entry: &str) -> &'a str {
        grammar
            .get("repository")
            .and_then(|repository| repository.get(entry))
            .and_then(|entry| entry.get("match"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("TextMate repository entry `{entry}` missing match regex"))
    }

    fn textmate_repository_patterns<'a>(grammar: &'a Value, entry: &str) -> &'a Vec<Value> {
        grammar
            .get("repository")
            .and_then(|repository| repository.get(entry))
            .and_then(|entry| entry.get("patterns"))
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("TextMate repository entry `{entry}` missing patterns"))
    }

    fn textmate_includes(grammar: &Value) -> BTreeSet<String> {
        grammar
            .get("patterns")
            .and_then(Value::as_array)
            .expect("TextMate grammar missing top-level patterns")
            .iter()
            .map(|pattern| json_str(pattern, "include").to_owned())
            .collect()
    }

    fn textmate_regex_matches_literal(pattern: &str, literal: &str) -> bool {
        let anchored = format!("^(?:{pattern})$");
        Regex::new(&anchored)
            .unwrap_or_else(|err| panic!("TextMate regex `{pattern}` must compile: {err}"))
            .is_match(literal)
    }

    fn textmate_regex_spans(pattern: &str, source: &str) -> Vec<(usize, usize)> {
        Regex::new(pattern)
            .unwrap_or_else(|err| panic!("TextMate regex `{pattern}` must compile: {err}"))
            .find_iter(source)
            .map(|match_| (match_.start(), match_.end()))
            .collect()
    }

    fn vscode_extension_manifest() -> Value {
        let root = repo_root().expect("repo root");
        let manifest = fs::read_to_string(root.join("editors/vscode/package.json"))
            .expect("VS Code extension package manifest");
        serde_json::from_str(&manifest).expect("VS Code extension manifest must be valid JSON")
    }

    fn read_json_file(path: &Path) -> Value {
        let file = fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("JSON file `{}` must be readable: {err}", path.display()));
        serde_json::from_str(&file)
            .unwrap_or_else(|err| panic!("JSON file `{}` must parse: {err}", path.display()))
    }

    fn json_array<'a>(value: &'a Value, key: &str) -> &'a Vec<Value> {
        value
            .get(key)
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("JSON object missing array field `{key}`"))
    }

    fn json_object<'a>(value: &'a Value, key: &str) -> &'a serde_json::Map<String, Value> {
        value
            .get(key)
            .and_then(Value::as_object)
            .unwrap_or_else(|| panic!("JSON object missing object field `{key}`"))
    }

    fn json_string_array_contains(value: &Value, key: &str, expected: &str) -> bool {
        json_array(value, key).iter().any(|item| item == expected)
    }

    #[derive(Debug)]
    struct TreeSitterCorpusExample {
        title: String,
        source: String,
        expected_tree: String,
    }

    fn parse_tree_sitter_corpus(corpus: &str) -> Vec<TreeSitterCorpusExample> {
        let lines = corpus.lines().collect::<Vec<_>>();
        let mut examples = Vec::new();
        let mut index = 0usize;

        while index < lines.len() {
            while index < lines.len() && !is_tree_sitter_delimiter(lines[index]) {
                index += 1;
            }
            if index == lines.len() {
                break;
            }

            index += 1;
            assert!(index < lines.len(), "Tree-sitter corpus missing title");
            let title = lines[index].trim().to_owned();
            index += 1;
            assert!(
                index < lines.len() && is_tree_sitter_delimiter(lines[index]),
                "Tree-sitter corpus example `{title}` missing title delimiter"
            );
            index += 1;
            if index < lines.len() && lines[index].trim().is_empty() {
                index += 1;
            }

            let mut source_lines = Vec::new();
            while index < lines.len() && lines[index].trim() != "---" {
                source_lines.push(lines[index]);
                index += 1;
            }
            assert!(
                index < lines.len(),
                "Tree-sitter corpus example `{title}` missing expected-tree delimiter"
            );
            index += 1;

            let mut expected_tree_lines = Vec::new();
            while index < lines.len() && !is_tree_sitter_delimiter(lines[index]) {
                expected_tree_lines.push(lines[index]);
                index += 1;
            }

            examples.push(TreeSitterCorpusExample {
                title,
                source: source_lines.join("\n").trim().to_owned(),
                expected_tree: expected_tree_lines.join("\n").trim().to_owned(),
            });
        }

        examples
    }

    fn is_tree_sitter_delimiter(line: &str) -> bool {
        let trimmed = line.trim();
        !trimmed.is_empty() && trimmed.chars().all(|ch| ch == '=')
    }

    #[test]
    fn tree_sitter_grammar_declares_current_editor_contract() {
        let root = repo_root().expect("repo root");
        let grammar = fs::read_to_string(root.join("grammars/tree-sitter-edict/grammar.js"))
            .expect("Tree-sitter grammar source");
        let highlights =
            fs::read_to_string(root.join("grammars/tree-sitter-edict/queries/highlights.scm"))
                .expect("Tree-sitter highlight query");

        for rule in [
            "source_file",
            "package_declaration",
            "use_declaration",
            "type_declaration",
            "enum_declaration",
            "variant_type",
            "intent_declaration",
            "intent_clause",
            "block",
            "let_statement",
            "return_statement",
            "require_statement",
            "guarantee_statement",
            "assert_statement",
            "if_statement",
            "for_statement",
            "yield_block",
            "match_expression",
            "call_expression",
            "call_type_arguments",
            "call_argument_list",
            "record_literal",
            "type_reference",
            "comment",
            "string",
            "number",
        ] {
            assert!(
                tree_sitter_rule_declared(&grammar, rule),
                "Tree-sitter grammar must declare `{rule}`"
            );
        }

        for keyword in [
            "package",
            "use",
            "type",
            "enum",
            "variant",
            "intent",
            "returns",
            "profile",
            "implements",
            "basis",
            "footprint",
            "budget",
            "where",
            "let",
            "return",
            "require",
            "guarantee",
            "assert",
            "if",
            "then",
            "else",
            "for",
            "in",
            "bounded",
            "yield",
            "match",
            "shape",
            "lawpack",
            "target",
            "core",
            "as",
            "digest",
            "true",
            "false",
        ] {
            assert!(
                tree_sitter_literal_declared(&grammar, keyword),
                "Tree-sitter grammar must recognize keyword `{keyword}`"
            );
        }

        for capture in [
            "@comment",
            "@keyword",
            "@string",
            "@number",
            "@operator",
            "@punctuation",
            "@type",
            "@function",
        ] {
            assert!(
                highlights.contains(capture),
                "Tree-sitter highlight query must emit `{capture}`"
            );
        }
    }

    #[test]
    fn tree_sitter_query_covers_public_keyword_roles() {
        let root = repo_root().expect("repo root");
        let highlights =
            fs::read_to_string(root.join("grammars/tree-sitter-edict/queries/highlights.scm"))
                .expect("Tree-sitter highlight query");
        let source = r"
package examples.keywords@1;
use capability caps.auth@1 as caps;
const demo = 1;
fn run() {}
";
        let public_keywords = edict_syntax::highlight_source(source)
            .expect("source lexes for highlighting")
            .into_iter()
            .filter(|token| token.role == edict_syntax::HighlightRole::Keyword)
            .map(|token| token.lexeme(source).to_owned())
            .collect::<BTreeSet<_>>();
        let query_keywords = tree_sitter_query_atoms_for_capture(&highlights, "@keyword");

        for keyword in public_keywords {
            assert!(
                query_keywords.contains(&keyword),
                "Tree-sitter highlight query must capture public keyword `{keyword}`"
            );
        }
    }

    #[test]
    fn tree_sitter_query_operator_and_punctuation_roles_match_public_highlighter() {
        let root = repo_root().expect("repo root");
        let highlights =
            fs::read_to_string(root.join("grammars/tree-sitter-edict/queries/highlights.scm"))
                .expect("Tree-sitter highlight query");
        let source = "= == != < <= > >= + - * / % ! && || => :: ... ; : , . @ ( ) { }";
        let public_roles = edict_syntax::highlight_source(source)
            .expect("operator and punctuation source lexes")
            .into_iter()
            .map(|token| (token.lexeme(source).to_owned(), token.role))
            .collect::<std::collections::BTreeMap<_, _>>();
        let query_operators = tree_sitter_query_atoms_for_capture(&highlights, "@operator");
        let query_punctuation = tree_sitter_query_atoms_for_capture(&highlights, "@punctuation");

        if let Some(duplicate) = query_operators.intersection(&query_punctuation).next() {
            panic!("Tree-sitter query assigns `{duplicate}` to both operator and punctuation");
        }

        for operator in query_operators {
            if let Some(role) = public_roles.get(&operator) {
                assert_eq!(
                    *role,
                    edict_syntax::HighlightRole::Operator,
                    "Tree-sitter query marks `{operator}` as operator but public highlighter emits {role:?}"
                );
            }
        }

        for punctuation in query_punctuation {
            if let Some(role) = public_roles.get(&punctuation) {
                assert_eq!(
                    *role,
                    edict_syntax::HighlightRole::Punctuation,
                    "Tree-sitter query marks `{punctuation}` as punctuation but public highlighter emits {role:?}"
                );
            }
        }
    }

    #[test]
    fn textmate_grammar_declares_current_editor_contract() {
        let grammar = textmate_grammar();
        assert_eq!(json_str(&grammar, "name"), "Edict");
        assert_eq!(json_str(&grammar, "scopeName"), "source.edict");
        let file_types = grammar
            .get("fileTypes")
            .and_then(Value::as_array)
            .expect("TextMate grammar missing file types");
        assert!(
            file_types.iter().any(|file_type| file_type == "edict"),
            "TextMate grammar must register .edict files"
        );

        let includes = textmate_includes(&grammar);
        for include in [
            "#comments",
            "#strings",
            "#numbers",
            "#keywords",
            "#types",
            "#operators",
            "#punctuation",
            "#identifiers",
        ] {
            assert!(
                includes.contains(include),
                "TextMate grammar must include repository pattern `{include}`"
            );
        }

        let comments = textmate_repository_patterns(&grammar, "comments");
        assert!(
            comments
                .iter()
                .any(|pattern| json_str(pattern, "name") == "comment.line.double-slash.edict"),
            "TextMate grammar must scope line comments"
        );
        assert!(
            comments
                .iter()
                .any(|pattern| json_str(pattern, "name") == "comment.block.edict"),
            "TextMate grammar must scope block comments"
        );
    }

    #[test]
    fn textmate_grammar_covers_public_highlight_roles() {
        let grammar = textmate_grammar();
        let keyword_regex = textmate_repository_match(&grammar, "keywords");
        let operator_regex = textmate_repository_match(&grammar, "operators");
        let punctuation_regex = textmate_repository_match(&grammar, "punctuation");
        let type_regex = textmate_repository_match(&grammar, "types");
        let identifier_regex = textmate_repository_match(&grammar, "identifiers");
        let source = "package use type enum variant intent returns profile implements basis \
            footprint budget where let return require guarantee assert if then else for in \
            bounded yield match shape lawpack target core capability as digest fn const true \
            false HelloInput input = == != < <= > >= + - * / % ! && || => -> :: ... ; : , . @ \
            ( ) { } [ ] \"text\" 123";
        let highlights = edict_syntax::highlight_source(source).expect("highlight role source");

        for token in highlights {
            let lexeme = token.lexeme(source).trim();
            if lexeme.is_empty() {
                continue;
            }
            match token.role {
                edict_syntax::HighlightRole::Keyword => assert!(
                    textmate_regex_matches_literal(keyword_regex, lexeme),
                    "TextMate keyword regex must cover `{lexeme}`"
                ),
                edict_syntax::HighlightRole::Operator => assert!(
                    textmate_regex_matches_literal(operator_regex, lexeme),
                    "TextMate operator regex must cover `{lexeme}`"
                ),
                edict_syntax::HighlightRole::Punctuation => assert!(
                    textmate_regex_matches_literal(punctuation_regex, lexeme),
                    "TextMate punctuation regex must cover `{lexeme}`"
                ),
                edict_syntax::HighlightRole::TypeIdentifier => assert!(
                    textmate_regex_matches_literal(type_regex, lexeme),
                    "TextMate type regex must cover `{lexeme}`"
                ),
                edict_syntax::HighlightRole::Identifier => assert!(
                    textmate_regex_matches_literal(identifier_regex, lexeme),
                    "TextMate identifier regex must cover `{lexeme}`"
                ),
                edict_syntax::HighlightRole::Comment
                | edict_syntax::HighlightRole::Number
                | edict_syntax::HighlightRole::String => {}
            }
        }
    }

    #[test]
    fn textmate_grammar_scopes_public_number_spans_in_version_labels() {
        let grammar = textmate_grammar();
        let number_regex = textmate_repository_match(&grammar, "numbers");
        let source = "package examples.tooling@1_beta;";
        let public_number = edict_syntax::highlight_source(source)
            .expect("version label source lexes")
            .into_iter()
            .find(|token| {
                token.role == edict_syntax::HighlightRole::Number && token.lexeme(source) == "1_"
            })
            .expect("public highlighter emits leading numeric version-label span");
        let number_spans = textmate_regex_spans(number_regex, source);

        assert!(
            number_spans.iter().any(|(start, end)| {
                *start == public_number.span.start && *end == public_number.span.end
            }),
            "TextMate number regex must exactly scope public version-label number span `{}`",
            public_number.lexeme(source)
        );
    }

    #[test]
    fn vscode_extension_declares_textmate_language_contract() {
        let root = repo_root().expect("repo root");
        let manifest = vscode_extension_manifest();
        assert_eq!(json_str(&manifest, "name"), "edict-vscode");
        assert_eq!(json_str(&manifest, "publisher"), "flyingrobots");

        let engines = json_object(&manifest, "engines");
        assert_eq!(
            engines.get("vscode").and_then(Value::as_str),
            Some("^1.85.0")
        );
        assert!(
            manifest.get("main").is_none()
                && manifest.get("browser").is_none()
                && manifest.get("activationEvents").is_none(),
            "thin grammar extension must not declare a runtime activation surface"
        );

        let contributes = manifest
            .get("contributes")
            .expect("VS Code extension manifest must contribute editor behavior");
        let languages = json_array(contributes, "languages");
        let language = languages
            .iter()
            .find(|language| json_str(language, "id") == "edict")
            .expect("VS Code extension must register the Edict language");
        assert!(
            json_string_array_contains(language, "extensions", ".edict"),
            "VS Code extension must register .edict files"
        );
        assert!(
            json_string_array_contains(language, "aliases", "Edict"),
            "VS Code extension must expose the Edict language alias"
        );
        assert_eq!(
            json_str(language, "configuration"),
            "./language-configuration.json"
        );

        let grammars = json_array(contributes, "grammars");
        let grammar = grammars
            .iter()
            .find(|grammar| json_str(grammar, "language") == "edict")
            .expect("VS Code extension must map the Edict language to a grammar");
        assert_eq!(json_str(grammar, "scopeName"), "source.edict");
        assert_eq!(
            json_str(grammar, "path"),
            "./syntaxes/edict.tmLanguage.json"
        );

        let extension_grammar = read_json_file(
            &root
                .join("editors/vscode")
                .join(json_str(grammar, "path").trim_start_matches("./")),
        );
        assert_eq!(
            extension_grammar,
            textmate_grammar(),
            "VS Code extension grammar must match the canonical TextMate grammar"
        );
    }

    #[test]
    fn vscode_language_configuration_matches_lexer_boundaries() {
        let root = repo_root().expect("repo root");
        let manifest = vscode_extension_manifest();
        let contributes = manifest
            .get("contributes")
            .expect("VS Code extension manifest must contribute editor behavior");
        let language = json_array(contributes, "languages")
            .iter()
            .find(|language| json_str(language, "id") == "edict")
            .expect("VS Code extension must register the Edict language");
        let config = read_json_file(
            &root
                .join("editors/vscode")
                .join(json_str(language, "configuration").trim_start_matches("./")),
        );

        let comments = config
            .get("comments")
            .expect("language configuration must declare comments");
        assert_eq!(json_str(comments, "lineComment"), "//");
        assert_eq!(
            json_array(comments, "blockComment")
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>(),
            vec!["/*", "*/"]
        );

        let brackets = json_array(&config, "brackets")
            .iter()
            .map(|pair| {
                pair.as_array()
                    .expect("bracket pair must be an array")
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for pair in [vec!["{", "}"], vec!["[", "]"], vec!["(", ")"]] {
            assert!(
                brackets.iter().any(|bracket| bracket == &pair),
                "language configuration must expose bracket pair {pair:?}"
            );
        }
    }

    #[test]
    fn tree_sitter_corpus_examples_match_reference_parser() {
        let root = repo_root().expect("repo root");
        let corpus = fs::read_to_string(
            root.join("grammars/tree-sitter-edict/test/corpus/current-subset.txt"),
        )
        .expect("Tree-sitter corpus");
        let examples = parse_tree_sitter_corpus(&corpus);
        assert!(
            !examples.is_empty(),
            "Tree-sitter corpus must include accepted Edict examples"
        );

        let mut titles = BTreeSet::new();
        for example in &examples {
            assert!(
                titles.insert(example.title.as_str()),
                "duplicate Tree-sitter corpus example `{}`",
                example.title
            );
            assert!(
                !example.expected_tree.trim().is_empty(),
                "Tree-sitter corpus example `{}` must include an expected tree",
                example.title
            );
            parse_module(&example.source).unwrap_or_else(|err| {
                panic!(
                    "Tree-sitter corpus example `{}` must parse with the reference parser: {err}",
                    example.title
                );
            });
        }

        for title in [
            "bounded hello",
            "conditional blob",
            "spaced type call",
            "read greeting",
            "uppercase bare identifiers",
            "color match",
        ] {
            assert!(
                titles.contains(title),
                "Tree-sitter corpus must cover `{title}`"
            );
        }
    }

    #[test]
    fn release_workflow_publishes_only_main_reachable_tags() {
        let root = repo_root().expect("repo root");
        let workflow =
            fs::read_to_string(root.join(".github/workflows/release.yml")).expect("workflow");
        for needle in [
            "tags:",
            "\"v*\"",
            "workflow_dispatch:",
            "contents: write",
            "git merge-base --is-ancestor",
            "origin/main",
            "docs/releases/${TAG}.md",
            "release create",
            "--verify-tag",
            "--prerelease",
            "prerelease=true",
        ] {
            assert!(
                workflow.contains(needle),
                "release workflow missing expected guard/action: {needle}"
            );
        }
        assert!(
            !workflow.contains("cargo publish"),
            "release workflow must not publish crates"
        );
        assert!(
            !workflow.contains("git fetch --force"),
            "release workflow must not force-fetch refs"
        );
    }

    #[test]
    fn release_workflow_supports_dispatch_and_milestone_closure() {
        let root = repo_root().expect("repo root");
        let workflow =
            fs::read_to_string(root.join(".github/workflows/release.yml")).expect("workflow");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for needle in [
            "workflow_dispatch:",
            "RELEASE_TAG:",
            "inputs.tag",
            "issues: write",
            "milestones?state=all",
            "OPEN_ISSUES",
            "gh release view",
            "Close release milestone",
            "--method PATCH",
            "-f state=closed",
        ] {
            assert!(
                workflow.contains(needle),
                "release workflow missing dispatch/milestone guard: {needle}"
            );
        }
        assert!(
            policy.contains("publication_before_milestone_closure = true"),
            "release policy must require publication before milestone closure"
        );
        let publish_step = workflow
            .find("name: Publish GitHub release")
            .expect("release workflow must publish the GitHub Release");
        let close_step = workflow
            .find("name: Close release milestone")
            .expect("release workflow must close the milestone");
        assert!(
            publish_step < close_step,
            "release workflow must close the milestone only after publishing the release"
        );
        assert!(
            workflow.contains("MILESTONE_NUMBER=\"${{ steps.release.outputs.milestone_number }}\"")
                && workflow
                    .contains("MILESTONE_STATE=\"${{ steps.release.outputs.milestone_state }}\"",),
            "milestone closure must be driven by release verification outputs"
        );
    }

    #[test]
    fn release_workflow_paginates_milestone_lookup() {
        let root = repo_root().expect("repo root");
        let workflow =
            fs::read_to_string(root.join(".github/workflows/release.yml")).expect("workflow");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        assert!(
            policy.contains("milestone_lookup = \"paginated_all_states\""),
            "release automation policy must require paginated all-state milestone lookup"
        );
        let verification_step = workflow
            .split("name: Verify tag target, release notes, and milestone")
            .nth(1)
            .and_then(|tail| tail.split("name: Publish GitHub release").next())
            .expect("release verification step");
        assert!(
            verification_step
                .contains("gh api --paginate \"repos/${GITHUB_REPOSITORY}/milestones?state=all&per_page=100\""),
            "release workflow must paginate all-state milestone lookup"
        );
    }

    #[test]
    fn release_milestone_lookup_consumes_complete_paginated_stream() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        let release_workflow =
            fs::read_to_string(root.join(".github/workflows/release.yml")).expect("workflow");
        let auto_workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        assert!(
            policy.contains("milestone_selection = \"complete_stream_first_match\""),
            "release automation policy must require full-stream milestone selection"
        );
        let release_verification = workflow_block(
            &release_workflow,
            "name: Verify tag target, release notes, and milestone",
            "name: Publish GitHub release",
        );
        assert!(
            milestone_lookup_consumes_complete_stream(release_verification),
            "release workflow must consume the full paginated milestone stream"
        );
        let auto_tag_job = workflow_block(
            &auto_workflow,
            "name: Create immutable release tag",
            "dispatch-release-publication:",
        );
        assert!(
            milestone_lookup_consumes_complete_stream(auto_tag_job),
            "auto-release workflow must consume the full paginated milestone stream"
        );
    }

    #[test]
    fn milestone_lookup_contract_rejects_head_truncation() {
        let lookup = r#"MILESTONE_JSON="$(gh api --paginate "repos/${GITHUB_REPOSITORY}/milestones?state=all&per_page=100" \
            --jq ".[] | select(.title == \"${TAG}\")" | head -n 1)""#;
        assert!(
            !milestone_lookup_consumes_complete_stream(lookup),
            "milestone lookup contract must reject head-based truncation"
        );
    }

    #[test]
    fn release_workflow_checks_out_release_tag_for_dispatch() {
        let root = repo_root().expect("repo root");
        let workflow =
            fs::read_to_string(root.join(".github/workflows/release.yml")).expect("workflow");
        let checkout_step = workflow
            .split("actions/checkout@")
            .nth(1)
            .and_then(|tail| tail.split("- name: Verify tag target").next())
            .expect("release workflow checkout block");
        let release_ref =
            "${{ github.event_name == 'workflow_dispatch' && inputs.tag || github.ref_name }}";
        assert!(
            checkout_step.contains(&format!("ref: {release_ref}")),
            "release workflow must read release notes from the tag being published"
        );
    }

    #[test]
    fn auto_release_tag_workflow_is_guarded() {
        let root = repo_root().expect("repo root");
        let workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for needle in [
            "workflow_run:",
            "workflow_dispatch:",
            "workflows: [\"CI\"]",
            "branches: [main]",
            "github.event.workflow_run.conclusion == 'success'",
            "github.event.workflow_run.event == 'push'",
            "inputs.tag",
            "inputs.sha",
            "/commits/${SHA}/pulls",
            "^release/v[0-9]+",
            "docs/releases/${TAG}.md",
            "git tag -a",
            "refusing to move",
            "gh workflow run release.yml",
            "actions: write",
        ] {
            assert!(
                workflow.contains(needle),
                "auto release workflow missing guard/action: {needle}"
            );
        }
        assert!(
            !workflow.contains("--force"),
            "auto release workflow must not force any git operation"
        );
        assert!(
            policy.contains("source_event = \"push\""),
            "release automation policy must restrict auto-tagging to push CI runs"
        );
        assert!(
            policy.contains("manual_recovery_trigger = \"workflow_dispatch\""),
            "release automation policy must allow manual recovery dispatch"
        );
        assert!(
            policy.contains("manual_recovery_requires = ["),
            "release automation policy must structure manual recovery requirements"
        );
        for required in [
            "main_ci_success",
            "merged_release_pr",
            "manual_tag_matches_release_prep_pr",
        ] {
            assert!(
                policy.contains(required),
                "release automation policy missing manual recovery requirement: {required}"
            );
        }
    }

    #[test]
    fn auto_release_tag_manual_dispatch_checks_verified_main_sha() {
        let root = repo_root().expect("repo root");
        let workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        let identify_job = workflow_block(&workflow, "identify-release-pr:", "create-release-tag:");
        let manual_recovery = workflow_block(
            identify_job,
            "if [[ \"${EVENT_NAME}\" == \"workflow_dispatch\" ]]; then",
            "exit 0",
        );
        for required in [
            "workflow_dispatch:",
            "tag:",
            "sha:",
            "github.event_name == 'workflow_dispatch'",
            "INPUT_TAG:",
            "INPUT_SHA:",
            "git fetch origin main:refs/remotes/origin/main --tags",
            "git merge-base --is-ancestor \"${SHA}\" origin/main",
            "actions/workflows/ci.yml/runs",
            "head_sha=${SHA}",
            "status=success",
            "Manual recovery SHA must have a successful main CI run",
            "/commits/${SHA}/pulls",
            "Manual recovery SHA must resolve to exactly one merged release-prep PR.",
            "if [[ \"${TAG}\" != \"${INPUT_TAG}\" ]]",
            "release=true",
            "tag=${TAG}",
            "sha=${SHA}",
        ] {
            assert!(
                manual_recovery.contains(required) || workflow.contains(required),
                "auto-release manual recovery missing verified-sha contract: {required}"
            );
        }
    }

    #[test]
    fn auto_release_tag_workflow_scopes_job_permissions() {
        let root = repo_root().expect("repo root");
        let workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        assert!(
            policy.contains("job_permissions = \"least_privilege\""),
            "release automation policy must require least-privilege job permissions"
        );
        let top_level_permissions = workflow
            .split("permissions:")
            .nth(1)
            .and_then(|tail| tail.split("\njobs:").next())
            .expect("top-level permissions block");
        assert!(
            top_level_permissions.contains("contents: read")
                && !top_level_permissions.contains("write"),
            "top-level auto-release permissions must be read-only"
        );
        let identify_job = workflow
            .split("identify-release-pr:")
            .nth(1)
            .and_then(|tail| tail.split("create-release-tag:").next())
            .expect("identify-release-pr job block");
        assert!(
            identify_job.contains(
                "permissions:\n      actions: read\n      contents: read\n      pull-requests: read",
            )
                && !identify_job.contains("write"),
            "identify-release-pr must only read actions, contents, and pull requests"
        );
        let tag_job = workflow
            .split("create-release-tag:")
            .nth(1)
            .and_then(|tail| tail.split("dispatch-release-publication:").next())
            .expect("create-release-tag job block");
        assert!(
            tag_job.contains("permissions:\n      contents: write\n      issues: read")
                && !tag_job.contains("actions: write")
                && !tag_job.contains("pull-requests: write"),
            "create-release-tag must only write contents and read issues"
        );
        let dispatch_job = workflow
            .split("dispatch-release-publication:")
            .nth(1)
            .expect("dispatch-release-publication job block");
        assert!(
            dispatch_job.contains("permissions:\n      actions: write")
                && !dispatch_job.contains("contents: write")
                && !dispatch_job.contains("issues: write")
                && !dispatch_job.contains("pull-requests: write"),
            "dispatch-release-publication must only write actions"
        );
    }

    #[test]
    fn auto_release_tag_uses_ephemeral_push_credentials() {
        let root = repo_root().expect("repo root");
        let workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        let tag_job = workflow
            .split("create-release-tag:")
            .nth(1)
            .and_then(|tail| tail.split("dispatch-release-publication:").next())
            .expect("create-release-tag job block");
        for required in [
            "persist-credentials: false",
            "AUTHENTICATED_ORIGIN=\"https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git\"",
            "git ls-remote --exit-code --tags \"${AUTHENTICATED_ORIGIN}\"",
            "git fetch \"${AUTHENTICATED_ORIGIN}\"",
            "git push \"${AUTHENTICATED_ORIGIN}\"",
        ] {
            assert!(
                tag_job.contains(required),
                "create-release-tag job missing ephemeral credential guard: {required}"
            );
        }
    }

    #[test]
    fn auto_release_tag_checks_milestone_before_tag_push() {
        let root = repo_root().expect("repo root");
        let workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        let milestone_check = workflow
            .find("OPEN_ISSUES")
            .expect("auto release workflow must check milestone open issue count");
        let tag_creation = workflow
            .find("git tag -a")
            .expect("auto release workflow must create an annotated tag");
        assert!(
            milestone_check < tag_creation,
            "auto release workflow must check milestone readiness before creating an immutable tag"
        );
    }

    #[test]
    fn auto_release_tag_dispatches_release_from_tag_ref() {
        let root = repo_root().expect("repo root");
        let workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        assert!(
            workflow.contains(r#"gh workflow run release.yml --ref "${TAG}" -f tag="${TAG}""#),
            "auto release workflow must dispatch release.yml from the created tag ref"
        );
    }

    #[test]
    fn auto_release_tag_dispatches_with_explicit_repo() {
        let root = repo_root().expect("repo root");
        let workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        let dispatch_job = workflow
            .split("dispatch-release-publication:")
            .nth(1)
            .expect("dispatch-release-publication job block");
        assert!(
            dispatch_job.contains("GH_REPO: ${{ github.repository }}")
                || dispatch_job.contains("--repo \"${GITHUB_REPOSITORY}\""),
            "release dispatch must name the repository explicitly"
        );
    }

    #[test]
    fn release_tag_recovery_policy_is_structured() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for needle in [
            "[release_tags]",
            "mutation = \"forbidden\"",
            "recovery = \"publish_existing_valid_tag\"",
            "requires_main_reachable_target = true",
            "normal_creation = \"auto_after_release_pr_main_ci\"",
            "manual_creation = \"operator_fallback\"",
        ] {
            assert!(
                policy.contains(needle),
                "release policy missing structured field: {needle}"
            );
        }
    }

    #[test]
    fn release_runbook_policy_is_structured() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for required in [
            "[release_runbook]",
            "prepare_branch",
            "write_release_thesis",
            "diff_previous_tag",
            "reconcile_signpost_docs",
            "merge_gate",
            "auto_tag_publish",
            "watch_workflow",
            "capture_evidence",
            "capture_release_report",
            "manual_fallback_target = \"verified_main_merge_commit\"",
            "post_release_milestone_lookup = \"all_states_paginated\"",
            "cargo xtask verify",
            "cargo test -p xtask release_",
            "git diff --stat <previous-tag>..HEAD",
            "git diff --name-status <previous-tag>..HEAD",
            "git log --oneline <previous-tag>..HEAD",
            "gh pr checks",
            "gh release view",
            "release_thesis",
            "release_issue",
            "previous_tag_diff_stat",
            "previous_tag_diff_name_status",
            "previous_tag_log",
            "milestone_zero_open_at_tag_time",
            "no_crates_io_publication",
            "release_report",
            "plan_versus_actual",
            "fallout_issues",
            "next_release_thesis",
        ] {
            assert!(
                policy.contains(required),
                "release runbook policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn rust_workspace_lints_define_safety_baseline() {
        let root = repo_root().expect("repo root");
        let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("workspace manifest");
        for required in [
            "[workspace.lints.rust]",
            "unsafe_code = \"forbid\"",
            "missing_debug_implementations = \"deny\"",
            "[workspace.lints.clippy]",
            "all = { level = \"deny\", priority = -1 }",
            "pedantic = { level = \"deny\", priority = -1 }",
        ] {
            assert!(
                manifest.contains(required),
                "workspace lint baseline missing structured field: {required}"
            );
        }
    }

    #[test]
    fn compiler_settings_schema_declares_jsonl_contract() {
        let root = repo_root().expect("repo root");
        let schema =
            read_json_file(&root.join("docs/schemas/edict.compiler-settings.v1.schema.json"));

        assert_eq!(
            json_str(&schema, "$schema"),
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(
            json_str(&schema, "$id"),
            "https://flyingrobots.dev/schemas/edict/compiler-settings/v1"
        );
        assert_eq!(json_str(&schema, "type"), "object");
        assert_eq!(
            schema.get("additionalProperties").and_then(Value::as_bool),
            Some(false)
        );
        for required in ["schema", "type", "operation"] {
            assert!(
                json_string_array_contains(&schema, "required", required),
                "compiler settings schema must require `{required}`"
            );
        }

        let properties = json_object(&schema, "properties");
        let record_schema = properties
            .get("schema")
            .unwrap_or_else(|| panic!("compiler settings schema missing `schema` property"));
        assert_eq!(
            record_schema.get("const").and_then(Value::as_str),
            Some("edict.compiler.settings/v1")
        );
        let record_type = properties
            .get("type")
            .unwrap_or_else(|| panic!("compiler settings schema missing `type` property"));
        assert_eq!(
            record_type.get("const").and_then(Value::as_str),
            Some("compilerSettings")
        );
        let operation = properties
            .get("operation")
            .unwrap_or_else(|| panic!("compiler settings schema missing `operation` property"));
        assert!(
            json_string_array_contains(operation, "enum", "check"),
            "compiler settings schema must declare the `check` operation"
        );
        assert!(
            properties.contains_key("directoryExtensions"),
            "compiler settings schema missing deterministic directory extension field"
        );
        assert!(
            properties.contains_key("followSymlinks"),
            "compiler settings schema missing symlink traversal field"
        );
    }

    fn cli_record_schema(root: &Path, file: &str, id: &str, record_schema: &str) -> Value {
        let schema = read_json_file(&root.join("docs/schemas").join(file));
        assert_eq!(
            json_str(&schema, "$schema"),
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(json_str(&schema, "$id"), id);
        assert_eq!(json_str(&schema, "type"), "object");
        assert_eq!(
            schema.get("additionalProperties").and_then(Value::as_bool),
            Some(false),
            "{file} must forbid additional properties"
        );
        let properties = json_object(&schema, "properties");
        assert_eq!(
            property_const(properties, "schema"),
            Some(record_schema),
            "{file} `schema` field must pin `{record_schema}`"
        );
        schema
    }

    fn property_const<'a>(
        properties: &'a serde_json::Map<String, Value>,
        key: &str,
    ) -> Option<&'a str> {
        properties.get(key)?.get("const")?.as_str()
    }

    fn property_enum_has_int(
        properties: &serde_json::Map<String, Value>,
        key: &str,
        expected: i64,
    ) -> bool {
        properties
            .get(key)
            .map(|property| json_array(property, "enum"))
            .is_some_and(|values| values.iter().any(|value| value.as_i64() == Some(expected)))
    }

    fn one_of_branch<'a>(schema: &'a Value, kind: &str) -> &'a Value {
        json_array(schema, "oneOf")
            .iter()
            .find(|branch| {
                branch
                    .pointer("/properties/kind/const")
                    .and_then(Value::as_str)
                    == Some(kind)
            })
            .unwrap_or_else(|| panic!("schema missing `oneOf` branch for kind `{kind}`"))
    }

    #[test]
    fn compiler_input_schema_declares_jsonl_contract() {
        let root = repo_root().expect("repo root");
        let schema = cli_record_schema(
            &root,
            "edict.compiler-input.v1.schema.json",
            "https://flyingrobots.dev/schemas/edict/compiler-input/v1",
            "edict.compiler.input/v1",
        );
        for required in ["schema", "type", "kind"] {
            assert!(
                json_string_array_contains(&schema, "required", required),
                "compiler input schema must require `{required}`"
            );
        }
        let properties = json_object(&schema, "properties");
        assert_eq!(property_const(properties, "type"), Some("compilerInput"));
        let kind = properties
            .get("kind")
            .unwrap_or_else(|| panic!("compiler input schema missing `kind` property"));
        for variant in ["source", "path", "pathList", "directory", "glob"] {
            assert!(
                json_string_array_contains(kind, "enum", variant),
                "compiler input schema must declare the `{variant}` input kind"
            );
        }
        for field in ["name", "source", "path", "paths", "pattern"] {
            assert!(
                properties.contains_key(field),
                "compiler input schema missing variant field `{field}`"
            );
        }

        // Input kinds must be mutually exclusive: one `oneOf` branch per kind,
        // each forbidding the other kinds' variant fields so hybrid records
        // (e.g. a `source` record that also carries `path`) cannot validate.
        let branches = json_array(&schema, "oneOf");
        assert_eq!(
            branches.len(),
            5,
            "compiler input schema must declare one `oneOf` branch per input kind"
        );
        let exclusivity = [
            ("source", ["path", "paths", "pattern"].as_slice()),
            ("path", ["name", "source", "paths", "pattern"].as_slice()),
            ("pathList", ["name", "source", "path", "pattern"].as_slice()),
            (
                "directory",
                ["name", "source", "paths", "pattern"].as_slice(),
            ),
            ("glob", ["name", "source", "path", "paths"].as_slice()),
        ];
        for (kind, forbidden) in exclusivity {
            let branch = one_of_branch(&schema, kind);
            for field in forbidden {
                assert_eq!(
                    branch.pointer(&format!("/properties/{field}")),
                    Some(&Value::Bool(false)),
                    "`{kind}` input branch must forbid `{field}` for mutual exclusivity"
                );
            }
        }
    }

    #[test]
    fn check_result_schema_declares_jsonl_contract() {
        let root = repo_root().expect("repo root");
        let schema = cli_record_schema(
            &root,
            "edict.cli-check-result.v1.schema.json",
            "https://flyingrobots.dev/schemas/edict/cli-check-result/v1",
            "edict.cli.check-result/v1",
        );
        for required in ["schema", "type", "command", "input", "status"] {
            assert!(
                json_string_array_contains(&schema, "required", required),
                "check result schema must require `{required}`"
            );
        }
        let properties = json_object(&schema, "properties");
        assert_eq!(property_const(properties, "type"), Some("checkResult"));
        assert_eq!(property_const(properties, "command"), Some("check"));
        assert_eq!(property_const(properties, "status"), Some("ok"));
        assert!(
            properties.contains_key("input"),
            "check result schema missing `input` descriptor field"
        );
    }

    #[test]
    fn diagnostic_schema_declares_jsonl_contract() {
        let root = repo_root().expect("repo root");
        let schema = cli_record_schema(
            &root,
            "edict.cli-diagnostic.v1.schema.json",
            "https://flyingrobots.dev/schemas/edict/cli-diagnostic/v1",
            "edict.cli.diagnostic/v1",
        );
        for required in [
            "schema", "type", "command", "severity", "stage", "kind", "input",
        ] {
            assert!(
                json_string_array_contains(&schema, "required", required),
                "diagnostic schema must require `{required}`"
            );
        }
        let properties = json_object(&schema, "properties");
        assert_eq!(property_const(properties, "type"), Some("diagnostic"));
        assert_eq!(property_const(properties, "command"), Some("check"));
        let stage = properties
            .get("stage")
            .unwrap_or_else(|| panic!("diagnostic schema missing `stage` property"));
        for value in ["parse", "semantic", "cli"] {
            assert!(
                json_string_array_contains(stage, "enum", value),
                "diagnostic schema must declare the `{value}` stage"
            );
        }
        for optional in ["span", "line", "message"] {
            assert!(
                properties.contains_key(optional),
                "diagnostic schema missing optional field `{optional}`"
            );
        }
    }

    #[test]
    fn event_schema_declares_jsonl_contract() {
        let root = repo_root().expect("repo root");
        let schema = cli_record_schema(
            &root,
            "edict.cli-event.v1.schema.json",
            "https://flyingrobots.dev/schemas/edict/cli-event/v1",
            "edict.cli.event/v1",
        );
        for required in [
            "schema", "type", "command", "status", "checked", "errors", "exitCode",
        ] {
            assert!(
                json_string_array_contains(&schema, "required", required),
                "event schema must require `{required}`"
            );
        }
        let properties = json_object(&schema, "properties");
        assert_eq!(property_const(properties, "type"), Some("status"));
        assert_eq!(property_const(properties, "command"), Some("check"));
        let status = properties
            .get("status")
            .unwrap_or_else(|| panic!("event schema missing `status` property"));
        for value in ["ok", "error"] {
            assert!(
                json_string_array_contains(status, "enum", value),
                "event schema must declare the `{value}` terminal status"
            );
        }
        for code in [0, 1, 2] {
            assert!(
                property_enum_has_int(properties, "exitCode", code),
                "event schema must declare exit code `{code}`"
            );
        }

        // `status` and `exitCode` must be coupled so the schema cannot accept
        // contradictory terminal records (e.g. `status: ok` with `exitCode: 1`).
        let coupling = json_array(&schema, "allOf");
        let rule = coupling
            .first()
            .unwrap_or_else(|| panic!("event schema must couple `status` to `exitCode`"));
        assert_eq!(
            rule.pointer("/then/properties/exitCode/const")
                .and_then(Value::as_i64),
            Some(0),
            "event schema must require exit code 0 when status is `ok`"
        );
        let error_codes = rule
            .pointer("/else/properties/exitCode/enum")
            .and_then(Value::as_array)
            .unwrap_or_else(|| {
                panic!("event schema must constrain exit code when status is `error`")
            });
        for code in [1, 2] {
            assert!(
                error_codes.iter().any(|value| value.as_i64() == Some(code)),
                "event schema must allow exit code `{code}` when status is `error`"
            );
        }
    }

    #[test]
    fn info_schema_declares_jsonl_contract() {
        let root = repo_root().expect("repo root");
        let schema = cli_record_schema(
            &root,
            "edict.cli-info.v1.schema.json",
            "https://flyingrobots.dev/schemas/edict/cli-info/v1",
            "edict.cli.info/v1",
        );
        for required in ["schema", "type", "topic", "version"] {
            assert!(
                json_string_array_contains(&schema, "required", required),
                "info schema must require `{required}`"
            );
        }
        let properties = json_object(&schema, "properties");
        assert_eq!(property_const(properties, "type"), Some("info"));
        let topic = properties
            .get("topic")
            .unwrap_or_else(|| panic!("info schema missing `topic` property"));
        for value in ["help", "version"] {
            assert!(
                json_string_array_contains(topic, "enum", value),
                "info schema must declare the `{value}` topic"
            );
        }
        for field in ["usage", "requestSchemas", "exitCodes", "docs"] {
            assert!(
                properties.contains_key(field),
                "info schema missing help field `{field}`"
            );
        }

        // `requestSchemas` is pinned to the exact accepted request identifiers.
        assert_eq!(
            schema
                .pointer("/properties/requestSchemas/const")
                .and_then(Value::as_array),
            Some(&vec![
                Value::from("edict.compiler.settings/v1"),
                Value::from("edict.compiler.input/v1"),
            ]),
            "info schema must pin the exact accepted request schemas"
        );
        // `exitCodes` is an ordered 0/1/2 triple, not any subset/order.
        let exit_prefix = schema
            .pointer("/properties/exitCodes/prefixItems")
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("info schema must pin exitCodes as an ordered triple"));
        let codes: Vec<i64> = exit_prefix
            .iter()
            .filter_map(|item| {
                item.pointer("/properties/code/const")
                    .and_then(Value::as_i64)
            })
            .collect();
        assert_eq!(
            codes,
            [0, 1, 2],
            "info schema exitCodes must be the 0/1/2 triple in order"
        );
        assert_eq!(
            schema.pointer("/properties/exitCodes/items"),
            Some(&Value::Bool(false)),
            "info schema must forbid extra exit codes beyond the triple"
        );

        // The conditional must key on `topic == "help"`, require the help-only
        // fields, and forbid them on the `version` topic.
        let rule = json_array(&schema, "allOf")
            .first()
            .unwrap_or_else(|| panic!("info schema must conditionally constrain by topic"));
        assert_eq!(
            rule.pointer("/if/properties/topic/const")
                .and_then(Value::as_str),
            Some("help"),
            "info schema conditional must key on the `help` topic"
        );
        for field in ["usage", "requestSchemas", "exitCodes", "docs"] {
            assert!(
                rule.pointer("/then/required")
                    .and_then(Value::as_array)
                    .is_some_and(|req| req.iter().any(|value| value == field)),
                "info schema `help` topic must require `{field}`"
            );
            assert_eq!(
                rule.pointer(&format!("/else/properties/{field}")),
                Some(&Value::Bool(false)),
                "info schema `version` topic must forbid help field `{field}`"
            );
        }
    }

    #[test]
    fn review_bot_fallback_policy_is_structured() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/review-process/policy.toml"))
            .expect("review policy");
        let bots = toml_section(&policy, "[review_bots]");
        assert_eq!(toml_string_value(&bots, "primary_bot"), "CodeRabbit");
        assert_eq!(
            toml_string_value(&bots, "primary_review_required_when"),
            "actively_reviewing"
        );
        assert_eq!(
            toml_array_values(&bots, "primary_unavailable_states"),
            [
                "rate_limited".to_owned(),
                "insufficient_usage_credits".to_owned(),
                "out_of_credits".to_owned(),
            ]
        );
        assert_eq!(
            toml_string_value(&bots, "fallback_request"),
            "@codex review please"
        );
        assert!(toml_bool_value(
            &bots,
            "fallback_required_when_primary_unavailable"
        ));
        assert!(toml_bool_value(
            &bots,
            "fallback_response_required_before_merge"
        ));
        assert_eq!(
            toml_string_value(&bots, "goal"),
            "at_least_one_automated_or_human_review"
        );
        assert_eq!(toml_string_value(&bots, "merge_without_review"), "blocked");
    }

    #[test]
    fn release_topic_audit_policy_sets_minimums() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for required in [
            "[release_topic_audit]",
            "scope = \"docs/topics\"",
            "timing = \"release_prep_before_pr\"",
            "evidence_location = \"release_issue_or_pr_before_merge\"",
            "release_blocking = true",
            "coverage_definition = \"audited_topic_shelves / total_topic_shelves\"",
            "accuracy_definition = \"accurate_audited_topic_shelves / audited_topic_shelves\"",
            "stale_current_truth = \"correct_or_remove_before_counting_accurate\"",
            "topic_shelf_total",
            "topic_shelf_audited",
            "accurate_audited_topic_shelves",
            "coverage_percent",
            "accuracy_percent",
            "findings_fixed_or_release_blocking",
        ] {
            assert!(
                policy.contains(required),
                "release topic audit policy missing structured field: {required}"
            );
        }

        assert!(
            toml_integer_value(&policy, "coverage_floor_percent") >= 90,
            "release topic audit coverage floor must be at least 90%"
        );
        assert!(
            toml_integer_value(&policy, "accuracy_floor_percent") >= 90,
            "release topic audit accuracy floor must be at least 90%"
        );
    }

    fn toml_integer_value(policy: &str, key: &str) -> u16 {
        policy
            .lines()
            .find_map(|line| {
                let (raw_key, raw_value) = line.trim().split_once('=')?;
                if raw_key.trim() == key {
                    raw_value.trim().parse::<u16>().ok()
                } else {
                    None
                }
            })
            .unwrap_or_else(|| panic!("release policy missing integer field `{key}`"))
    }

    fn toml_string_value(policy: &str, key: &str) -> String {
        policy
            .lines()
            .find_map(|line| {
                let (raw_key, raw_value) = line.trim().split_once('=')?;
                (raw_key.trim() == key).then(|| raw_value.trim().trim_matches('"').to_owned())
            })
            .unwrap_or_else(|| panic!("policy missing string field `{key}`"))
    }

    fn toml_bool_value(policy: &str, key: &str) -> bool {
        policy
            .lines()
            .find_map(|line| {
                let (raw_key, raw_value) = line.trim().split_once('=')?;
                (raw_key.trim() == key).then(|| raw_value.trim() == "true")
            })
            .unwrap_or_else(|| panic!("policy missing boolean field `{key}`"))
    }

    fn toml_array_values(policy: &str, key: &str) -> Vec<String> {
        let mut lines = policy.lines().map(str::trim);
        while let Some(line) = lines.next() {
            if !line.starts_with(key) {
                continue;
            }
            let mut values = Vec::new();
            for value_line in lines.by_ref() {
                if value_line == "]" {
                    return values;
                }
                values.push(
                    value_line
                        .trim_end_matches(',')
                        .trim_matches('"')
                        .to_owned(),
                );
            }
        }
        panic!("policy missing array field `{key}`")
    }

    #[test]
    fn release_automation_policy_is_structured() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for required in [
            "[release_automation]",
            "trigger = \"successful_main_ci_after_release_prep_merge\"",
            "source_branch_pattern = \"release/vX.Y.Z-alpha.N-prep\"",
            "tag_derivation = \"strip_release_prefix_and_prep_suffix\"",
            "publication_trigger = \"workflow_dispatch\"",
            "close_milestone = \"after_github_release_when_zero_open_issues\"",
            "idempotency = \"existing_same_target_tag_ok\"",
            "existing_different_target_tag = \"fail_without_mutation\"",
            "merged_release_pr",
            "tag_absent_or_same_target",
            "zero_open_milestone_issues",
        ] {
            assert!(
                policy.contains(required),
                "release automation policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_3_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for required in [
            "[release_notes.v0_3_0_alpha_1]",
            "tag = \"v0.3.0-alpha.1\"",
            "target_date = \"2026-07-15\"",
            "status = \"published\"",
            "compiler_spine",
            "surface_validation_split",
            "canonical_core_encoder",
            "reviewed_core_golden_fixture",
            "exact_core_digest_fixture",
            "no_target_lowering",
            "no_bundle_admission",
        ] {
            assert!(
                policy.contains(required),
                "v0.3 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_4_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for required in [
            "[release_notes.v0_4_0_alpha_1]",
            "tag = \"v0.4.0-alpha.1\"",
            "target_date = \"2026-07-29\"",
            "status = \"published\"",
            "target_profile_conformance",
            "lowerability_direct_adapter",
            "contract_bundle_manifest_validation",
            "no_target_lowerer_execution",
            "no_admission_policy",
            "no_crates_io_publish",
        ] {
            assert!(
                policy.contains(required),
                "v0.4 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_5_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for required in [
            "[release_notes.v0_5_0_alpha_1]",
            "tag = \"v0.5.0-alpha.1\"",
            "target_date = \"2026-08-12\"",
            "status = \"published\"",
            "edict_owned_continuum_participation_boundary",
            "gate_c_admission_request_validation",
            "gate_c_admission_receipt_validation",
            "admission_request_digest_binding",
            "policy_epoch_receipt_binding",
            "invocation_capability_evidence_validation",
            "release_automation_after_main_ci",
            "no_participant_policy_evaluation",
            "no_participant_identity_delegation_or_revocation",
            "no_admission_ledger_persistence",
            "no_signature_verification",
            "no_target_lowerer_execution",
            "no_bundle_digest_recomputation",
            "no_crates_io_publish",
        ] {
            assert!(
                policy.contains(required),
                "v0.5 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_6_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        let v0_6_policy = toml_section(&policy, "[release_notes.v0_6_0_alpha_1]");
        for required in [
            "[release_notes.v0_6_0_alpha_1]",
            "tag = \"v0.6.0-alpha.1\"",
            "target_date = \"2026-08-26\"",
            "status = \"published\"",
            "editor_highlight_roles",
            "tree_sitter_grammar_source",
            "textmate_grammar_artifact",
            "vscode_cursor_extension_package",
            "topic_shelf_coverage_audit",
            "no_compiler_cli",
            "no_language_server",
            "no_marketplace_publication",
            "no_target_lowering",
            "no_admission_tooling",
            "no_crates_io_publish",
        ] {
            assert!(
                v0_6_policy.contains(required),
                "v0.6 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_7_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        let v0_7_policy = toml_section(&policy, "[release_notes.v0_7_0_alpha_1]");
        for required in [
            "[release_notes.v0_7_0_alpha_1]",
            "tag = \"v0.7.0-alpha.1\"",
            "target_date = \"2026-09-09\"",
            "status = \"published\"",
            "release_issue = 59",
            "published_at = \"2026-06-27T22:31:49Z\"",
            "release_url = \"https://github.com/flyingrobots/edict/releases/tag/v0.7.0-alpha.1\"",
            "tag_object = \"f7888160f7f9a0d7b9b82d4f78bb38b886856a1e\"",
            "peeled_commit = \"6f9c731b4f36d3283dcb448b14761832ab916b07\"",
            "release_notes_source_commit = \"6f9c731b4f36d3283dcb448b14761832ab916b07\"",
            "post_publication_evidence_pr = 61",
            "main_ci_run = 28303787401",
            "auto_release_tag_run = 28303801200",
            "release_workflow_run = 28303809157",
            "milestone_number = 8",
            "milestone_closed_at = \"2026-06-27T22:31:50Z\"",
            "milestone_open_issues = 0",
            "release_assets = 0",
            "crates_io_published = false",
            "file_backed_authority_facts",
            "operation_profile_facts",
            "profile_write_class_allowances",
            "effect_write_classes",
            "budget_facts",
            "lawpack_source_identity",
            "target_profile_source_identity",
            "deterministic_loaded_fact_harness",
            "stable_load_failure_kinds",
            "authority_fact_governance_design_note",
            "release_policy_and_rust_standards_hardening",
            "review_bot_fallback_policy",
            "no_trusted_lawpack_or_target_profile_authorship",
            "no_full_lawpack_manifest_loading",
            "no_full_target_profile_manifest_loading",
            "no_obstruction_obligation_adapter_footprint_cost_or_target_capability_corpus_loading",
            "no_global_registry_trust_root_identity_system_or_revocation_model",
            "no_target_ir_generation",
            "no_full_effectful_source_lowering",
            "no_admission_execution_workflow",
            "no_crates_io_publish",
        ] {
            assert!(
                v0_7_policy.contains(required),
                "v0.7 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_8_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        let v0_8_policy = toml_section(&policy, "[release_notes.v0_8_0_alpha_1]");
        for required in [
            "[release_notes.v0_8_0_alpha_1]",
            "tag = \"v0.8.0-alpha.1\"",
            "target_date = \"2026-09-23\"",
            "status = \"published\"",
            "release_issue = 62",
            "published_at = \"2026-06-28T01:41:16Z\"",
            "release_url = \"https://github.com/flyingrobots/edict/releases/tag/v0.8.0-alpha.1\"",
            "tag_object = \"32e843c5e5f7b9252078c2b8a99afa23daeab411\"",
            "peeled_commit = \"c6a166ccea0fcb61fff9b8d76bfb5d51d613e2eb\"",
            "release_notes_source_commit = \"c6a166ccea0fcb61fff9b8d76bfb5d51d613e2eb\"",
            "main_ci_run = 28307840316",
            "auto_release_tag_run = 28307856119",
            "release_workflow_run = 28307864582",
            "milestone_number = 9",
            "milestone_closed_at = \"2026-06-28T01:41:16Z\"",
            "milestone_open_issues = 0",
            "release_assets = 0",
            "crates_io_published = false",
            "minimal_effectful_compiler_spine",
            "core_effect_node_model",
            "core_obstruction_arm_model",
            "canonical_effect_node_encoding",
            "file_backed_effectful_compiler_context",
            "annotated_effectful_let_lowering",
            "deterministic_obstruction_map_lowering",
            "source_order_stable_obstruction_binders",
            "unsupported_effectful_branch_yield_rejection",
            "chained_effect_call_rejection",
            "typed_effect_call_rejection",
            "duplicate_obstruction_failure_rejection",
            "pure_core_golden_stability",
            "no_target_ir_generation",
            "no_target_runtime_execution",
            "no_adapter_composition",
            "no_public_cli",
            "no_admission_execution_workflow",
            "no_lawpack_governance_implementation",
            "no_crates_io_publish",
        ] {
            assert!(
                v0_8_policy.contains(required),
                "v0.8 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_9_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        let v0_9_policy = toml_section(&policy, "[release_notes.v0_9_0_alpha_1]");
        for required in [
            "[release_notes.v0_9_0_alpha_1]",
            "tag = \"v0.9.0-alpha.1\"",
            "target_date = \"2026-10-07\"",
            "status = \"published\"",
            "release_issue = 70",
            "published_at = \"2026-06-28T07:04:06Z\"",
            "release_url = \"https://github.com/flyingrobots/edict/releases/tag/v0.9.0-alpha.1\"",
            "tag_object = \"c6a4ea6b10d438cd407cd7f273fecf1fd012b2d3\"",
            "peeled_commit = \"81bacc5a240bd3ea50af934a3611ce6b3f505043\"",
            "release_notes_source_commit = \"81bacc5a240bd3ea50af934a3611ce6b3f505043\"",
            "main_ci_run = 28314566818",
            "auto_release_tag_run = 28314582826",
            "release_workflow_run = 28314590143",
            "milestone_number = 10",
            "milestone_closed_at = \"2026-06-28T07:04:06Z\"",
            "milestone_open_issues = 0",
            "release_assets = 0",
            "crates_io_published = false",
            "first_target_ir_alpha",
            "echo_span_ir_review_artifact",
            "gitwarp_commit_reducer_ir_review_artifact",
            "lowerability_to_target_ir_bridge",
            "explicit_target_profile_selection",
            "stable_target_lowering_failure_kinds",
            "core_obligation_preservation",
            "target_ir_topic_shelf",
            "no_runtime_execution",
            "no_echo_verifier",
            "no_gitwarp_commit_creation",
            "no_gitwarp_crdt_reducer_verification",
            "no_general_target_plugin_dispatch",
            "no_canonical_target_ir_bytes_or_digests",
            "no_bundle_or_admission_generation",
            "no_v2_adapter_composition",
            "no_public_cli",
            "no_crates_io_publish",
        ] {
            assert!(
                v0_9_policy.contains(required),
                "v0.9 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_10_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        let v0_10_policy = toml_section(&policy, "[release_notes.v0_10_0_alpha_1]");
        for required in [
            "[release_notes.v0_10_0_alpha_1]",
            "tag = \"v0.10.0-alpha.1\"",
            "target_date = \"2026-10-21\"",
            "status = \"published\"",
            "release_issue = 76",
            "published_at = \"2026-06-29T04:21:11Z\"",
            "release_url = \"https://github.com/flyingrobots/edict/releases/tag/v0.10.0-alpha.1\"",
            "tag_object = \"11e516c8ea8be5fa6739efd545c5b8fb40cbc46d\"",
            "peeled_commit = \"622834138af249e70d717d6b7a940e4b01e23f4d\"",
            "release_notes_source_commit = \"622834138af249e70d717d6b7a940e4b01e23f4d\"",
            "main_ci_run = 28348355987",
            "auto_release_tag_run = 28348383070",
            "release_workflow_run = 28348397035",
            "milestone_number = 11",
            "milestone_closed_at = \"2026-06-29T04:21:11Z\"",
            "milestone_open_issues = 0",
            "release_assets = 0",
            "crates_io_published = false",
            "first_public_cli_surface",
            "jsonl_check_workflow",
            "deterministic_input_expansion",
            "compiler_settings_json_schema",
            "cli_stream_record_schemas",
            "structured_cli_diagnostics",
            "stable_diagnostic_kind_codes",
            "golden_cli_fixture_corpus",
            "cli_topic_shelf",
            "no_compile_lower_explain_bundle_or_admission_commands",
            "no_human_pretty_output",
            "no_embedded_json_schema_validation_engine",
            "no_language_server",
            "no_marketplace_packaging",
            "no_participant_policy_execution",
            "no_crates_io_publish",
        ] {
            assert!(
                v0_10_policy.contains(required),
                "v0.10 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_11_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        let v0_11_policy = toml_section(&policy, "[release_notes.v0_11_0_alpha_1]");
        for required in [
            "[release_notes.v0_11_0_alpha_1]",
            "tag = \"v0.11.0-alpha.1\"",
            "target_date = \"2026-11-04\"",
            "status = \"published\"",
            "release_issue = 109",
            "published_at = \"2026-06-30T07:58:29Z\"",
            "release_url = \"https://github.com/flyingrobots/edict/releases/tag/v0.11.0-alpha.1\"",
            "tag_object = \"e5d207527d737131e54a8d2614765e63ff7218e6\"",
            "peeled_commit = \"3eb71f6127e31b68ea4e0bb766623930ce24ae46\"",
            "release_notes_source_commit = \"3eb71f6127e31b68ea4e0bb766623930ce24ae46\"",
            "main_ci_run = 28429259737",
            "auto_release_tag_run = 28429294814",
            "release_workflow_run = 28429313876",
            "milestone_number = 12",
            "milestone_closed_at = \"2026-06-30T07:58:29Z\"",
            "milestone_open_issues = 0",
            "release_assets = 0",
            "crates_io_published = false",
            "contract_bundle_assembly",
            "semantic_bundle_digest_preimage",
            "release_bundle_digest_preimage",
            "bundle_digest_goldens",
            "canonical_target_ir_value_model",
            "canonical_target_ir_cbor_bytes",
            "target_ir_artifact_digest_frame",
            "target_ir_byte_digest_goldens",
            "computed_target_ir_bundle_assembly",
            "xtask_core_target_ir_bundle_golden_checks",
            "no_runtime_execution",
            "no_admission_execution",
            "no_participant_policy_logic",
            "no_verifier_completeness",
            "no_echo_verifier_completeness",
            "no_git_warp_commit_creation",
            "no_git_warp_crdt_reducer_verification",
            "no_general_target_plugin_dispatch",
            "no_additional_target_profiles",
            "no_extra_source_to_target_fixtures",
            "no_canonical_contract_bundle_manifest_bytes",
            "no_crates_io_publish",
        ] {
            assert!(
                v0_11_policy.contains(required),
                "v0.11 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn alpha_changelog_dates_match_release_policy() {
        let root = repo_root().expect("repo root");
        let changelog = fs::read_to_string(root.join("CHANGELOG.md")).expect("changelog");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for (tag, target) in [
            ("v0.2.0-alpha.1", "2026-07-01"),
            ("v0.3.0-alpha.1", "2026-07-15"),
            ("v0.4.0-alpha.1", "2026-07-29"),
            ("v0.5.0-alpha.1", "2026-08-12"),
            ("v0.6.0-alpha.1", "2026-08-26"),
            ("v0.7.0-alpha.1", "2026-09-09"),
            ("v0.8.0-alpha.1", "2026-09-23"),
            ("v0.9.0-alpha.1", "2026-10-07"),
            ("v0.10.0-alpha.1", "2026-10-21"),
            ("v0.11.0-alpha.1", "2026-11-04"),
        ] {
            assert!(
                policy.contains(&format!("tag = \"{tag}\"")),
                "release policy missing tag {tag}"
            );
            assert!(
                policy.contains(&format!("target_date = \"{target}\"")),
                "release policy missing target date {target}"
            );
            assert!(
                changelog.contains(&format!("## [{tag}] - {target}")),
                "{tag} changelog date must match release policy target date {target}"
            );
        }
    }

    #[test]
    fn release_policy_tracks_v0_2_boundary() {
        let root = repo_root().expect("repo root");
        let policy = fs::read_to_string(root.join("docs/topics/release-process/policy.toml"))
            .expect("release policy");
        for required in [
            "[release_notes.v0_2_0_alpha_1]",
            "tag = \"v0.2.0-alpha.1\"",
            "target_date = \"2026-07-01\"",
            "core_semantic_model",
            "normative_core_schema",
            "no_source_to_core_lowering",
            "no_canonical_encoder",
            "no_golden_core_bytes",
            "no_exact_core_digests",
            "no_target_lowering",
            "no_bundle_admission",
        ] {
            assert!(
                policy.contains(required),
                "v0.2 release policy missing structured field: {required}"
            );
        }
    }

    #[test]
    fn core_cddl_declares_v1_semantic_model() {
        let root = repo_root().expect("repo root");
        let cddl =
            fs::read_to_string(root.join("docs/abi/edict-core.cddl")).expect("Core schema exists");
        for needle in [
            "core-module =",
            "apiVersion: \"edict.core/v1\"",
            "core-type =",
            "core-intent =",
            "core-block =",
            "core-node =",
            "core-expr =",
            "core-predicate =",
            "core-fn-body =",
            "input-constraint =",
            "local-ref =",
            "alphaName:",
            "requiredOperationProfile:",
        ] {
            assert!(cddl.contains(needle), "Core CDDL missing {needle}");
        }
    }

    #[test]
    fn core_cddl_has_no_digest_freeze_fields() {
        let root = repo_root().expect("repo root");
        let cddl =
            fs::read_to_string(root.join("docs/abi/edict-core.cddl")).expect("Core schema exists");
        for forbidden in [
            "coreDigest",
            "canonicalCoreDigest",
            "canonicalBytes",
            "goldenBytes",
            "exactDigest",
        ] {
            assert!(
                !cddl.contains(forbidden),
                "v0.2 Core schema must not freeze byte/hash field `{forbidden}`"
            );
        }
    }

    #[test]
    fn core_schema_shape_fixtures_match_cddl() {
        let root = repo_root().expect("repo root");
        let cddl =
            fs::read_to_string(root.join("docs/abi/edict-core.cddl")).expect("Core schema exists");
        let fixture_root = root.join("fixtures/core/schema");
        let mut checked = 0usize;

        for expectation in [FixtureExpectation::Accept, FixtureExpectation::Reject] {
            let dir = fixture_root.join(expectation.dir_name());
            for entry in fs::read_dir(&dir).unwrap_or_else(|err| {
                panic!(
                    "read Core schema fixture directory {}: {err}",
                    dir.display()
                )
            }) {
                let path = entry.expect("fixture entry").path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("fields") {
                    continue;
                }
                let fixture = parse_core_schema_fixture(&path);
                assert_eq!(
                    fixture.expectation,
                    expectation,
                    "{} expectation disagrees with directory name",
                    path.display()
                );
                let shape = core_shape_fields(&cddl, &fixture.shape);
                let missing = shape
                    .required
                    .difference(&fixture.fields)
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let unknown = fixture
                    .fields
                    .difference(&shape.allowed)
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let valid = missing.is_empty() && unknown.is_empty();
                match expectation {
                    FixtureExpectation::Accept => assert!(
                        valid,
                        "{} should be accepted; missing {:?}, unknown {:?}",
                        path.display(),
                        missing,
                        unknown
                    ),
                    FixtureExpectation::Reject => assert!(
                        !valid,
                        "{} should be rejected by schema shape validation",
                        path.display()
                    ),
                }
                checked += 1;
            }
        }

        assert!(checked >= 5, "expected at least 5 Core schema fixtures");
    }

    #[derive(Debug)]
    struct CoreShapeFields {
        allowed: BTreeSet<String>,
        required: BTreeSet<String>,
    }

    #[derive(Debug)]
    struct CoreSchemaFixture {
        expectation: FixtureExpectation,
        fields: BTreeSet<String>,
        shape: String,
    }

    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    enum FixtureExpectation {
        Accept,
        Reject,
    }

    impl FixtureExpectation {
        fn dir_name(self) -> &'static str {
            match self {
                Self::Accept => "accepted",
                Self::Reject => "rejected",
            }
        }
    }

    fn core_shape_fields(cddl: &str, shape: &str) -> CoreShapeFields {
        let start = format!("{shape} = {{");
        let mut in_shape = false;
        let mut fields = CoreShapeFields {
            allowed: BTreeSet::new(),
            required: BTreeSet::new(),
        };
        for raw in cddl.lines() {
            let line = raw.split_once(';').map_or(raw, |(before, _)| before).trim();
            if !in_shape {
                if line == start {
                    in_shape = true;
                }
                continue;
            }
            if line.starts_with('}') {
                break;
            }
            let Some((field, _)) = line
                .strip_suffix(',')
                .unwrap_or(line)
                .strip_prefix("? ")
                .unwrap_or(line)
                .split_once(':')
            else {
                continue;
            };
            let field = field.trim();
            if field.is_empty() || field.contains(char::is_whitespace) {
                continue;
            }
            fields.allowed.insert(field.to_owned());
            if !line.starts_with("? ") {
                fields.required.insert(field.to_owned());
            }
        }
        assert!(
            !fields.allowed.is_empty(),
            "no fields parsed for Core shape {shape}"
        );
        fields
    }

    fn parse_core_schema_fixture(path: &Path) -> CoreSchemaFixture {
        let text = fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("read Core schema fixture {}: {err}", path.display()));
        let mut expectation = None;
        let mut shape = None;
        let mut fields = BTreeSet::new();
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once(' ') else {
                panic!("{} has malformed line `{line}`", path.display());
            };
            match key {
                "expect" => {
                    expectation = Some(match value {
                        "accept" => FixtureExpectation::Accept,
                        "reject" => FixtureExpectation::Reject,
                        other => panic!("{} has invalid expectation `{other}`", path.display()),
                    });
                }
                "field" => {
                    fields.insert(value.to_owned());
                }
                "shape" => {
                    shape = Some(value.to_owned());
                }
                other => panic!("{} has unknown directive `{other}`", path.display()),
            }
        }
        CoreSchemaFixture {
            expectation: expectation
                .unwrap_or_else(|| panic!("{} missing expectation", path.display())),
            fields,
            shape: shape.unwrap_or_else(|| panic!("{} missing shape", path.display())),
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("edict-xtask-{name}-{}", std::process::id()));
        fs::remove_dir_all(&dir).ok();
        fs::create_dir_all(&dir).expect("temp root");
        dir
    }
}
