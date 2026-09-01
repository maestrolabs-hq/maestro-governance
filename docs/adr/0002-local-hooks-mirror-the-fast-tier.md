# ADR-0002: Local hooks mirror the fast tier, and nothing else

- Status: accepted
- Date: 2026-08-31

## Context

CI is the copy of the gates that cannot be skipped. Local hooks exist for a
different reason: to fail in two seconds rather than two minutes, at a desk
rather than in a pull request.

That only holds if they stay quick. A commit that takes a minute is a commit
people learn to make with `--no-verify`, and the habit does not stay confined
to slow checks -- it applies to the ones that mattered too. A local hook that
is routinely bypassed is worse than none, because the repository still looks
guarded.

## Decision

**Hooks run through prek**, the Rust implementation of the pre-commit
protocol. Same configuration format, no Python needed to lint Rust.

**Definitions live once.** `maestrolabs-hq/.github` publishes
`.pre-commit-hooks.yaml` with every hook for all three languages; a repository
references it by URL and pins a revision. What stays local is the revision and
any repository-specific argument.

**Stages are chosen by cost, not by importance:**

| `pre-commit` | `pre-push` |
| --- | --- |
| merge-conflict markers | tests |
| TOML, JSON, YAML syntax | copy-paste detection |
| end-of-file, trailing whitespace | unused dependencies |
| `gitleaks` | advisories, licences |
| format | type checking |
| lint | |

Formatting and lint touch only the files being committed. Everything that
compiles or runs a test tree waits for the push, which is the last moment
before the work leaves the machine and the first where the wait buys anything.

`gitleaks` is the exception that stays at commit time despite scanning
history: a secret is the one mistake that cannot be undone by a later commit.

**Hooks mirror the fast tier only.** Nothing from the heavy tier -- no
mutation run, no Scorecard, no WSL toolchain run -- ever executes locally.
Those exist to be slow somewhere that is not a person's terminal.

The fast tier gained one check a hook cannot mirror: `cross-platform` builds
on Windows and macOS, and a contributor's machine is one of them at most. The
mirror is a rule about cost, not a promise of completeness, and this is the
first place the two come apart -- the check is cheap, and still unrunnable
where the hook runs. A local pass therefore no longer implies a green pull
request, which is worth knowing before pushing.

## Consequences

A hook and its CI counterpart can drift apart, and then a green local run
means nothing. `just check` runs the same commands CI does rather than
equivalents, which narrows the gap but does not close it; only running the
same definitions in both places would, and CI deliberately does not depend on
prek being installed.

`pre-merge-commit` is installed alongside the other two, so a merge that
resolves conflicts by hand is checked like any other commit.
