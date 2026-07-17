use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use wasm_encoder::{CustomSection, Encode, Section};
use wit_component::{embed_component_metadata, ComponentEncoder, StringEncoding};
use wit_parser::Resolve;

use crate::util::run_cmd;

const CUSTOM_SECTION: &str = "edict:target-provider-contract";
const FIXTURE_ROOT: &str = "fixtures/providers/components";

const SOURCE_INPUTS: &[&str] = &[
    "Cargo.lock",
    "docs/abi/edict-target-provider.wit",
    "fixtures/providers/components/instantiation-failure-lowerer.wat",
    "fixtures/providers/components/instantiation-fuel-lowerer.wat",
    "fixtures/providers/components/malformed-lowerer.wat",
    "fixtures/providers/components/guests/Cargo.lock",
    "fixtures/providers/components/guests/Cargo.toml",
    "fixtures/providers/components/guests/lowerer/Cargo.toml",
    "fixtures/providers/components/guests/lowerer/src/lib.rs",
    "fixtures/providers/components/guests/verifier/Cargo.toml",
    "fixtures/providers/components/guests/verifier/src/lib.rs",
    "fixtures/target-ir/canonical/echo-effectful.target-ir.cbor",
    "xtask/Cargo.toml",
    "xtask/src/provider_components.rs",
];

