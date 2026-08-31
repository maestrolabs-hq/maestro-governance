# ADR-0001: How the CI gates are shaped

- Status: accepted
- Date: 2026-08-31

## Context

Every repository in this organisation is gated the same way, in Rust, Python
and TypeScript. Three questions have to be answered once rather than per
repository: how checks are named, when they block, and how far a language's
checks may differ from another's.

## Decision

### One job per concern, and the job name is the context

A required context called `quality` says a repository is broken and nothing
more. The failure has to be opened and read before anyone knows whether it is
a formatting slip or a licence violation, and the cost of that is paid by
whoever did not write the check.

A fused job also cannot be adopted gradually. Adding a check to it blocks
every pull request in every repository at once, so there is no way to
introduce something, watch it across a few merges, and then require it.

Each check is therefore its own job:

```text
fast / rust-format    fast / py-arch      fast / ts-types
fast / rust-lint      fast / py-types     fast / ts-arch
...                   ...                 ...
common / secrets-scan   common / actions-security   common / dependency-review
```

Jobs run in parallel, so the wall clock is the slowest check rather than the
sum. Checks that do not depend on the language live in `common-*`, so a second
language does not mean a second copy of the secret scan.

### Two tiers, and only the fast one blocks

**Fast** runs on every pull request and every push to a protected branch, and
every check in it is a required context. It is the contract: nothing reaches
the default branch without these passing.

**Heavy** runs weekly and on demand, and nothing in it is ever required. It
holds the checks worth running but not worth waiting for — a matrix across
three operating systems, an oldest-supported-version build, a supply-chain
score, mutation testing.

The split is by cost, not by importance. A team that waits twenty-five minutes
for a one-line fix learns to bypass its own gates, and that habit then applies
to the checks that mattered. What matters for heavy is freshness: a result
older than about a week is not evidence of anything.

### Each language's checks come from its own toolchain

Copying one language's shape onto another produces checks that exist because
somewhere else needed them, and misses the ones this language needs.

Common to all three: format, lint, test, dependency hygiene, and at least two
independent advisory databases across the two tiers. Two sources, because the
point of a redundant pass is that they disagree before a vulnerability is
public in both.

Different, with a reason:

| Check | Rust | TypeScript | Python |
| --- | --- | --- | --- |
| types | in the compiler | `ts-types`, tsc | `py-types`, mypy and ty |
| architecture | Cargo, structurally | `ts-arch`, dependency-cruiser | `py-arch`, import-linter |
| coverage floor | measured only | gated, branch | gated, branch |
| oldest supported | `msrv` | `node-matrix` | `python-matrix` |
| mutation | `cargo-mutants` | Stryker | `mutmut` |

**Architecture** is a gate where the language allows any module to import any
other, because a layering rule is a sentence in a README until something fails
the build over it. Rust gets the property from Cargo: a crate cannot reference
a crate absent from its dependency list.

**Two type checkers in Python** because they disagree usefully — `ty` parses
past a syntax error where mypy stops at the first. A second opinion on types
earns its place for the same reason a second advisory database does.

**A coverage floor only where the toolchain reports branch data.** A line
floor that ignores untaken branches claims a guarantee it cannot make, so Rust
measures and publishes instead of gating.

**Mutation testing is reported, not gated.** Coverage says a line ran;
mutation says a bug on that line would have failed something. It is the only
check that tests the tests. A floor waits for a baseline, because a number
invented before one exists is the kind of claim these gates refuse to make.

### What is deliberately absent

**Property-based testing** — proptest, fast-check and hypothesis are how tests
are written, not jobs. They run inside the existing test contexts, and a
context for them would gate nothing.

**Fuzzing** — earns its place when something parses input we did not produce.
Every parser here reads a file one of our own tools wrote, and property-based
tests cover that far more cheaply.

## Consequences

Renaming a job renames a required context, and a ruleset still naming the old
one blocks every pull request until it is updated. The workflow and the
ruleset have to change together; the fleet audit is what notices when they
have not.

Adding a language means reading its ecosystem rather than filling in a
template. That is slower, and it is the work.

Two gaps stay open and are recorded rather than implied. Nothing enforces the
heavy tier's freshness window, so a heavy run failing for a month is as
invisible as one that never ran. And Cargo enforces boundaries between crates,
not between modules inside one — two modules in the same crate can reach
across a layer nobody declared, and nothing checks it.
