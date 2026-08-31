//! Drift is not one thing, and the endpoint that fixes one kind cannot fix
//! another.
//!
//! Three kinds are read: repository settings, branch rules, and tracked file
//! content. Only the first can be written by the repository settings endpoint.
//! When all three were carried in one vector, apply sent every one of them to
//! that endpoint -- so a drifted `clippy.toml` became
//! `-F clippy.toml=<blob hash>`. GitHub ignores unknown fields and returns 200,
//! so the tool printed `updated` and `Apply complete` having changed nothing,
//! and reported the same drift on the next run for as long as it went unread.
//!
//! These tests keep the kinds apart at the type level, so that vector cannot be
//! assembled again.

use governance::baseline::{self, Kind};

const SAMPLE: &str = "\
repo maestro-core

setting delete_branch_on_merge true
setting allow_merge_commit false

rule pull_request Organization

file clippy.toml 5f1fb12f00000000000000000000000000000000
";

fn desired() -> baseline::Baseline {
    baseline::parse(SAMPLE).expect("the sample baseline should parse")
}

#[test]
fn a_settings_drift_is_marked_as_one() {
    let actual = [("delete_branch_on_merge".to_owned(), "false".to_owned())]
        .into_iter()
        .collect();
    let found = baseline::drift(&desired(), &actual);
    assert!(
        found.iter().any(|d| d.key == "delete_branch_on_merge"),
        "the differing setting should be reported"
    );
    assert!(
        found.iter().all(|d| d.kind == Kind::Setting),
        "everything from drift() is a repository setting: {found:?}"
    );
}

/// Neither a branch rule nor a tracked file is a repository setting, and
/// sending either as one is accepted, ignored, and returns 200.
#[test]
fn what_the_settings_endpoint_cannot_fix_is_not_marked_as_a_setting() {
    let file_drift = baseline::drift_files(
        &desired(),
        "maestro-core",
        &[("clippy.toml".to_owned(), "0000000".to_owned())]
            .into_iter()
            .collect(),
    );
    let rule_drift = baseline::drift_rules(&desired(), &std::collections::BTreeMap::new());

    for (what, found, expected) in [
        ("a tracked file", file_drift, Kind::File),
        ("a branch rule", rule_drift, Kind::Rule),
    ] {
        assert_eq!(found.len(), 1, "{what} differs, so it should be reported");
        assert_eq!(
            found[0].kind, expected,
            "{what} is not a repository setting"
        );
    }
}

#[test]
fn only_settings_reach_the_settings_endpoint() {
    // One setting differs, the other matches, so exactly one settings drift.
    let mut all = baseline::drift(
        &desired(),
        &[
            ("delete_branch_on_merge".to_owned(), "false".to_owned()),
            ("allow_merge_commit".to_owned(), "false".to_owned()),
        ]
        .into_iter()
        .collect(),
    );
    all.extend(baseline::drift_rules(
        &desired(),
        &std::collections::BTreeMap::new(),
    ));
    all.extend(baseline::drift_files(
        &desired(),
        "maestro-core",
        &[("clippy.toml".to_owned(), "0000000".to_owned())]
            .into_iter()
            .collect(),
    ));
    assert_eq!(all.len(), 3, "one of each kind");

    let fields = baseline::settings_fields(&all);
    assert_eq!(
        fields,
        vec!["delete_branch_on_merge=true".to_owned()],
        "the rule and the file must not be sent as repository settings"
    );
}

#[test]
fn a_vector_carrying_more_than_settings_is_not_writable() {
    let all = baseline::drift_files(
        &desired(),
        "maestro-core",
        &[("clippy.toml".to_owned(), "0000000".to_owned())]
            .into_iter()
            .collect(),
    );
    assert!(
        baseline::settings_fields(&all).is_empty(),
        "file drift alone produces nothing to write, so apply must refuse \
         rather than PATCH an empty body and call it success"
    );
}
