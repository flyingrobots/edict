use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use edict_syntax::{
    compile_to_core, decode_lawpack_adapter, decode_lawpack_bundle, digest_core_module,
    digest_target_ir_artifact, encode_canonical_cbor, encode_core_module,
    encode_target_ir_artifact, lower_to_target_ir, parse_module, prepare_lawpack_compilation,
    CanonicalValue, TargetLoweringStatus,
};
use sha2::{Digest, Sha256};

use crate::goldens::{check_golden_file_with_command, write_golden_file};

const DIGEST_FRAME: &str = "edict.digest/v1";
const EXPORTS_COORDINATE: &str = "hello.echo.exports/v1";
const FIXTURE_ROOT: &str = "fixtures/lawpack/hello-echo";
const MANIFEST_CBOR: &str = "fixtures/lawpack/hello-echo/manifest.cbor";
const MANIFEST_DIGEST: &str = "fixtures/lawpack/hello-echo/manifest.sha256";
const EXPORTS_CBOR: &str = "fixtures/lawpack/hello-echo/exports.cbor";
const EXPORTS_DIGEST: &str = "fixtures/lawpack/hello-echo/exports.sha256";
const ADAPTER_CBOR: &str = "fixtures/lawpack/hello-echo/adapter.cbor";
const ADAPTER_DIGEST: &str = "fixtures/lawpack/hello-echo/adapter.sha256";
const TARGET_CONFIGURATION_CBOR: &str =
    "fixtures/lawpack/hello-echo/echo-operation-configuration.cbor";
const TARGET_CONFIGURATION_DIGEST: &str =
    "fixtures/lawpack/hello-echo/echo-operation-configuration.sha256";
const CREATE_GREETING_SOURCE: &str = "fixtures/lawpack/hello-echo/create-greeting.edict";
const CREATE_GREETING_CORE_CBOR: &str = "fixtures/lawpack/hello-echo/create-greeting.core.cbor";
const CREATE_GREETING_CORE_DIGEST: &str = "fixtures/lawpack/hello-echo/create-greeting.core.sha256";
const CREATE_GREETING_TARGET_IR_CBOR: &str =
    "fixtures/lawpack/hello-echo/create-greeting.target-ir.cbor";
const CREATE_GREETING_TARGET_IR_DIGEST: &str =
    "fixtures/lawpack/hello-echo/create-greeting.target-ir.sha256";
const CREATE_GREETING_RESULT_PROJECTION_CBOR: &str =
    "fixtures/lawpack/hello-echo/create-greeting.result-projection.cbor";
const CREATE_GREETING_RESULT_PROJECTION_DIGEST: &str =
    "fixtures/lawpack/hello-echo/create-greeting.result-projection.sha256";
