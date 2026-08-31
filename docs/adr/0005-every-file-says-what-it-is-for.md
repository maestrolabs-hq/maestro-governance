# 5. Every file says what it is for, in the language's own convention

## Status

Accepted.

## Context

A reader arriving at a file cold has to reconstruct why it exists from its
contents. That reconstruction is guesswork, and it is wrong most often on the
files that matter -- the ones holding a decision rather than a data structure.

The practice was already universal here without anyone deciding it: all 26 Rust
files opened with a `//!` brief. Universal and unenforced is the worst
combination, because the first file that skips it looks like precedent rather
than an omission.

There was also a question of which documentation convention to adopt across the
estate, prompted by Google's Python docstring style (`Args:` / `Returns:` /
`Raises:`).

## Decision

**Every source file opens with a brief saying what it does and why it exists.**
Enforced in the shared fast tier, so it covers repositories that do not exist
yet -- the same reason the prose gate lives there.

**Each language uses the convention its own tooling understands.** Not one style
imposed across all of them.

| Language   | Convention                                | Enforced by                               |
| ---------- | ----------------------------------------- | ----------------------------------------- |
| Rust       | `//!` brief, `# Errors` / `# Panics`      | `rust-docs`, `clippy::pedantic`           |
| Python     | Google (`Args:` / `Returns:` / `Raises:`) | `ruff` `D` rules, `convention = "google"` |
| TypeScript | TSDoc (`@param` / `@returns`)             | Biome                                     |

Google style is right for Python and only for Python. Written into Rust doc
comments it is plain text: rustdoc renders it as one grey blob, clippy still
demands a separate `# Errors` section, and the doctest never runs. Adopting it
estate-wide would mean writing every function's contract twice, in two
conventions, one of which no tool reads.

The Python tier fails if a repository has not declared its convention in
`pyproject.toml`. The style is a decision the repository records, not one the
shared workflow smuggles in -- and pydocstyle requires the choice anyway, since
`D203` and `D211` contradict each other and leaving it unset silently enables
both.

## Consequences

The brief gate cost nothing to adopt: every existing file already passed. That
is the point. A ratchet installed while the cost is zero holds a standard that
would be expensive to reach later.

**`missing_docs` was deliberately not enabled.** It demands a line above every
public field and variant, which produces `/// The key.` above `key: String` --
documentation that restates the identifier and teaches people that doc comments
are a tax to be paid rather than a place to put reasoning. The brief is where
the thinking goes; the rest of the file should be readable without narration.

The gate checks that a brief exists, not that it is good. Nothing can check
that. What it prevents is the specific failure of a file with no stated reason
to exist, which is the state every deleted crate in this estate was in before
it was deleted.

Two of those briefs already earned themselves. `plan.rs` explains why a saved
plan is checked against both the repository and the machine, and `apply.rs`
records that it is the only code permitted to write to the user's disk. Neither
fact is visible from the code.
