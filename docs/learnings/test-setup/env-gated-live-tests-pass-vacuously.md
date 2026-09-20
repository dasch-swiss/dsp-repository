---
title: "A live test that skips itself when unconfigured reports as passed: ignore it by default and fail strictly in CI"
date: 2026-09-20
category: test-setup
component: rust_crate
module: dsp-cli/tests/common/mod.rs + .github/scripts/check-live-tests-ignored.sh
problem_type: test-setup
severity: high
symptoms:
  - "`cargo nextest run --all-features` reported dsp-cli's 20 live tests as passed on every CI run, with no server configured"
  - "dsp-api drift was never detected although the live tests were the only detector"
  - "A drift job pointed at a broken stack would also have stayed green"
root_cause: "The live tests read their configuration with a helper that returns `None` and the test early-returns, which the harness counts as a pass; `--all-features` enables the `live` feature in CI, so the tests ran and 'passed' without ever talking to a server."
tags: [live-tests, integration-tests, vacuous-pass, ignore, strict-mode, nextest, run-ignored, env-vars, drift-detection, ci-gate]
related:
  - insta-snapshot-accept-by-rename-keeps-assertion-line.md
issue: "DEV-7330"
---

# A live test that skips itself when unconfigured reports as passed: ignore it by default and fail strictly in CI

## Problem

dsp-cli has 20 live tests in 14 `tests/live_*.rs` binaries behind a `live` cargo feature. Each
reads `DSP_TEST_SERVER` and friends with a `require_env` helper and returns early when a variable
is unset:

```rust
let server = match require_env("DSP_TEST_SERVER") { Some(v) => v, None => return };
```

An early return is a passing test. dsp-repository's `test.yml` runs
`cargo nextest run --locked --all-features --all-targets`, which enables `live`, so from the day
the crate joined the workspace every CI run would have reported the live tests green while none
of them made a request. They are the only detector of dsp-api drift, so drift would have gone
unnoticed indefinitely, and a drift job whose stack failed to start would have been green too.

## Investigation

The plan's intake measured the baseline: 1005 unit and 379 integration tests in the incubator,
"0 live tests run in any CI". Reading `test.yml` showed `--all-features`. The helper was copied
verbatim into 13 of the 14 live files. dsp-cli/ADR-0009 had deferred stack-based tests until
"CI without credentials" or "isolation from a shared environment" was wanted; both were now
wanted, and the vacuous pass was the plan's highest-rated risk.

## Root Cause

"Skip when unconfigured" was encoded as a return from the test body. The harness has no notion of
a skip inside a test function; only `#[ignore]` produces a result that is neither pass nor fail.
Combined with a feature flag that `--all-features` turns on, the safety valve became a silent
pass.

## Solution

Three parts, each necessary:

1. **Every live test is `#[ignore]`d**, placed after `#[test]`, with a message naming the recipe:

   ```rust
   #[test]
   #[ignore = "needs a DSP stack; run with just dsp-cli-test-live"]
   fn live_project_list_returns_non_empty_vec_with_valid_shortcodes() { … }
   ```

   nextest now reports them as **skipped** (20 skipped in the full suite), never as passed. One
   test that hits crates.io instead of DSP carries its own message and is excluded from the recipe.

2. **A strict mode for the job that means it.** The 13 copies of the helper moved to
   `tests/common/mod.rs`, keeping the `Option<String>` return so every call site is unchanged,
   and panicking when `DSP_LIVE_STRICT=1`:

   ```rust
   pub fn require_env(name: &str) -> Option<String> {
       match env::var(name) {
           Ok(v) if !v.trim().is_empty() => Some(v),
           _ => {
               if env::var("DSP_LIVE_STRICT").as_deref() == Ok("1") {
                   panic!("DSP_LIVE_STRICT=1: required environment variable {name} is not set");
               }
               eprintln!("skipping live test: {name} not set");
               None
           }
       }
   }
   ```

   The recipe that runs them sets it and selects only the ignored live binaries:

   ```
   dsp-cli-test-live:
       DSP_LIVE_STRICT=1 cargo nextest run -p dsp-cli --features live --run-ignored only -E 'binary(/^live_/) & !binary(live_update_check)'
   ```

3. **A gate that keeps it so.** `.github/scripts/check-live-tests-ignored.sh` scans every
   `#[test]` in `dsp-cli/tests/live_*.rs` for an `#[ignore` attribute with a small state machine
   (the attribute may precede `#[test]` and doc comments may sit between), exits non-zero on any
   offender and on an empty glob, and runs in `just check` with its own `.test.sh` in `just test`.

The drift workflow's `pinned` job runs the recipe against a containerised dsp-api; a missing
variable there is a failure, not a skip.

## Prevention

- An environment-gated test is `#[ignore]`d, never self-skipping. The acceptance criterion is
  worded as "reports skipped, not passed".
- The job that is supposed to exercise the tests sets a strict variable so a broken stack or a
  lost `$GITHUB_ENV` export turns into a loud panic naming the variable.
- Invariants that erode one test at a time get a gate script, not a review note.
- `optional_env` stays lenient in both modes; only variables the test cannot run without are
  strict. Two variables (`DSP_TEST_USER`, `DSP_TEST_PASSWORD`) are unreachable in CI because
  `DSP_TOKEN` is always set first; if the token export ever breaks they become reachable and the
  run panics instead of skipping, which is the intended failure.

## Verification

Observed 2026-09-20 on the branch:

```
$ nix develop --command cargo nextest run --locked --all-features --all-targets
… 3432 passed, 20 skipped
$ nix develop --command bash .github/scripts/check-live-tests-ignored.sh
✓ live tests: 20 #[test] across 14 file(s), all #[ignore]
```

The `pinned` drift job passed on CI in 2 min 39 s with `DSP_LIVE_STRICT=1`.

## References

- PR: https://github.com/dasch-swiss/dsp-repository/pull/409, commits `build(dsp-cli,docs): register the crate and harmonize it with the workspace` and `chore(ci): run dsp-cli live tests against a pinned dsp-api stack`
- `dsp-cli/tests/common/mod.rs`, `.github/scripts/check-live-tests-ignored.sh`, `justfile` (`dsp-cli-test-live`), `.github/workflows/dsp-cli-drift.yml`
- `dsp-cli/docs/adr/0009-testing-strategy.md` (amendment 2026-09-19, option C4 adopted), `docs/src/dsp-cli/testing-strategy.md`
- Plan: "Two CI traps found in the baseline"; the `ARCH-MAP.md` dsp-cli boundary rule "live tests never run in the default suite"