const ADAPTER_COORDINATE: &str = "hello.echo.echo-dpo-adapter/v1";
const TARGET_CONFIGURATION_COORDINATE: &str = "hello.echo.echo-operation-configuration/v1";
const ECHO_TARGET_PROFILE_DIGEST: [u8; 32] = [
    0x2e, 0x24, 0x94, 0x12, 0x1a, 0xec, 0xf5, 0xe6, 0xa2, 0xd9, 0x20, 0xf5, 0xfb, 0x85, 0x40, 0x88,
    0x25, 0xd3, 0x94, 0x76, 0x5f, 0xad, 0x41, 0x48, 0x4c, 0x41, 0x63, 0x97, 0xc9, 0x20, 0xfb, 0x04,
];
const ECHO_TARGET_IR_DIGEST: [u8; 32] = [
    0x00, 0x57, 0x16, 0x7e, 0x68, 0xf5, 0x0c, 0x99, 0xdc, 0xce, 0x08, 0x7b, 0x3e, 0x1c, 0xd6, 0x77,
    0xd1, 0x7c, 0x5d, 0x1d, 0xc2, 0x38, 0xbd, 0xb5, 0x2d, 0x89, 0x46, 0x9e, 0x14, 0x72, 0xfc, 0x2f,
];
const WRITE_COMMAND: &str = "cargo xtask lawpack-goldens --write";
const CAUSAL_CELL_FIXTURE_ROOT: &str = "fixtures/lawpack/causal-cell";
const CAUSAL_CELL_MANIFEST_CBOR: &str = "fixtures/lawpack/causal-cell/manifest.cbor";
const CAUSAL_CELL_MANIFEST_DIGEST: &str = "fixtures/lawpack/causal-cell/manifest.sha256";
const CAUSAL_CELL_EXPORTS_CBOR: &str = "fixtures/lawpack/causal-cell/exports.cbor";
const CAUSAL_CELL_EXPORTS_DIGEST: &str = "fixtures/lawpack/causal-cell/exports.sha256";
const CAUSAL_CELL_ADAPTER_CBOR: &str = "fixtures/lawpack/causal-cell/adapter.cbor";
const CAUSAL_CELL_ADAPTER_DIGEST: &str = "fixtures/lawpack/causal-cell/adapter.sha256";
const CAUSAL_CELL_CONFIGURATION_CBOR: &str =
    "fixtures/lawpack/causal-cell/echo-operation-configuration.cbor";
const CAUSAL_CELL_CONFIGURATION_DIGEST: &str =
    "fixtures/lawpack/causal-cell/echo-operation-configuration.sha256";
const CAUSAL_CELL_EXPORTS_COORDINATE: &str = "causal.cell.exports/v1";
const CAUSAL_CELL_ADAPTER_COORDINATE: &str = "causal.cell.echo-adapter/v1";
const CAUSAL_CELL_CONFIGURATION_COORDINATE: &str = "echo.operation-lowering-configuration/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LawpackGoldenMode {
    Check,
    Write,
}

pub(crate) fn lawpack_goldens(root: &Path, mode: LawpackGoldenMode) -> Result<(), String> {
    let artifacts = hello_echo_golden_artifacts(root)?
        .into_iter()
        .chain(causal_cell_golden_artifacts()?);
    for (path, bytes) in artifacts {
        match mode {
            LawpackGoldenMode::Check => {
                check_golden_file_with_command(root, path, &bytes, WRITE_COMMAND)?;
            }
            LawpackGoldenMode::Write => write_golden_file(&root.join(path), &bytes)?,
        }
    }

    println!(
        "lawpack-goldens: {FIXTURE_ROOT} and {CAUSAL_CELL_FIXTURE_ROOT} {}",
        match mode {
            LawpackGoldenMode::Check => "checked",
            LawpackGoldenMode::Write => "written",
        }
    );
    Ok(())
}

