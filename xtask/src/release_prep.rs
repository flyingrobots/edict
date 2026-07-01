use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::util::read_to_string;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReleasePrepVersion {
    tag: String,
    package_version: String,
    policy_key: String,
    boundary_test_name: String,
}

impl ReleasePrepVersion {
    fn parse(input: &str) -> Result<Self, String> {
        let package_version = input
            .strip_prefix('v')
            .ok_or_else(|| "release-prep version must start with `v`".to_owned())?;
        let (numeric, alpha) = package_version
            .split_once("-alpha.")
            .ok_or_else(|| "release-prep version must match vX.Y.Z-alpha.N".to_owned())?;
        let mut parts = numeric.split('.');
        let major = parse_version_part(parts.next(), "major")?;
        let minor = parse_version_part(parts.next(), "minor")?;
        let patch = parse_version_part(parts.next(), "patch")?;
        if parts.next().is_some() {
            return Err("release-prep version must match vX.Y.Z-alpha.N".into());
        }
        let alpha = parse_decimal(alpha, "alpha")?;
        let policy_key = format!("v{major}_{minor}_{patch}_alpha_{alpha}");
        let boundary_key = if patch == 0 && alpha == 1 {
            format!("v{major}_{minor}")
        } else {
            policy_key.clone()
        };
        Ok(Self {
            tag: input.to_owned(),
            package_version: package_version.to_owned(),
            policy_key,
            boundary_test_name: format!("release_policy_tracks_{boundary_key}_boundary"),
        })
    }

    fn policy_section(&self) -> String {
        format!("[release_notes.{}]", self.policy_key)
    }
}

fn parse_version_part(part: Option<&str>, name: &str) -> Result<u16, String> {
    let part = part.ok_or_else(|| format!("release-prep version missing {name} version part"))?;
    parse_decimal(part, name)
}

fn parse_decimal(part: &str, name: &str) -> Result<u16, String> {
    if part.is_empty() || !part.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!("release-prep {name} version part must be numeric"));
    }
    part.parse::<u16>()
        .map_err(|err| format!("parse release-prep {name} version part `{part}`: {err}"))
}

pub(crate) fn release_prep(root: &Path, input: &str) -> Result<(), String> {
    let version = ReleasePrepVersion::parse(input)?;
    let cli_manifest_path = root.join("crates/edict-cli/Cargo.toml");
    let syntax_manifest_path = root.join("crates/edict-syntax/Cargo.toml");
    let lockfile_path = root.join("Cargo.lock");
    let changelog_path = root.join("CHANGELOG.md");
    let policy_path = root.join("docs/topics/release-process/policy.toml");
    let test_plan_path = root.join("docs/topics/release-process/test-plan.md");
    let xtask_tests_path = root.join("xtask/src/tests.rs");
    let release_notes_path = release_notes_path(root, &version);

    if release_notes_path.exists() {
        return Err(format!(
            "release notes already exist: {}",
            release_notes_path.display()
        ));
    }

    let policy = read_to_string(&policy_path)?;
    let target_date = next_release_target_date(&policy)?;
    let cli_manifest = replace_first_version_line(
        &read_to_string(&cli_manifest_path)?,
        &version.package_version,
    )?;
    let syntax_manifest = replace_first_version_line(
        &read_to_string(&syntax_manifest_path)?,
        &version.package_version,
    )?;
    let lockfile =
        replace_lock_package_versions(&read_to_string(&lockfile_path)?, &version.package_version)?;
    let changelog = insert_release_changelog_section(
        &read_to_string(&changelog_path)?,
        &version.tag,
        &target_date,
    )?;
    let policy = append_release_policy_block(&policy, &version, &target_date)?;
    let release_notes = render_release_notes_stub(&version, &target_date);
    let test_plan = insert_release_test_plan_rows(&read_to_string(&test_plan_path)?, &version)?;
    let xtask_tests = insert_release_boundary_test_stub(
        &read_to_string(&xtask_tests_path)?,
        &version,
        &target_date,
    )?;

    write_file(&cli_manifest_path, &cli_manifest)?;
    write_file(&syntax_manifest_path, &syntax_manifest)?;
    write_file(&lockfile_path, &lockfile)?;
    write_file(&changelog_path, &changelog)?;
    write_file(&policy_path, &policy)?;
    write_file(&release_notes_path, &release_notes)?;
    write_file(&test_plan_path, &test_plan)?;
    write_file(&xtask_tests_path, &xtask_tests)?;

    println!(
        "release-prep: scaffolded {} with target date {}",
        version.tag, target_date
    );
    Ok(())
}

