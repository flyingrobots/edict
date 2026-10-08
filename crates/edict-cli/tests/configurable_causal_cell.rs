#[path = "../examples/configurable_causal_cell/limits.rs"]
mod limits;

use edict_syntax::{encode_canonical_cbor, CanonicalValue};
use limits::{LimitError, Limits};

#[test]
fn configurable_cell_budgets_match_maximum_unicode_encoding() {
    for (key_scalars, value_scalars, replacement_bytes) in [
        (64, 64, 256),
        (64, 1024, 4096),
        (1, 5, 20),
        (6, 6, 24),
        (64, 16384, 65536),
    ] {
        let limits = Limits {
            key_scalars,
            value_scalars,
            replacement_bytes,
        };
        let budget = limits.budgets().expect("consistent finite parameters");
        let value = CanonicalValue::Map(vec![
            (
                CanonicalValue::Text("key".into()),
                CanonicalValue::Text("😀".repeat(usize::try_from(key_scalars).unwrap())),
            ),
            (
                CanonicalValue::Text("value".into()),
                CanonicalValue::Text("😀".repeat(usize::try_from(value_scalars).unwrap())),
            ),
        ]);
        let encoded = encode_canonical_cbor(&value).unwrap();
        assert_eq!(budget.output, u64::try_from(encoded.len()).unwrap());
        assert_eq!(budget.write, replacement_bytes + 64);
        assert!(budget.allocated >= budget.output + replacement_bytes);
        assert_eq!(limits.budgets(), Ok(budget), "calculation is deterministic");
    }
}

#[test]
fn configurable_cell_limits_reject_inconsistent_and_overflowing_parameters() {
    for limits in [
        Limits {
            key_scalars: 0,
            value_scalars: 64,
            replacement_bytes: 256,
        },
        Limits {
            key_scalars: 64,
            value_scalars: 0,
            replacement_bytes: 256,
        },
        Limits {
            key_scalars: 64,
            value_scalars: 64,
            replacement_bytes: 0,
        },
    ] {
        assert_eq!(limits.budgets(), Err(LimitError::Zero));
    }
    assert_eq!(
        Limits {
            key_scalars: 64,
            value_scalars: 256,
            replacement_bytes: 256
        }
        .budgets(),
        Err(LimitError::ScalarBytesExceedCap)
    );
    for limits in [
        Limits {
            key_scalars: u64::MAX,
            value_scalars: 1,
            replacement_bytes: 4,
        },
        Limits {
            key_scalars: 1,
            value_scalars: u64::MAX,
            replacement_bytes: u64::MAX,
        },
        Limits {
            key_scalars: 1,
            value_scalars: 1,
            replacement_bytes: u64::MAX,
        },
    ] {
        assert_eq!(limits.budgets(), Err(LimitError::Overflow));
    }
}

#[path = "../examples/configurable_causal_cell/document.rs"]
mod document;

#[test]
fn configurable_cell_documents_author_repeatable_valid_closures() {
    use edict_syntax::{
        author_lawpack, decode_lawpack_adapter, decode_lawpack_bundle, LawpackArtifactKind,
        LawpackAuthoringDefinition,
    };
    let mut identities = Vec::new();
    for (value_scalars, replacement_bytes) in [(64, 256), (1024, 4096)] {
        let limits = Limits {
            key_scalars: 64,
            value_scalars,
            replacement_bytes,
        };
        let doc = document::document(limits).expect("generate authoring JSON");
        assert_eq!(doc, document::document(limits).unwrap());
        let definition: LawpackAuthoringDefinition =
            serde_json::from_value(doc["lawpack"].clone()).expect("public typed authoring schema");
        let artifacts = author_lawpack(&definition, &[]).expect("public authoring succeeds");
        assert_eq!(artifacts, author_lawpack(&definition, &[]).unwrap());
        let manifest = artifacts.artifact(LawpackArtifactKind::Manifest).unwrap();
        let exports = artifacts.artifact(LawpackArtifactKind::Exports).unwrap();
        let bundle = decode_lawpack_bundle(manifest.bytes(), exports.bytes()).unwrap();
        let adapter = artifacts.artifact(LawpackArtifactKind::Adapter).unwrap();
        decode_lawpack_adapter(&bundle, "echo.dpo@1", adapter.bytes())
            .expect("exact authored adapter");
        assert_eq!(bundle.exports().types[0].definition, format!("Record<basis:String<max=128,canonical=raw-utf8>,key:String<max=64,canonical=raw-utf8>,value:String<max={value_scalars},canonical=raw-utf8>>"));
        assert_eq!(
            doc["lawpack"]["localResources"][0]["value"]["maxReplacementBytes"],
            replacement_bytes
        );
        assert_eq!(
            doc["lawpack"]["targetAdapters"][0]["budgets"]["example.cell@1.createBudget"]
                ["maxOutputBytes"],
            limits.budgets().unwrap().output
        );
        identities.push(manifest.digest().to_owned());
    }
    assert_ne!(
        identities[0], identities[1],
        "changed semantic bounds change identity"
    );
}

#[test]
fn configurable_cell_public_authoring_matches_in_memory_artifacts() {
    use edict_syntax::{author_lawpack, LawpackAuthoringDefinition};
    use std::io::Write;
    use std::process::{Command, Stdio};
    for (value_scalars, replacement_bytes) in [(64, 256), (1024, 4096)] {
        let doc = document::document(Limits {
            key_scalars: 64,
            value_scalars,
            replacement_bytes,
        })
        .unwrap();
        let definition: LawpackAuthoringDefinition =
            serde_json::from_value(doc["lawpack"].clone()).unwrap();
        let expected = author_lawpack(&definition, &[]).unwrap();
        for copy in 0..2 {
            let root = std::env::temp_dir().join(format!(
                "edict-configurable-{}-{value_scalars}-{copy}",
                std::process::id()
            ));
            std::fs::create_dir(&root).expect("create owned application tree");
            std::fs::write(
                root.join("edict.lawpack.json"),
                serde_json::to_vec(&doc).unwrap(),
            )
            .unwrap();
            let mut child = Command::new(env!("CARGO_BIN_EXE_edict"))
                .current_dir(&root)
                .env_remove(edict_cli::MAX_STDIN_BYTES_ENV)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(b"{\"schema\":\"edict.compiler.settings/v1\",\"type\":\"compilerSettings\",\"operation\":\"build\",\"lawpack\":\"edict.lawpack.json\"}\n").unwrap();
            let output = child.wait_with_output().unwrap();
            assert_eq!(output.status.code(), Some(0), "public build: {output:?}");
            assert!(output.stderr.is_empty(), "{output:?}");
            for artifact in expected.artifacts() {
                assert_eq!(
                    std::fs::read(root.join("vendor/cell").join(artifact.path())).unwrap(),
                    artifact.bytes(),
                    "exact public artifact {}",
                    artifact.path()
                );
            }
            std::fs::remove_dir_all(root).unwrap();
        }
    }
}
