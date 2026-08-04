//! Reconcile recorded release dates against the git tags that published them.
//!
//! The structured release policy, `CHANGELOG.md`, and `docs/releases/*.md` all
//! record a date per release. Those three surfaces are written by
//! `cargo xtask release-prep` from a single field, so comparing them against
//! each other only proves the copy succeeded. This check compares them against
//! `git for-each-ref refs/tags`, which is the independent authority for when a
//! release actually happened.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::util::read_to_string;

/// One `[release_notes.*]` block parsed from the structured release policy.
///
/// Parsing into blocks keeps every field lookup scoped to the release that owns
/// it. A whole-file substring search cannot distinguish one release's
/// `target_date` from another's, which matters because releases tagged on the
/// same day legitimately share a date string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReleasePolicyBlock {
    pub(crate) section: String,
    pub(crate) tag: Option<String>,
    pub(crate) target_date: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) body: String,
}

pub(crate) fn parse_release_policy_blocks(policy: &str) -> Vec<ReleasePolicyBlock> {
    let mut blocks = Vec::new();
    let mut current: Option<ReleasePolicyBlock> = None;
    for line in policy.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if let Some(block) = current.take() {
                blocks.push(block);
            }
            if let Some(section) = trimmed
                .strip_prefix("[release_notes.")
                .and_then(|rest| rest.strip_suffix(']'))
            {
                current = Some(ReleasePolicyBlock {
                    section: section.to_owned(),
                    tag: None,
                    target_date: None,
                    status: None,
                    body: String::new(),
                });
            }
            continue;
        }
        if let Some(block) = current.as_mut() {
            block.body.push_str(line);
            block.body.push('\n');
            if let Some(value) = quoted_field(trimmed, "tag") {
                block.tag = Some(value);
            } else if let Some(value) = quoted_field(trimmed, "target_date") {
                block.target_date = Some(value);
            } else if let Some(value) = quoted_field(trimmed, "status") {
                block.status = Some(value);
            }
        }
    }
    if let Some(block) = current.take() {
        blocks.push(block);
    }
    blocks
}

fn quoted_field(line: &str, name: &str) -> Option<String> {
    let rest = line.strip_prefix(name)?.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}

pub(crate) fn changelog_release_date(changelog: &str, tag: &str) -> Option<String> {
    let needle = format!("## [{tag}] - ");
    changelog.lines().find_map(|line| {
        line.trim_end()
            .strip_prefix(&needle)
            .map(|date| date.trim().to_owned())
    })
}

pub(crate) fn release_notes_date(notes: &str) -> Option<String> {
    notes.lines().find_map(|line| {
        line.trim_end()
            .strip_prefix("Target date: ")
            .map(|date| date.trim().to_owned())
    })
}

fn git_tag_dates(root: &Path) -> Result<BTreeMap<String, String>, String> {
    let output = Command::new("git")
        .args([
            "for-each-ref",
            "--format=%(refname:short)\t%(creatordate:short)",
            "refs/tags",
        ])
        .current_dir(root)
        .output()
        .map_err(|err| format!("failed to run `git for-each-ref`: {err}"))?;
    if !output.status.success() {
        return Err("`git for-each-ref refs/tags` failed".to_owned());
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|err| format!("`git for-each-ref` output is not utf-8: {err}"))?;
    let mut dates = BTreeMap::new();
    for line in text.lines() {
        if let Some((tag, date)) = line.split_once('\t') {
            let (tag, date) = (tag.trim(), date.trim());
            if !tag.is_empty() && !date.is_empty() {
                dates.insert(tag.to_owned(), date.to_owned());
            }
        }
    }
    Ok(dates)
}

/// Outcome of comparing recorded dates against tag dates.
///
/// A recorded date that contradicts its tag is `drift` and fails the gate. A
/// surface that does not exist has no date to contradict anything; the earliest
/// releases predate these surfaces, so absence is reported as a `gap` rather
/// than failing an otherwise-correct history.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ReleaseDateReport {
    pub(crate) drift: Vec<String>,
    pub(crate) gaps: Vec<String>,
}

