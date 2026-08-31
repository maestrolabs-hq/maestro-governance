# ADR-0001: Checks are named, one per concern

- Status: accepted
- Date: 2026-08-31

## Context

The first CI ran every gate in one job called `quality`: formatting, lint,
tests, unused dependencies, advisories, copy-paste detection. It was quick to
write and it worked, in the sense that a violation turned the build red.

Two things it could not do:

**Say what broke.** A required context named `quality` tells a reader that
something failed, and nothing else. The failure has to be opened and read
before anyone knows whether the branch has a formatting slip or a licence
violation.

**Be adopted gradually.** Turning on a new check means adding it to the fused
job, where it immediately blocks every pull request in every repository. There
is no way to introduce one check, watch it across a few merges, and then
require it. The choice is all at once or not at all.

Both matter more as the number of repositories grows, because the cost of a
confusing red check is paid by whoever did not write it.

## Decision

Each check is its own job, and its job name is the required context.

```text
fast / rust-format     fast / rust-audit        common / secrets-scan
fast / rust-lint       ts-format ... ts-audit   common / actions-security
fast / rust-test                                common / dependency-review
```

Jobs run in parallel, so the wall clock is the slowest check rather than the
sum. The cost is repeated checkout and toolchain setup per job, which is
seconds, against a failure that names itself.

Checks that do not depend on the language live in `common-*`. They were inside
the Rust workflow, which was fine while Rust was the only language; a second
language would have meant a second copy of the secret scan. They moved before
that happened rather than after.

## Consequences

Renaming a job renames a required context, and a ruleset that still names the
old one blocks every pull request until it is updated. This has already
happened twice during the split. The ruleset and the workflow have to change
together, and the fleet audit is what notices when they have not.

A repository adopting a new check adds one job and requires one context, on
its own schedule. That is the property the fused job did not have.
