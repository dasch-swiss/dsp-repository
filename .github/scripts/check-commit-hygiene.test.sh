#!/usr/bin/env bash
#
# Tests for check-commit-hygiene.sh.
#
# Dependency-free: needs only bash + git. Unit tests exercise the pure logic
# (override matching, count boundary); integration tests build throwaway git
# repos to exercise the range/fixup/merge checks against real git behaviour.
#
# Run: bash .github/scripts/check-commit-hygiene.test.sh
#      (or `just test-commit-hygiene`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the checker for its functions. SKIP_COG keeps the conventional-commit
# check (an external tool) out of these tests — we test our orchestration, not
# cocogitto's internals.
export SKIP_COG=1
# shellcheck source=./check-commit-hygiene.sh disable=SC1091
source "$SCRIPT_DIR/check-commit-hygiene.sh"

PASS=0
FAIL=0

# assert_ok "desc" cmd...   — expects the command to exit 0
assert_ok() {
  local desc="$1"; shift
  if "$@" >/dev/null 2>&1; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected success, got exit $?)"
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

# assert_eq "desc" expected actual
assert_eq() {
  local desc="$1" expected="$2" actual="$3"
  if [ "$expected" = "$actual" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected '$expected', got '$actual')"
  fi
}

# --- Fixtures -------------------------------------------------------------

# make_repo: create a throwaway repo with a `base` ref (one commit on the
# default branch) and a `feature` branch of N empty commits on top. Echoes
# the repo path. The default branch stays named `base` so tests pass
# BASE_REF=base to compute_range.
make_repo() {
  local dir
  dir="$(mktemp -d)"
  (
    cd "$dir" || exit 1
    git init -q -b base
    git config user.email test@example.com
    git config user.name "Test"
    git commit -q --allow-empty -m "chore: base"
  )
  printf '%s' "$dir"
}

commit_on() {
  # commit_on <repo> <subject> [--branch <name>]
  local repo="$1" subject="$2"
  ( cd "$repo" && git commit -q --allow-empty -m "$subject" )
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
MAX_COMMITS=5

assert_ok   "count: 1 <= 5"                        check_count 1 ""
assert_ok   "count: exactly 5 passes"              check_count 5 ""
assert_fail "count: 6 > 5 fails"                   check_count 6 ""
assert_ok   "count: 6 with override passes"        check_count 6 "allow-many-commits"
assert_ok   "count: 100 with override passes"      check_count 100 "let's allow-many-commits"
assert_fail "count: 6 with UNticked template box still fails" check_count 6 "- [ ] allow-many-commits"
assert_ok   "count: 6 with ticked box passes"      check_count 6 "- [x] allow-many-commits"

# --- Integration: compute_range (merge-base correctness) ------------------

test_range_ignores_base_movement() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    git commit -q --allow-empty -m "feat: f1"
    git commit -q --allow-empty -m "feat: f2"
    git commit -q --allow-empty -m "feat: f3"
    # base moves ahead after the branch point
    git checkout -q base
    git commit -q --allow-empty -m "chore: base2"
    git checkout -q feature
    local range count
    range="$(compute_range base)"
    count="$(git rev-list --count "$range")"
    # range must be merge-base(base,HEAD)..HEAD = only f1,f2,f3 (not base2)
    [ "$count" = "3" ]
  )
  local rc=$?
  rm -rf "$repo"
  return $rc
}
assert_ok "range: excludes commits added to base after branch point" test_range_ignores_base_movement

# --- Integration: check_fixup --------------------------------------------

test_fixup_clean() {
  local repo; repo="$(make_repo)"
  ( cd "$repo" && git checkout -q -b feature \
      && git commit -q --allow-empty -m "feat: a" \
      && check_fixup "$(compute_range base)" )
  local rc=$?; rm -rf "$repo"; return $rc
}
test_fixup_present() {
  local repo; repo="$(make_repo)"
  ( cd "$repo" && git checkout -q -b feature \
      && git commit -q --allow-empty -m "feat: a" \
      && git commit -q --allow-empty -m "fixup! feat: a" \
      && check_fixup "$(compute_range base)" )
  local rc=$?; rm -rf "$repo"; return $rc
}
assert_ok   "fixup: clean branch passes"   test_fixup_clean
assert_fail "fixup: fixup! commit fails"   test_fixup_present

