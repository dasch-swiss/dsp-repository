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
    git commit -q --allow-empty -m "chore: base"
  )
  printf '%s' "$dir"
}

# --- git helpers ----------------------------------------------------------

test_range_and_summary() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    echo hello >a.txt && git add a.txt && git commit -q -m "feat: add a"
    git commit -q --allow-empty -m "fix: tweak"
    local range summary
    range="$(compute_range base)"
    [ "$(git rev-list --count "$range")" = "2" ] || exit 1
    summary="$(build_commit_summary "$range")"
    [[ "$summary" == *"feat: add a"* ]] || exit 1
    [[ "$summary" == *"fix: tweak"* ]] || exit 1
    [[ "$summary" == *"a.txt"* ]] || exit 1   # --stat lists the file
    [[ "$summary" != *"+hello"* ]] || exit 1  # but not the diff body
  )
  local rc=$?; rm -rf "$repo"; return $rc
}
if test_range_and_summary; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); echo "FAIL: range + commit summary (messages, stats, no diff)"; fi

test_diffs_include_patch() {
  local repo out; repo="$(make_repo)"
  out="$(
    cd "$repo" || exit 1
    git checkout -q -b feature
    printf 'line one\n' >a.txt && git add a.txt && git commit -q -m "feat: add a"
    build_diffs "$(compute_range base)"
  )"
  rm -rf "$repo"
  echo "$out"
}
diffs_out="$(test_diffs_include_patch)"
assert_contains "diffs: includes patch body" "+line one" "$diffs_out"

# --- extract_text ---------------------------------------------------------

assert_eq "extract_text: pulls first text block" '{"flagged":false,"issues":[]}' \
  "$(extract_text '{"content":[{"type":"text","text":"{\"flagged\":false,\"issues\":[]}"}]}')"

assert_eq "extract_text: empty on API error envelope" "" \
  "$(extract_text '{"type":"error","error":{"type":"overloaded_error","message":"x"}}')"

assert_eq "extract_text: empty on malformed json" "" \
  "$(extract_text 'not json at all')"

# --- issues_count ---------------------------------------------------------

assert_eq "issues_count: zero for empty array" "0" "$(issues_count '[]')"
assert_eq "issues_count: counts entries" "2" "$(issues_count '[{"a":1},{"a":2}]')"

# --- format_comment: haiku ------------------------------------------------

haiku_issues='[{"commit":"abc123","problem":"placeholder message \"fix: stuff\"","suggestion":"describe the actual change"}]'
haiku_comment="$(BASE_REF=origin/main format_comment haiku "$haiku_issues")"
assert_contains "haiku comment: hidden marker for edit-last dedup" "<!-- commit-hygiene-advisory -->" "$haiku_comment"
assert_contains "haiku comment: does not block" "does **not** block" "$haiku_comment"
assert_contains "haiku comment: renders the commit" "abc123" "$haiku_comment"
assert_contains "haiku comment: renders the fix" "describe the actual change" "$haiku_comment"

# --- format_comment: sonnet -----------------------------------------------

sonnet_issues='[{"kind":"squash","commits":"abc123, def456","rationale":"both touch the same parser fix","suggestion":"squash into one fix: commit"}]'
sonnet_comment="$(BASE_REF=origin/main format_comment sonnet "$sonnet_issues")"
assert_contains "sonnet comment: hidden marker" "<!-- commit-hygiene-advisory -->" "$sonnet_comment"
assert_contains "sonnet comment: advisory framing" "Advisory only" "$sonnet_comment"
assert_contains "sonnet comment: renders the kind" "squash" "$sonnet_comment"
assert_contains "sonnet comment: renders the rationale" "same parser fix" "$sonnet_comment"

# --- Summary --------------------------------------------------------------

echo
echo "commit-advisory tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
