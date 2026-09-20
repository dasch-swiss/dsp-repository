---
title: "release-please attributes a commit, and its Release-As footer, to every package whose path it touches"
date: 2026-09-20
category: configuration-errors
component: ci_pipeline
module: dsp-repository/.github/release-please/config.json
problem_type: configuration
severity: moderate
symptoms:
  - "A `Release-As: 0.3.0` footer meant for one package would have bumped the workspace root to 0.3.0 too"
  - "A dependency bump in one package also adds a line to the root package's release PR"
  - "A top-level `extra-files` default with a jsonpath that exists only in the root manifest is inherited by every package"
root_cause: "In manifest mode release-please splits each commit by the paths it touches and attributes it to every package that owns one of them; the root package `.` owns everything not under `exclude-paths`. Footers such as `Release-As` apply per attributed package, and per-package config keys override the top-level defaults rather than merge with them."
tags: [release-please, monorepo, release-as, exclude-paths, extra-files, separate-pull-requests, cargo-workspace, versioning]
related:
  - gh-jq-array-output-breaks-github-output.md
issue: "DEV-7330"
---

# release-please attributes a commit, and its Release-As footer, to every package whose path it touches

## Problem

dsp-cli joined the dsp-repository workspace with its own release line: a second release-please
package (`dsp-cli`, `release-type: simple`, `include-component-in-tag: true`, tags
`dsp-cli-vX.Y.Z`) beside the root package `.` (the whole workspace, tags `vX.Y.Z`). The first
release from the new home was to be 0.3.0, chosen with a `Release-As: 0.3.0` footer on the
hardening commit rather than left to the conventional-commit bump (a `feat` alone gives 0.2.2
under `bump-minor-pre-major`).

Three things about attribution were not obvious from the config schema:

1. A `Release-As` footer applies to **every** package the commit is attributed to. The
   hardening commit needed `url` for scheme validation; adding it as a direct dependency would
   have changed the root `Cargo.lock`, attributing the commit to `.` as well and releasing DPE as
   0.3.0.
2. A dsp-cli dependency bump edits the root `Cargo.lock`, so it bumps both packages. Accepted and
   documented rather than worked around: both are pre-1.0 and release often.
3. A top-level `extra-files` entry is a default inherited by every package. The root's entry
   points at `$.workspace.package.version`, which does not exist in `dsp-cli/Cargo.toml`.

## Investigation

release-please's `src/util/commit-split.ts` assigns a commit to a package when any touched file is
under that package's path; the root `.` receives a commit unless **all** its files fall under an
`exclude-paths` entry. `mergeReleaserConfig()` applies per-package keys with `??` over the
top-level ones, so `extra-files` overrides, never concatenates. `separate-pull-requests: true`
keeps the two release PRs apart, and `src/util/branch-name.ts` then names the dsp-cli branch
`release-please--branches--main--components--dsp-cli`; an existing root release PR may be closed
and reopened under the analogous name on the first run after the change.

Verified during the session: `git diff --stat HEAD~1 HEAD -- . ':!dsp-cli'` on the hardening
commit was kept empty after every amend, and `git log -1 --format='%(trailers)'` showed both
`Release-As: 0.3.0` and the `Co-Authored-By` trailer. Written as two paragraphs, git's trailer
parser had read only the last one and dropped `Release-As`; the two lines must share the final
paragraph.

## Root Cause

Attribution is by path, not by commit scope, footer or intent. The root package is the catch-all
for anything not excluded, so a workspace-level file such as `Cargo.lock` or a page under
`docs/src/` belongs to `.` even when the change is about one member crate.

## Solution

- Root `exclude-paths: [".github", "dsp-cli"]`; `separate-pull-requests: true`; dsp-cli's own
  `extra-files: [{type: toml, path: Cargo.toml, jsonpath: $.package.version}]` (paths are relative
  to the package directory).
- The commit carrying `Release-As: 0.3.0` touches only `dsp-cli/**`. `url` is reached through
  `reqwest::Url` (reqwest re-exports it) instead of a new dependency; the book page that documents
  the new flag is a separate `docs(dsp-cli)` commit without the footer.
- The root's `extra-files` moved inside the `"."` package so the root-only jsonpath is not the
  inherited default for the next package (clarity, not a fix: dsp-cli's own entry already
  overrode it).
- A comment above `version` in `dsp-cli/Cargo.toml` says release-please writes it and points at
  `Release-As` for deliberate versions.
- The double-bump rule and the components branch name are documented in
  `docs/src/deployment.md#dsp-cli`.

## Prevention

- Before committing anything with a `Release-As` footer in a multi-package manifest, run
  `git diff --stat HEAD~1 HEAD -- . ':!<package-path>'` and require it empty.
- Check trailers with `git log -1 --format='%(trailers)'`, never by eye; git reads only the last
  paragraph.
- When adding a package, look at every top-level key as a default the new package inherits, and
  move root-specific ones (`extra-files` with a root-only jsonpath) under `"."`.
- Expect the root release PR to churn its branch name once after enabling
  `separate-pull-requests`; it is not a fault.

## Verification

Observed 2026-09-20 on the dsp-cli hardening commit before push:

```
$ git log -1 --format='%(trailers)' 53c39471
Release-As: 0.3.0
Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>
$ git diff --stat 53c39471~1 53c39471 -- . ':!dsp-cli' | wc -l
0
```

The first `main` run after merge (plan Phase 6) is what confirms the attribution end to end; it
had not happened when this was written.

## References

- PR: https://github.com/dasch-swiss/dsp-repository/pull/409, commits `chore(ci): release dsp-cli on its own version line` and `fix(dsp-cli): close the pre-migration security and correctness backlog`
- `.github/release-please/config.json`, `.github/release-please/manifest.json`, `docs/src/deployment.md`
- release-please `docs/manifest-releaser.md` (`exclude-paths`, `separate-pull-requests`, `include-component-in-tag`), `src/util/commit-split.ts`, `src/util/branch-name.ts`; googleapis/release-please#2111 (why `release-type: simple` and not `rust` for a `version.workspace = true` member)
- Plan: `docs/specs/2026-09-18-dsp-cli-migration/01-feat-dsp-cli-migration-plan.md`, "Release attribution is by path, not scope"; journal rounds 3 to 5
