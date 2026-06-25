#![allow(clippy::print_stderr)]
#![allow(clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use edict_syntax::{
    compile_to_core, digest_core_module, encode_core_module, parse_module, CompilerContext,
    CoreBudget,
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
        Some("verify") => verify(&repo_root()?),
        Some(cmd) => Err(format!("unknown xtask command `{cmd}`")),
        None => Err("usage: cargo xtask <verify|contract-check|core-goldens>".into()),
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
            check_core_golden_file(root, case.bytes, &bytes)?;
            check_core_golden_file(root, case.digest, digest_text.as_bytes())?;
        }
        CoreGoldenMode::Write => {
            write_core_golden_file(&root.join(case.bytes), &bytes)?;
            write_core_golden_file(&root.join(case.digest), digest_text.as_bytes())?;
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

fn check_core_golden_file(root: &Path, relative: &str, expected: &[u8]) -> Result<(), String> {
    let path = root.join(relative);
    let actual = fs::read(&path).map_err(|err| format!("read {}: {err}", path.display()))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "{} does not match generated Core golden; run `cargo xtask core-goldens --write`",
            path.display()
        ))
    }
}

fn write_core_golden_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
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

    use super::{check_topic, contract_check, repo_root};

    #[test]
    fn contract_graph_is_valid() {
        contract_check(&repo_root().expect("repo root")).expect("contract graph is valid");
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
        assert!(
            workflow.contains("gh api --paginate"),
            "release workflow must paginate milestone lookup"
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
            "workflows: [\"CI\"]",
            "branches: [main]",
            "github.event.workflow_run.conclusion == 'success'",
            "github.event.workflow_run.event == 'push'",
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
    }

    #[test]
    fn auto_release_tag_workflow_scopes_job_permissions() {
        let root = repo_root().expect("repo root");
        let workflow = fs::read_to_string(root.join(".github/workflows/auto-release-tag.yml"))
            .expect("auto release workflow");
        for required in [
            "identify-release-pr:",
            "contents: read",
            "pull-requests: read",
            "create-release-tag:",
            "contents: write",
            "dispatch-release-publication:",
            "actions: write",
        ] {
            assert!(
                workflow.contains(required),
                "auto release workflow missing scoped job permission contract: {required}"
            );
        }
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
            "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY}.git",
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
            "merge_gate",
            "auto_tag_publish",
            "watch_workflow",
            "capture_evidence",
            "manual_fallback_target = \"verified_main_merge_commit\"",
            "cargo xtask verify",
            "gh pr checks",
            "gh release view",
        ] {
            assert!(
                policy.contains(required),
                "release runbook policy missing structured field: {required}"
            );
        }
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
            "status = \"publish_ready\"",
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
