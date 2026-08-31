# ADR-0003: Each language gets its own checks, not a translation of Rust's

- Status: accepted
- Date: 2026-08-31

## Context

Rust was gated first, so its shape was available to copy. Copying it is the
obvious move and the wrong one: it produces checks that exist because another
language needed them, and misses the ones a language needs and Rust does not.

The first TypeScript tier was written that way, and the omission was found by
being asked whether it really matched what Python and Rust carry. It did not.
Rust runs a second advisory database in the heavy tier; TypeScript ran one and
the workflow asserted there was no second worth having. There is -- `npm
audit` reads GitHub's advisory data and OSV is a different corpus.

## Decision

Each language's checks come from reading that language's real toolchain, and
the differences are recorded where they occur rather than smoothed away.

**The same everywhere.** Format, lint, test, dependency hygiene, and at least
two independent advisory sources across the two tiers. These are properties of
maintaining software, not of a language.

**Different, with a reason:**

| Check | Rust | TypeScript | Python |
| --- | --- | --- | --- |
| types | in the compiler | `ts-types`, tsc | `py-types`, mypy and ty |
| architecture | Cargo, structurally | `ts-arch`, dependency-cruiser | `py-arch`, import-linter |
| coverage floor | not gated | gated, branch | gated, branch |
| oldest supported | `msrv` | `node-matrix` | `python-matrix` |
| mutation | `cargo-mutants` | Stryker | `mutmut` |

`py-types` runs mypy and ty because they disagree usefully -- ty parses past a
syntax error where mypy stops at the first. A second opinion on types earns
its place for the same reason a second advisory database does.

`py-arch` exists because any Python module can import any other, so a layering
rule is a sentence in a README until something fails the build over it. Rust
gets the property from Cargo without a job: a crate cannot reference a crate
absent from its dependency list. TypeScript has neither, and that gap is
written in `ts-fast.yml` rather than left for someone to notice.

Rust alone does not gate a coverage floor, because its toolchain reports no
branch data. A line-coverage floor that ignores untaken branches claims a
guarantee it cannot make, so Rust measures and publishes instead. Python and
TypeScript both report branch coverage -- vitest through istanbul -- and gate
on it.

Every language runs mutation testing in the heavy tier, reported rather than
gated. Coverage says a line ran; mutation says a bug on that line would have
failed something. It is this estate's own rule -- a gate that cannot fail is
not a gate -- applied to the test suite itself. A floor waits for a baseline,
because a number invented today would be the kind of claim these gates exist
to refuse.

Two techniques are deliberately absent. Property-based testing (proptest,
fast-check, hypothesis) is how tests are written, not a job; it runs inside the
existing test contexts and a context for it would gate nothing. Fuzzing earns
its place when something parses input we did not produce -- today every parser
here reads a file one of our own tools wrote.

## Consequences

Adding a language means reading its ecosystem, not filling in a template. That
is slower and it is the work.

The table above will drift from the workflows unless something checks it. It
is not checked today. The honest options are to gate it or to accept that it
is a snapshot with a date on it, and for now it is the latter -- said here so
the next reader does not trust it more than it deserves.

The TypeScript architecture gap this ADR originally recorded is closed:
`ts-arch` runs dependency-cruiser, matching `py-arch`. Rust still needs no such
job, because Cargo refuses at compile time what the others need a tool to
detect -- but that only holds *between* crates. Two modules inside one crate
can still reach across a boundary nobody declared, and nothing checks that.
That gap is open.
