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

Set it with:

```text
gh secret set GOVERNANCE_AUDIT_TOKEN --repo maestrolabs-hq/maestro-governance
```

## Proving it works

The workflow carries `workflow_dispatch` precisely so it can be run once by
hand before its first scheduled firing. A schedule that has never executed is
not evidence of anything.
