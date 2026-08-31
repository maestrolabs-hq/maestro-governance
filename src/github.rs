//! Everything that talks to GitHub.
//!
//! Reached through the `gh` CLI: it already holds the credentials and the
//! API surface, so a crate would add a second authentication path for no
//! gain. Every function here returns `key=value` readings; nothing here
//! decides anything.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::process::Command;

use crate::baseline;

/// The keys read back from a repository, exactly those the baseline can set.
const READABLE: &[&str] = &[
    "allow_squash_merge",
    "allow_merge_commit",
    "allow_rebase_merge",
    "delete_branch_on_merge",
    "has_wiki",
    "has_projects",
    "has_issues",
    "web_commit_signoff_required",
];

pub fn org() -> String {
    env::var("MAESTRO_GITHUB_ORG").unwrap_or_else(|_| "maestrolabs-hq".to_owned())
}

pub fn apply_settings(repo: &str, drift: &[baseline::Drift]) -> Result<(), String> {
    let mut cmd = Command::new("gh");
    cmd.args(["api", "-X", "PATCH", &format!("repos/{}/{repo}", org())]);
    for d in drift {
        cmd.args(["-F", &format!("{}={}", d.key, d.desired)]);
    }
    let out = cmd.output().map_err(|e| format!("gh: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        Err(format!(
            "writing {repo}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

/// Reads that are not per-repository. Each is one `gh` call with a jq
/// expression that flattens the answer to `key=value`, so nothing here needs a
/// JSON parser.
fn gh_lines(args: &[&str]) -> Result<Vec<String>, String> {
    let out = Command::new("gh")
        .args(args)
        .output()
        .map_err(|e| format!("gh could not be run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "{}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
}

/// Every reading is the same shape: run `gh` with a jq expression that
/// flattens the answer to `key=value` lines, and merge. Only the queries
/// differ, so they are data rather than four near-identical functions -- which
/// is what the duplication gate said when they were.
fn read(queries: &[[&str; 2]]) -> Result<BTreeMap<String, String>, String> {
    let mut all = BTreeMap::new();
    for [path, jq] in queries {
        all.extend(
            gh_lines(&["api", path, "--jq", jq])?
                .iter()
                .filter_map(|l| l.split_once('='))
                .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned())),
        );
    }
    Ok(all)
}

pub fn read_settings(repo: &str) -> Result<BTreeMap<String, String>, String> {
    let jq = format!(
        "\"{}\"",
        READABLE
            .iter()
            .map(|k| format!("{k}=\\(.{k})"))
            .collect::<Vec<_>>()
            .join("\\n")
    );
    read(&[[&format!("repos/{}/{repo}", org()), &jq]])
}

pub fn read_org() -> Result<BTreeMap<String, String>, String> {
    let o = org();
    read(&[
        [
            &format!("orgs/{o}"),
            r#""two_factor_requirement_enabled=\(.two_factor_requirement_enabled)""#,
        ],
        [
            &format!("orgs/{o}/actions/permissions"),
            r#""actions.sha_pinning_required=\(.sha_pinning_required)""#,
        ],
        [
            &format!("orgs/{o}/actions/permissions/workflow"),
            r#""actions.default_workflow_permissions=\(.default_workflow_permissions)\nactions.can_approve_pull_request_reviews=\(.can_approve_pull_request_reviews)""#,
        ],
        [
            &format!("orgs/{o}/properties/schema"),
            r#".[] | select(.property_name=="tier") | "properties.tier.required=\(.required)\nproperties.tier.default_value=\(.default_value)""#,
        ],
    ])
}

/// The rules actually in effect on a repository's default branch, keyed by
/// rule type with its source. Repository-scoped, so the audit token needs no
/// Every file at the repository root, by path, with its git blob hash. One
/// call rather than one per file, and the hash is what GitHub already stores
/// -- `git hash-object` reproduces it, so a baseline entry can be written from
/// a copy known to be right.
pub fn read_files(repo: &str) -> Result<BTreeMap<String, String>, String> {
    read(&[[
        &format!("repos/{}/{repo}/git/trees/{}", org(), default_branch()),
        r#".tree[] | select(.type=="blob") | "\(.path)=\(.sha)""#,
    ]])
}

/// organisation write -- which listing the rulesets would have required.
pub fn read_rules(repo: &str) -> Result<BTreeMap<String, String>, String> {
    let lines = gh_lines(&[
        "api",
        &format!("repos/{}/{repo}/rules/branches/{}", org(), default_branch()),
        "--jq",
        r#".[] | "\(.type)=\(.ruleset_source_type)""#,
    ])?;
    // A rule type can be in effect from more than one place, so the sources are
    // gathered rather than overwritten. `read` keeps the last value, which
    // silently dropped the organisation entry and reported drift on a correct
    // fleet.
    let mut sources: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (kind, source) in lines.iter().filter_map(|l| l.split_once('=')) {
        sources
            .entry(kind.trim().to_owned())
            .or_default()
            .insert(source.trim().to_owned());
    }
    Ok(sources
        .into_iter()
        .map(|(k, v)| (k, v.into_iter().collect::<Vec<_>>().join(",")))
        .collect())
}

fn default_branch() -> String {
    env::var("MAESTRO_DEFAULT_BRANCH").unwrap_or_else(|_| "main".to_owned())
}