fn causal_cell_golden_artifacts() -> Result<Vec<(&'static str, Vec<u8>)>, String> {
    let exports_value = causal_cell_exports();
    let exports_bytes = encode_canonical_cbor(&exports_value)
        .map_err(|error| format!("encode causal.cell exports: {error}"))?;
    let exports_digest = digest_value(CAUSAL_CELL_EXPORTS_COORDINATE, &exports_value)?;

    let configuration_value = causal_cell_target_configuration();
    let configuration_bytes = encode_canonical_cbor(&configuration_value)
        .map_err(|error| format!("encode causal.cell target configuration: {error}"))?;
    let configuration_digest =
        digest_value(CAUSAL_CELL_CONFIGURATION_COORDINATE, &configuration_value)?;

    let adapter_value = causal_cell_adapter(configuration_digest);
    let adapter_bytes = encode_canonical_cbor(&adapter_value)
        .map_err(|error| format!("encode causal.cell adapter: {error}"))?;
    let adapter_digest = digest_value(CAUSAL_CELL_ADAPTER_COORDINATE, &adapter_value)?;

    let manifest_value = causal_cell_manifest(exports_digest, adapter_digest);
    let manifest_bytes = encode_canonical_cbor(&manifest_value)
        .map_err(|error| format!("encode causal.cell manifest: {error}"))?;
    let bundle = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .map_err(|failures| format!("validate causal.cell lawpack: {failures:?}"))?;
    let adapter = decode_lawpack_adapter(&bundle, "echo.dpo@1", &adapter_bytes)
        .map_err(|failures| format!("validate causal.cell adapter: {failures:?}"))?;

    let source = causal_cell_application_source(&bundle.manifest_digest_review_string());
    let module = parse_module(&source)
        .map_err(|error| format!("parse causal.cell application witness: {error:?}"))?;
    let preparation = prepare_lawpack_compilation(&module, &bundle, &adapter)
        .map_err(|failures| format!("prepare causal.cell application witness: {failures:?}"))?;
    let core = compile_to_core(&module, preparation.compiler_context())
        .map_err(|error| format!("compile causal.cell application witness: {error:?}"))?;
    let target_ir = lower_to_target_ir(&core, preparation.target_ir_facts());
    if target_ir.status != TargetLoweringStatus::Lowered || target_ir.artifact.is_none() {
        return Err(format!(
            "lower causal.cell application witness: expected artifact, got {:?}",
            target_ir.failures
        ));
    }

    Ok(vec![
        (CAUSAL_CELL_MANIFEST_CBOR, manifest_bytes),
        (
            CAUSAL_CELL_MANIFEST_DIGEST,
            format!("{}\n", bundle.manifest_digest_review_string()).into_bytes(),
        ),
        (CAUSAL_CELL_EXPORTS_CBOR, exports_bytes),
        (
            CAUSAL_CELL_EXPORTS_DIGEST,
            format!("{}\n", bundle.manifest().exports.digest_review_string()).into_bytes(),
        ),
        (CAUSAL_CELL_ADAPTER_CBOR, adapter_bytes),
        (
            CAUSAL_CELL_ADAPTER_DIGEST,
            format!("{}\n", sha256_review_string(&adapter_digest)).into_bytes(),
        ),
        (CAUSAL_CELL_CONFIGURATION_CBOR, configuration_bytes),
        (
            CAUSAL_CELL_CONFIGURATION_DIGEST,
            format!("{}\n", sha256_review_string(&configuration_digest)).into_bytes(),
        ),
    ])
}

fn causal_cell_manifest(exports_digest: [u8; 32], adapter_digest: [u8; 32]) -> CanonicalValue {
    map([
        ("apiVersion", text("edict.lawpack/v1")),
        ("id", text("causal.cell")),
        ("version", text("1")),
        (
            "acceptedCoreAbi",
            CanonicalValue::Array(vec![text("edict.core/v1")]),
        ),
        ("dependencies", CanonicalValue::Array(Vec::new())),
        (
            "exports",
            resource_ref(CAUSAL_CELL_EXPORTS_COORDINATE, exports_digest),
        ),
        (
            "targetAdapters",
            CanonicalValue::Array(vec![map([
                (
                    "acceptedTargetProfile",
                    resource_ref("echo.dpo@1", ECHO_TARGET_PROFILE_DIGEST),
                ),
                (
                    "acceptedTargetIr",
                    resource_ref("echo.span-ir/v1", ECHO_TARGET_IR_DIGEST),
                ),
                (
                    "adapter",
                    resource_ref(CAUSAL_CELL_ADAPTER_COORDINATE, adapter_digest),
                ),
            ])]),
        ),
        (
            "verifier",
            map([
                ("class", text("declarative")),
                (
                    "ruleset",
                    resource_ref("causal.cell.verifier-rules/v1", [0x74; 32]),
                ),
            ]),
        ),
        (
            "compatibility",
            resource_ref("causal.cell.compatibility/v1", [0x75; 32]),
        ),
        (
            "conformanceFixtureCorpus",
            resource_ref("causal.cell.fixtures/v1", [0x76; 32]),
        ),
    ])
}