# --- Integration: check_merges -------------------------------------------

test_merges_clean() {
  local repo; repo="$(make_repo)"
  ( cd "$repo" && git checkout -q -b feature \
      && git commit -q --allow-empty -m "feat: a" \
      && check_merges "$(compute_range base)" )
  local rc=$?; rm -rf "$repo"; return $rc
}
test_merges_present() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    git commit -q --allow-empty -m "feat: a"
    git checkout -q base
    git commit -q --allow-empty -m "chore: base2"
    git checkout -q feature
    git merge -q --no-ff -m "Merge branch 'base' into feature" base
    check_merges "$(compute_range base)"
  )
  local rc=$?; rm -rf "$repo"; return $rc
}
assert_ok   "merges: linear branch passes"  test_merges_clean
assert_fail "merges: merge commit fails"    test_merges_present

# --- Integration: main (end to end) --------------------------------------

test_main_clean() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    git commit -q --allow-empty -m "feat: a"
    git commit -q --allow-empty -m "fix: b"
    BASE_REF=base PR_BODY="" main
  )
  local rc=$?; rm -rf "$repo"; return $rc
}
test_main_too_many() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    for i in 1 2 3 4 5 6; do git commit -q --allow-empty -m "feat: c$i"; done
    BASE_REF=base PR_BODY="" main
  )
  local rc=$?; rm -rf "$repo"; return $rc
}
test_main_too_many_override() {
  local repo; repo="$(make_repo)"
  (
    cd "$repo" || exit 1
    git checkout -q -b feature
    for i in 1 2 3 4 5 6; do git commit -q --allow-empty -m "feat: c$i"; done
    BASE_REF=base PR_BODY="allow-many-commits" main
  )
  local rc=$?; rm -rf "$repo"; return $rc
}
assert_ok   "main: clean 2-commit branch passes"        test_main_clean
assert_fail "main: 6 commits, no override, fails"        test_main_too_many
assert_ok   "main: 6 commits with override passes"       test_main_too_many_override

# --- Integration: check_conventional (only when cog is installed) ---------
# The suite is dependency-free by default (SKIP_COG=1). When cog happens to be
# available, also exercise the real conventional-commit path.

if command -v cog >/dev/null 2>&1; then
  # SKIP_COG is cleared inside each subshell so check_conventional runs cog for real.
  test_conv_good() {
    local repo; repo="$(make_repo)"
    (
      cd "$repo" || exit 1
      git checkout -q -b feature
      git commit -q --allow-empty -m "feat: a valid subject"
      SKIP_COG=""
      check_conventional "$(compute_range base)"
    )
    local rc=$?; rm -rf "$repo"; return $rc
  }
  test_conv_bad() {
    local repo; repo="$(make_repo)"
    (
      cd "$repo" || exit 1
      git checkout -q -b feature
      git commit -q --allow-empty -m "no type separator here"
      SKIP_COG=""
      check_conventional "$(compute_range base)"
    )
    local rc=$?; rm -rf "$repo"; return $rc
  }
  test_conv_ignores_fixup() {
    local repo; repo="$(make_repo)"
    (
      cd "$repo" || exit 1
      git checkout -q -b feature
      git commit -q --allow-empty -m "feat: a"
      git commit -q --allow-empty -m "fixup! feat: a"
      SKIP_COG=""
      check_conventional "$(compute_range base)"
    )
    local rc=$?; rm -rf "$repo"; return $rc
  }
  assert_ok   "conventional: valid commit passes (cog)"          test_conv_good
  assert_fail "conventional: non-conventional commit fails (cog)" test_conv_bad
  assert_ok   "conventional: fixup! ignored by cog (own check)"  test_conv_ignores_fixup
else
  echo "• skipping cog-backed conventional tests (cog not installed)"
fi

# --- Summary --------------------------------------------------------------

echo
echo "commit-hygiene tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
