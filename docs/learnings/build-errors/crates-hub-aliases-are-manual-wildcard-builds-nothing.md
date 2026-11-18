---
title: "bazel build @crates//... builds nothing: the hub aliases are manual"
date: 2026-11-18
category: build-errors
component: rust_crate
module: MODULE.bazel
problem_type: build-error
severity: low
symptoms:
  - "bazel build @crates//... succeeds immediately with no actions"
  - "A crate that fails to build for a platform goes unnoticed until a first-party target needs it"
root_cause: "The crate_universe hub aliases are tagged manual, and wildcard target patterns skip manual targets."
tags: [bazel, crate_universe, manual, wildcard, bazel-query, crates]
related:
  - dasch-specs/learnings/best-practices/sipi-nix-to-bazel-migration-lessons.md
issue: "DEV-7699"
---

# bazel build @crates//... builds nothing: the hub aliases are manual

## Problem

To check that the whole crate universe builds (for a new platform, or after a repin), `bazel build @crates//...` reports success. It built nothing.

## Root Cause

The aliases in the `@crates` hub are tagged `manual`. Wildcard patterns such as `//...` and `:all` exclude `manual` targets, so the pattern expands to an empty set.

## Solution

Build the aliases by name, taken from a query:

```bash
bazel build $(bazel query 'kind(alias, @crates//:*)')
```

This built all 266 aliases (2585 actions, 208 s cold on macOS), and again for `//platforms:linux_x86_64_musl`.

## Prevention

- Treat a wildcard build with zero actions as a failed check, not a passing one.
- Use the query form for any universe-wide check, including the musl leg in CI.

## References

- https://github.com/dasch-swiss/dsp-repository/pull/592
- `MODULE.bazel` (the `crate.spec` list and the `crates` hub)