fn causal_cell_adapter(configuration_digest: [u8; 32]) -> CanonicalValue {
    map([
        ("apiVersion", text("edict.lawpack-adapter/v1")),
        ("class", text("declarative")),
        (
            "operationProfiles",
            map([(
                "causal.cell@1.createIfAbsent",
                map([
                    ("core", text("continuum.profile.create/v1")),
                    (
                        "semanticEffects",
                        CanonicalValue::Array(vec![text("causal.cell@1.createIfAbsent")]),
                    ),
                ]),
            )]),
        ),
        (
            "effectImplementations",
            map([(
                "causal.cell@1.createIfAbsent",
                map([
                    (
                        "targetIntrinsic",
                        text("echo.dpo@1.anchored-node-attachment-create-if-absent"),
                    ),
                    (
                        "targetConfiguration",
                        resource_ref(CAUSAL_CELL_CONFIGURATION_COORDINATE, configuration_digest),
                    ),
                    ("writeClass", text("create")),
                    (
                        "footprintObligation",
                        text("causal.cell@1.cellKeyFootprint"),
                    ),
                    ("costObligation", text("causal.cell@1.smallCreateBudget")),
                    (
                        "failureMappings",
                        map([(
                            "alreadyExists",
                            text("echo.executable-operation/precondition-mismatch/v1"),
                        )]),
                    ),
                ]),
            )]),
        ),
        (
            "budgets",
            map([(
                "causal.cell@1.smallCreateBudget",
                map([
                    ("maxSteps", CanonicalValue::Integer(16)),
                    ("maxAllocatedBytes", CanonicalValue::Integer(2_048)),
                    ("maxOutputBytes", CanonicalValue::Integer(512)),
                ]),
            )]),
        ),
    ])
}

fn causal_cell_target_configuration() -> CanonicalValue {
    map([
        (
            "apiVersion",
            text("echo.operation-lowering-configuration/v1"),
        ),
        (
            "programKind",
            text("anchored-node-attachment-create-if-absent/v1"),
        ),
        ("requiredNodeTypeProfile", text("causal.cell.node/value/v1")),
        (
            "requiredAttachmentTypeProfile",
            text("causal.cell.attachment/value/v1"),
        ),
        ("maxReplacementBytes", CanonicalValue::Integer(256)),
        (
            "authorityProfile",
            text("causal.cell.authority.application/v1"),
        ),
        (
            "budgetCeiling",
            map([
                ("steps", CanonicalValue::Integer(16)),
                ("readBytes", CanonicalValue::Integer(64)),
                ("writeBytes", CanonicalValue::Integer(320)),
            ]),
        ),
        (
            "invocationBinding",
            map([
                ("nodeKeyField", text("key")),
                ("replacementField", text("value")),
                ("nodeIdDerivation", text("sha256-utf8/v1")),
                ("warpIdSource", text("action-lane/v1")),
            ]),
        ),
    ])
}

