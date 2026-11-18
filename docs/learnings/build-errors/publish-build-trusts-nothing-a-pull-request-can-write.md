---
title: "A publish build trusts nothing a pull request can write: no cached action results, and the registry login after the build"
date: 2026-11-18
category: build-errors
component: github-actions
module: .github/actions/docker-publish
problem_type: security-issue
severity: high
symptoms:
  - "A publish build reading the shared remote cache could ship an image built from an action result a pull request wrote"
  - "With build and push in one job, a login first exposes the Docker Hub token to crate build scripts and local fallback actions"
root_cause: "Every Bazel run, same-repo pull requests included, uses one remote-cache instance and holds the mTLS client certificate, so a PR can write any action result; and `docker login` writes the token to ~/.docker/config.json, where any code the build runs can read it. The old setup avoided both: GitHub Actions caches were branch-scoped, and building and pushing were separate jobs."
tags: [bazel, remote-cache, cache-poisoning, nativelink, skip_cache_lookup, docker-login, credentials, docker-publish, ci, supply-chain]
related:
  - dasch-specs/learnings/best-practices/sipi-nix-to-bazel-migration-lessons.md
issue: "DEV-7699"
---

# A publish build trusts nothing a pull request can write

## Problem

Moving the image builds to Bazel opened two paths from a pull request, or from third-party build code, into a published image or its credentials:

1. All Bazel runs share one remote-cache instance, and a PR workflow holds the client certificate. A PR can write any action result, and a publish build that reads the cache could ship one it planted.
2. The build job and the push job became one. If the job logs in to Docker Hub first, the token sits on disk while the build's own code runs: crate build scripts and local fallback actions.

## Root Cause

The cache is keyed by action digest, not by who wrote the result, and every workflow that holds the certificate can write. `docker/login-action` persists the credentials for the rest of the job, and build code runs with the job's filesystem access.

## Solution

`.github/actions/docker-publish/action.yml`:

- Adds `--noremote_accept_cached` to publish builds. Bazel sends `skip_cache_lookup`, and NativeLink neither reads the cache for the action nor joins an in-flight run of it, so every action runs again on the executor. Previews pass `trust-remote-cache: "true"`, because they build the PR's own code.
- Runs build, log in, push, in that order. The push runs with the same flags as the build, so it executes only the push script and rebuilds nothing. The previews still authenticate to GCP before the build: their build code is the PR's own, and its author can already edit the workflow.

Not closed: the executors are shared, so isolation between a PR's actions and a publish build's actions is up to the worker setup. Repository secrets have no `environment:` gate, so anyone with write access can read them from a branch workflow (already true before the migration).

## Prevention

- Treat a cache that untrusted workflows can write as untrusted input for any build that publishes or deploys.
- Write credentials after the last step that runs untrusted or third-party build code, and give the build and push steps identical flags, or the push rebuilds with the token on disk.
- A publish build re-runs every action: watch the first `main` run of `dpe-docker-publish.yml` against its 30-minute timeout, because that path runs only after merge.

## References

- https://github.com/dasch-swiss/dsp-repository/pull/601
- `.github/actions/docker-publish/action.yml`, `.github/actions/bazel-setup/action.yml`
- `docs/src/bazel.md` (Remote cache and execution)
