//! Reconcile recorded release dates against the git tags that published them.
//!
//! The structured release policy, `CHANGELOG.md`, and `docs/releases/*.md` all
//! record a date per release. Those three surfaces are written by
//! `cargo xtask release-prep` from a single field, so comparing them against
//! each other only proves the copy succeeded. This check compares them against
//! `git for-each-ref refs/tags/v*`, which is the independent authority for when
//! a release was tagged.
//!
//! The recorded date is the tag date, not the GitHub Release publication
//! timestamp. Those can differ: `v0.4.0-alpha.1` was tagged 2026-06-24 while the
//! prerelease was published 2026-06-25. Release notes record the publication
//! timestamp separately.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use crate::util::read_to_string;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Surface {
    PolicyBlock,
    PolicyTargetDate,
    PolicyScope,
    ChangelogSection,
    ReleaseNotesFile,
    ReleaseNotesDate,
}

/// Surfaces that legitimately do not exist for a given tag.
///
/// Only releases that predate a surface belong here. Every other absent surface
/// is a regression, so deleting a date-bearing surface fails the gate instead of
/// silently downgrading to an advisory line.
const LEGACY_UNCOVERED: &[(&str, Surface)] = &[
    // The first release predates the structured release policy entirely.
    ("v0.1.0-alpha.1", Surface::PolicyBlock),
];

/// Route an absent surface to the failing or advisory bucket.
///
/// Taking both vectors as arguments avoids a closure that would hold them
/// mutably borrowed across the rest of the comparison.
fn record_absent(
    tag: &str,
    surface: Surface,
    message: String,
    drift: &mut Vec<String>,
    gaps: &mut Vec<String>,
) {
    if is_legacy_uncovered(tag, surface) {
        gaps.push(message);
    } else {
        drift.push(message);
    }
}

fn is_legacy_uncovered(tag: &str, surface: Surface) -> bool {
    LEGACY_UNCOVERED
        .iter()
        .any(|(legacy_tag, legacy_surface)| *legacy_tag == tag && *legacy_surface == surface)
}

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

/// The `[release_notes.*]` key a tag must own: `v0.9.0-alpha.1` maps to
/// `v0_9_0_alpha_1`.
pub(crate) fn policy_section_key(tag: &str) -> String {
    tag.replace(['.', '-'], "_")
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

/// A release tag and the date it was created.
///
/// `annotated` distinguishes a tag object, whose `taggerdate` records when the
/// tag was made, from a lightweight tag, which has no tagger and would fall back
/// to the tagged commit's committer date. That fallback would let a tag placed
/// on an older commit report a date that never corresponded to a release, so
/// lightweight release tags are rejected rather than trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TagRecord {
    pub(crate) date: Option<String>,
    pub(crate) annotated: bool,
}

fn git_tag_dates(root: &Path) -> Result<BTreeMap<String, TagRecord>, String> {
    let output = Command::new("git")
        .args([
            "for-each-ref",
            // `format-local` renders in the TZ set below, so the rendered day
            // does not shift with an operator's local timezone.
            "--format=%(refname:short)\t%(objecttype)\t%(taggerdate:format-local:%Y-%m-%d)",
            // Release tags only; scratch tags would otherwise report permanent
            // uncovered lines on every run.
            "refs/tags/v*",
        ])
        .env("TZ", "UTC")
        .current_dir(root)
        .output()
        .map_err(|err| format!("failed to run `git for-each-ref`: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "`git for-each-ref refs/tags/v*` failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|err| format!("`git for-each-ref` output is not utf-8: {err}"))?;
    let mut tags = BTreeMap::new();
    for line in text.lines() {
        let mut fields = line.split('\t');
        let (Some(tag), Some(object_type)) = (fields.next(), fields.next()) else {
            continue;
        };
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        let date = fields.next().map(str::trim).filter(|date| !date.is_empty());
        tags.insert(
            tag.to_owned(),
            TagRecord {
                date: date.map(ToOwned::to_owned),
                annotated: object_type.trim() == "tag",
            },
        );
    }
    Ok(tags)
}

/// Outcome of comparing recorded dates against tag dates.
///
/// `drift` fails the gate: a recorded date contradicts its tag, or a surface
/// that should exist is absent. `gaps` are advisory and cover only the two cases
/// that are not contradictions: surfaces that predate a release, and the window
/// between tag creation and the post-publication change that flips a block from
/// `prep` to `published`.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ReleaseDateReport {
    pub(crate) drift: Vec<String>,
    pub(crate) gaps: Vec<String>,
}