fn write_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| format!("create {}: {err}", parent.display()))?;
    }
    fs::write(path, text).map_err(|err| format!("write {}: {err}", path.display()))
}

fn replace_first_version_line(text: &str, package_version: &str) -> Result<String, String> {
    let mut replaced = false;
    let mut lines = Vec::new();
    for line in text.lines() {
        if !replaced && line.trim_start().starts_with("version = ") {
            lines.push(format!("version = \"{package_version}\""));
            replaced = true;
        } else {
            lines.push(line.to_owned());
        }
    }
    if !replaced {
        return Err("package manifest missing version field".into());
    }
    Ok(join_lines_preserving_final_newline(&lines, text))
}

fn replace_lock_package_versions(text: &str, package_version: &str) -> Result<String, String> {
    let mut current_package = None;
    let mut replaced = BTreeSet::new();
    let mut lines = Vec::new();
    for line in text.lines() {
        if line == "[[package]]" {
            current_package = None;
            lines.push(line.to_owned());
            continue;
        }
        if let Some(name) = line.strip_prefix("name = ") {
            current_package = parse_quoted_string(name);
            lines.push(line.to_owned());
            continue;
        }
        if line.starts_with("version = ")
            && matches!(
                current_package.as_deref(),
                Some("edict-cli" | "edict-syntax")
            )
        {
            if let Some(package) = current_package.as_deref() {
                replaced.insert(package.to_owned());
            }
            lines.push(format!("version = \"{package_version}\""));
            continue;
        }
        lines.push(line.to_owned());
    }
    for package in ["edict-cli", "edict-syntax"] {
        if !replaced.contains(package) {
            return Err(format!("Cargo.lock missing package version for {package}"));
        }
    }
    Ok(join_lines_preserving_final_newline(&lines, text))
}

fn parse_quoted_string(text: &str) -> Option<String> {
    let trimmed = text.trim();
    trimmed
        .strip_prefix('"')?
        .strip_suffix('"')
        .map(ToOwned::to_owned)
}

