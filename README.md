<div align="center">

# Maestro-Governance

**What the repositories should look like, and the drift from it**

One baseline for the organisation. Audited weekly, reported never corrected.

  <a href="https://github.com/maestrolabs-hq/maestro-governance/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/maestrolabs-hq/maestro-governance/ci.yml?branch=main&style=for-the-badge&label=CI&labelColor=1c1c1c&color=2ea043"></a>
  <a href="https://github.com/maestrolabs-hq/maestro-governance/actions/workflows/heavy.yml"><img alt="Heavy" src="https://img.shields.io/github/actions/workflow/status/maestrolabs-hq/maestro-governance/heavy.yml?branch=main&style=for-the-badge&label=Heavy&labelColor=1c1c1c&color=8957e5"></a>
  <a href="https://scorecard.dev/viewer/?uri=github.com/maestrolabs-hq/maestro-governance"><img alt="OpenSSF Scorecard" src="https://img.shields.io/ossf-scorecard/github.com/maestrolabs-hq/maestro-governance?style=for-the-badge&label=Scorecard&labelColor=1c1c1c"></a>
  <a href="https://github.com/maestrolabs-hq/maestro-governance/blob/main/LICENSE"><img alt="License" src="https://img.shields.io/badge/License-MIT-1c1c1c?style=for-the-badge&labelColor=1c1c1c&color=0969da"></a>

  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.98-CE422B?style=flat-square&logo=rust&logoColor=white">

</div>

---

## Why it exists

Two repositories were configured by hand, from the same JSON pasted twice.
Nothing recorded what the settings were meant to be, and nothing would have
noticed if one drifted from the other. That is the hole this fills: the
baseline is the only place a repository setting is decided.

## Use

```text
just plan                  what would change on the organisation
just apply --auto-approve  make it so
```

`baseline.txt` is the desired state -- one directive per line, readable
without a parser. Changing a repository means changing that file.

## What it covers

Six directives, and the difference matters, because only one of them is
something `apply` can write:

- `repo` -- a repository the baseline covers.
- `setting` -- a repository setting. Read, compared, and **written** by `apply`.
- `org` -- an organisation-wide setting. Read and compared, never written: most
  are refused by the API, and the rest are too wide to change unattended.
- `rule` -- a branch rule that must be in effect, and where it comes from.
  Read and compared. It lives in a ruleset, so `apply` cannot write it.
- `file` -- a tracked file, pinned by git blob hash, optionally scoped to named
  repositories. Read and compared. It is fixed by a commit, not by an API call.
- `pending` -- a control with no readable API yet, recorded so the audit prints
  a promotion note the day the key becomes readable.

`apply` writes `setting` lines and refuses outright if any other kind drifted.
Sending a blob hash or a rule name to the repository settings endpoint is
accepted, ignored, and returns 200 -- which is exactly how a tool comes to
report success while changing nothing.

## What it does not do

It does not write organisation controls, and `apply` **fails** rather than
skipping them quietly -- a tool that reports success while leaving half the
drift in place is worse than one that refuses.

It does not own the body of a ruleset. Expressing that nested JSON here would
mean reimplementing GitHub's schema and getting it subtly wrong; asserting
presence costs nothing and still turns a deleted ruleset into a failure
instead of a silence.

`two_factor_requirement_enabled` is checked and never applied: `PATCH /orgs`
accepts the field and silently leaves it unchanged, so it is a web-interface
setting that this tool can only report on.

## Design

GitHub is reached through the `gh` CLI: it already holds the credentials and
the API surface, so a crate would add a second authentication path for no gain.
The dependency list is empty.

`plan` and `apply` follow `terraform`, and match `pi-config` in the sibling
repository: a plan reports only real differences, and an apply refuses to act
without `--auto-approve`.

## Decisions

| ADR | What it settles |
| --- | --- |
| [0001](docs/adr/0001-how-the-ci-gates-are-shaped.md) | Every gate, per language, and why each tool is there |
| [0002](docs/adr/0002-local-hooks-mirror-the-fast-tier.md) | What runs on commit, what waits for push, and why |
| [0003](docs/adr/0003-shared-where-possible-tracked-where-not.md) | What is centralised, what cannot be, and how the rest is tracked |
| [0004](docs/adr/0004-what-the-platform-enforces.md) | Rulesets, organisation parameters and the security configuration |
