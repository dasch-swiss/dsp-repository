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

assert_not_contains() {
  local desc="$1" needle="$2" haystack="$3"
  if [[ "$haystack" != *"$needle"* ]]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected NOT to contain '$needle')"
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

# --- build_diffs: per-file cap --------------------------------------------
# A small file keeps its full hunk; a file whose patch exceeds the cap collapses
# to header + marker, and its bulk never reaches the payload.

test_per_file_cap() {
  local repo out; repo="$(make_repo)"
  out="$(
    cd "$repo" || exit 1
    git checkout -q -b feature
    printf 'small change\n' >small.txt
    yes 'a big generated line of dump content xxxxxxxxxxxxxxxx' | head -n 2000 >big.txt
    git add small.txt big.txt
    git commit -q -m "feat(dpe-web): add small and big"
    MAX_FILE_PATCH_BYTES=1024 build_diffs "$(compute_range base)"
  )"
  rm -rf "$repo"
  printf '%s' "$out"
}
cap_out="$(test_per_file_cap)"
assert_contains     "cap: keeps small file's hunk"     "+small change"           "$cap_out"
assert_contains     "cap: still names the big file"    "big.txt"                 "$cap_out"
assert_contains     "cap: marker with change counts"   "[patch omitted: +2000/-0 lines" "$cap_out"
assert_not_contains "cap: big file hunk not inlined"   "+a big generated line"   "$cap_out"

# --- build_diffs: total cap -----------------------------------------------

test_total_cap() {
  local repo out; repo="$(make_repo)"
  out="$(
    cd "$repo" || exit 1
    git checkout -q -b feature
    printf 'a\nb\nc\n' >f.txt && git add f.txt && git commit -q -m "feat(dpe-web): add f"
    MAX_TOTAL_DIFF_BYTES=40 build_diffs "$(compute_range base)"
  )"
  rm -rf "$repo"
  printf '%s' "$out"
}
total_out="$(test_total_cap)"
assert_contains "total cap: appends truncation marker" "[diff truncated" "$total_out"

# --- build_request_body: argv-safe for large diffs ------------------------
# A user string well past the ~128 KB single-argument limit must still build a
# valid request body (regression for the jq "Argument list too long" failure).

big_user="$(yes 'line of diff content to make the payload large' | head -n 5000)"
body_out="$(build_request_body "claude-sonnet-5" 8000 "sys" "$big_user" '{"effort":"medium"}')"
assert_contains "request body: builds with oversized user" '"role": "user"'            "$body_out"
assert_contains "request body: carries the model"          '"model": "claude-sonnet-5"' "$body_out"
assert_eq "request body: user content preserved intact" \
  "$(printf '%s' "$big_user" | wc -c | tr -d ' ')" \
  "$(jq -rj '.messages[0].content' <<<"$body_out" | wc -c | tr -d ' ')"

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
