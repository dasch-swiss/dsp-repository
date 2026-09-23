#!/usr/bin/env bash
#
# Datastar RC.6 renamed the attribute delimiter from `-` to `:`. The pre-RC.6
# forms (data-on-, data-attr-, data-class-, data-style-) are inert: the browser
# logs a console error and the control does nothing, while the element still
# renders and still passes any snapshot test asserting the attribute is present.
# Nothing else in the toolchain catches this — Maud accepts either spelling.
#
# The trailing `=` is load-bearing. It matches an attribute *assignment* and so
# skips `assert!(!out.contains("data-on-submit"))`, the negative assertions that
# pin this very rule in the page tests. Without it the gate would fire on the
# tests that exist to prevent the bug. `"?` before it keeps the quoted form
# (`"data-on-change__debounce.1s"=`) in scope, which Maud requires for dotted
# names and which is where a real slip is most likely.
#
# Safe to `source`. Dependencies: bash, git.

# Must stay a quoted array, so git expands the glob rather than bash — bash's *
# does not cross `/` where a git pathspec's does, and an expanded-too-early glob
# silently reduces the gate to top-level files.
MAUD_PATHSPECS=('modules/*/*/src/*.rs')

DELIMITER_PATTERN='data-(on|attr|class|style)-[A-Za-z][A-Za-z0-9_.:-]*"?='

main() {
  local count violations rc

  count="$(git ls-files -- "${MAUD_PATHSPECS[@]}" | wc -l | tr -d '[:space:]')"

  # Absence is an error, not zero work: a gate that reads nothing would pass
  # while enforcing nothing.
  [ "$count" -gt 0 ] || { echo "✗ no Maud sources matched (${MAUD_PATHSPECS[*]}). Run from the repo root" >&2; return 1; }

  violations="$(git grep -nE "$DELIMITER_PATTERN" -- "${MAUD_PATHSPECS[@]}")" && rc=0 || rc=$?
  # git grep exits 1 for no matches, >1 for a real failure. Only the latter must
  # not be reported as a clean tree.
  [ "$rc" -le 1 ] || { echo "✗ git grep failed; the gate could not run" >&2; return 1; }

  if [ -n "$violations" ]; then
    printf '%s\n\n' "$violations" >&2
    echo "✗ a Maud template uses the pre-RC.6 Datastar hyphen delimiter. Write the" >&2
    echo "  attribute with a colon instead (data-on:click, data-attr:disabled)." >&2
    echo "  The hyphen form renders fine and does nothing. See modules/editor/CLAUDE.md" >&2
    echo "  → Common Pitfalls → Datastar attribute syntax." >&2
    return 1
  fi
  echo "✓ Datastar attributes: no pre-RC.6 hyphen delimiters ($count files checked)"

  # `retry: 'never'` covers only HTTP statuses: a network failure is still
  # retried for about three minutes before `retries-failed`, which is what a
  # no-JS fallback listens for. Only `retryMaxCount: 0` stops that.
  violations="$(git grep -n "retry: 'never'" -- "${MAUD_PATHSPECS[@]}")" && rc=0 || rc=$?
  [ "$rc" -le 1 ] || { echo "✗ git grep failed; the gate could not run" >&2; return 1; }
  violations="$(printf '%s\n' "$violations" | grep -v "retryMaxCount: 0")" || true
  if [ -n "$violations" ]; then
    printf '%s\n\n' "$violations" >&2
    echo "✗ a Datastar action sets retry: 'never' without retryMaxCount: 0, so a network" >&2
    echo "  failure still retries for minutes. See docs/src/dpe/architecture.md" >&2
    echo "  → Datastar Attribute Conventions." >&2
    return 1
  fi
  echo "✓ Datastar actions: every retry: 'never' also sets retryMaxCount: 0"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
