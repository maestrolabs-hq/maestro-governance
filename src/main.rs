//! `governance` -- what the repositories should look like, and what they do.
//!
//! GitHub is reached through the `gh` CLI: it already holds the credentials
//! and the API surface, so a crate would add a second auth path for no gain.

mod baseline;
mod github;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn repo_root() -> PathBuf {
    env::var_os("MAESTRO_GOVERNANCE_REPO")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
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
    let org_drift = match github::read_org() {
        Ok(actual) => baseline::drift_org(&desired, &actual),
        Err(e) => {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
    };
    let mut found = Vec::new();
    for repo in &desired.repos {
        let mut d = match github::read_settings(repo) {
            Ok(actual) => baseline::drift(&desired, &actual),
            Err(e) => {
                eprintln!("governance: {e}");
                return ExitCode::FAILURE;
            }
        };
        match github::read_rules(repo) {
            Ok(actual) => d.extend(baseline::drift_rules(&desired, &actual)),
            Err(e) => {
                eprintln!("governance: {e}");
                return ExitCode::FAILURE;
            }
        }
        found.push((repo.clone(), d));
    }

    let total: usize = found.iter().map(|(_, d)| d.len()).sum::<usize>() + org_drift.len();
    if total == 0 {
        println!(
            "No drift. {} repositories match the baseline.",
            desired.repos.len()
        );
        return ExitCode::SUCCESS;
    }
    println!("governance found the following drift:\n");
    if !org_drift.is_empty() {
        println!("  {} (organisation)", github::org());
        for d in &org_drift {
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
    if !org_drift.is_empty() {
        eprintln!(
            "\ngovernance: {} organisation control(s) drifted and this tool does not write them.",
            org_drift.len()
        );
        eprintln!("They are checked, not applied. Correct them and re-run.");
        return ExitCode::FAILURE;
    }
    for (repo, drift) in &found {
        if drift.is_empty() {
            continue;
        }
        if let Err(e) = github::apply_settings(repo, drift) {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
        println!("  updated {repo}");
    }
    println!("\nApply complete. {total} setting(s) changed.");
    ExitCode::SUCCESS
}
