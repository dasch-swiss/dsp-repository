---
title: "gh's --jq pretty-prints arrays: compact a value before writing it to a workflow output"
date: 2026-09-20
category: configuration-errors
component: ci_pipeline
module: dsp-repository/.github/workflows/release-please.yml
problem_type: configuration
severity: high
symptoms:
  - "A step doing `echo \"key=$VALUE\" >> \"$GITHUB_OUTPUT\"` fails with `Invalid format` whenever the value is a non-empty JSON array from `gh … --jq`"
  - "The same step passes when the list is empty, because `[]` serialises on one line"
  - "Downstream `fromJSON(needs.job.outputs.key)` never runs, so a matrix job silently never happens"
root_cause: "`gh`'s built-in `--jq` (gojq) pretty-prints a non-empty array across several lines and has no compact flag; a multi-line value written with the single-line `key=value` form is invalid `$GITHUB_OUTPUT` syntax."
tags: [github-actions, gh-cli, jq, github-output, fromjson, matrix, release-please, workflow-outputs]
related:
  - rustfmt-nightly-drift-between-flake-lock-and-ci.md
issue: "DEV-7330"
---

# gh's --jq pretty-prints arrays: compact a value before writing it to a workflow output

## Problem

The dsp-cli migration turned release-please's single "amend `Cargo.lock` on the release PR" step
into a matrix job over every pending release branch. The branch list was produced with:

```yaml
run: |
  BRANCHES=$(gh pr list --repo "${{ github.repository }}" --label "autorelease: pending" --json headRefName --jq '[.[].headRefName]')
  echo "branches=$BRANCHES" >> "$GITHUB_OUTPUT"
```

The plan specified exactly that command (plan line 517), a worker implemented it verbatim, and a
review of the reworked commit caught it before it ever ran on `main`: with one open release
branch the value is three lines, and the step would have failed on nearly every push to `main`
(a release PR is open almost all the time). Only the genuinely empty case survives.

## Investigation

The devops reviewer ran the command and looked at the bytes; the session confirmed it:

```
$ gh pr list --repo dasch-swiss/dsp-repository --label "autorelease: pending" --json headRefName --jq '[.[].headRefName]' | wc -l
3
```

`gh --jq` is gojq with pretty-printing on and no `-c`/compact option. GitHub's `$GITHUB_OUTPUT`
accepts a multi-line value only in the heredoc form (`key<<EOF` … `EOF`); the one-line
`key=value` form with an embedded newline is rejected with "Invalid format". Empty arrays are
special-cased by jq onto one line, which is why a first test on a quiet repository passes.

## Root Cause

Two facts that are each documented but rarely combined: `gh --jq` output is pretty JSON, and
`$GITHUB_OUTPUT` is line-oriented. A value that is a scalar (one branch name) never trips it, so
the pattern survives until someone stores a list.

## Solution

Compact with real `jq` (preinstalled on `ubuntu-latest`) before writing, and say why in the step:

```yaml
run: |
  # Compact to one line: a multi-line GITHUB_OUTPUT value is invalid, and gh pretty-prints arrays.
  BRANCHES=$(gh pr list --repo "${{ github.repository }}" --label "autorelease: pending" --json headRefName --jq '[.[].headRefName]' | jq -c .)
  echo "branches=$BRANCHES" >> "$GITHUB_OUTPUT"
```

`[]` still serialises as `[]`, so the consumer's guard keeps working. The consumer also checks
that the producing job ran, because a skipped job's outputs read as the empty string, and
`'' != '[]'` is true:

```yaml
amend-lockfile:
  needs: release-please
  if: needs.release-please.result == 'success' && needs.release-please.outputs.branches != '[]'
  strategy:
    fail-fast: false
    matrix:
      branch: ${{ fromJSON(needs.release-please.outputs.branches) }}
```

The alternative is the heredoc form of `$GITHUB_OUTPUT`, which accepts the multi-line value as is;
`fromJSON` parses either. Compacting is preferred because the value is then also greppable in the
run log.

## Prevention

- Any `gh … --json … --jq` whose result can be a list or an object goes through `| jq -c .` (or
  the heredoc output form) before `$GITHUB_OUTPUT`. Treat a bare `--jq` array as a smell in review.
- Test the producing step with a **non-empty** result. An empty list is the one case that cannot
  reveal the bug.
- A job consuming another job's output with `fromJSON` guards on `needs.<job>.result == 'success'`
  as well as on the value, so forks and skipped jobs do not reach `fromJSON('')`.
- `actionlint` does not catch this: the YAML and the expressions are valid. Only a run with a real
  list, or a reviewer who runs the command, does.

## Verification

Observed 2026-09-20 with `gh` 2.x on macOS, one `autorelease: pending` PR open:

```
$ gh pr list --repo dasch-swiss/dsp-repository --label "autorelease: pending" --json headRefName --jq '[.[].headRefName]'
[
  "release-please--branches--main"
]
$ … | jq -c .
["release-please--branches--main"]
```

## References

- PR: https://github.com/dasch-swiss/dsp-repository/pull/409, commit `chore(ci): release dsp-cli on its own version line`
- `.github/workflows/release-please.yml` (`find-prs` step, `amend-lockfile` job)
- Plan: `docs/specs/2026-09-18-dsp-cli-migration/01-feat-dsp-cli-migration-plan.md` line 517 (corrected in place); journal round 4, chunk A1
- GitHub docs, "Workflow commands for GitHub Actions": multiline strings in `GITHUB_OUTPUT` need the delimiter form
