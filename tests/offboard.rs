//! Taking a repository out of the baseline.
//!
//! Not a `destroy`. Terraform destroys what it built; this builds nothing -- it
//! only changes settings that already exist, and a setting cannot be un-set,
//! only set to something else. The branch rules are real objects that could be
//! deleted, but this tool did not create them and deliberately cannot see them:
//! listing rulesets needs organisation write, which would let the audit rewrite
//! what it audits.
//!
//! So the useful inverse is not "undo the controls", it is "stop watching this
//! repository". Nothing on GitHub changes. Only the baseline does.

use governance::baseline;

const SAMPLE: &str = "\
repo maestro-core
repo maestro-pi-config

setting delete_branch_on_merge true

rule pull_request Organization

file clippy.toml 5f1fb12f00000000000000000000000000000000
file deny.toml aaaa000000000000000000000000000000000000 maestro-core
file rust-toolchain.toml bbbb000000000000000000000000000000000000 maestro-core maestro-pi-config
";

#[test]
fn the_repo_is_no_longer_listed() {
    let out = baseline::offboard(SAMPLE, "maestro-core").expect("offboard");
    let after = baseline::parse(&out).expect("the result must still parse");
    assert_eq!(
        after.repos,
        vec!["maestro-pi-config".to_owned()],
        "the offboarded repo should be gone and the others untouched"
    );
}

/// A file scoped to only that repository has no reason to remain: nothing is
/// left for it to describe.
#[test]
fn a_file_scoped_only_to_that_repo_goes_with_it() {
    let out = baseline::offboard(SAMPLE, "maestro-core").expect("offboard");
    assert!(
        !out.contains("deny.toml"),
        "deny.toml was scoped to maestro-core alone:\n{out}"
    );
}

/// A file scoped to several repositories loses one name and keeps the rest.
/// Dropping the whole line would silently stop tracking a repository nobody
/// asked to offboard.
#[test]
fn a_file_shared_with_others_only_loses_this_repo() {
    let out = baseline::offboard(SAMPLE, "maestro-core").expect("offboard");
    let line = out
        .lines()
        .find(|l| l.contains("rust-toolchain.toml"))
        .expect("the shared file must survive");
    assert!(
        line.contains("maestro-pi-config"),
        "the other repo keeps it:\n{line}"
    );
    assert!(
        !line.contains("maestro-core"),
        "and this one does not:\n{line}"
    );
}

/// An unscoped file applies to every repository, so offboarding one is not a
/// reason to touch it.
#[test]
fn an_unscoped_file_is_left_alone() {
    let out = baseline::offboard(SAMPLE, "maestro-core").expect("offboard");
    assert!(
        out.contains("file clippy.toml 5f1fb12f"),
        "an unscoped file covers whatever repos remain:\n{out}"
    );
}

/// Settings and rules are org-wide statements, not per-repository ones.
#[test]
fn settings_and_rules_are_untouched() {
    let out = baseline::offboard(SAMPLE, "maestro-core").expect("offboard");
    assert!(out.contains("setting delete_branch_on_merge true"));
    assert!(out.contains("rule pull_request Organization"));
}

/// Offboarding something that was never managed is a mistake worth reporting,
/// not a silent success. A typo in a repository name would otherwise look like
/// it worked.
#[test]
fn offboarding_an_unmanaged_repo_is_refused() {
    let err = baseline::offboard(SAMPLE, "maestro-typo").expect_err("should refuse");
    assert!(
        err.contains("maestro-typo"),
        "the error should name what was not found: {err}"
    );
}
