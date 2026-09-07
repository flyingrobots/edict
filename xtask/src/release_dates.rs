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
    Tag,
    PolicyBlock,
    PolicySection,
    PolicyStatus,
    PolicyTargetDate,
    PolicyScope,
    PolicyNonGoals,
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
    drift: &mut Vec<ReleaseDateFinding>,
    gaps: &mut Vec<ReleaseDateFinding>,
) {
    let finding = ReleaseDateFinding::new(
        ReleaseDateFindingKind::MissingSurface,
        tag,
        surface,
        None,
        None,
    );
    if is_legacy_uncovered(tag, surface) {
        gaps.push(finding);
    } else {
        drift.push(finding);
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

/// Stable categories produced by release-date reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReleaseDateFindingKind {
    MissingSurface,
    DateMismatch,
    SectionMismatch,
    InvalidStatus,
    AwaitingPublication,
    LightweightTag,
    MissingTaggerDate,
}

/// Structured evidence; expected and actual values belong to the named surface.
/// Dates are ISO date strings, sections are policy keys, and statuses are policy
/// status values. Missing surfaces have neither comparison value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReleaseDateFinding {
    pub(crate) kind: ReleaseDateFindingKind,
    pub(crate) tag: String,
    pub(crate) surface: Surface,
    pub(crate) expected: Option<String>,
    pub(crate) actual: Option<String>,
}

impl ReleaseDateFinding {
    fn new(
        kind: ReleaseDateFindingKind,
        tag: &str,
        surface: Surface,
        expected: Option<&str>,
        actual: Option<&str>,
    ) -> Self {
        Self {
            kind,
            tag: tag.to_owned(),
            surface,
            expected: expected.map(str::to_owned),
            actual: actual.map(str::to_owned),
        }
    }
}

/// `drift` fails the gate. `gaps` cover only allowlisted legacy omissions and
/// the window between tag creation and the post-publication status change.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct ReleaseDateReport {
    pub(crate) drift: Vec<ReleaseDateFinding>,
    pub(crate) gaps: Vec<ReleaseDateFinding>,
}

fn reconcile_policy_block(
    tag: &str,
    tag_date: &str,
    blocks: &[ReleasePolicyBlock],
    drift: &mut Vec<ReleaseDateFinding>,
    gaps: &mut Vec<ReleaseDateFinding>,
) {
    let Some(block) = blocks
        .iter()
        .find(|block| block.tag.as_deref() == Some(tag))
    else {
        record_absent(tag, Surface::PolicyBlock, drift, gaps);
        return;
    };
    let expected_section = policy_section_key(tag);
    if block.section != expected_section {
        drift.push(ReleaseDateFinding::new(
            ReleaseDateFindingKind::SectionMismatch,
            tag,
            Surface::PolicySection,
            Some(&expected_section),
            Some(&block.section),
        ));
    }
    match block.target_date.as_deref() {
        Some(date) if date == tag_date => {}
        Some(date) => drift.push(ReleaseDateFinding::new(
            ReleaseDateFindingKind::DateMismatch,
            tag,
            Surface::PolicyTargetDate,
            Some(tag_date),
            Some(date),
        )),
        None => record_absent(tag, Surface::PolicyTargetDate, drift, gaps),
    }
    match block.status.as_deref() {
        Some("published") => {}
        Some("prep") => gaps.push(ReleaseDateFinding::new(
            ReleaseDateFindingKind::AwaitingPublication,
            tag,
            Surface::PolicyStatus,
            Some("published"),
            Some("prep"),
        )),
        other => drift.push(ReleaseDateFinding::new(
            ReleaseDateFindingKind::InvalidStatus,
            tag,
            Surface::PolicyStatus,
            Some("published or prep"),
            other,
        )),
    }
    for (field, surface) in [
        ("scope = [", Surface::PolicyScope),
        ("non_goals = [", Surface::PolicyNonGoals),
    ] {
        if !block.body.contains(field) {
            record_absent(tag, surface, drift, gaps);
        }
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
            drift.push(ReleaseDateFinding::new(
                ReleaseDateFindingKind::LightweightTag,
                tag,
                Surface::Tag,
                Some("annotated"),
                Some("lightweight"),
            ));
            continue;
        }
        let Some(tag_date) = record.date.as_deref() else {
            drift.push(ReleaseDateFinding::new(
                ReleaseDateFindingKind::MissingTaggerDate,
                tag,
                Surface::Tag,
                None,
                None,
            ));
            continue;
        };
        reconcile_policy_block(tag, tag_date, &blocks, &mut drift, &mut gaps);
        match changelog_release_date(changelog, tag) {
            Some(date) if date == tag_date => {}
            Some(date) => drift.push(ReleaseDateFinding::new(
                ReleaseDateFindingKind::DateMismatch,
                tag,
                Surface::ChangelogSection,
                Some(tag_date),
                Some(&date),
            )),
            None => record_absent(tag, Surface::ChangelogSection, &mut drift, &mut gaps),
        }
        match release_notes.get(tag) {
            Some(Some(date)) if date == tag_date => {}
            Some(Some(date)) => drift.push(ReleaseDateFinding::new(
                ReleaseDateFindingKind::DateMismatch,
                tag,
                Surface::ReleaseNotesDate,
                Some(tag_date),
                Some(date),
            )),
            Some(None) => record_absent(tag, Surface::ReleaseNotesDate, &mut drift, &mut gaps),
            None => record_absent(tag, Surface::ReleaseNotesFile, &mut drift, &mut gaps),
        }
    }
    ReleaseDateReport { drift, gaps }
}

fn render_finding(finding: &ReleaseDateFinding) -> String {
    let surface = match finding.surface {
        Surface::Tag => "git tag",
        Surface::PolicyBlock => "policy block",
        Surface::PolicySection => "policy section key",
        Surface::PolicyStatus => "policy status",
        Surface::PolicyTargetDate => "policy target_date",
        Surface::PolicyScope => "policy scope",
        Surface::PolicyNonGoals => "policy non_goals",
        Surface::ChangelogSection => "changelog section",
        Surface::ReleaseNotesFile => "release notes file",
        Surface::ReleaseNotesDate => "release notes Target date",
    };
    let detail = match (&finding.expected, &finding.actual) {
        (Some(expected), Some(actual)) => format!("; expected {expected}, actual {actual}"),
        (Some(expected), None) => format!("; expected {expected}, actual value missing"),
        (None, Some(actual)) => format!("; actual {actual}"),
        (None, None) => String::new(),
    };
    format!("{:?}: {} [{surface}]{detail}", finding.kind, finding.tag)
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
        println!("release-dates: uncovered - {}", render_finding(gap));
    }
    if report.drift.is_empty() {
        let (count, gap_count) = (tags.len(), report.gaps.len());
        println!("release-dates: {count} tag(s) reconciled against git, {gap_count} uncovered surface(s)");
        return Ok(());
    }
    let findings: Vec<_> = report.drift.iter().map(render_finding).collect();
    Err(format!(
        "release dates disagree with git tags:\n  {}",
        findings.join("\n  ")
    ))
}
