---
title: "`--remote_download_minimal` in `.bazelrc` leaves top-level outputs remote under CI's remote flags: build copied outputs with `--remote_download_toplevel`"
date: 2026-11-18
category: build-errors
component: bazel
module: justfile + .bazelrc
problem_type: configuration-error
severity: medium
symptoms:
  - "`just css*` and the Playwright recipes fail on pull request and `main` runs, but pass locally"
  - "The stylesheet and the release binary a recipe copies are not on disk after `bazel build`"
root_cause: "`.bazelrc` sets `--remote_download_minimal`; with the remote cache and executor flags that CI adds, a plain `bazel build` leaves its outputs in the remote store, and a recipe that copies them finds nothing."
tags: [bazel, remote-cache, remote_download_minimal, remote_download_toplevel, ci, justfile, playwright, a11y]
related:
  - dasch-specs/learnings/best-practices/sipi-nix-to-bazel-migration-lessons.md
issue: "DEV-7699"
---

# `--remote_download_minimal` leaves top-level outputs remote in CI

## Problem

Recipes that build a target and then copy its output (`_css-copy` for the stylesheets, `_release-bin` for the release binary used by the Playwright and a11y recipes) worked locally. With CI's remote flags the outputs stayed remote, so the recipes would fail on the PR and `main` paths. A local run cannot show this; the devops review found it.

## Root Cause

`.bazelrc` sets `build --remote_download_minimal`, which fetches only what a local action needs. Without remote flags, local execution produces the files. With CI's flags, executors produce them and Bazel does not download them.

## Solution

Both helpers build with `--remote_download_toplevel`, as `build-fuzz` already did (`justfile`). The flag downloads the top-level targets' outputs and keeps minimal downloads for intermediates.

## Prevention

- Any recipe that reads a build output from `bazel-bin` or `bazel-out` passes `--remote_download_toplevel`.
- The path is only exercised under CI's remote flags, so read the first PR runs of the recipes that copy outputs (here the two a11y workflows) before trusting them.

## References

- https://github.com/dasch-swiss/dsp-repository/pull/602
- `.bazelrc`, `justfile` (`_css-copy`, `_release-bin`, `build-fuzz`), `.github/actions/bazel-setup/action.yml`
