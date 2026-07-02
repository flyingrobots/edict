use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::util::{dirs, markdown_files, read_to_string, rust_files};

pub(crate) fn contract_check(root: &Path) -> Result<(), String> {
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

pub(crate) fn check_topic(
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

pub(crate) fn check_links(root: &Path, doc: &Path) -> Result<(), String> {
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
