use std::path::Path;

use edict_syntax::{decode_lawpack_bundle, encode_canonical_cbor, CanonicalValue};
use sha2::{Digest, Sha256};

use crate::goldens::{check_golden_file_with_command, write_golden_file};

const DIGEST_FRAME: &str = "edict.digest/v1";
const EXPORTS_COORDINATE: &str = "hello.echo.exports/v1";
const FIXTURE_ROOT: &str = "fixtures/lawpack/hello-echo";
const MANIFEST_CBOR: &str = "fixtures/lawpack/hello-echo/manifest.cbor";
const MANIFEST_DIGEST: &str = "fixtures/lawpack/hello-echo/manifest.sha256";
const EXPORTS_CBOR: &str = "fixtures/lawpack/hello-echo/exports.cbor";
const EXPORTS_DIGEST: &str = "fixtures/lawpack/hello-echo/exports.sha256";
const WRITE_COMMAND: &str = "cargo xtask lawpack-goldens --write";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LawpackGoldenMode {
    Check,
    Write,
}

pub(crate) fn lawpack_goldens(root: &Path, mode: LawpackGoldenMode) -> Result<(), String> {
    let exports_value = hello_echo_exports();
    let exports_bytes = encode_canonical_cbor(&exports_value)
        .map_err(|error| format!("encode Hello Echo exports: {error}"))?;
    let exports_digest = digest_value(EXPORTS_COORDINATE, &exports_value)?;
    let manifest_value = hello_echo_manifest(exports_digest);
    let manifest_bytes = encode_canonical_cbor(&manifest_value)
        .map_err(|error| format!("encode Hello Echo manifest: {error}"))?;
    let bundle = decode_lawpack_bundle(&manifest_bytes, &exports_bytes)
        .map_err(|failures| format!("validate Hello Echo lawpack: {failures:?}"))?;
    let manifest_digest = format!("{}\n", bundle.manifest_digest_review_string());
    let exports_digest = format!("{}\n", bundle.manifest().exports.digest_review_string());

    match mode {
        LawpackGoldenMode::Check => {
            check_golden_file_with_command(root, MANIFEST_CBOR, &manifest_bytes, WRITE_COMMAND)?;
            check_golden_file_with_command(
                root,
                MANIFEST_DIGEST,
                manifest_digest.as_bytes(),
                WRITE_COMMAND,
            )?;
            check_golden_file_with_command(root, EXPORTS_CBOR, &exports_bytes, WRITE_COMMAND)?;
            check_golden_file_with_command(
                root,
                EXPORTS_DIGEST,
                exports_digest.as_bytes(),
                WRITE_COMMAND,
            )?;
        }
        LawpackGoldenMode::Write => {
            write_golden_file(&root.join(MANIFEST_CBOR), &manifest_bytes)?;
            write_golden_file(&root.join(MANIFEST_DIGEST), manifest_digest.as_bytes())?;
            write_golden_file(&root.join(EXPORTS_CBOR), &exports_bytes)?;
            write_golden_file(&root.join(EXPORTS_DIGEST), exports_digest.as_bytes())?;
        }
    }

    println!(
        "lawpack-goldens: {FIXTURE_ROOT} {}",
        match mode {
            LawpackGoldenMode::Check => "checked",
            LawpackGoldenMode::Write => "written",
        }
    );
    Ok(())
}

fn hello_echo_manifest(exports_digest: [u8; 32]) -> CanonicalValue {
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
                    resource_ref("echo.dpo@1", [0x11; 32]),
                ),
                (
                    "acceptedTargetIr",
                    resource_ref("echo.span-ir/v1", [0x22; 32]),
                ),
                (
                    "adapter",
                    resource_ref("hello.echo.echo-dpo-adapter/v1", [0x33; 32]),
                ),
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
