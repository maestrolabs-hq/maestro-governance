# ADR-0003: Shared where the tools allow it, tracked where they do not

- Status: accepted
- Date: 2026-08-31

## Context

Every repository needs the same lint thresholds, the same licence policy, the
same toolchain pin, the same editor settings. Copied into each repository,
those files drift: one gets updated, the others do not, and nothing notices
until two repositories disagree about what "clean" means.

Centralising them is the obvious answer and is only partly available. Most of
these files are read from a fixed path by a tool that offers no remote source.

## Decision

**Share what the tools permit.**

| What | Where it lives | How a repository gets it |
| --- | --- | --- |
| CI job definitions | `.github/workflows/*-fast.yml`, `*-heavy.yml` | `workflow_call` |
| Hook definitions | `.github/.pre-commit-hooks.yaml` | referenced by URL, revision pinned |
| Action pinning policy | `.github` repo, `zizmor.yml` | checked out by the workflow that applies it |
| Community health files | `.github/` | GitHub serves them org-wide |

**Track what they do not.** `clippy.toml`, `deny.toml`, `rust-toolchain.toml`,
`.editorconfig` and `.gitattributes` are read from the repository root by
cargo, rustup and editorconfig, none of which accept a remote source. They
stay duplicated.

Duplication that cannot be removed still has to be noticed when it diverges,
so `baseline.txt` pins each path to a **git blob hash** -- the value GitHub
already stores and `git hash-object` reproduces, so an entry can be written
from a copy known to be correct:

```text
file clippy.toml 5f1fb12f... maestro-core maestro-pi-config maestro-governance
file .editorconfig 0e21de11...
```

A trailing repository list narrows an entry, because `clippy.toml` means
nothing in a repository with no Rust. No list means every repository.

`governance plan` reads one git tree per repository, which covers every
tracked file in a single call, and reports any hash that does not match.

## Consequences

Changing a shared file means changing it in three places **and** updating the
baseline hash. That is more work than editing one file, and it is the price of
a guarantee: the audit fails until every copy agrees, so the estate cannot
half-adopt a change.

The first run of this check found `.github` missing `.editorconfig` and
`.gitattributes`, which it should have carried -- line endings matter in YAML
as much as in Rust.

A file is tracked only if it is listed. Nothing detects a *new* shared file
that someone forgets to add, so the list is a decision that has to be
maintained rather than a property that maintains itself.
