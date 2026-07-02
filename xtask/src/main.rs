#![allow(clippy::print_stderr)]
#![allow(clippy::print_stdout)]

mod contract_check;
mod goldens;
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
use release_prep::release_prep;
use util::{diff_check_base, repo_root, run_cmd};

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
        Some("verify") => verify(&repo_root()?),
        Some(cmd) => Err(format!("unknown xtask command `{cmd}`")),
        None => Err(
            "usage: cargo xtask <verify|contract-check|core-goldens|bundle-goldens|cli-goldens|target-ir-goldens|release-prep>"
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
    cli_goldens(root, CliGoldenMode::Check)?;
    contract_check(root)?;
    let base = diff_check_base(root)?;
    run_cmd(root, "git", ["diff", "--check", &format!("{base}...HEAD")])?;
    Ok(())
}
