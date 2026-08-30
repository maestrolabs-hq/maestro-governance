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
];

fn org() -> String {
    env::var("MAESTRO_GITHUB_ORG").unwrap_or_else(|_| "maestrolabs-hq".to_owned())
}

fn repo_root() -> PathBuf {
    env::var_os("MAESTRO_GOVERNANCE_REPO")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

/// One `gh` call per repository, returning `key=value` lines.
fn read_settings(repo: &str) -> Result<BTreeMap<String, String>, String> {
    let query = READABLE
        .iter()
        .map(|k| format!("{k}=\\(.{k})"))
        .collect::<Vec<_>>()
        .join("\n");
    let out = Command::new("gh")
        .args([
            "api",
            &format!("repos/{}/{repo}", org()),
            "--jq",
            &format!("\"{query}\""),
        ])
        .output()
        .map_err(|e| format!("gh is required and could not be run: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "reading {repo}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_owned(), v.trim().to_owned()))
        .collect())
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

    let total: usize = found.iter().map(|(_, d)| d.len()).sum();
    if total == 0 {
        println!(
            "No drift. {} repositories match the baseline.",
            desired.repos.len()
        );
        return ExitCode::SUCCESS;
    }
    println!("governance will change the following settings:\n");
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
