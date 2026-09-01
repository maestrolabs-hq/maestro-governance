# TODO

Findings from an adversarial review of the estate, ordered by what they cost if
left alone. Three independent reviewers read the source; two of them found P0-2
without seeing each other's work.

Severity means: **P0** the tool is lying about its own results. **P1** a control
that is claimed but absent. **P2** correctness or cost that bites at scale.
**P3** worth doing, harmless to defer.

---

## Done

The two P0s and two P1s below were fixed in the same pass that found
them. They are kept here because the failure is more instructive than the
fix.

---

## ~~P0~~ FIXED -- the tool reported success while doing nothing

### 1. `plan` exits 0 on drift, so the weekly audit can never fire

`src/main.rs:116`

```rust
if verb == "plan" {
    return ExitCode::SUCCESS;
}
```

`fleet-audit.yml` derives its only signal from that exit code:

```bash
echo "drifted=$([ "$status" -eq 0 ] && echo false || echo true)" >> "$GITHUB_OUTPUT"
```

`status` is 0 whenever `gh` itself succeeded, drift or not. `drifted` is
therefore always `false`. The tracking issue is never opened, and the
`elif [ -n "$existing" ]` branch actively **closes** a real drift issue while
the fleet is still broken. The report is written to the summary of a green
scheduled run that nobody opens.

`docs/fleet-audit.md` names this exact failure as the one worse than not
running at all: *"the audit would read nothing, find nothing, and report a
clean fleet."* We have it from the other direction.

**Fix:** return `FAILURE` from the `plan` path when `total > 0`. Leave `apply`
as it is. Add a test on the exit code -- nothing covers `main` today.

### 2. `apply` PATCHes file hashes and rule types as repository settings

`src/main.rs:22-25`, `src/github.rs:34-36`

`gather` flattens three kinds of drift into one vector:

```rust
let mut d = baseline::drift(desired, &github::read_settings(repo)?);
d.extend(baseline::drift_rules(desired, &github::read_rules(repo)?));
d.extend(baseline::drift_files(desired, repo, &github::read_files(repo)?));
```

`apply_settings` sends every element to the repo-settings endpoint:

```rust
cmd.args(["api", "-X", "PATCH", &format!("repos/{}/{repo}", org())]);
for d in drift {
    cmd.args(["-F", &format!("{}={}", d.key, d.desired)]);
}
```

So a drifted `clippy.toml` becomes `-F clippy.toml=5f1fb12f...`, and a missing
org rule becomes `-F deletion=Organization`. GitHub ignores unknown fields and
returns 200, so we print `updated {repo}` and `Apply complete. N setting(s)
changed.` having changed nothing. The count is wrong too: `total` includes rule
and file drift that apply cannot touch.

Worse, the bogus fields ride in the **same** PATCH as the legitimate ones, so a
422 abandons the real settings for that repo as well.

The README claims the opposite guarantee: *"a tool that reports success while
leaving half the drift in place is worse than one that refuses."* The org-drift
guard implements that refusal. `file` and `rule` lines walk straight past it.

**Fix:** make `Gathered` carry the three kinds separately. Pass only settings
drift to `apply_settings`. Refuse on file/rule drift the way org drift already
refuses.

---

## P1 -- documented controls that do not exist

### 3. ~~The documented remediation command is not a recipe~~ FIXED

`README.md:31` says:

```text
just apply --auto-approve  make it so
```

`just --list` offers: `check`, `coverage`, `doctor`, `fmt`, `install`, `setup`.
There is no `plan` recipe and no `apply` recipe. The first thing a reader tries
fails.

**Fix:** add both recipes, or change the README to `cargo run -- plan`.

### 4. `required_status_checks` is enforced by hand and audited almost not at all

`baseline.txt` asserts three rule types: `deletion`, `non_fast_forward`,
`pull_request`. ADR-0004 is explicit that required contexts cannot live at org
level and must be set per repository -- and then nothing checks that they were.

The only merge-blocking control in the estate is the one control the audit
cannot see. `read_rules` already returns every rule type in effect, so the data
is fetched and discarded.

Partially addressed: `baseline.txt:64-74` now carries `rule
required_status_checks Repository`, which asserts the rule is *in effect* and
says so explicitly -- it does not assert which contexts it lists, so a check
removed from the list still passes. The fast tier produces eleven contexts
today and fourteen once the open pull requests merge, not the seven this entry
originally named.

**Fix:** a directive that carries a value, so the audit can assert the list
rather than the rule's existence.

### 4a. Four fast-tier gates run on every pull request and block nothing