fn causal_cell_exports() -> CanonicalValue {
    map([
        ("types", CanonicalValue::Array(Vec::new())),
        ("constants", CanonicalValue::Array(Vec::new())),
        ("pureFunctions", CanonicalValue::Array(Vec::new())),
        (
            "effects",
            CanonicalValue::Array(vec![map([
                ("coordinate", text("causal.cell@1.createIfAbsent")),
                ("typeParameters", CanonicalValue::Array(Vec::new())),
                ("inputType", text("causal.cell@1.CreateInput")),
                ("outputType", text("causal.cell@1.CreateReceipt")),
                ("executionClass", text("runtime")),
                ("effectKindHint", text("create")),
                (
                    "footprintObligation",
                    text("causal.cell@1.cellKeyFootprint"),
                ),
                ("costObligation", text("causal.cell@1.smallCreateBudget")),
                (
                    "effectFailures",
                    map([(
                        "alreadyExists",
                        map([
                            ("authorityClass", text("domainMappable")),
                            ("payloadType", text("causal.cell@1.ExistingValue")),
                        ]),
                    )]),
                ),
                ("guardSupport", CanonicalValue::Bool(true)),
            ])]),
        ),
        (
            "obstructions",
            CanonicalValue::Array(vec![map([
                ("coordinate", text("causal.cell@1.AlreadyExists")),
                ("authorityClass", text("domainMappable")),
                ("payloadSchema", text("causal.cell@1.ExistingValue")),
            ])]),
        ),
        (
            "operationProfiles",
            map([(
                "causal.cell@1.createIfAbsent",
                map([
                    (
                        "opticTemplate",
                        map([
                            ("opticKind", text("affectReintegration")),
                            ("boundaryKind", text("affect")),
                            ("supportPolicy", text("causal.cell@1.directSupport")),
                            ("lossDisposition", text("causal.cell@1.lossless")),
                            (
                                "apertureRequirement",
                                map([
                                    ("kind", text("abstractFootprintObligation")),
                                    ("ref", text("causal.cell@1.cellKeyFootprint")),
                                ]),
                            ),
                        ]),
                    ),
                    (
                        "effectPredicate",
                        text("causal.cell@1.createIfAbsentEffect"),
                    ),
                ]),
            )]),
        ),
    ])
}

fn causal_cell_application_source(manifest_digest: &str) -> String {
    format!(
        r#"package examples.hello_echo@1;

use lawpack causal.cell@1 digest "{manifest_digest}" as cell;

type CreateGreetingInput = {{
  basis: String<max=128>,
  key: String<max=64>,
  value: String<max=256>,
}};

type CellCreateReceipt = {{
  key: String<max=64>,
}};

type GreetingCreated = {{
  key: String<max=64>,
  message: String<max=256>,
}};

intent createGreeting(input: CreateGreetingInput) returns GreetingCreated
  profile cell.createIfAbsent
  basis input.basis
  budget <= cell.smallCreateBudget
{{
  let receipt: CellCreateReceipt = cell.createIfAbsent(input)
    else {{ alreadyExists(existing) => cell.AlreadyExists }};
  return {{
    key: receipt.key,
    message: input.value,
  }};
}}
"#
    )
}

