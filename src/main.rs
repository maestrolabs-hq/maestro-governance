//! `governance` -- what the repositories should look like, and what they do.
//!
//! GitHub is reached through the `gh` CLI: it already holds the credentials
//! and the API surface, so a crate would add a second auth path for no gain.

use governance::baseline;
mod github;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Everything the baseline can be compared against, read once.
///
/// Organisation-wide readings first: a drift there is wider than any single
/// repository's, because it covers repositories that do not exist yet.
type Gathered = (Vec<baseline::Drift>, Vec<(String, Vec<baseline::Drift>)>);

fn gather(desired: &baseline::Baseline) -> Result<Gathered, String> {
    let org_drift = baseline::drift_org(desired, &github::read_org()?);

    let mut found = Vec::new();
    for repo in &desired.repos {
        let mut d = baseline::drift(desired, &github::read_settings(repo)?);
        d.extend(baseline::drift_rules(desired, &github::read_rules(repo)?));
        d.extend(baseline::drift_files(
            desired,
            repo,
            &github::read_files(repo)?,
        ));
        found.push((repo.clone(), d));
    }
    Ok((org_drift, found))
}

fn repo_root() -> PathBuf {
    env::var_os("MAESTRO_GOVERNANCE_REPO")
        .map_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")), PathBuf::from)
}

fn usage() -> ExitCode {
    eprintln!("usage: governance <plan|apply [--auto-approve]|offboard <repo>>");
    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let path = repo_root().join("baseline.txt");
    let text = match fs::read_to_string(&path) {
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

    // Before gather(): offboarding edits a text file and reaches nothing, so it
    // must not fail because GitHub is unreachable.
    if args.first().map(String::as_str) == Some("offboard") {
        return offboard(&path, &text, &desired, args.get(1).map(String::as_str));
    }

    let Some(verb @ ("plan" | "apply")) = args.first().map(String::as_str) else {
        return usage();
    };

    let (org_drift, found) = match gather(&desired) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
    };

    // A pending control that has become readable is not drift; it is the
    // signal to promote it into a real setting.
    match github::read_org() {
        Ok(actual) => {
            for key in baseline::arrived(&desired, &actual) {
                println!("  note: `{key}` is now readable. Promote it from `pending` to `org`.");
            }
        }
        Err(e) => {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
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
        // Non-zero because the fleet audit derives its only signal from this.
        // Exiting 0 here made every scheduled run green, left the tracking
        // issue unopened, and closed any issue a human had opened.
        return ExitCode::FAILURE;
    }
    carry_out(&args, &org_drift, &found, total)
}

/// Everything after the plan has been printed and the verb is `apply`.
///
/// Split from `main` because deciding what to write is a different question
/// from deciding what to report, and the two were 110 lines in one place.
fn carry_out(
    args: &[String],
    org_drift: &[baseline::Drift],
    found: &[(String, Vec<baseline::Drift>)],
    total: usize,
) -> ExitCode {
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
    let unwritable: usize = found
        .iter()
        .flat_map(|(_, d)| d.iter())
        .filter(|d| d.kind != baseline::Kind::Setting)
        .count();
    if unwritable > 0 {
        eprintln!(
            "\ngovernance: {unwritable} drift(s) are not repository settings and this tool does not write them."
        );
        eprintln!("A branch rule lives in a ruleset; a tracked file is fixed by a commit.");
        eprintln!("They are checked, not applied. Correct them and re-run.");
        return ExitCode::FAILURE;
    }
    let mut written = 0;
    for (repo, drift) in found {
        if drift.is_empty() {
            continue;
        }
        match github::apply_settings(repo, drift) {
            Ok(n) => {
                written += n;
                println!("  updated {repo}");
            }
            Err(e) => {
                eprintln!("governance: {e}");
                eprintln!("{written} setting(s) were already written before this failed.");
                return ExitCode::FAILURE;
            }
        }
    }
    println!("\nApply complete. {written} setting(s) changed.");
    ExitCode::SUCCESS
}

/// Stop watching a repository. Edits the baseline and calls nothing.
///
/// Deliberately not named `destroy`. Nothing on GitHub changes: the settings
/// and rules stay exactly as they are, they simply stop being reported when
/// they drift. Naming it destroy would suggest the controls were removed.
fn offboard(path: &Path, text: &str, before: &baseline::Baseline, repo: Option<&str>) -> ExitCode {
    let Some(repo) = repo else {
        eprintln!("usage: governance offboard <repo>");
        return ExitCode::from(2);
    };
    let updated = match baseline::offboard(text, repo) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("governance: {e}");
            return ExitCode::FAILURE;
        }
    };
    let after = match baseline::parse(&updated) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("governance: the result does not parse: {e}");
            return ExitCode::FAILURE;
        }
    };

    println!(
        "No longer managed:
"
    );
    println!("  repo {repo}");
    println!(
        "  tracked files             {}",
        before.files.len() - after.files.len()
    );
    println!("  settings left as they are {}", before.settings.len());
    println!("  rules left in effect      {}", before.rules.len());
    println!(
        "
Nothing on GitHub changed. Those controls stay exactly as they are --"
    );
    println!(
        "they simply stop being reported if they drift.
"
    );

    let removed = text.lines().count() - updated.lines().count();
    if let Err(e) = fs::write(path, &updated) {
        eprintln!("governance: {}: {e}", path.display());
        return ExitCode::FAILURE;
    }
    println!("{}: {removed} line(s) removed.", path.display());
    ExitCode::SUCCESS
}
