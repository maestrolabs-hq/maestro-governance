//! The desired state, and the difference between it and a machine-readable
//! reading of what a repository actually has.
//!
//! Pure: nothing here reaches the network. `main` fetches the readings and
//! hands them in, so every rule below is exercised by a test that constructs
//! its input literally.

mod offboard;
pub use offboard::offboard;

use std::collections::BTreeMap;

/// What the repositories should look like.
#[derive(Debug, Default)]
pub struct Baseline {
    pub repos: Vec<String>,
    pub settings: Vec<(String, String)>,
    pub org: Vec<(String, String)>,
    pub rules: Vec<(String, String)>,
    pub files: Vec<Tracked>,
    pub pending: Vec<(String, String)>,
}

/// A file every repository in `scope` must hold, byte for byte. An empty
/// scope means every repository.
#[derive(Debug)]
pub struct Tracked {
    pub path: String,
    pub blob: String,
    pub scope: Vec<String>,
}

/// One setting that does not match.
#[derive(Debug)]
pub struct Drift {
    pub key: String,
    pub desired: String,
    pub actual: String,
    /// Which reading produced this, and therefore which endpoint can fix it.
    /// Carried on the value rather than tracked by the caller, because the
    /// caller merged all three into one vector and sent the lot to the
    /// repository settings endpoint.
    pub kind: Kind,
}

/// The three things read, kept apart because only one of them is writable by
/// the endpoint `apply` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A repository setting. Writable by a `PATCH` on the repository.
    Setting,
    /// A branch rule. Lives in a ruleset, not in repository settings.
    Rule,
    /// Tracked file content, compared by blob hash. Fixed by a commit.
    File,
    /// An organisation-wide control. Never written by this tool.
    Org,
}

/// The `key=value` pairs that may be sent to the repository settings endpoint.
///
/// Anything that is not a setting is dropped here rather than at the call site,
/// so there is one place that decides what is writable.
#[must_use]
pub fn settings_fields(drift: &[Drift]) -> Vec<String> {
    drift
        .iter()
        .filter(|d| d.kind == Kind::Setting)
        .map(|d| format!("{}={}", d.key, d.desired))
        .collect()
}

/// One directive per line, `<kind> <argument...>`, after `provision.txt` in
/// maestro-pi-config -- readable without a parser and diffable without a tool.
///
/// # Errors
///
/// Returns the offending line when a directive is unknown or malformed. A
/// baseline that half-parses is worse than one that refuses: the unread half
/// becomes a control nobody is checking.
pub fn parse(text: &str) -> Result<Baseline, String> {
    let mut b = Baseline::default();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        match (parts.next(), parts.next(), parts.next()) {
            (Some("repo"), Some(name), None) => b.repos.push(name.to_owned()),
            (Some("setting"), Some(key), Some(value)) => {
                b.settings.push((key.to_owned(), value.to_owned()));
            }
            (Some("org"), Some(key), Some(value)) => {
                b.org.push((key.to_owned(), value.to_owned()));
            }
            (Some("rule"), Some(kind), Some(source)) => {
                b.rules.push((kind.to_owned(), source.to_owned()));
            }
            (Some("pending"), Some(key), Some(value)) => {
                b.pending.push((key.to_owned(), value.to_owned()));
            }
            (Some("file"), Some(path), Some(blob)) => {
                b.files.push(Tracked {
                    path: path.to_owned(),
                    blob: blob.to_owned(),
                    scope: parts.map(str::to_owned).collect(),
                });
            }
            (Some(other), _, _) => {
                return Err(format!("line {}: unknown directive `{other}`", n + 1));
            }
            _ => return Err(format!("line {}: incomplete directive", n + 1)),
        }
    }
    Ok(b)
}

/// Pending keys the API has started answering. Not drift -- an invitation to
/// promote the line from `pending` to a real setting.
#[must_use]
pub fn arrived(baseline: &Baseline, actual: &BTreeMap<String, String>) -> Vec<String> {
    baseline
        .pending
        .iter()
        .filter(|(k, _)| actual.contains_key(k))
        .map(|(k, _)| k.clone())
        .collect()
}

/// Organisation-wide settings. Same comparison as the repository one, over a
/// different reading.
#[must_use]
pub fn drift_org(baseline: &Baseline, actual: &BTreeMap<String, String>) -> Vec<Drift> {
    compare(&baseline.org, actual, UNREADABLE, Kind::Org)
}

