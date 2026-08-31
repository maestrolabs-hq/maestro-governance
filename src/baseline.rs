//! The desired state, and the difference between it and a machine-readable
//! reading of what a repository actually has.
//!
//! Pure: nothing here reaches the network. `main` fetches the readings and
//! hands them in, so every rule below is exercised by a test that constructs
//! its input literally.

use std::collections::BTreeMap;

/// What the repositories should look like.
#[derive(Debug, Default)]
pub struct Baseline {
    pub repos: Vec<String>,
    pub settings: Vec<(String, String)>,
    pub org: Vec<(String, String)>,
    pub present: Vec<Expected>,
}

/// An object whose body this format cannot express -- a ruleset, a security
/// configuration -- asserted to exist and to be in a given state. Owning the
/// nested JSON here would mean reimplementing GitHub's schema; asserting
/// presence costs nothing and still turns a deleted ruleset into a failure
/// rather than a silence.
#[derive(Debug)]
pub struct Expected {
    pub kind: String,
    pub name: String,
    pub state: String,
}

/// One setting that does not match.
#[derive(Debug)]
pub struct Drift {
    pub key: String,
    pub desired: String,
    pub actual: String,
}

/// One directive per line, `<kind> <argument...>`, after `provision.txt` in
/// maestro-pi-config -- readable without a parser and diffable without a tool.
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
            (Some("present"), Some(kind), Some(name)) => {
                let Some(state) = parts.next() else {
                    return Err(format!("line {}: `present` needs a state", n + 1));
                };
                b.present.push(Expected {
                    kind: kind.to_owned(),
                    name: name.to_owned(),
                    state: state.to_owned(),
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

/// Organisation-wide settings. Same comparison as the repository one, over a
/// different reading.
pub fn drift_org(baseline: &Baseline, actual: &BTreeMap<String, String>) -> Vec<Drift> {
    compare(&baseline.org, actual, UNREADABLE)
}

/// Keyed `<kind>/<name>` so a ruleset and a configuration cannot collide.
pub fn drift_present(baseline: &Baseline, actual: &BTreeMap<String, String>) -> Vec<Drift> {
    let wanted: Vec<(String, String)> = baseline
        .present
        .iter()
        .map(|e| (format!("{}/{}", e.kind, e.name), e.state.clone()))
        .collect();
    // A different sentinel on purpose: a setting we could not read is not the
    // same failure as an object that is not there.
    compare(&wanted, actual, "<absent>")
}

/// A setting the reading does not carry counts as drift. Treating it as
/// satisfied is how an audit reports a clean fleet it never looked at.
pub fn drift(baseline: &Baseline, actual: &BTreeMap<String, String>) -> Vec<Drift> {
    compare(&baseline.settings, actual, UNREADABLE)
}

const UNREADABLE: &str = "<unreadable>";

fn compare(
    wanted: &[(String, String)],
    actual: &BTreeMap<String, String>,
    missing: &str,
) -> Vec<Drift> {
    wanted
        .iter()
        .filter_map(|(key, desired)| {
            let observed = actual.get(key).map_or(missing, String::as_str);
            (observed != desired).then(|| Drift {
                key: key.clone(),
                desired: desired.clone(),
                actual: observed.to_owned(),
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

    /// Rulesets and security configurations are nested JSON that this format
    /// cannot express, and pretending otherwise would be worse than not
    /// covering them. What it can do is assert they exist and are active, so
    /// a deleted ruleset is a failure rather than a silence.
    #[test]
    fn a_present_directive_asserts_existence_without_owning_the_body() {
        let b = parse("present ruleset floor-no-destruction active\n").expect("parses");
        assert_eq!(b.present.len(), 1);
        assert_eq!(b.present[0].kind, "ruleset");
        assert_eq!(b.present[0].name, "floor-no-destruction");
        assert_eq!(b.present[0].state, "active");
    }

    #[test]
    fn a_missing_expected_object_is_drift() {
        let b = parse("present ruleset ghost active\n").expect("parses");
        let d = drift_present(&b, &BTreeMap::new());
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].actual, "<absent>");
    }

    #[test]
    fn an_object_present_but_in_the_wrong_state_is_drift() {
        let b = parse("present ruleset floor active\n").expect("parses");
        let actual = BTreeMap::from([("ruleset/floor".to_owned(), "evaluate".to_owned())]);
        let d = drift_present(&b, &actual);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].actual, "evaluate");
        assert_eq!(d[0].desired, "active");
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
