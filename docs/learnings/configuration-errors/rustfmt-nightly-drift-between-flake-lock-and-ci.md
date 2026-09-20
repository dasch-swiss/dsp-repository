---
title: "Two floating nightly rustfmts have no shared fixed point: pin CI and the Nix dev shell to one"
date: 2026-09-20
category: configuration-errors
component: ci_pipeline
module: dsp-repository/flake.nix + .github/workflows/check.yml
problem_type: configuration
severity: moderate
symptoms:
  - "`check` workflow red at `cargo +nightly fmt --check --all` with pure comment re-wrap diffs; local `just check` green"
  - "Ten hunks in six `dsp-cli/` files, each moving one word to the next comment line"
  - "Reformatting to CI's output makes the local rustfmt want three of the paragraphs back"
root_cause: "CI installs the floating `dtolnay/rust-toolchain@nightly`; the flake pins `rust-bin.nightly.latest` at the locked `rust-overlay` revision, five months older. With `wrap_comments = true` the two rustfmt builds wrap at different effective widths in both directions."
tags: [rustfmt, nightly, toolchain-drift, nix-flake, rust-overlay, github-actions, ci-local-parity, wrap-comments, pinning, RUSTFMT]
related:
  - dasch-specs/learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md
  - dasch-specs/learnings/build-errors/version-drift-marketplace-plugin.md
  - dasch-specs/learnings/configuration-errors/github-actions-composite-action-main-ref-pr-isolation.md
issue: "DEV-7330"
---

# Two floating nightly rustfmts have no shared fixed point: pin CI and the Nix dev shell to one

## Problem

The first CI run of dasch-swiss/dsp-repository#409 (2026-09-20) failed the `check` workflow at
`cargo +nightly fmt --check --all`, the rustfmt step inside `just check`. The log listed ten
hunks in six files under `dsp-cli/`, all `//` comment paragraphs re-wrapped one word earlier:

```
Diff in .../dsp-cli/src/actions/auth/set_token.rs:82:
-    // 1. Local decode: extract metadata without verifying the signature. Failure means the input is not
-    //    structurally a JWT → Usage error (exit 2). We do NOT locally enforce `exp`; a locally-expired
+    // 1. Local decode: extract metadata without verifying the signature. Failure means the input is
+    //    not structurally a JWT → Usage error (exit 2). We do NOT locally enforce `exp`; a
```

Locally, `just fmt` had been run and `just check` was green in the Nix dev shell. The other
crates in the workspace were unaffected; the 467 files just copied in from dsp-incubator were the
first whose comments sat exactly on the width edge.

## Investigation

