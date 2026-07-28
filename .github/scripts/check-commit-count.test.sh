#!/usr/bin/env bash
#
# Tests for check-commit-count.sh.
#
# Dependency-free: needs only bash + git. The unit tests cover the two things
# that actually carry risk — tick-aware override matching and the count
# boundary; the integration tests build throwaway git repos so merge-base
# behaviour is exercised against real git.
#
# Message-format rules are commitlint's, not ours, and are not tested here.
#
# Run: bash .github/scripts/check-commit-count.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-commit-count.sh disable=SC1091
source "$SCRIPT_DIR/check-commit-count.sh"

PASS=0
FAIL=0

# assert_ok "desc" cmd...   — expects the command to exit 0
assert_ok() {
  local desc="$1"; shift
  if "$@" >/dev/null 2>&1; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected success)"
  fi
}

# assert_fail "desc" cmd... — expects the command to exit non-zero
assert_fail() {
  local desc="$1"; shift
  if "$@" >/dev/null 2>&1; then
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected failure, got success)"
  else
    PASS=$((PASS + 1))
  fi
}

# --- Fixtures -------------------------------------------------------------

# make_repo: throwaway repo whose default branch is `base` with one commit.
# Tests pass BASE_REF=base to compute_range.
make_repo() {
  local dir
  dir="$(mktemp -d)"
  (
    cd "$dir" || exit 1
    git init -q -b base
    git config user.email test@example.com
    git config user.name "Test"
    git commit -q --allow-empty -m "chore(ci): base"
  )
  printf '%s' "$dir"
}

# --- Unit: has_override ---------------------------------------------------

# Consumed by has_override in the sourced script, not directly here.
# shellcheck disable=SC2034
OVERRIDE_TOKEN="allow-many-commits"

assert_fail "override: empty body"                 has_override ""
assert_ok   "override: exact token"                has_override "allow-many-commits"
assert_ok   "override: case-insensitive"           has_override "Allow-Many-Commits"
assert_ok   "override: token within prose"         has_override "please allow-many-commits for this PR"
assert_ok   "override: ticked checkbox"            has_override "- [x] allow-many-commits"
assert_ok   "override: ticked checkbox uppercase"  has_override "- [X] allow-many-commits"
assert_fail "override: UNticked checkbox (template default)" has_override "- [ ] allow-many-commits"
assert_ok   "override: ticked box amid template"   has_override "$(printf '## Summary\n- [ ] other thing\n- [x] allow-many-commits\n')"
assert_fail "override: only unticked box present"  has_override "$(printf '## Summary\nstuff\n- [ ] allow-many-commits\n')"
assert_fail "override: prefixed word (disallow-)"  has_override "disallow-many-commits-please"
assert_fail "override: no hyphens"                 has_override "allowmanycommits"

# --- Unit: check_count ----------------------------------------------------

# Consumed by check_count in the sourced script, not directly here.
# shellcheck disable=SC2034
MAX_COMMITS=1

assert_ok   "count: 1 is the default maximum"      check_count 1 ""
assert_fail "count: 2 fails without override"      check_count 2 ""
assert_ok   "count: 2 with override passes"        check_count 2 "allow-many-commits"
assert_ok   "count: 20 with override passes"       check_count 20 "let's allow-many-commits"
assert_fail "count: 2 with UNticked template box still fails" check_count 2 "- [ ] allow-many-commits"
assert_ok   "count: 2 with ticked box passes"      check_count 2 "- [x] allow-many-commits"

# --- Integration: compute_range (merge-base correctness) ------------------

test_range_ignores_base_movement() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    git commit -q --allow-empty -m "feat(dpe-web): f1"
    git commit -q --allow-empty -m "feat(dpe-web): f2"
    # base moves ahead after the branch point
    git checkout -q base
    git commit -q --allow-empty -m "chore(ci): base2"
    git checkout -q feature
    # range must be merge-base(base,HEAD)..HEAD = only f1,f2 (not base2)
    [ "$(git rev-list --count "$(compute_range base)")" = "2" ]
  )
  local rc=$?
  rm -rf "$repo"
  return $rc
}
assert_ok "range: excludes commits added to base after branch point" test_range_ignores_base_movement

# --- Integration: main (end to end) --------------------------------------

test_main_single_commit() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    git commit -q --allow-empty -m "feat(dpe-web): a"
    BASE_REF=base PR_BODY="" MAX_COMMITS=1 main
  )
  local rc=$?; rm -rf "$repo"; return $rc
}
test_main_two_commits() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    git commit -q --allow-empty -m "feat(dpe-web): a"
    git commit -q --allow-empty -m "fix(dpe-web): b"
    BASE_REF=base PR_BODY="" MAX_COMMITS=1 main
  )
  local rc=$?; rm -rf "$repo"; return $rc
}
test_main_two_commits_override() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    git commit -q --allow-empty -m "feat(dpe-web): a"
    git commit -q --allow-empty -m "fix(dpe-web): b"
    BASE_REF=base PR_BODY="- [x] allow-many-commits" MAX_COMMITS=1 main
  )
  local rc=$?; rm -rf "$repo"; return $rc
}
assert_ok   "main: single-commit branch passes"          test_main_single_commit
assert_fail "main: 2 commits, no override, fails"        test_main_two_commits
assert_ok   "main: 2 commits with ticked override passes" test_main_two_commits_override

# --- Summary --------------------------------------------------------------

echo
echo "commit-count tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