/// Pure comparison over already-read inputs.
///
/// `release_notes` maps a tag to the date parsed from its release notes; an
/// absent key means the notes file itself is missing, and a `None` value means
/// the file exists without a `Target date:` line.
pub(crate) fn reconcile_release_dates(
    tags: &BTreeMap<String, String>,
    policy: &str,
    changelog: &str,
    release_notes: &BTreeMap<String, Option<String>>,
) -> ReleaseDateReport {
    let blocks = parse_release_policy_blocks(policy);
    let mut drift = Vec::new();
    let mut gaps = Vec::new();

    for (tag, tag_date) in tags {
        match blocks
            .iter()
            .find(|block| block.tag.as_deref() == Some(tag.as_str()))
        {
            Some(block) => {
                let section = &block.section;
                match block.target_date.as_deref() {
                    Some(date) if date == tag_date => {}
                    Some(date) => drift.push(format!(
                        "policy.toml [release_notes.{section}] target_date is {date}, but tag {tag} was created {tag_date}"
                    )),
                    None => gaps.push(format!(
                        "policy.toml [release_notes.{section}] has no target_date for tag {tag}"
                    )),
                }
                if block.status.as_deref() != Some("published") {
                    let status = block.status.as_deref().unwrap_or("<missing>");
                    drift.push(format!(
                        "policy.toml [release_notes.{section}] has status `{status}`, but tag {tag} exists"
                    ));
                }
                for field in ["scope = [", "non_goals = ["] {
                    if !block.body.contains(field) {
                        gaps.push(format!(
                            "policy.toml [release_notes.{section}] is missing `{field}` for tag {tag}"
                        ));
                    }
                }
            }
            None => gaps.push(format!(
                "policy.toml has no [release_notes.*] block for tag {tag}"
            )),
        }

        match changelog_release_date(changelog, tag) {
            Some(date) if date == *tag_date => {}
            Some(date) => drift.push(format!(
                "CHANGELOG.md `## [{tag}]` is dated {date}, but the tag was created {tag_date}"
            )),
            None => gaps.push(format!("CHANGELOG.md has no `## [{tag}]` section")),
        }

        match release_notes.get(tag) {
            Some(Some(date)) if date == tag_date => {}
            Some(Some(date)) => drift.push(format!(
                "docs/releases/{tag}.md records `Target date: {date}`, but the tag was created {tag_date}"
            )),
            Some(None) => gaps.push(format!("docs/releases/{tag}.md has no `Target date:` line")),
            None => gaps.push(format!("docs/releases/{tag}.md is missing")),
        }
    }

    ReleaseDateReport { drift, gaps }
}

pub(crate) fn release_dates(root: &Path) -> Result<(), String> {
    let tags = git_tag_dates(root)?;
    if tags.is_empty() {
        // A shallow clone without tags cannot reconcile anything. Say so out
        // loud rather than reporting a vacuous pass.
        println!("release-dates: no git tags present; reconciliation skipped");
        return Ok(());
    }

    let policy = read_to_string(&root.join("docs/topics/release-process/policy.toml"))?;
    let changelog = read_to_string(&root.join("CHANGELOG.md"))?;
    let mut release_notes = BTreeMap::new();
    for tag in tags.keys() {
        let notes_path = root.join(format!("docs/releases/{tag}.md"));
        if notes_path.is_file() {
            release_notes.insert(
                tag.clone(),
                release_notes_date(&read_to_string(&notes_path)?),
            );
        }
    }

    let report = reconcile_release_dates(&tags, &policy, &changelog, &release_notes);
    for gap in &report.gaps {
        println!("release-dates: uncovered - {gap}");
    }
    if report.drift.is_empty() {
        let (count, gap_count) = (tags.len(), report.gaps.len());
        println!(
            "release-dates: {count} tag(s) reconciled against git, {gap_count} uncovered surface(s)"
        );
        return Ok(());
    }
    Err(format!(
        "release dates disagree with git tags:\n  {}",
        report.drift.join("\n  ")
    ))
}