/// The rules in effect on one repository's default branch, keyed by rule type
/// with the source (`Organization` or `Repository`) as the value.
/// Every listed repository must hold this exact file content. The value is a
/// git blob hash, which GitHub returns directly and `git hash-object`
/// reproduces, so the baseline can be written from a known-good copy.
#[must_use]
pub fn drift_files(
    baseline: &Baseline,
    repo: &str,
    actual: &BTreeMap<String, String>,
) -> Vec<Drift> {
    let wanted: Vec<(String, String)> = baseline
        .files
        .iter()
        .filter(|f| f.scope.is_empty() || f.scope.iter().any(|r| r == repo))
        .map(|f| (f.path.clone(), f.blob.clone()))
        .collect();
    compare(&wanted, actual, "<missing>", Kind::File)
}

#[must_use]
pub fn drift_rules(baseline: &Baseline, actual: &BTreeMap<String, String>) -> Vec<Drift> {
    baseline
        .rules
        .iter()
        .filter_map(|(kind, source)| {
            let observed = actual.get(kind).map_or("<not in effect>", String::as_str);
            // Membership, not equality: a rule may apply from the organisation
            // and the repository at the same time.
            (!observed.split(',').any(|s| s == source)).then(|| Drift {
                key: kind.clone(),
                desired: source.clone(),
                actual: observed.to_owned(),
                kind: Kind::Rule,
            })
        })
        .collect()
}

/// A setting the reading does not carry counts as drift. Treating it as
/// satisfied is how an audit reports a clean fleet it never looked at.
#[must_use]
pub fn drift(baseline: &Baseline, actual: &BTreeMap<String, String>) -> Vec<Drift> {
    compare(&baseline.settings, actual, UNREADABLE, Kind::Setting)
}

const UNREADABLE: &str = "<unreadable>";

