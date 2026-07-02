use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(crate) fn run_cmd<const N: usize>(
    root: &Path,
    program: &str,
    args: [&str; N],
) -> Result<(), String> {
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

pub(crate) fn diff_check_base(root: &Path) -> Result<String, String> {
    choose_diff_check_base(|candidate| git_ref_exists(root, candidate))
}

pub(crate) fn choose_diff_check_base(
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

pub(crate) fn markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    files_with_extension(root, "md")
}

pub(crate) fn rust_files(root: &Path) -> Result<Vec<PathBuf>, String> {
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

pub(crate) fn dirs(root: &Path) -> Result<Vec<PathBuf>, String> {
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

pub(crate) fn read_to_string(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|err| format!("read {}: {err}", path.display()))
}

pub(crate) fn repo_root() -> Result<PathBuf, String> {
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
