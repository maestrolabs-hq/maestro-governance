# The scheduled audit

`governance plan` is a command someone has to remember to run, which is the
same failure mode as a setting applied by hand. `.github/workflows/fleet-audit.yml`
runs it every Monday and on demand, publishes the report to the run summary
whether or not there is drift, and opens a tracking issue that closes itself
once the fleet is clean again.

## Why it needs its own token

A workflow's `GITHUB_TOKEN` is scoped to the repository it runs in. It cannot
read organisation settings, organisation rulesets, custom properties, or a
sibling repository's configuration -- which is most of what the baseline
covers.

With only that token the audit would read nothing, find nothing, and report a
clean fleet. **That is worse than not running it**: a green result nobody
should trust is how an estate rots while looking healthy. So the workflow
checks for the token first and stops with an error if it is absent.

## Creating it

A fine-grained personal access token, stored as the repository secret
`GOVERNANCE_AUDIT_TOKEN`:

| Scope | Access |
| --- | --- |
| Resource owner | `maestrolabs-hq` |
| Repository access | All repositories |
| Repository permissions | Metadata: read, Administration: read |
| Organization permissions | Administration: read, Custom properties: read |

Read-only throughout. The audit reports drift; it does not correct it, and a
token that could would be a token worth stealing.

This does **not** buy the four merge-method fields. See below.

## Why the rules are read per repository

Listing organisation rulesets (`GET /orgs/{org}/rulesets`) requires
**"Administration" organisation permissions (write)**. GitHub offers no
read-only path to that list, so an audit built on it would need a token that
can rewrite the rules it is checking -- stored as a repository secret, on a
schedule, unattended.

The audit reads `GET /repos/{owner}/{repo}/rules/branches/{branch}` instead.
That needs only repository read, and it is the better assertion: it proves a
rule is in effect on the branch rather than that a ruleset object exists
somewhere. It also records where each rule comes from, so a repository-level
copy of the organisation floor -- which whoever owns that repository could
delete -- is reported as drift rather than passing.

Set it with:

```text
gh secret set GOVERNANCE_AUDIT_TOKEN --repo maestrolabs-hq/maestro-governance
```

## Proving it works

The workflow carries `workflow_dispatch` precisely so it can be run once by
hand before its first scheduled firing. A schedule that has never executed is
not evidence of anything.

## When the report says `<unreadable>`

```text
~ allow_squash_merge : <unreadable> -> true
```

That is not drift on the repository. It says the audit asked for the field and
the response did not carry it.

### The four merge-method fields

`allow_squash_merge`, `allow_merge_commit`, `allow_rebase_merge` and
`delete_branch_on_merge` are returned by `GET /repos/{owner}/{repo}` **only to a
token with classic `repo` scope**. A fine-grained token does not receive them,
whatever its permissions say -- measured three times against an approved,
organisation-allowed, all-repositories token carrying `Administration: read`.

The endpoint's own reference asks for `Metadata: read` and documents no
per-field gating, so this is not discoverable from the documentation. It was
found by elimination: every other reading the audit performs -- five repository
settings, six organisation settings, four rules, fifteen files -- succeeded with
the same token, and only these four came back empty.

The original table above was written from inference and never proved. Between
2026-08-31 and 2026-09-13 the audit reported drift on all eight repositories
for this reason, and the values were correct throughout.

Those four lines are `pending` in `baseline.txt` today. To restore them to
`setting`, give the audit a token that can read them -- a classic PAT with
`repo`, or a GitHub App installation token -- and the next run prints a
promotion note naming the keys, because that is what a `pending` key does once
it starts answering.

### Any other field

Check the token against the table above and reissue it if it falls short. Until
it does, that setting is unaudited: the line says so rather than asserting a
value nobody read.