fn join_lines_preserving_final_newline(lines: &[String], original: &str) -> String {
    let mut text = lines.join("\n");
    if original.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn insert_release_changelog_section(
    changelog: &str,
    tag: &str,
    target_date: &str,
) -> Result<String, String> {
    if changelog.contains(&format!("## [{tag}]")) {
        return Err(format!(
            "CHANGELOG.md already contains release section for {tag}"
        ));
    }
    replace_once(
        changelog,
        "## [Unreleased]\n\n",
        &format!("## [Unreleased]\n\n## [{tag}] - {target_date}\n\n"),
        "CHANGELOG.md Unreleased section",
    )
}

fn append_release_policy_block(
    policy: &str,
    version: &ReleasePrepVersion,
    target_date: &str,
) -> Result<String, String> {
    if policy.contains(&version.policy_section()) {
        return Err(format!(
            "release policy already contains {}",
            version.policy_section()
        ));
    }
    let mut output = policy.trim_end().to_owned();
    output.push_str("\n\n");
    output.push_str(&version.policy_section());
    output.push('\n');
    output.push_str("tag = \"");
    output.push_str(&version.tag);
    output.push_str("\"\n");
    output.push_str("target_date = \"");
    output.push_str(target_date);
    output.push_str("\"\n");
    output.push_str(
        "\
status = \"prep\"
release_issue = 0
scope = [
  \"TODO_release_scope\",
]
non_goals = [
  \"TODO_release_non_goal\",
]
",
    );
    Ok(output)
}

fn release_notes_path(root: &Path, version: &ReleasePrepVersion) -> PathBuf {
    root.join("docs")
        .join("releases")
        .join(format!("{}.md", version.tag))
}

fn render_release_notes_stub(version: &ReleasePrepVersion, target_date: &str) -> String {
    format!(
        "\
# {tag}

Target date: {target_date}

## Release Thesis

- TODO: write the release thesis before editing release artifacts.

## Released

- TODO: list what shipped.

## Not Released

- TODO: list explicit non-goals.

## Evidence

- TODO: record local verification, CI, tag, release, and milestone evidence.

## Plan Versus Actual

- TODO: reconcile the plan with what landed.

## Fallout Issues

- TODO: list follow-up issues or state none.

## Next Release Thesis

- TODO: capture the next release thesis.
",
        tag = version.tag,
    )
}

fn insert_release_test_plan_rows(
    test_plan: &str,
    version: &ReleasePrepVersion,
) -> Result<String, String> {
    if test_plan.contains(&version.tag) || test_plan.contains(&version.boundary_test_name) {
        return Err(format!(
            "release-process test plan already contains {}",
            version.tag
        ));
    }
    let req_id = next_numbered_id(test_plan, "RELEASE-REQ-")?;
    let with_requirement = insert_before_once(
        test_plan,
        "\n## Fixtures\n",
        &format!(
            "| {req_id} | planned | Structured release policy captures the `{tag}` release boundary; replace scaffold placeholders before release. | docs/topics/release-process/policy.toml |\n",
            tag = version.tag,
        ),
        "release-process requirements table",
    )?;
    let with_fixture = insert_before_once(
        &with_requirement,
        "\n## Test Cases\n",
        &format!(
            "| docs/releases/{tag}.md | Release-prep notes stub for `{tag}`. | The release workflow will look up this file by full tag name after the release-prep PR merges. |\n",
            tag = version.tag,
        ),
        "release-process fixtures table",
    )?;
    let tp_id = next_numbered_id(&with_fixture, "RELEASE-TP-")?;
    insert_before_once(
        &with_fixture,
        "\n## Determinism Obligations\n",
        &format!(
            "| {tp_id} | planned | Boundary guard | {req_id} | Structured policy captures the `{tag}` scope and non-goal boundary after scaffold placeholders are replaced. | {test_name} | docs/topics/release-process/policy.toml, docs/releases/{tag}.md | Scaffolded by `cargo xtask release-prep {tag}`; reviewers must replace TODO scope/non-goal values before release. |\n",
            tag = version.tag,
            test_name = version.boundary_test_name,
        ),
        "release-process test case table",
    )
}

fn insert_release_boundary_test_stub(
    source: &str,
    version: &ReleasePrepVersion,
    target_date: &str,
) -> Result<String, String> {
    if source.contains(&format!("fn {}()", version.boundary_test_name)) {
        return Err(format!(
            "xtask source already contains {}",
            version.boundary_test_name
        ));
    }
    let date_case = format!("            (\"{}\", \"{target_date}\"),\n", version.tag);
    let with_date = insert_in_alpha_changelog_date_cases(source, &date_case)?;
    let alpha_marker =
        if with_date.contains("    #[test]\n    fn alpha_changelog_dates_match_release_policy()") {
            "    #[test]\n    fn alpha_changelog_dates_match_release_policy()"
        } else {
            "fn alpha_changelog_dates_match_release_policy()"
        };
    insert_before_once(
        &with_date,
        alpha_marker,
        &format!(
            "\
    #[test]
    fn {test_name}() {{
        let root = repo_root().expect(\"repo root\");
        let policy = fs::read_to_string(root.join(\"docs/topics/release-process/policy.toml\"))
            .expect(\"release policy\");
        let release_policy = toml_section(&policy, \"{section}\");
        for required in [
            \"{section}\",
            \"tag = \\\"{tag}\\\"\",
            \"target_date = \\\"{target_date}\\\"\",
            \"status = \\\"prep\\\"\",
            \"TODO_release_scope\",
            \"TODO_release_non_goal\",
        ] {{
            assert!(
                release_policy.contains(required),
                \"{tag} release policy missing structured field: {{required}}\"
            );
        }}
    }}

",
            test_name = version.boundary_test_name,
            section = version.policy_section(),
            tag = version.tag,
        ),
        "xtask alpha changelog date guard",
    )
}

fn insert_in_alpha_changelog_date_cases(source: &str, date_case: &str) -> Result<String, String> {
    if source.contains(date_case.trim()) {
        return Err("alpha changelog date guard already contains release".into());
    }
    let function_start = source
        .find("fn alpha_changelog_dates_match_release_policy()")
        .ok_or_else(|| {
            "xtask source missing alpha_changelog_dates_match_release_policy".to_owned()
        })?;
    let tail = &source[function_start..];
    let list_end = tail
        .find("] {")
        .ok_or_else(|| "alpha changelog date guard missing tuple list terminator".to_owned())?;
    let line_start = tail[..list_end].rfind('\n').map_or(0, |index| index + 1);
    let insert_at = function_start + line_start;
    let mut output = String::with_capacity(source.len() + date_case.len());
    output.push_str(&source[..insert_at]);
    output.push_str(date_case);
    output.push_str(&source[insert_at..]);
    Ok(output)
}

fn next_numbered_id(text: &str, prefix: &str) -> Result<String, String> {
    let mut max_id = None;
    let mut width = 3usize;
    for (index, _) in text.match_indices(prefix) {
        let rest = &text[index + prefix.len()..];
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if digits.is_empty() {
            continue;
        }
        width = width.max(digits.len());
        let value = digits
            .parse::<u16>()
            .map_err(|err| format!("parse numbered id `{prefix}{digits}`: {err}"))?;
        max_id = Some(max_id.map_or(value, |current: u16| current.max(value)));
    }
    let next = max_id.ok_or_else(|| format!("no existing IDs with prefix {prefix}"))? + 1;
    Ok(format!("{prefix}{next:0width$}"))
}

fn insert_before_once(
    text: &str,
    needle: &str,
    insertion: &str,
    description: &str,
) -> Result<String, String> {
    let index = text
        .find(needle)
        .ok_or_else(|| format!("{description} missing insertion marker `{needle}`"))?;
    let mut output = String::with_capacity(text.len() + insertion.len());
    output.push_str(&text[..index]);
    output.push_str(insertion);
    output.push_str(&text[index..]);
    Ok(output)
}

fn replace_once(
    text: &str,
    needle: &str,
    replacement: &str,
    description: &str,
) -> Result<String, String> {
    if !text.contains(needle) {
        return Err(format!("{description} missing marker `{needle}`"));
    }
    Ok(text.replacen(needle, replacement, 1))
}

fn next_release_target_date(policy: &str) -> Result<String, String> {
    let latest = policy
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("target_date = ")
                .and_then(parse_quoted_string)
        })
        .next_back()
        .ok_or_else(|| "release policy contains no target_date fields".to_owned())?;
    add_days_to_iso_date(&latest, 14)
}

fn add_days_to_iso_date(date: &str, days: u8) -> Result<String, String> {
    let mut parts = date.split('-');
    let mut year = parse_date_part(parts.next(), "year")?;
    let mut month = parse_date_part(parts.next(), "month")?;
    let mut day = parse_date_part(parts.next(), "day")?;
    if parts.next().is_some() || month == 0 || month > 12 || day == 0 {
        return Err(format!("invalid ISO date `{date}`"));
    }
    for _ in 0..days {
        day += 1;
        let month_days = days_in_month(year, month)?;
        if day > month_days {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn parse_date_part(part: Option<&str>, name: &str) -> Result<u16, String> {
    part.ok_or_else(|| format!("date missing {name}"))?
        .parse::<u16>()
        .map_err(|err| format!("parse date {name}: {err}"))
}

fn days_in_month(year: u16, month: u16) -> Result<u16, String> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Ok(31),
        4 | 6 | 9 | 11 => Ok(30),
        2 if is_leap_year(year) => Ok(29),
        2 => Ok(28),
        _ => Err(format!("invalid month `{month}`")),
    }
}

fn is_leap_year(year: u16) -> bool {
    (is_divisible_by(year, 4) && !is_divisible_by(year, 100)) || is_divisible_by(year, 400)
}

fn is_divisible_by(value: u16, divisor: u16) -> bool {
    value.rem_euclid(divisor) == 0
}
