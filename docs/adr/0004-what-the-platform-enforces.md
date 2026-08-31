# ADR-0004: What the platform enforces, and at which level

- Status: accepted
- Date: 2026-08-31

## Context

Gates in CI can be added to a repository, and can be missing from one. A
repository created in a hurry has no workflows, no ruleset, and nobody
notices until it holds something that matters.

The controls that survive that are the ones applied above the repository,
where a new repository inherits them before anyone configures it.

## Decision

### Organisation rulesets: the floor

| Ruleset | Applies to | Rules |
| --- | --- | --- |
| `floor-no-destruction` | every repository | `deletion`, `non_fast_forward` |
| `floor-release-tags` | every repository, `refs/tags/v*` | `deletion`, `non_fast_forward`, `update` |
| `mature-discipline` | `tier=mature` | `pull_request`, squash only, zero approvals |
| `visibility-is-frozen` | every repository | no visibility transition, in any direction |

No bypass actors on any of them, including the owner. A floor with an
exemption for the person most likely to be in a hurry is not a floor.

`visibility-is-frozen` is a **repository-target** ruleset rather than a branch
one, and it exists for a cost reason rather than a security one. Every feature
this estate relies on -- code scanning, secret scanning, push protection,
dependency review -- is free on a public repository and billed per active
committer on a private one. Actions minutes are unmetered on public runners
and metered on private ones, at two times for Windows and ten for macOS, and
the heavy tier runs both every week.

A repository flipped to private would therefore start billing quietly while
every other check stayed green. The rule refuses the transition outright, for
the owner too -- verified by attempting it and being refused with *"Visibility
can't be changed by this user because of rulesets"*. `baseline.txt` also
records the expected visibility, because a ruleset can be deleted and an audit
notices a setting that moved.

`mature-discipline` selects on the **`tier` custom property**, which is
`required` with default `mature`. A repository created tomorrow carries the
property without anyone setting it, and the strict ruleset therefore applies
to it on creation.

### Repository rulesets: the required checks

The org floor cannot name status checks, because contexts differ per
repository. Each repository's own `main` ruleset adds `required_status_checks`
with `strict_required_status_checks_policy` — the branch must be current with
its base before merging — naming exactly the fast-tier contexts.

Zero approving reviews are required. A single maintainer cannot approve their
own pull request, so a nonzero count would block every merge; the gate here is
the checks, not a rubber stamp.

### Organisation parameters

| Parameter | Value | Why |
| --- | --- | --- |
| `sha_pinning_required` | true | A tag or branch can be moved by whoever publishes the action |
| default `GITHUB_TOKEN` | read | A workflow that needs to write should say so |
| token can approve PRs | false | Otherwise a workflow can satisfy the review requirement |
| 2FA required | true, secure methods | SMS is defeated by a SIM swap |
| `tier` property | required, default `mature` | The selector the strict ruleset depends on |

Actions from within the organisation are reference-pinned rather than
hash-pinned, recorded in `zizmor.yml`: pinning our own reusable workflow by
SHA would mean editing every caller each time a pin inside it moves, and the
trust boundary is identical either way.

### Security configuration

`maestrolabs-baseline`, enforced, and **default for all new repositories**:
dependency graph, Dependabot alerts and security updates, secret scanning,
push protection, private vulnerability reporting, and CodeQL default setup.

Push protection is the one that changes outcomes rather than reporting them:
it refuses the push rather than filing an alert after the secret is public.

## Consequences

An enforced configuration overwrites settings applied by hand. Enabling CodeQL
on a repository directly and then attaching the configuration silently
reverted it, because the configuration is authoritative and did not mention
it. Anything that should hold has to be *in* the configuration, not merely
true.

Renaming a CI job renames a required context, and a ruleset still naming the
old one blocks every pull request until updated. They change together.

`governance plan` reads the applied rules from each repository's own branch
endpoint rather than listing organisation rulesets, because listing them needs
organisation `Administration: write` — a token that could rewrite the rules it
audits. Reading the applied rules needs only repository read, and proves the
rule is in effect rather than that a ruleset object exists.
