#!/usr/bin/env bash
#
# The deterministic half of the commit-hygiene gate: a PR lands as ONE commit
# unless it explicitly opts out. Message format (type allowlist + mandatory
# scope) is the other half and belongs to commitlint-rs — see .commitlintrc.yml
# and `just commit-lint`.
#
# Only this one rule lives here, because the others are already covered:
#   • fixup!/squash! commits do not parse as Conventional Commits, so
#     commitlint rejects them — including under the override.
#   • merge commits cannot land on main at all: the `main` ruleset enables
#     `required_linear_history`. commitlint skips them by design.
#
# Configuration (environment variables, with defaults):
#   BASE_REF        base to diff against            (default: origin/main)
#   PR_BODY         PR description, for the override (default: empty)
#   MAX_COMMITS     commit-count cap                (default: 1)
#   OVERRIDE_TOKEN  phrase in PR_BODY that lifts the cap
#                                                   (default: allow-many-commits)
#
# The file is safe to `source` (for unit testing): it defines functions and
# only runs `main` when executed directly.

: "${BASE_REF:=origin/main}"
: "${PR_BODY:=}"
: "${MAX_COMMITS:=1}"
: "${OVERRIDE_TOKEN:=allow-many-commits}"

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

# check_count <count> <body>
check_count() {
  local count="$1" body="$2"
  if [ "$count" -le "$MAX_COMMITS" ]; then
    echo "✓ commit count: $count (max $MAX_COMMITS)"
    return 0
  fi
  if has_override "$body"; then
    echo "✓ commit count: $count > $MAX_COMMITS, but '$OVERRIDE_TOKEN' is ticked in the PR body"
    return 0
  fi
  {
    echo "✗ commit count: $count exceeds the maximum of $MAX_COMMITS."
    echo "  This repo rebase-merges, so a PR lands as one commit by default."
    echo "  Squash the branch before merging:"
    echo "      git rebase -i $BASE_REF"
    echo "  Or, if these are genuinely independent changes that each deserve"
    echo "  their own line in main's history, tick the '$OVERRIDE_TOKEN'"
    echo "  checkbox in the PR description (an unticked box does not count)."
  } >&2
  return 1
}

main() {
  local range count
  range="$(compute_range "$BASE_REF")"
  count="$(git rev-list --count "$range")"
  echo "Checking commit count over $range"
  check_count "$count" "$PR_BODY"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
