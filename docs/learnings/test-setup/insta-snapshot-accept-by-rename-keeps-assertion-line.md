---
title: "Accept insta snapshots with cargo insta, not by renaming: the rename keeps line-number metadata that churns"
date: 2026-09-20
category: test-setup
component: rust_crate
module: dsp-cli/tests/snapshots
problem_type: test-setup
severity: low
symptoms:
  - "Snapshot files change on unrelated commits because their `assertion_line:` header moved"
  - "Some `.snap` files carry `assertion_line:` and most do not, with no rule saying which"
  - "After moving a crate into a workspace every snapshot's `source:` line is stale until the next accept"
root_cause: "insta writes `assertion_line:` into the `.snap.new` candidate it generates; `cargo insta accept` strips it when promoting, a plain `mv .snap.new .snap` does not. `source:` is recorded relative to the workspace root and only rewrites itself on the next accept."
tags: [insta, snapshot-testing, cargo-insta, assertion-line, metadata, churn, workspace, rust]
related:
  - env-gated-live-tests-pass-vacuously.md
issue: "DEV-7330"
---

# Accept insta snapshots with cargo insta, not by renaming: the rename keeps line-number metadata that churns

## Problem

dsp-cli keeps 333 insta snapshots under `tests/snapshots/`. During the hardening phase a worker
accepted new and changed snapshots by renaming the `.snap.new` files, and the review found the
accepted files carried an `assertion_line:` header while the neighbouring ones did not:

```
---
source: tests/auth_snapshots.rs
assertion_line: 466
expression: buf_to_string(&buf)
---
```

That header names the source line of the `assert_*_snapshot!` call. Any edit above that call in
the test file moves the number, so the snapshot shows up as changed in a diff that touched nothing
it asserts. 26 of the 333 files carried it already from earlier work in the incubator.

A second header line moved for a different reason: after the crate joined the workspace, insta
records `source: dsp-cli/tests/cli.rs` instead of `source: tests/cli.rs`. insta compares content,
not metadata, so nothing failed, but every snapshot's header was stale until its next accept, and
the 34 snapshots the hardening phase legitimately changed all rewrote it at once.

## Investigation

`cargo insta accept` (and `cargo insta review`) promote a `.snap.new` and drop the
`assertion_line:` key; renaming keeps the candidate's full header. The worker had no `cargo insta`
on the path it used and reached for `mv`. The review compared headers across the directory and
found the two populations.

## Root Cause

`assertion_line:` exists so `cargo insta review` can jump to the assertion; it is candidate
metadata, not part of the accepted snapshot. The promotion step is where it is removed, and only
the tool does that step.

## Solution

- The 34 snapshots the phase touched had the key stripped. The 26 pre-existing ones were left
  alone, because the brief allowed snapshot changes only where the change was the point; they are
  a separate cleanup.
- `just dsp-cli-snap-review` wraps `cargo insta review`; the crate's `CLAUDE.md` names it as the
  way to accept. Workers are told to use it, or `cargo insta accept`, and to read the candidate
  before accepting.

## Prevention

- Accept only through `cargo insta accept` / `cargo insta review` (via `just dsp-cli-snap-review`),
  never by renaming. A `.snap` containing `assertion_line:` in a diff is the tell.
- After moving a crate within a workspace, expect one `source:` header rewrite per snapshot on
  its next accept and do not read it as a content change; do not bulk re-accept to "fix" it,
  because that buries the real snapshot changes in the same commit.
- The 26 older files with the key are worth one dedicated cleanup commit some day, so line-number
  churn stops appearing in unrelated diffs.

## Verification

Observed 2026-09-20 on the branch:

```
$ grep -rl 'assertion_line' dsp-cli/tests/snapshots | wc -l
26
$ ls dsp-cli/tests/snapshots/*.snap | wc -l
333
$ head -4 dsp-cli/tests/snapshots/cli__help_top.snap
---
source: dsp-cli/tests/cli.rs
expression: text
---
```

## References

- PR: https://github.com/dasch-swiss/dsp-repository/pull/409, commit `fix(dsp-cli): close the pre-migration security and correctness backlog`
- `dsp-cli/tests/snapshots/`, `justfile` (`dsp-cli-snap-review`), `dsp-cli/CLAUDE.md`
- Journal round 4, "Two snapshot-hygiene findings"
- insta documentation: snapshot file format and `cargo insta accept`
