#![allow(clippy::print_stderr)]
#![allow(clippy::print_stdout)]

mod contract_check;
mod goldens;
mod provider_components;
mod provider_dependencies;
mod release_prep;
mod util;

#[cfg(test)]
mod tests;

use std::env;
use std::path::Path;
use std::process::ExitCode;

use contract_check::contract_check;
use goldens::{
    bundle_goldens, cli_goldens, core_goldens, target_ir_goldens, BundleGoldenMode, CliGoldenMode,
    CoreGoldenMode, TargetIrGoldenMode,
};
use provider_components::{provider_component_fixtures, ProviderComponentFixtureMode};
use provider_dependencies::provider_runtime_dependencies;
use release_prep::release_prep;
use util::{diff_check_base, repo_root, run_cmd, run_cmd_slice};

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
        Some("cli-goldens") => {
            let mode = match args.next().as_deref() {
                Some("--write") => CliGoldenMode::Write,
                Some("--check") | None => CliGoldenMode::Check,
                Some(flag) => return Err(format!("unknown cli-goldens flag `{flag}`")),
            };
            if let Some(extra) = args.next() {
                return Err(format!("unexpected cli-goldens argument `{extra}`"));
            }
            cli_goldens(&repo_root()?, mode)
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
        Some("release-prep") => {
            let version = args
                .next()
                .ok_or_else(|| "usage: cargo xtask release-prep <version>".to_owned())?;
            if let Some(extra) = args.next() {
                return Err(format!("unexpected release-prep argument `{extra}`"));
            }
            release_prep(&repo_root()?, &version)
        }
        Some("provider-component-fixtures") => {
            let mode = match args.next().as_deref() {
                Some("--write") => ProviderComponentFixtureMode::Write,
                Some("--check") | None => ProviderComponentFixtureMode::Check,
                Some(flag) => {
                    return Err(format!(
                        "unknown provider-component-fixtures flag `{flag}`"
                    ));
                }
            };
            if let Some(extra) = args.next() {
                return Err(format!(
                    "unexpected provider-component-fixtures argument `{extra}`"
                ));
            }
            provider_component_fixtures(&repo_root()?, mode)
        }
        Some("provider-runtime-dependencies") => {
            if let Some(extra) = args.next() {
                return Err(format!(
                    "unexpected provider-runtime-dependencies argument `{extra}`"
                ));
            }
            provider_runtime_dependencies(&repo_root()?)
        }
        Some("verify") => verify(&repo_root()?),
        Some(cmd) => Err(format!("unknown xtask command `{cmd}`")),
        None => Err(
            "usage: cargo xtask <verify|contract-check|core-goldens|bundle-goldens|cli-goldens|target-ir-goldens|provider-component-fixtures|provider-runtime-dependencies|release-prep>"
                .into(),
        ),
    }
}

fn verify(root: &Path) -> Result<(), String> {
    verify_rust_commands_with(root, run_cmd_slice)?;
    core_goldens(root, CoreGoldenMode::Check)?;
    target_ir_goldens(root, TargetIrGoldenMode::Check)?;
    bundle_goldens(root, BundleGoldenMode::Check)?;
    cli_goldens(root, CliGoldenMode::Check)?;
    provider_component_fixtures(root, ProviderComponentFixtureMode::Check)?;
    provider_runtime_dependencies(root)?;
    contract_check(root)?;
    let base = diff_check_base(root)?;
    run_cmd(root, "git", ["diff", "--check", &format!("{base}...HEAD")])?;
    Ok(())
}

fn verify_rust_commands_with(
    root: &Path,
    mut run: impl FnMut(&Path, &str, &[&str]) -> Result<(), String>,
) -> Result<(), String> {
    run(root, "cargo", &["fmt", "--all", "--check"])?;
    run(
        root,
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run(root, "cargo", &["test", "--workspace", "--all-features"])?;
    Ok(())
}