fn hello_echo_golden_artifacts(root: &Path) -> Result<Vec<(&'static str, Vec<u8>)>, String> {
    let exports_value = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports_value)
        .map_err(|error| format!("encode Hello Echo exports: {error}"))?;
    let exports_digest = digest_value(EXPORTS_COORDINATE, &exports_value)?;
    let target_configuration_value = hello_echo_target_configuration();
    let target_configuration_bytes = encode_canonical_cbor(&target_configuration_value)
        .map_err(|error| format!("encode Hello Echo target configuration: {error}"))?;
    let target_configuration_digest =
        digest_value(TARGET_CONFIGURATION_COORDINATE, &target_configuration_value)?;
    let adapter_value = hello_echo_adapter(target_configuration_digest);
    let adapter_bytes = encode_canonical_cbor(&adapter_value)
        .map_err(|error| format!("encode Hello Echo adapter: {error}"))?;
    let adapter_digest = digest_value(ADAPTER_COORDINATE, &adapter_value)?;
    let manifest_value = hello_echo_manifest(exports_digest, adapter_digest);
    let manifest_bytes = encode_canonical_cbor(&manifest_value)
        .map_err(|error| format!("encode Hello Echo manifest: {error}"))?;
    let bundle = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .map_err(|failures| format!("validate Hello Echo lawpack: {failures:?}"))?;
    let adapter = decode_lawpack_adapter(&bundle, "echo.dpo@1", &adapter_bytes)
        .map_err(|failures| format!("validate Hello Echo adapter: {failures:?}"))?;
    let source = fs::read_to_string(root.join(CREATE_GREETING_SOURCE))
        .map_err(|error| format!("read {CREATE_GREETING_SOURCE}: {error}"))?;
    let module = parse_module(&source)
        .map_err(|error| format!("parse {CREATE_GREETING_SOURCE}: {error:?}"))?;
    let preparation = prepare_lawpack_compilation(&module, &bundle, &adapter)
        .map_err(|failures| format!("prepare Hello Echo compilation: {failures:?}"))?;
    let core = compile_to_core(&module, preparation.compiler_context())
        .map_err(|error| format!("compile Hello Echo Core: {error:?}"))?;
    let core_bytes =
        encode_core_module(&core).map_err(|error| format!("encode Hello Echo Core: {error}"))?;
    let core_digest = format!(
        "{}\n",
        digest_core_module(&core)
            .map_err(|error| format!("digest Hello Echo Core: {error}"))?
            .to_review_string()
    );
    let target_ir_report = lower_to_target_ir(&core, preparation.target_ir_facts());
    if target_ir_report.status != TargetLoweringStatus::Lowered {
        return Err(format!(
            "lower Hello Echo Target IR: expected lowered status, got {:?}",
            target_ir_report.status
        ));
    }
    let result_projection = target_ir_report
        .result_projections
        .get("createGreeting")
        .cloned()
        .ok_or_else(|| {
            "lower Hello Echo Target IR: lowered report omitted result projection".to_owned()
        })?;
    let target_ir = target_ir_report
        .artifact
        .ok_or_else(|| "lower Hello Echo Target IR: lowered report omitted artifact".to_owned())?;
    let target_ir_bytes = encode_target_ir_artifact(&target_ir)
        .map_err(|error| format!("encode Hello Echo Target IR: {error}"))?;
    let target_ir_digest = format!(
        "{}\n",
        digest_target_ir_artifact(&target_ir)
            .map_err(|error| format!("digest Hello Echo Target IR: {error}"))?
            .to_review_string()
    );
    let result_projection_digest = format!("{}\n", result_projection.digest.to_review_string());
    let manifest_digest = format!("{}\n", bundle.manifest_digest_review_string());
    let exports_digest = format!("{}\n", bundle.manifest().exports.digest_review_string());
    let adapter_digest = format!("{}\n", sha256_review_string(&adapter_digest));
    let target_configuration_digest =
        format!("{}\n", sha256_review_string(&target_configuration_digest));

    Ok(vec![
        (MANIFEST_CBOR, manifest_bytes),
        (MANIFEST_DIGEST, manifest_digest.into_bytes()),
        (EXPORTS_CBOR, exports_bytes),
        (EXPORTS_DIGEST, exports_digest.into_bytes()),
        (ADAPTER_CBOR, adapter_bytes),
        (ADAPTER_DIGEST, adapter_digest.into_bytes()),
        (TARGET_CONFIGURATION_CBOR, target_configuration_bytes),
        (
            TARGET_CONFIGURATION_DIGEST,
            target_configuration_digest.into_bytes(),
        ),
        (CREATE_GREETING_CORE_CBOR, core_bytes),
        (CREATE_GREETING_CORE_DIGEST, core_digest.into_bytes()),
        (CREATE_GREETING_TARGET_IR_CBOR, target_ir_bytes),
        (
            CREATE_GREETING_TARGET_IR_DIGEST,
            target_ir_digest.into_bytes(),
        ),
        (
            CREATE_GREETING_RESULT_PROJECTION_CBOR,
            result_projection.canonical_bytes,
        ),
        (
            CREATE_GREETING_RESULT_PROJECTION_DIGEST,
            result_projection_digest.into_bytes(),
        ),
    ])
}