enum FixtureSource {
    Guest(&'static str),
    Wat {
        path: &'static str,
        world: &'static str,
    },
}

struct FixtureSpec {
    name: &'static str,
    source: FixtureSource,
    contract: &'static str,
}

const FIXTURES: &[FixtureSpec] = &[
    FixtureSpec {
        name: "lowerer",
        source: FixtureSource::Guest("provider_fixture_lowerer.wasm"),
        contract: "edict:target-provider/lowerer@1.0.0",
    },
    FixtureSpec {
        name: "verifier",
        source: FixtureSource::Guest("provider_fixture_verifier.wasm"),
        contract: "edict:target-provider/verifier@1.0.0",
    },
    FixtureSpec {
        name: "malformed-lowerer",
        source: FixtureSource::Wat {
            path: "fixtures/providers/components/malformed-lowerer.wat",
            world: "lowerer",
        },
        contract: "edict:target-provider/lowerer@1.0.0",
    },
    FixtureSpec {
        name: "instantiation-failure-lowerer",
        source: FixtureSource::Wat {
            path: "fixtures/providers/components/instantiation-failure-lowerer.wat",
            world: "lowerer",
        },
        contract: "edict:target-provider/lowerer@1.0.0",
    },
    FixtureSpec {
        name: "instantiation-fuel-lowerer",
        source: FixtureSource::Wat {
            path: "fixtures/providers/components/instantiation-fuel-lowerer.wat",
            world: "lowerer",
        },
        contract: "edict:target-provider/lowerer@1.0.0",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderComponentFixtureMode {
    Check,
    Write,
}

pub(crate) fn provider_component_fixtures(
    root: &Path,
    mode: ProviderComponentFixtureMode,
) -> Result<(), String> {
    match mode {
        ProviderComponentFixtureMode::Write => write_fixtures(root),
        ProviderComponentFixtureMode::Check => check_fixtures(root),
    }
}

fn write_fixtures(root: &Path) -> Result<(), String> {
    run_cmd(
        root,
        "rustup",
        [
            "run",
            "1.94.0",
            "cargo",
            "build",
            "--manifest-path",
            "fixtures/providers/components/guests/Cargo.toml",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--locked",
            "--offline",
        ],
    )?;
    let output_root = root.join(FIXTURE_ROOT);
    let mut component_digests = serde_json::Map::new();
    for fixture in FIXTURES {
        let core = fixture_core(root, fixture)?;
        let mut component = ComponentEncoder::default()
            .validate(true)
            .merge_imports_based_on_semver(false)
            .module(&core)
            .map_err(|error| format!("componentize {} fixture: {error}", fixture.name))?
            .encode()
            .map_err(|error| format!("encode {} fixture: {error:#}", fixture.name))?;
        append_contract(&mut component, fixture.contract);
        let output = output_root.join(format!("{}.component.wasm", fixture.name));
        fs::write(&output, &component)
            .map_err(|error| format!("write {}: {error}", output.display()))?;
        component_digests.insert(fixture.name.to_owned(), Value::String(digest(&component)));
    }
    let inventory = json!({
        "apiVersion": "edict.provider-component-fixtures/v1",
        "sourceDigest": source_digest(root)?,
        "components": component_digests,
    });
    let inventory_path = output_root.join("inventory.json");
    let rendered = serde_json::to_string_pretty(&inventory)
        .map_err(|error| format!("render fixture inventory: {error}"))?;
    fs::write(&inventory_path, format!("{rendered}\n"))
        .map_err(|error| format!("write {}: {error}", inventory_path.display()))?;
    Ok(())
}

fn fixture_core(root: &Path, fixture: &FixtureSpec) -> Result<Vec<u8>, String> {
    match fixture.source {
        FixtureSource::Guest(core_file) => fs::read(guest_target(root).join(core_file))
            .map_err(|error| format!("read {} core fixture: {error}", fixture.name)),
        FixtureSource::Wat { path, world } => {
            let mut module = wat::parse_file(root.join(path))
                .map_err(|error| format!("parse {} core fixture: {error:#}", fixture.name))?;
            let mut resolve = Resolve::default();
            let (package, _) = resolve
                .push_path(root.join("docs/abi/edict-target-provider.wit"))
                .map_err(|error| format!("resolve provider WIT: {error:#}"))?;
            let world = resolve
                .select_world(&[package], Some(world))
                .map_err(|error| format!("select {world} provider world: {error:#}"))?;
            embed_component_metadata(&mut module, &resolve, world, StringEncoding::UTF8)
                .map_err(|error| format!("embed {} component metadata: {error:#}", fixture.name))?;
            Ok(module)
        }
    }
}

fn check_fixtures(root: &Path) -> Result<(), String> {
    let inventory_path = root.join(FIXTURE_ROOT).join("inventory.json");
    let inventory: Value = serde_json::from_slice(
        &fs::read(&inventory_path)
            .map_err(|error| format!("read {}: {error}", inventory_path.display()))?,
    )
    .map_err(|error| format!("parse {}: {error}", inventory_path.display()))?;
    if inventory["apiVersion"] != "edict.provider-component-fixtures/v1" {
        return Err("unsupported provider component fixture inventory".to_owned());
    }
    if inventory["sourceDigest"].as_str() != Some(source_digest(root)?.as_str()) {
        return Err(
            "provider component fixture sources changed; regenerate with `cargo xtask provider-component-fixtures --write`"
                .to_owned(),
        );
    }
    for fixture in FIXTURES {
        let path = root
            .join(FIXTURE_ROOT)
            .join(format!("{}.component.wasm", fixture.name));
        let bytes = fs::read(&path).map_err(|error| format!("read {}: {error}", path.display()))?;
        if inventory["components"][fixture.name].as_str() != Some(digest(&bytes).as_str()) {
            return Err(format!(
                "provider component fixture digest mismatch: {}",
                path.display()
            ));
        }
    }
    println!(
        "provider-component-fixtures: checked {} fixture(s)",
        FIXTURES.len()
    );
    Ok(())
}

fn append_contract(component: &mut Vec<u8>, contract: &str) {
    let section = CustomSection {
        name: Cow::Borrowed(CUSTOM_SECTION),
        data: Cow::Borrowed(contract.as_bytes()),
    };
    component.push(section.id());
    section.encode(component);
}

fn source_digest(root: &Path) -> Result<String, String> {
    let mut hasher = Sha256::new();
    for relative in SOURCE_INPUTS {
        let bytes = fs::read(root.join(relative))
            .map_err(|error| format!("read fixture source {relative}: {error}"))?;
        hasher.update((relative.len() as u64).to_be_bytes());
        hasher.update(relative.as_bytes());
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn guest_target(root: &Path) -> PathBuf {
    root.join(FIXTURE_ROOT)
        .join("guests/target/wasm32-unknown-unknown/release")
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