fn compare(
    wanted: &[(String, String)],
    actual: &BTreeMap<String, String>,
    missing: &str,
    kind: Kind,
) -> Vec<Drift> {
    wanted
        .iter()
        .filter_map(|(key, desired)| {
            let observed = actual.get(key).map_or(missing, String::as_str);
            (observed != desired).then(|| Drift {
                key: key.clone(),
                desired: desired.clone(),
                actual: observed.to_owned(),
                kind,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_org_directive_is_scoped_to_the_organisation() {
        let b = parse("org actions.sha_pinning_required true\n").expect("parses");
        assert_eq!(
            b.org,
            [("actions.sha_pinning_required".to_owned(), "true".to_owned())]
        );
        assert!(
            b.settings.is_empty(),
            "an org setting is not a repo setting"
        );
    }

    /// Listing organisation rulesets needs "Administration" organisation
    /// permissions **write** -- GitHub offers no read-only way to see them.
    /// The audit reads the rules actually in effect on each branch instead,
    /// which needs only repository read and is better evidence: it proves the
    /// rule applies, not merely that a ruleset object exists somewhere.
    /// Some configuration cannot be centralised: cargo, rustup and
    /// editorconfig read from the repository root and offer no remote source.
    /// Duplication that cannot be removed still has to be noticed when it
    /// diverges, so the baseline records the exact content each copy must
    /// have.
    /// Some controls are enabled in the web interface and exposed through no
    /// API. They cannot be audited, and pretending they are covered would be
    /// worse than saying so. A `pending` line records the intent and the exact
    /// key to watch, and the audit reports when that key becomes readable --
    /// so the control gets enforced the day it becomes enforceable rather than
    /// whenever somebody happens to re-read the documentation.
    #[test]
    fn a_pending_directive_records_a_control_the_api_cannot_yet_reach() {
        let b = parse("pending code_quality enabled\n").expect("parses");
        assert_eq!(
            b.pending,
            [("code_quality".to_owned(), "enabled".to_owned())]
        );
    }

    #[test]
    fn a_pending_key_that_becomes_readable_is_reported() {
        let b = parse("pending code_quality enabled\n").expect("parses");
        assert!(arrived(&b, &BTreeMap::new()).is_empty());
        let actual = BTreeMap::from([("code_quality".to_owned(), "disabled".to_owned())]);
        assert_eq!(arrived(&b, &actual), ["code_quality"]);
    }

    #[test]
    fn a_file_directive_pins_a_path_to_a_blob() {
        let b = parse("file clippy.toml abc123\n").expect("parses");
        assert_eq!(b.files[0].path, "clippy.toml");
        assert_eq!(b.files[0].blob, "abc123");
    }

    /// Not every shared file belongs in every repository: clippy.toml means
    /// nothing without Rust. A trailing repository list narrows the entry, and
    /// no list means all of them.
    #[test]
    fn a_file_can_be_scoped_to_named_repositories() {
        let b = parse("file clippy.toml abc123 alpha beta\n").expect("parses");
        assert_eq!(b.files[0].scope, ["alpha", "beta"]);
        assert!(
            parse("file .editorconfig abc\n").expect("parses").files[0]
                .scope
                .is_empty()
        );
    }

    #[test]
    fn a_file_outside_its_scope_is_not_drift() {
        let b = parse("file clippy.toml abc123 alpha\n").expect("parses");
        assert!(drift_files(&b, "beta", &BTreeMap::new()).is_empty());
        assert_eq!(drift_files(&b, "alpha", &BTreeMap::new()).len(), 1);
    }

    #[test]
    fn a_file_with_different_content_is_drift() {
        let b = parse("file clippy.toml abc123\n").expect("parses");
        let actual = BTreeMap::from([("clippy.toml".to_owned(), "def456".to_owned())]);
        let d = drift_files(&b, "alpha", &actual);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].actual, "def456");
    }

    /// A repository simply missing the file is drift too, and a different
    /// failure from one that holds the wrong version.
    #[test]
    fn a_missing_file_is_drift() {
        let b = parse("file clippy.toml abc123\n").expect("parses");
        let d = drift_files(&b, "alpha", &BTreeMap::new());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].actual, "<missing>");
    }

    #[test]
    fn a_rule_directive_names_a_type_and_where_it_must_come_from() {
        let b = parse("rule deletion Organization\n").expect("parses");
        assert_eq!(b.rules.len(), 1);
        assert_eq!(b.rules[0].0, "deletion");
        assert_eq!(b.rules[0].1, "Organization");
    }

    #[test]
    fn a_rule_not_in_effect_is_drift() {
        let b = parse("rule deletion Organization\n").expect("parses");
        let d = drift_rules(&b, &BTreeMap::new());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].actual, "<not in effect>");
    }

    /// A rule in effect from the wrong place is drift: a repository-level copy
    /// of the floor can be deleted by whoever owns that repository, which is
    /// the whole reason the floor lives on the organisation.
    /// A rule can be in effect from the organisation *and* from the
    /// repository at once. Collapsing that to one value loses the organisation
    /// entry and reports drift on a fleet that is correct -- which is what the
    /// first version of this did.
    #[test]
    fn a_rule_in_effect_from_several_sources_satisfies_any_of_them() {
        let b = parse("rule deletion Organization\n").expect("parses");
        let actual =
            BTreeMap::from([("deletion".to_owned(), "Organization,Repository".to_owned())]);
        assert!(drift_rules(&b, &actual).is_empty());
    }

    #[test]
    fn a_rule_from_the_wrong_source_is_drift() {
        let b = parse("rule deletion Organization\n").expect("parses");
        let actual = BTreeMap::from([("deletion".to_owned(), "Repository".to_owned())]);
        let d = drift_rules(&b, &actual);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].actual, "Repository");
    }

    #[test]
    fn a_baseline_lists_its_repositories_and_settings() {
        let b = parse("repo alpha\nrepo beta\nsetting has_wiki false\n").expect("parses");
        assert_eq!(b.repos, ["alpha", "beta"]);
        assert_eq!(b.settings, [("has_wiki".to_owned(), "false".to_owned())]);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let b =
            parse("# a note\n\nrepo alpha\n\n# another\nsetting has_wiki false\n").expect("parses");
        assert_eq!(b.repos, ["alpha"]);
        assert_eq!(b.settings.len(), 1);
    }

    /// A typo that silently configures nothing is worse than one that stops.
    #[test]
    fn an_unknown_directive_is_refused() {
        let err = parse("repoo alpha\n").expect_err("must refuse");
        assert!(err.contains("repoo"), "the message should name it: {err}");
    }

    #[test]
    fn a_setting_needs_a_key_and_a_value() {
        assert!(parse("setting has_wiki\n").is_err());
    }

    #[test]
    fn matching_settings_are_not_drift() {
        let b = parse("repo alpha\nsetting has_wiki false\n").expect("parses");
        let actual = BTreeMap::from([("has_wiki".to_owned(), "false".to_owned())]);
        assert!(drift(&b, &actual).is_empty());
    }

    #[test]
    fn a_differing_setting_is_reported_with_both_values() {
        let b = parse("repo alpha\nsetting has_wiki false\n").expect("parses");
        let actual = BTreeMap::from([("has_wiki".to_owned(), "true".to_owned())]);
        let d = drift(&b, &actual);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].key, "has_wiki");
        assert_eq!(d[0].actual, "true");
        assert_eq!(d[0].desired, "false");
    }

    /// A setting the reading does not mention is drift, not a pass. Treating a
    /// missing value as satisfied is how an audit reports a clean fleet it
    /// never actually looked at.
    #[test]
    fn a_setting_absent_from_the_reading_is_drift() {
        let b = parse("repo alpha\nsetting has_wiki false\n").expect("parses");
        let d = drift(&b, &BTreeMap::new());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].actual, "<unreadable>");
    }
}
