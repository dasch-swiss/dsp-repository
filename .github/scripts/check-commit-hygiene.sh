#!/usr/bin/env bash
#
# Layer 0 of the commit-hygiene CI: deterministic, reproducible gate over a
# branch's commits. Fails the build (exit 1) with actionable messages when a
# rule is broken, so authors know what to fix and how. Semantic judgement
# (atomicity, "should these squash") is NOT here — that is the advisory layer.
#
# Checks:
#   1. Conventional Commits format ...... via `cog` (cocogitto), if available
#   2. No fixup!/squash! commits ........ leftover autosquash markers
#   3. No merge commits ................. branch must be rebased, not merged
#   4. Commit count <= MAX_COMMITS ...... unless the PR body opts out
#
# Configuration (environment variables, with defaults):
#   BASE_REF        base to diff against            (default: origin/main)
#   PR_BODY         PR description, for the override (default: empty)
#   MAX_COMMITS     commit-count cap                (default: 5)
#   OVERRIDE_TOKEN  phrase in PR_BODY that lifts the cap
#                                                   (default: allow-many-commits)
#   SKIP_COG=1      skip the conventional-commit check (used by the test suite)
#
# The file is safe to `source` (for unit testing): it defines functions and
# only runs `main` when executed directly.

: "${BASE_REF:=origin/main}"
: "${PR_BODY:=}"
: "${MAX_COMMITS:=5}"
: "${OVERRIDE_TOKEN:=allow-many-commits}"
: "${SKIP_COG:=}"

# has_override <body> — true if the opt-out token appears as a standalone word
# on a line that is NOT an unticked checkbox. This is deliberate: the PR template
# seeds an unticked `- [ ] allow-many-commits`, and its mere presence must not
# lift the cap — the author has to tick the box (or write the token in prose).
# Case-insensitive; a leading/trailing alphanumeric (e.g. "disallow-...") does
# not count, so the token must stand on its own.
has_override() {
  printf '%s' "$1" \
    | grep -iE "(^|[^[:alnum:]])${OVERRIDE_TOKEN}([^[:alnum:]]|$)" \
    | grep -qvE '\[[[:space:]]\]'
}

# compute_range <base_ref> — echo "<merge-base>..HEAD" so only the branch's own
# commits are considered, even if the base moved on after the branch point.
compute_range() {
  local base
  base="$(git merge-base "$1" HEAD)"
  printf '%s..HEAD' "$base"
}

# check_conventional <range>
check_conventional() {
  local range="$1"
  if [ "$SKIP_COG" = "1" ]; then
    echo "• conventional commits: skipped (SKIP_COG=1)"
    return 0
  fi
  if ! command -v cog >/dev/null 2>&1; then
    echo "• conventional commits: cog (cocogitto) not installed — skipped locally, CI enforces it" >&2
    return 0
  fi
  # fixup!/squash! and merge commits are handled by dedicated checks below with
  # clearer messages, so cog ignores them and owns only the format rule.
  if cog check --ignore-merge-commits --ignore-fixup-commits "$range" >/dev/null 2>&1; then
    echo "✓ conventional commits"
    return 0
  fi
  {
    echo "✗ one or more commits are not valid Conventional Commits."
    echo "  Run 'cog check --ignore-merge-commits --ignore-fixup-commits $range' to see which."
    echo "  Reword with 'git rebase -i $BASE_REF'."
  } >&2
  return 1
}

# check_fixup <range>
check_fixup() {
  local range="$1" bad
  bad="$(git log --format='%s' "$range" | grep -iE '^(fixup|squash)!' || true)"
  if [ -z "$bad" ]; then
    echo "✓ no fixup!/squash! commits"
    return 0
  fi
  {
    echo "✗ fixup!/squash! commits are still present:"
    printf '%s\n' "$bad" | sed 's/^/      /'
    echo "  Autosquash them before merging:"
    echo "      git rebase -i --autosquash $BASE_REF"
  } >&2
  return 1
}

# check_merges <range>
check_merges() {
  local range="$1" merges
  merges="$(git rev-list --merges "$range")"
  if [ -z "$merges" ]; then
    echo "✓ no merge commits"
    return 0
  fi
  {
    echo "✗ the branch contains merge commits:"
    printf '%s\n' "$merges" | sed 's/^/      /'
    echo "  This repo rebases; do not merge $BASE_REF in. Rebase instead:"
    echo "      git rebase $BASE_REF"
  } >&2
  return 1
}

# check_count <count> <body>
check_count() {
  local count="$1" body="$2"
  if [ "$count" -le "$MAX_COMMITS" ]; then
    echo "✓ commit count: $count (max $MAX_COMMITS)"
    return 0
  fi
  if has_override "$body"; then
    echo "✓ commit count: $count > $MAX_COMMITS, but '$OVERRIDE_TOKEN' is set in the PR body"
    return 0
  fi
  {
    echo "✗ commit count: $count exceeds the maximum of $MAX_COMMITS."
    echo "  Squash working commits before merging:"
    echo "      git rebase -i $BASE_REF"
    echo "  Or, if these are genuinely independent changes, tick the"
    echo "  '$OVERRIDE_TOKEN' checkbox in the PR description (an unticked box does not count)."
  } >&2
  return 1
}

main() {
  local range count failures=0
  range="$(compute_range "$BASE_REF")"
  echo "Checking commit hygiene over $range"
  echo

  check_conventional "$range" || failures=$((failures + 1))
  check_fixup "$range"        || failures=$((failures + 1))
  check_merges "$range"       || failures=$((failures + 1))
  count="$(git rev-list --count "$range")"
  check_count "$count" "$PR_BODY" || failures=$((failures + 1))

  echo
  if [ "$failures" -gt 0 ]; then
    echo "Commit hygiene: FAILED ($failures issue(s) above)."
    return 1
  fi
  echo "Commit hygiene: passed."
  return 0
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
