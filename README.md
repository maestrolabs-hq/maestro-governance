# maestro-governance

What the repositories should look like, and what they actually do.

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

Three kinds of line, and the difference matters:

- `setting` -- a repository setting. Read, compared, and **written** by `apply`.
- `org` -- an organisation-wide setting. Read and compared; most are written by
  hand because the API refuses them.
- `present` -- an object whose body this format does not own: a ruleset, a
  security configuration. Asserted to exist and to be in a given state.

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
