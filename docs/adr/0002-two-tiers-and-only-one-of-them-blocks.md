# ADR-0002: Two tiers, and only one of them blocks

- Status: accepted
- Date: 2026-08-31

## Context

Some checks are worth waiting for on every push. Others are worth running, but
not worth blocking a merge on: a matrix across three operating systems, a
minimum-version build, a supply-chain score. Putting them all in the same
place forces a bad trade. Either the expensive checks block every merge, or
they are dropped and the coverage is lost.

A team that waits twenty-five minutes for a one-line documentation fix learns
to bypass its own gates. That is worse than not having the check, because the
bypass habit applies to the checks that mattered too.

## Decision

Two tiers.

**Fast** runs on every pull request and every push to a protected branch, and
every one of its checks is a required context. It is the contract: nothing
lands on the default branch without these passing.

**Heavy** runs weekly and on demand, and **no heavy check is ever a required
context**. It is evidence, not a gate. What matters for it is freshness -- a
heavy result older than about a week is not evidence of anything.

The split is by cost and by blast radius, not by importance. `cross-platform`
is in heavy despite being the only thing that compiles the `cfg(not(unix))`
branches at all, because a Windows runner takes minutes and a compilation
error there does not endanger the default branch the way a failing test does.

## Consequences

The heavy tier earned itself on its first run: `maestro-pi-config` carried two
`#[cfg(not(unix))]` functions that no gate had ever compiled, because every
check until then ran on Linux. They were correct, but nothing had established
that.

Nothing currently enforces heavy freshness. A heavy run that has been failing
for a month is as invisible as one that never ran, and the tier is not a
required context precisely so that nobody is forced to look. Enforcing the
freshness window belongs in the fleet audit, and is not yet built. Stated here
so the gap is a decision rather than an oversight.