/// Compare one tag against the policy block that should own it.
///
/// Split out of `reconcile_release_dates` so each comparison stays readable.
fn reconcile_policy_block(
    tag: &str,
    tag_date: &str,
    blocks: &[ReleasePolicyBlock],
    drift: &mut Vec<String>,
    gaps: &mut Vec<String>,
) {
    match blocks
        .iter()
        .find(|block| block.tag.as_deref() == Some(tag))
    {
        Some(block) => {
            let section = &block.section;
            let expected_section = policy_section_key(tag);
            if *section != expected_section {
                drift.push(format!(
                    "policy.toml [release_notes.{section}] declares tag {tag}, which belongs in [release_notes.{expected_section}]"
                ));
            }
            match block.target_date.as_deref() {
                Some(date) if date == tag_date => {}
                Some(date) => drift.push(format!(
                    "policy.toml [release_notes.{section}] target_date is {date}, but tag {tag} was created {tag_date}"
                )),
                None => record_absent(
                    tag,
                    Surface::PolicyTargetDate,
                    format!(
                        "policy.toml [release_notes.{section}] has no target_date for tag {tag}"
                    ),
                    drift,
                    gaps,
                ),
            }
            match block.status.as_deref() {
                Some("published") => {}
                // The tag is created before the post-publication evidence
                // change flips the status, so `prep` is a lagging surface
                // rather than a contradiction. Failing here would make
                // `verify` red on `main` for every unrelated branch until
                // that second change lands.
                Some("prep") => gaps.push(format!(
                    "policy.toml [release_notes.{section}] still has status `prep` for existing tag {tag}"
                )),
                other => {
                    let status = other.unwrap_or("<missing>");
                    drift.push(format!(
                        "policy.toml [release_notes.{section}] has status `{status}`, but tag {tag} exists"
                    ));
                }
            }
            for field in ["scope = [", "non_goals = ["] {
                if !block.body.contains(field) {
                    record_absent(
                        tag,
                        Surface::PolicyScope,
                        format!(
                            "policy.toml [release_notes.{section}] is missing `{field}` for tag {tag}"
                        ),
                        drift,
                        gaps,
                    );
                }
            }
        }
        None => record_absent(
            tag,
            Surface::PolicyBlock,
            format!("policy.toml has no [release_notes.*] block for tag {tag}"),
            drift,
            gaps,
        ),
    }
}

/// Pure comparison over already-read inputs.
///
/// `release_notes` maps a tag to the date parsed from its release notes; an
/// absent key means the notes file itself is missing, and a `None` value means
/// the file exists without a `Target date:` line.
pub(crate) fn reconcile_release_dates(
    tags: &BTreeMap<String, TagRecord>,
    policy: &str,
    changelog: &str,
    release_notes: &BTreeMap<String, Option<String>>,
) -> ReleaseDateReport {
    let blocks = parse_release_policy_blocks(policy);
    let mut drift = Vec::new();
    let mut gaps = Vec::new();

    for (tag, record) in tags {
        if !record.annotated {
            drift.push(format!(
                "release tag {tag} is lightweight; release tags must be annotated so the tag date records the release"
            ));
            continue;
        }
        let Some(tag_date) = record.date.as_deref() else {
            drift.push(format!("release tag {tag} has no tagger date"));
            continue;
        };

        reconcile_policy_block(tag, tag_date, &blocks, &mut drift, &mut gaps);

        match changelog_release_date(changelog, tag) {
            Some(date) if date == tag_date => {}
            Some(date) => drift.push(format!(
                "CHANGELOG.md `## [{tag}]` is dated {date}, but the tag was created {tag_date}"
            )),
            None => record_absent(
                tag,
                Surface::ChangelogSection,
                format!("CHANGELOG.md has no `## [{tag}]` section"),
                &mut drift,
                &mut gaps,
            ),
        }

        match release_notes.get(tag) {
            Some(Some(date)) if date == tag_date => {}
            Some(Some(date)) => drift.push(format!(
                "docs/releases/{tag}.md records `Target date: {date}`, but the tag was created {tag_date}"
            )),
            Some(None) => record_absent(
                tag,
                Surface::ReleaseNotesDate,
                format!("docs/releases/{tag}.md has no `Target date:` line"),
                &mut drift,
                &mut gaps,
            ),
            None => record_absent(
                tag,
                Surface::ReleaseNotesFile,
                format!("docs/releases/{tag}.md is missing"),
                &mut drift,
                &mut gaps,
            ),
        }
    }

    ReleaseDateReport { drift, gaps }
}

pub(crate) fn release_dates(root: &Path) -> Result<(), String> {
    let tags = git_tag_dates(root)?;
    if tags.is_empty() {
        // A missing tag authority is not approval. Fail closed rather than
        // reporting a vacuous pass in a clone that never fetched tags.
        return Err(
            "no `v*` release tags are present, so recorded dates cannot be reconciled; fetch tags with `git fetch --tags` before running this check"
                .to_owned(),
        );
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