The fast tier produces eleven contexts. The ruleset requires seven, identically
on all three Rust repositories:

```text
$ gh api repos/maestrolabs-hq/maestro-governance/rules/branches/main \
    --jq '.[]|select(.type=="required_status_checks")|.parameters.required_status_checks[].context'
fast / rust-format      common / secrets-scan
fast / rust-lint        common / actions-security
fast / rust-test        common / dependency-review
fast / rust-audit
```

`common / prose`, `common / brief`, `common / markdown` and `common / toml`
run, report, go red, and merge anyway.

The prose says otherwise in four places:

- `docs/adr/0001-how-the-ci-gates-are-shaped.md:44` -- "every check in it is a
  required context. It is the contract"
- `docs/adr/0001-how-the-ci-gates-are-shaped.md:111` -- "Fast tier -- each row
  is a required context."
- `docs/adr/0004-what-the-platform-enforces.md:51` -- "naming exactly the
  fast-tier contexts"
- `docs/adr/0005-every-file-says-what-it-is-for.md:25` -- "Enforced in the
  shared fast tier"

ADR-0005's whole premise -- a ratchet installed while the cost is zero -- is
unenforced.

**Fix:** add the four contexts to each repository's ruleset, or amend the four
sentences to say which contexts are required and why the rest are advisory.

### 4b. The open pull requests document enforcement they do not wire

PR #31 adds to ADR-0001:

> So it is a fast-tier **required context**, per operating system:
> `fast / cross-platform (windows-latest)` / `(macos-latest)`

and to `AGENTS.md:90` -- "Windows, macOS and Linux, **on every pull request**".
No pull request in the set touches a ruleset. On merge, `cross-platform` moves
out of heavy (weekly, not required) into fast (per-PR, still not required), and
`no-absolute-paths` arrives advisory. Observable enforcement gain: zero, while
three documents assert a blocking contract.

This is the identical defect the same commit message names.

**Fix:** add `fast / cross-platform (windows-latest)`, `fast / cross-platform
(macos-latest)` and `common / no-absolute-paths` to the ruleset in the same
change that merges the set -- or do not merge the documentation claiming them.

### 4c. `.github` has no required status checks and no repository ruleset

```text
$ gh api repos/maestrolabs-hq/.github/rules/branches/main --jq '.[].ruleset_source_type'
Organization   (x5, and nothing else)
```

Only the organisation floor applies, and its `pull_request` rule carries
`required_approving_review_count: 0`. A pull request with every check red
merges on one click -- into the repository whose `main` all four repositories
execute on every push, including `rust-release.yml` with `contents: write`,
`id-token: write` and `attestations: write`.

Its own `ci.yml` header says: *"The one repository whose compromise reaches all
the others was the only one with no gate."* It has CI now; it still has no gate.

**Fix:** create a repository ruleset on `.github` `main` requiring the eight
contexts it produces.

### 5. ~~`present` is documented and rejected~~ FIXED

`README.md:44` documents a `present` directive. `src/baseline.rs` accepts
`repo`, `setting`, `org`, `rule`, `pending`, `file` -- and errors on anything
else. Following the README produces a parse failure.

**Fix:** delete the paragraph, or implement it.

---

## P2 -- breaks on contact with growth

### 6. The first release breaks the audit in three ways

`baseline.txt` pins `CHANGELOG.md` and the release-please manifest by blob
hash. release-please rewrites both on the first release. The audit will report
drift against files that are correct, every run, until someone re-pins by hand.

**Fix:** stop tracking generated files. Track `.github/release-please/config.json`
(hand-written) and not the manifest or the changelog.

### 7. Onboarding repo #5 costs ten hand edits and fifteen byte-exact copies

Each new repo needs its name appended to ten scoped lines in `baseline.txt` and
fifteen tracked files copied byte-for-byte. At 25 repos this is the dominant
cost of the whole system, and every step is a chance to typo a repo name into a
line that then silently audits nothing.

**Fix:** invert the default -- track for all repos, list exceptions. Then add
`governance adopt <repo>`.

### 8. `read_org` runs twice per invocation

`src/main.rs:20` and `:78`. Two identical API calls, doubling the rate-limit
cost of the org read for no gain.

### 9. ADR-0002 describes a shared-hook mechanism no repository uses

`docs/adr/0002-*.md:23-27` and `baseline.txt:85` both describe hooks coming
from the `.github` repo. Every repo has its own `.pre-commit-config.yaml`.

**Fix:** make the ADR describe what is true, or make it true.

---

### 9a. `just install` omits a tool `just check` hard-requires

