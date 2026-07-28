#!/usr/bin/env bash
#
# Tests for commit-advisory.sh — the deterministic helpers only (input
# building, response parsing, comment formatting). The API call and PR posting
# are exercised in CI, not here. Needs only bash + git + jq.
#
# Run: bash .github/scripts/commit-advisory.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./commit-advisory.sh disable=SC1091
source "$SCRIPT_DIR/commit-advisory.sh"

PASS=0
FAIL=0

assert_contains() {
  local desc="$1" needle="$2" haystack="$3"
  if [[ "$haystack" == *"$needle"* ]]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected to contain '$needle')"
    echo "      got: $haystack"
  fi
}

assert_eq() {
  local desc="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected '$expected', got '$actual')"
  fi
}

make_repo() {
  local dir; dir="$(mktemp -d)"
  (
    cd "$dir" || exit 1
    git init -q -b base
    git config user.email t@e.com
    git config user.name T
    git commit -q --allow-empty -m "chore(ci): base"
  )
  printf '%s' "$dir"
}

# --- git helpers ----------------------------------------------------------

test_range_and_diffs() {
  local repo out; repo="$(make_repo)"
  out="$(
    cd "$repo" || exit 1
    git checkout -q -b feature
    printf 'line one\n' >a.txt && git add a.txt && git commit -q -m "feat(dpe-web): add a"
    [ "$(git rev-list --count "$(compute_range base)")" = "1" ] || exit 1
    build_diffs "$(compute_range base)"
  )"
  rm -rf "$repo"
  echo "$out"
}
diffs_out="$(test_range_and_diffs)"
assert_contains "diffs: includes the subject"   "feat(dpe-web): add a" "$diffs_out"
assert_contains "diffs: includes patch body"    "+line one"           "$diffs_out"
assert_contains "diffs: names the changed file" "a.txt"               "$diffs_out"

# --- extract_text ---------------------------------------------------------

assert_eq "extract_text: pulls first text block" '{"issues":[]}' \
  "$(extract_text '{"content":[{"type":"text","text":"{\"issues\":[]}"}]}')"

assert_eq "extract_text: empty on API error envelope" "" \
  "$(extract_text '{"type":"error","error":{"type":"overloaded_error","message":"x"}}')"

assert_eq "extract_text: empty on malformed json" "" \
  "$(extract_text 'not json at all')"

# --- issues_count ---------------------------------------------------------

assert_eq "issues_count: zero for empty array" "0" "$(issues_count '[]')"
assert_eq "issues_count: counts entries" "2" "$(issues_count '[{"a":1},{"a":2}]')"

# --- format_comment -------------------------------------------------------

issues='[{"kind":"squash","commits":"abc123, def456","rationale":"both touch the same parser fix","suggestion":"squash into one fix(dpe-core): commit"}]'
comment="$(BASE_REF=origin/main format_comment "$issues")"
assert_contains "comment: hidden marker for edit-last dedup" "<!-- commit-hygiene-advisory -->" "$comment"
assert_contains "comment: advisory framing"                  "Advisory only"                   "$comment"
assert_contains "comment: renders the kind"                  "squash"                          "$comment"
assert_contains "comment: renders the commits"               "abc123, def456"                  "$comment"
assert_contains "comment: renders the rationale"             "same parser fix"                 "$comment"
assert_contains "comment: renders the rebase hint"           "git rebase -i origin/main"       "$comment"

# --- Summary --------------------------------------------------------------

echo
echo "commit-advisory tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