fn hello_echo_manifest(exports_digest: [u8; 32], adapter_digest: [u8; 32]) -> CanonicalValue {
    map([
        ("apiVersion", text("edict.lawpack/v1")),
        ("id", text("hello.echo")),
        ("version", text("1")),
        (
            "acceptedCoreAbi",
            CanonicalValue::Array(vec![text("edict.core/v1")]),
        ),
        ("dependencies", CanonicalValue::Array(Vec::new())),
        ("exports", resource_ref(EXPORTS_COORDINATE, exports_digest)),
        (
            "targetAdapters",
            CanonicalValue::Array(vec![map([
                (
                    "acceptedTargetProfile",
                    resource_ref("echo.dpo@1", ECHO_TARGET_PROFILE_DIGEST),
                ),
                (
                    "acceptedTargetIr",
                    resource_ref("echo.span-ir/v1", ECHO_TARGET_IR_DIGEST),
                ),
                ("adapter", resource_ref(ADAPTER_COORDINATE, adapter_digest)),
            ])]),
        ),
        (
            "verifier",
            map([
                ("class", text("declarative")),
                (
                    "ruleset",
                    resource_ref("hello.echo.verifier-rules/v1", [0x44; 32]),
                ),
            ]),
        ),
        (
            "compatibility",
            resource_ref("hello.echo.compatibility/v1", [0x55; 32]),
        ),
        (
            "conformanceFixtureCorpus",
            resource_ref("hello.echo.fixtures/v1", [0x66; 32]),
        ),
    ])
}

fn hello_echo_adapter(target_configuration_digest: [u8; 32]) -> CanonicalValue {
    map([
        ("apiVersion", text("edict.lawpack-adapter/v1")),
        ("class", text("declarative")),
        (
            "operationProfiles",
            map([(
                "hello.echo@1.createGreeting",
                map([
                    ("core", text("continuum.profile.create/v1")),
                    (
                        "semanticEffects",
                        CanonicalValue::Array(vec![text("hello.echo@1.createGreeting")]),
                    ),
                ]),
            )]),
        ),
        (
            "effectImplementations",
            map([(
                "hello.echo@1.createGreeting",
                map([
                    (
                        "targetIntrinsic",
                        text("echo.dpo@1.anchored-node-attachment-create-if-absent"),
                    ),
                    (
                        "targetConfiguration",
                        resource_ref(TARGET_CONFIGURATION_COORDINATE, target_configuration_digest),
                    ),
                    ("writeClass", text("create")),
                    (
                        "footprintObligation",
                        text("hello.echo@1.greetingKeyFootprint"),
                    ),
                    ("costObligation", text("hello.echo@1.smallCreateBudget")),
                    (
                        "failureMappings",
                        map([(
                            "alreadyExists",
                            text("echo.executable-operation/precondition-mismatch/v1"),
                        )]),
                    ),
                ]),
            )]),
        ),
        (
            "budgets",
            map([(
                "hello.echo@1.smallCreateBudget",
                map([
                    ("maxSteps", CanonicalValue::Integer(16)),
                    ("maxAllocatedBytes", CanonicalValue::Integer(2_048)),
                    ("maxOutputBytes", CanonicalValue::Integer(512)),
                ]),
            )]),
        ),
    ])
}

fn hello_echo_target_configuration() -> CanonicalValue {
    map([
        (
            "apiVersion",
            text("echo.operation-lowering-configuration/v1"),
        ),
        (
            "programKind",
            text("anchored-node-attachment-create-if-absent/v1"),
        ),
        (
            "requiredNodeTypeProfile",
            text("hello.echo.node.greeting/v1"),
        ),
        (
            "requiredAttachmentTypeProfile",
            text("hello.echo.attachment.greeting-message/v1"),
        ),
        ("maxReplacementBytes", CanonicalValue::Integer(256)),
        (
            "authorityProfile",
            text("hello.echo.authority.local-demo/v1"),
        ),
        (
            "budgetCeiling",
            map([
                ("steps", CanonicalValue::Integer(16)),
                ("readBytes", CanonicalValue::Integer(64)),
                ("writeBytes", CanonicalValue::Integer(320)),
            ]),
        ),
        (
            "invocationBinding",
            map([
                ("nodeKeyField", text("key")),
                ("replacementField", text("message")),
                ("nodeIdDerivation", text("sha256-utf8/v1")),
                ("warpIdSource", text("action-lane/v1")),
            ]),
        ),
    ])
}