`justfile:19` binstalls `prek cargo-deny cargo-machete cargo-llvm-cov`.
`tests/duplication.rs:82` requires `similarity-rs` and panics by design when it
is absent -- *"A gate that skips when its tool is missing reports green while
looking at nothing."* `just check` runs `cargo test --all-targets`, which
includes it. On a clean machine the documented setup followed by the documented
check panics.

This also breaks the `wsl-toolchain` job in the open dot-github pull request,
which runs `just setup && just check` without `just install` -- so it fails
first on `prek`, then on `similarity-rs`. The one job added to prove the
toolchain works cannot pass.

**Fix:** add `similarity-rs` to the binstall list; add `just install` before
`just setup` in `wsl-toolchain`.

### 9b. The hook labelled "similarity-rs (copy-paste)" runs the module-size test

`.pre-commit-config.yaml` names a copy-paste hook whose entry invokes the
module-size test instead. The copy-paste gate does not run locally at all,
under a label saying it does.

**Fix:** point the entry at the duplication test.

### 9c. Spliced doc comments leave a public function undocumented

`src/github.rs:129` is cut mid-clause and `src/github.rs:150` carries the
orphaned tail as the whole doc for the wrong function. `src/baseline.rs:138`
has the same splice, and `pub fn drift_rules` at `:159` has no doc comment at
all. Introduced by `65303bd` and unnoticed since.

ADR-0005 makes "every file says what it is for" a decision with a gate -- but
the gate checks the file-level `//!` only, so item-level rot passes.

**Fix:** restore the four doc comments to the functions they describe.

### 9d. `baseline.txt` watches three of the five organisation rules in effect

`commit_message_pattern` and `repository_visibility` are in effect and
unwatched. Both are floor controls the estate relies on.

**Fix:** add both.

---

## P3

### 10. `.pre-commit-config.yaml` is the documented entrypoint and is untracked

`CONTRIBUTING.md` tells a contributor to run the hooks. The one file that
defines them drifted between repos twice this week -- both times caught by the
baseline only because it was tracked. It is tracked now; keep it that way and
add a test.

---

## Round 2 -- the audit auditing itself

Ten agents swept the estate: six lenses across all repositories, four deep
per-repository passes. Every finding below was reproduced by execution, not
inferred. The theme is uncomfortable and consistent: **the controls that report
on other controls are the least tested things in the estate.**

### 11. The weekly audit reports sixteen false drifts and cannot see the one real one

Issue #30, opened by the scheduled run at `2026-08-31T15:00:56Z`, still open:

```text
governance found the following drift:
  maestro-core
    ~ allow_squash_merge : null -> true
    ~ allow_merge_commit : null -> false
    ~ allow_rebase_merge : null -> false
    ~ delete_branch_on_merge : null -> true
  ... identically for maestro-pi-config, maestro-governance, .github
Plan: 16 setting(s) to change.
```

The same command, same fleet, run locally with an admin token:

```text
$ cargo run --quiet -- plan
  .github
    ~ required_status_checks : <not in effect> -> Repository
Plan: 1 setting(s) to change.
```

Cause: `GET /repos/{o}/{r}` returns the four merge-policy fields only to a
caller with push or admin access. `docs/fleet-audit.md:30` prescribes
`Metadata: read, Administration: read`, which does not include them. Confirmed
independently against a repository where the reader has read-only access -- the
four withheld fields come back `null`, and the five the audit reports as clean
are exactly the five the API returns to anyone.

`src/github.rs:100` builds the projection as `format!("{k}=\\(.{k})")`. jq
renders an absent field as the literal string `null`, so the key arrives
*present* with the value `"null"`, and `src/baseline.rs:195` compares
`"null" != "true"` and calls it drift.

**Why it matters:** the estate's only automated drift signal has a 100%
false-positive rate on settings, weekly, forever. A real drift arrives
indistinguishable from sixteen standing lies. `just apply --auto-approve`
against this plan would PATCH sixteen already-correct fields and report
`16 setting(s) changed` -- the exact deception P0 #2 was written to remove,
re-entering by a different door.

**Fix:** emit the projection with `has()` or `to_entries` so an absent key stays
absent and falls through to `<unreadable>`; then either widen the token or move
those four keys to a directive the token can observe, and correct
`docs/fleet-audit.md:30` either way.

### 12. `<unreadable>` is unreachable, and the test guarding it can never fail

Consequence of #11 and worth its own entry. `compare()` substitutes
`UNREADABLE` only when `actual.get(key)` returns `None`, which the jq
projection never produces. The guard test
`a_setting_absent_from_the_reading_is_drift` (`src/baseline.rs:391`) builds
`BTreeMap::new()` by hand -- a state `read_settings` cannot generate -- and its
own docstring names the stake: *"Treating a missing value as satisfied is how
an audit reports a clean fleet it never actually looked at."*

