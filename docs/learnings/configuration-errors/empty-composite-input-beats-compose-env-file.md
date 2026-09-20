---
title: "An empty composite-action input is a set variable, and compose interpolation prefers it to --env-file"
date: 2026-09-20
category: configuration-errors
component: ci_pipeline
module: dsp-repository/.github/actions/dsp-cli-stack-test/action.yml + dsp-cli/ci/stack/docker-compose.yml
problem_type: configuration
severity: moderate
symptoms:
  - "`docker compose --env-file stack.env up` would pull `daschswiss/knora-api:` with an empty tag"
  - "The pinned job and the nightly `latest` job share one composite action but need different image tags"
root_cause: "docker compose resolves `${VAR}` from the shell environment first and falls back to `--env-file` only when the variable is unset; a composite-action input with an empty default is exported to the steps as a set-but-empty variable, which wins over the file."
tags: [docker-compose, env-file, interpolation, github-actions, composite-action, inputs, precedence, drift-ci]
related:
  - gh-jq-array-output-breaks-github-output.md
  - dasch-specs/learnings/configuration-errors/github-actions-composite-action-main-ref-pr-isolation.md
issue: "DEV-7330"
---

# An empty composite-action input is a set variable, and compose interpolation prefers it to --env-file

## Problem

The dsp-cli drift workflow runs the same nine steps twice: `pinned` on pull requests against the
dsp-api and Fuseki versions in `dsp-cli/ci/stack/stack.env` (`API=v38.1.0`, `DB=v38.1.0`), and
`latest` nightly against `daschswiss/knora-api:latest` and `daschswiss/apache-jena-fuseki:latest`.
The compose file interpolates both:

```yaml
services:
  db:
    image: daschswiss/apache-jena-fuseki:${DB}
  api:
    image: daschswiss/knora-api:${API}
```

A review asked for the shared steps to become a composite action,
`.github/actions/dsp-cli-stack-test/action.yml`, and the obvious design was two inputs, `api` and
`db`, defaulting to empty so that `pinned` falls through to the env file and `latest` passes
`latest`.

## Investigation

docker compose's variable precedence for `${VAR}` in a compose file is: shell environment of the
`docker compose` process, then the `--env-file` (or `.env`), then the default in the expression.
A variable that is **set but empty** counts as set. A composite action's input reaches a step
only through `${{ inputs.<name> }}`, and the natural way to hand it to compose is
`env: { API: ${{ inputs.api }} }` on the step; with `default: ''` that mapping exports `API` as
present and empty. `${API}` then resolves to the empty string and compose asks for
`daschswiss/knora-api:`, which fails to pull.

## Root Cause

Two precedence rules composed the wrong way round: the action makes the value exist, and compose
treats existence as authority. The env file can only supply what the shell does not.

## Solution

The tags are not inputs. `stack.env` stays the only source in the composite action's steps, and
the nightly job overrides at job level, because job-level `env:` flows into a composite's steps:

```yaml
latest:
  if: github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'
  env:
    # overriding API/DB here is enough to pull `latest` images without a second env file.
    API: latest
    DB: latest
  steps:
    - uses: actions/checkout@v4
    - uses: ./.github/actions/dsp-cli-stack-test
      with:
        summarize-failures: 'true'
```

The one real input, `summarize-failures`, is a string flag with a `'false'` default and gates a
step, so its emptiness never reaches compose. `load-fixtures.sh` re-sources `stack.env`
internally, so the nightly job still checks dsp-api out at the **pinned** `API` tag for fixtures;
there is no dsp-api ref for `latest`, and the docs say so.

## Prevention

- Do not pass a compose interpolation variable through an action input with an empty default.
  Either give the input the real default (`v38.1.0`) and pass it explicitly everywhere, or keep
  the variable out of the action and override with job-level `env:`.
- When a compose file uses `--env-file`, check `env | grep '^VAR='` in the step before `up`; a
  present-but-empty line is the bug, not a missing one.
- `docker compose config` with the same env file and shell environment prints the resolved
  `image:` lines; run it in CI once (it is cheap) before `up --wait`, or at least locally when
  changing the plumbing.
- A composite action cannot see `steps.<id>.outcome` of its own steps from the calling job, so a
  failure summary that needs the test step's outcome lives inside the action, gated by an input.

## Verification

Precedence is docker's documented rule (Compose file reference, "Interpolation": environment
variables take precedence over the env file), confirmed by the reviewer on 2026-09-20 by reading
the resolved config; the empty-tag pull was reasoned from that rule and the action's input
semantics, not observed on a runner, because the design was corrected before the first run. The
first `pinned` run passed in 2 min 39 s and the second in 1 min 39 s.

## References

- PR: https://github.com/dasch-swiss/dsp-repository/pull/409, commit `chore(ci): run dsp-cli live tests against a pinned dsp-api stack`
- `.github/actions/dsp-cli-stack-test/action.yml`, `.github/workflows/dsp-cli-drift.yml`, `dsp-cli/ci/stack/docker-compose.yml`, `dsp-cli/ci/stack/stack.env`, `dsp-cli/ci/stack/load-fixtures.sh`
- Journal round 4, "Three composite-action constraints that shaped B2"
- Docker docs: Compose file reference, interpolation and `--env-file` precedence
