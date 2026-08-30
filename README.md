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

## What it does not cover yet

The branch rulesets. They are applied and enforced, but by hand, which is the
same hole one level up. `baseline.txt` says so rather than implying a coverage
it does not have.

## Design

GitHub is reached through the `gh` CLI: it already holds the credentials and
the API surface, so a crate would add a second authentication path for no gain.
The dependency list is empty.

`plan` and `apply` follow `terraform`, and match `pi-config` in the sibling
repository: a plan reports only real differences, and an apply refuses to act
without `--auto-approve`.
