//! Taking a repository out of the baseline.
//!
//! Deliberately not a `destroy`. Terraform destroys what it built; this builds
//! nothing. It changes settings that already exist, and a setting cannot be
//! un-set, only set to something else. The branch rules are real objects that
//! could be deleted, but this tool did not create them and cannot list them:
//! that needs organisation write, which would let the audit rewrite what it
//! audits.
//!
//! So the inverse of managing a repository is not undoing its controls, it is
//! no longer watching it. Nothing on GitHub changes.

use super::parse;

/// Where the repository list starts on a `file` line, or `None` when the file
/// is unscoped and therefore applies to every repository.
fn scope_start(line: &str) -> Option<usize> {
    let mut seen = 0;
    let mut in_field = false;
    for (i, c) in line.char_indices() {
        if c.is_whitespace() {
            in_field = false;
        } else if !in_field {
            in_field = true;
            seen += 1;
            // file <path> <blob> <repo>...
            if seen == 4 {
                return Some(i);
            }
        }
    }
    None
}

/// Take a repository out of the baseline, returning the new text.
///
/// Not a destroy. Terraform destroys what it built; this builds nothing. It
/// changes settings that already exist, and a setting cannot be un-set, only
/// set to something else. The branch rules are real objects that could be
/// deleted, but this tool did not create them and deliberately cannot list
/// them: that needs organisation write, which would let the audit rewrite what
/// it audits.
///
/// So the inverse is not "undo the controls", it is "stop watching this one".
/// Nothing on GitHub changes.
///
/// Works on the text rather than on a parsed `Baseline` so comments, blank
/// lines and ordering survive. A file that is re-serialised loses the reasoning
/// written into it, and the reasoning is most of what it is for.
///
/// # Errors
///
/// Returns an error naming the repository when it is not in the baseline. A
/// typo would otherwise look exactly like success.
pub fn offboard(text: &str, repo: &str) -> Result<String, String> {
    if !parse(text)?.repos.iter().any(|r| r == repo) {
        return Err(format!("{repo} is not in the baseline"));
    }

    let mut out = Vec::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        match (fields.next(), fields.next()) {
            // The repository itself.
            (Some("repo"), Some(name)) if name == repo => {}
            // A tracked file, which may be scoped to several repositories.
            //
            // Only the scope is rewritten. Everything up to it is copied
            // verbatim, because the columns in this file are aligned by hand
            // and reformatting them would turn one removed name into a diff
            // touching every tracked file.
            (Some("file"), Some(_)) => {
                let Some(scope_at) = scope_start(line) else {
                    out.push(line.to_owned());
                    continue;
                };
                let (head, scope) = line.split_at(scope_at);
                let kept: Vec<&str> = scope.split_whitespace().filter(|r| *r != repo).collect();
                // Scoped to this repository alone: nothing left to describe.
                if kept.is_empty() {
                    continue;
                }
                out.push(format!("{head}{}", kept.join(" ")));
            }
            _ => out.push(line.to_owned()),
        }
    }
    let mut text = out.join("\n");
    text.push('\n');
    Ok(text)
}
