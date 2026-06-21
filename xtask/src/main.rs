#![allow(clippy::print_stderr)]
#![allow(clippy::print_stdout)]

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

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
        Some("verify") => verify(&repo_root()?),
        Some(cmd) => Err(format!("unknown xtask command `{cmd}`")),
        None => Err("usage: cargo xtask <verify|contract-check>".into()),
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
            "planned" | "gap" => {}
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
            return Err(format!("{id} has no planned or implemented case"));
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
        ] {
            assert!(
                policy.contains(needle),
                "release policy missing structured field: {needle}"
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
