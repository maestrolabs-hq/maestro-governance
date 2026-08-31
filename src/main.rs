//! `governance` -- what the repositories should look like, and what they do.
//!
//! GitHub is reached through the `gh` CLI: it already holds the credentials
//! and the API surface, so a crate would add a second auth path for no gain.

mod baseline;

use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

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

fn org() -> String {
    env::var("MAESTRO_GITHUB_ORG").unwrap_or_else(|_| "maestrolabs-hq".to_owned())
}

fn repo_root() -> PathBuf {
    env::var_os("MAESTRO_GOVERNANCE_REPO")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

fn apply_settings(repo: &str, drift: &[baseline::Drift]) -> Result<(), String> {
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

fn read_settings(repo: &str) -> Result<BTreeMap<String, String>, String> {
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

fn read_org() -> Result<BTreeMap<String, String>, String> {
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

/// `<kind>/<name>` -> state, for objects the baseline asserts but does not own.
fn read_present() -> Result<BTreeMap<String, String>, String> {
    let o = org();
    read(&[
        [
            &format!("orgs/{o}/rulesets"),
            r#".[] | "ruleset/\(.name)=\(.enforcement)""#,
        ],
        [
            &format!("orgs/{o}/code-security/configurations"),
            r#".[] | select(.target_type != "global") | "security-configuration/\(.name)=\(.enforcement)""#,
        ],
    ])
}

fn usage() -> ExitCode {
    eprintln!("usage: governance <plan|apply [--auto-approve]>");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let path = repo_root().join("baseline.txt");
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("governance: cannot read {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let desired = match baseline::parse(&text) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
    };

    let args: Vec<String> = env::args().skip(1).collect();
    let Some(verb @ ("plan" | "apply")) = args.first().map(String::as_str) else {
        return usage();
    };

    // Organisation-wide first: these cover repositories that do not exist yet,
    // so a drift here is wider than any single repository's.
    let org_drift = match read_org() {
        Ok(actual) => baseline::drift_org(&desired, &actual),
        Err(e) => {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
    };
    let present_drift = match read_present() {
        Ok(actual) => baseline::drift_present(&desired, &actual),
        Err(e) => {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mut found = Vec::new();
    for repo in &desired.repos {
        match read_settings(repo) {
            Ok(actual) => found.push((repo.clone(), baseline::drift(&desired, &actual))),
            Err(e) => {
                eprintln!("governance: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    let total: usize =
        found.iter().map(|(_, d)| d.len()).sum::<usize>() + org_drift.len() + present_drift.len();
    if total == 0 {
        println!(
            "No drift. {} repositories match the baseline.",
            desired.repos.len()
        );
        return ExitCode::SUCCESS;
    }
    println!("governance found the following drift:\n");
    if !org_drift.is_empty() || !present_drift.is_empty() {
        println!("  {} (organisation)", org());
        for d in org_drift.iter().chain(present_drift.iter()) {
            println!("    ~ {} : {} -> {}", d.key, d.actual, d.desired);
        }
    }
    for (repo, drift) in &found {
        if drift.is_empty() {
            continue;
        }
        println!("  {repo}");
        for d in drift {
            println!("    ~ {} : {} -> {}", d.key, d.actual, d.desired);
        }
    }
    println!("\nPlan: {total} setting(s) to change.");

    if verb == "plan" {
        return ExitCode::SUCCESS;
    }
    if !args.iter().any(|a| a == "--auto-approve") {
        println!("\nRefusing to change {total} setting(s) unprompted.");
        println!("Review the plan above, then: just apply --auto-approve");
        return ExitCode::FAILURE;
    }
    if !org_drift.is_empty() || !present_drift.is_empty() {
        eprintln!(
            "\ngovernance: {} organisation control(s) drifted and this tool does not write them.",
            org_drift.len() + present_drift.len()
        );
        eprintln!("They are checked, not applied. Correct them and re-run.");
        return ExitCode::FAILURE;
    }
    for (repo, drift) in &found {
        if drift.is_empty() {
            continue;
        }
        if let Err(e) = apply_settings(repo, drift) {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
        println!("  updated {repo}");
    }
    println!("\nApply complete. {total} setting(s) changed.");
    ExitCode::SUCCESS
}
