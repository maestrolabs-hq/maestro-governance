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

### 4. `required_status_checks` is enforced by hand and audited by nothing

`baseline.txt` asserts three rule types: `deletion`, `non_fast_forward`,
`pull_request`. ADR-0004 is explicit that required contexts cannot live at org
level and must be set per repository -- and then nothing checks that they were.

The only merge-blocking control in the estate is the one control the audit
cannot see. `read_rules` already returns every rule type in effect, so the data
is fetched and discarded.

**Fix:** add `rule required_status_checks` lines naming the seven fast-tier
contexts, per repo.

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

## P3

### 10. `.pre-commit-config.yaml` is the documented entrypoint and is untracked

`CONTRIBUTING.md` tells a contributor to run the hooks. The one file that
defines them drifted between repos twice this week -- both times caught by the
baseline only because it was tracked. It is tracked now; keep it that way and
add a test.