fn hello_echo_exports() -> CanonicalValue {
    map([
        ("types", CanonicalValue::Array(Vec::new())),
        ("constants", CanonicalValue::Array(Vec::new())),
        ("pureFunctions", CanonicalValue::Array(Vec::new())),
        (
            "effects",
            CanonicalValue::Array(vec![map([
                ("coordinate", text("hello.echo@1.createGreeting")),
                ("typeParameters", CanonicalValue::Array(Vec::new())),
                ("inputType", text("hello.echo@1.CreateGreetingInput")),
                ("outputType", text("hello.echo@1.GreetingReceipt")),
                ("executionClass", text("runtime")),
                ("effectKindHint", text("create")),
                (
                    "footprintObligation",
                    text("hello.echo@1.greetingKeyFootprint"),
                ),
                ("costObligation", text("hello.echo@1.smallCreateBudget")),
                (
                    "effectFailures",
                    map([(
                        "alreadyExists",
                        map([
                            ("authorityClass", text("domainMappable")),
                            ("payloadType", text("hello.echo@1.ExistingGreeting")),
                        ]),
                    )]),
                ),
                ("guardSupport", CanonicalValue::Bool(true)),
            ])]),
        ),
        (
            "obstructions",
            CanonicalValue::Array(vec![map([
                ("coordinate", text("hello.echo@1.AlreadyExists")),
                ("authorityClass", text("domainMappable")),
                ("payloadSchema", text("hello.echo@1.ExistingGreeting")),
            ])]),
        ),
        (
            "operationProfiles",
            map([(
                "hello.echo@1.createGreeting",
                map([
                    (
                        "opticTemplate",
                        map([
                            ("opticKind", text("affectReintegration")),
                            ("boundaryKind", text("affect")),
                            ("supportPolicy", text("hello.echo@1.directSupport")),
                            ("lossDisposition", text("hello.echo@1.lossless")),
                            (
                                "apertureRequirement",
                                map([
                                    ("kind", text("abstractFootprintObligation")),
                                    ("ref", text("hello.echo@1.greetingKeyFootprint")),
                                ]),
                            ),
                        ]),
                    ),
                    ("effectPredicate", text("hello.echo@1.createGreetingEffect")),
                ]),
            )]),
        ),
    ])
}

fn digest_value(domain: &str, value: &CanonicalValue) -> Result<[u8; 32], String> {
    let framed = CanonicalValue::Array(vec![text(DIGEST_FRAME), text(domain), value.clone()]);
    let bytes = encode_canonical_cbor(&framed)
        .map_err(|error| format!("encode lawpack digest frame: {error}"))?;
    Ok(Sha256::digest(bytes).into())
}

fn sha256_review_string(digest: &[u8; 32]) -> String {
    let mut review = String::with_capacity(71);
    review.push_str("sha256:");
    for byte in digest {
        write!(&mut review, "{byte:02x}").expect("writing to a String cannot fail");
    }
    review
}

fn resource_ref(id: &str, digest: [u8; 32]) -> CanonicalValue {
    map([
        ("id", text(id)),
        (
            "digest",
            CanonicalValue::Array(vec![text("sha256"), CanonicalValue::Bytes(digest.to_vec())]),
        ),
    ])
}

fn map<const N: usize>(entries: [(&str, CanonicalValue); N]) -> CanonicalValue {
    CanonicalValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (text(key), value))
            .collect(),
    )
}

fn text(value: &str) -> CanonicalValue {
    CanonicalValue::Text(value.to_owned())
}