1. **Dead end: the known justfile defect.** `main`'s justfile fails `just --check --fmt` locally
   (one missing blank line, fixed on #406), so the red `check` job was assumed to be that. Reading
   the log showed that step had passed on CI; only rustfmt failed.
2. **Dead end: reproducing with rustup.** `~/.cargo/bin/cargo +nightly fmt -p dsp-cli --check`
   with rustup's nightly (2025-12-18) reported nothing. `rustup update nightly` brought
   2026-09-18, the same build CI had installed, and still reported nothing.
3. **Why the reproduction lied.** `flake.nix` exports `RUSTFMT = "${rustNightly}/bin/rustfmt"` so
   that its `cargo` wrapper, which strips the `+nightly` argument, still formats with the Nix
   nightly. `cargo fmt` honours `RUSTFMT`, and the variable was present in the session's shell, so
   rustup's `cargo +nightly fmt` ran the Nix April rustfmt too. With `env -u RUSTFMT`, `cargo fmt`
   fell through to the stable rustfmt on `PATH` instead (warnings that `imports_granularity` is
   nightly-only). Only `RUSTFMT=~/.rustup/toolchains/nightly-aarch64-apple-darwin/bin/rustfmt`
   reproduced CI's ten hunks exactly.
4. **Breakthrough: no fixed point.** After applying CI's ten hunks, the flake's rustfmt
   (1.9.0-nightly, 2026-04-06) reported three diffs back: it re-joins paragraphs the newer build
   (1.10.0-nightly, 2026-09-18) wraps. Two rustfmt builds disagreeing in both directions means no
   comment text is green under both. The versions had to converge; the text was never the problem.

## Root Cause

Two independent sources of "nightly":

- `.github/workflows/check.yml` installs `dtolnay/rust-toolchain@nightly`, which floats to the
  current nightly on every run.
- `flake.nix` defines `rustNightly = pkgs.rust-bin.nightly.latest.minimal.override { extensions = [ "rustfmt" ]; }`.
  `nightly.latest` is the latest nightly known to the **locked** `rust-overlay` revision in
  `flake.lock`, which was `d8b1b209` (2026-04-07).

`.rustfmt.toml` sets `wrap_comments = true` and `comment_width = 100`. Comment re-flow is where
rustfmt builds differ most between nightlies, and the disagreement is not monotonic, so a
five-month gap produced wraps in both directions. The stable toolchain never drifted because
`rustStable = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml` pins it to the same
`1.93.0` CI uses.

## Solution

Converge the versions, not the text:

```sh
nix flake update rust-overlay          # d8b1b209 (2026-04-07) → 26a71e6 (2026-09-19); 6-line flake.lock diff
nix develop --command cargo +nightly fmt --version   # rustfmt 1.10.0-nightly (420ed2a0c3 2026-09-18), same as CI
nix develop --command rustc --version                # rustc 1.93.0, unchanged
nix develop --command just fmt
nix develop --command cargo +nightly fmt --check --all   # clean
```

Landed as `350f4af5 chore(deps): update rust-overlay so the dev shell's nightly rustfmt matches CI`.
The ten re-wrapped comment blocks were folded into the Phase 2 format commit
(`chore(dsp-cli): format under the workspace rustfmt configuration`) because `git blame` showed
every affected line came from the verbatim copy and that format pass. The second CI run was green
on all 19 checks.

To reproduce CI's rustfmt without Nix, point the variable at rustup's binary explicitly; unsetting
it is not enough because the stable `rustfmt` earlier on `PATH` wins:

```sh
RUSTFMT=~/.rustup/toolchains/nightly-*/bin/rustfmt ~/.cargo/bin/cargo +nightly fmt --all --check
```

## Prevention

- **Pin both sides to one nightly date and bump them in the same commit.** In `flake.nix`,
  `pkgs.rust-bin.nightly."2026-09-18".minimal.override { … }` instead of `nightly.latest`; in
  `check.yml`, `uses: dtolnay/rust-toolchain@master` with `toolchain: nightly-2026-09-18` instead of
  `@nightly`. Then neither side can move alone. This is the repository decision that removes the
  class of failure; it was not taken in #409, which only re-synchronised the lock.
- **Cheap tripwire until then:** print the version next to the check so drift reads as two
  different version strings in the logs rather than as a style disagreement. In the `justfile`
  `check` recipe, `cargo +nightly fmt --version` before `cargo +nightly fmt --check --all`, and the
  same line as a step in `check.yml`.
- **When a fmt step disagrees between local and CI, compare versions first.** `nix develop
  --command cargo +nightly fmt --version` against the `rustc … nightly` line the dtolnay action
  prints. If they differ, `nix flake update rust-overlay` and re-run `just fmt`; do not edit the
  comments by hand.
- **Anti-patterns seen here:** hand-editing text to satisfy two formatters; assuming a red `check`
  job is the known justfile defect without reading the failing step; reproducing "CI's toolchain"
  through rustup while the dev shell's `RUSTFMT` is still exported.

## Verification

Mutable facts, as observed on 2026-09-20:

```
$ gh run view 35479310695 -R dasch-swiss/dsp-repository --log | grep 'rustc 1.100'
rustc 1.100.0-nightly (420ed2a0c 2026-09-18)          # CI, dtolnay/rust-toolchain@nightly

$ nix develop --command cargo +nightly fmt --version   # before the lock update
rustfmt 1.9.0-nightly (bcded33165 2026-04-06)

$ nix develop --command cargo +nightly fmt --version   # after `nix flake update rust-overlay`
rustfmt 1.10.0-nightly (420ed2a0c3 2026-09-18)

$ env | grep ^RUSTFMT
RUSTFMT=/nix/store/…-rust-minimal-1.96.0-nightly-2026-04-07/bin/rustfmt
```

## References

- PR: https://github.com/dasch-swiss/dsp-repository/pull/409 (commits `350f4af5`, `78267ff0`)
- Plan and execution journal: `docs/specs/2026-09-18-dsp-cli-migration/` (the journal's side findings record the CI run)
- `flake.nix` (`rustNightly`, `cargoWrapper`, `RUSTFMT`), `.rustfmt.toml`, `.github/workflows/check.yml`, `justfile` (`fmt`, `check`)
- Related learnings in dasch-specs: `learnings/best-practices/maudfmt-adoption-no-check-mode-and-clippy-gotchas.md` (the other formatter in the same `check` step), `learnings/build-errors/version-drift-marketplace-plugin.md` (two declaration sites drifting), `learnings/configuration-errors/github-actions-composite-action-main-ref-pr-isolation.md` (a floating reference masking the change under test)