Today it errs loud, because no desired value is the string `null`. A directive
whose desired value is legitimately absent would erase the distinction
silently.

**Fix:** test through `read_settings`' jq rather than a literal map.

### 13. A token failure closes the drift issue and comments that the fleet is clean

`.github/workflows/fleet-audit.yml:44-58` writes `drifted` only on success;
`:73-92` runs `if: always()` and treats any non-`true` value as clean. Replayed
with the file's verbatim shell:

```text
DRIFTED='true'   -> gh issue comment 42 --body ...
DRIFTED='false'  -> gh issue close 42 --comment The fleet matches the baseline again.
DRIFTED=''       -> gh issue close 42 --comment The fleet matches the baseline again.
```

`DRIFTED` is empty whenever the audit step fails or is skipped, and both have
happened -- runs `33352344273` and `33351919461` both show `Audit the fleet ->
failure` followed by `Open or close the tracking issue -> success`.

The missing-token guard at `:35-42` is defeated by the same path: it fails the
job, the audit step is skipped, `drifted` is empty, and the issue is closed.
The file's own comment says a tokenless audit *"would see nothing and report a
clean fleet, which is worse than not running. So it stops here instead."* It
does not stop -- it retracts a real finding.

**Fix:** guard the close branch on a positive signal:
`if: steps.audit.outcome == 'success'`, and `[ "$DRIFTED" = "false" ]` rather
than an `elif` on the absence of a negative.

### 14. A baseline that watches nothing reports a clean estate, and `offboard` gets there unapproved

```text
=== repo lines removed, everything else intact ===
No drift. 0 repositories match the baseline.
exit=0
=== still declared in that "clean" baseline ===
setting 9   rule 4   file 6   pending 2   org 6   repo 0
```

`src/main.rs:98-105` prints success when `total` is zero; nothing requires the
baseline to name a repository. `governance offboard` (`:195-241`) writes
`baseline.txt` immediately, with no `--auto-approve` -- unlike `apply`
(`:143-147`).

Twenty-one declared controls and four repositories become unwatched while the
arbiter prints success and exits 0, which the fleet audit reads as clean and
uses to close the tracking issue.

**Fix:** refuse a baseline whose repository list is empty while settings, rules
or files are not; print the count of repositories checked beside the verdict;
give `offboard` the approval gate `apply` has.

### 15. Nothing ever executes the `governance` binary, and every safety guarantee survives deletion

`grep -rn "CARGO_BIN_EXE\|Command::new" tests/` returns one hit, and it is
`similarity-rs`. Three mutations, each verified to have changed the built
artifact, against the full suite:

```text
[G1 plan exits 0 with drift -- the fleet-audit signal]  SURVIVED
[G2 apply needs no approval]                            SURVIVED
[G3 a key dropped from READABLE]                        SURVIVED
[CONTROL compare never reports drift]                   killed
```

G1 changes the exact line whose comment reads *"Non-zero because the fleet
audit derives its only signal from this."* All 242 lines of `main.rs` and 177
of `github.rs` are uncovered: the exit code, the `--auto-approve` refusal, the
org-drift write refusal, the unwritable-kind refusal, the jq expression.

**Fix:** integration tests through `CARGO_BIN_EXE_governance`, starting with
the exit code and the approval refusal.

### 16. The parser half-parses four of its six directives

Trailing tokens are silently dropped on every directive except `file`. A
misspelled repository name in a `file` scope silently unenforces that file for
every repository. A duplicated setting key makes `apply` oscillate forever.
`repo` misreports its own count.

The parser's doc comment forbids exactly this.

**Fix:** reject unknown trailing tokens; reject a duplicate key; validate scope
names against the repository list.

### 17. Smaller, verified

- `offboard` reports `tracked files 0` for three of the four repositories.
- Nothing reconciles the baseline's repository list against the organisation's,
  so a repository added to the org is never audited and nothing says so.
- The `pending` mechanism can never fire, and one pending control is readable
  *and* currently wrong.
- `security_and_analysis` is readable on an endpoint the tool already calls and
  is declared nowhere.
- `src/github.rs` has no tests at all, and both previously-fixed bugs lived
  there.
- ADR-0001 claims the fleet audit notices a stale required context;
  `baseline.txt:70` says it cannot.
- The binary hard-codes the build machine's home directory.
- `read_org` is fetched twice per run.
- The README's worked `offboard` example prints a number the tool no longer
  prints.
