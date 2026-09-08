#!/usr/bin/env bash
#
# Tests for update-branch.sh.
#
# Dependency-free: needs only bash + git. Every integration test builds a real
# clone of a real (bare) remote and runs the whole script against it, then
# asserts on what the **remote** ends up holding — because the thing that has
# to work is the push, and a test that stopped at the local rebase would pass
# while the push was broken.
#
# Run: bash .github/scripts/update-branch.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPT="$SCRIPT_DIR/update-branch.sh"
# shellcheck source=./update-branch.sh disable=SC1091
source "$SCRIPT"

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

# assert_zero "desc" <rc> / assert_nonzero "desc" <rc> — for a status already in
# hand, where the command had to run earlier so its side effects could be
# inspected. Written as if/else rather than `A && B || C`, which is not
# if-then-else: C also runs when B fails (shellcheck SC2015).
assert_zero() {
  if [ "$2" -eq 0 ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $1 (expected exit 0, got $2)"
  fi
}

assert_nonzero() {
  if [ "$2" -ne 0 ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $1 (expected a non-zero exit, got 0)"
  fi
}

# assert_eq "desc" expected actual
assert_eq() {
  if [ "$2" = "$3" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $1 (expected '$2', got '$3')"
  fi
}

# --- Fixtures -------------------------------------------------------------

# make_remote_and_clone: a bare `origin` whose default branch is `main` with one
# commit, plus a clone of it. Echoes "<clone-dir> <remote-dir>".
#
# A real remote rather than a stub, so `git push --force-with-lease` is the
# real thing — the lease semantics are the part most likely to be wrong.
make_remote_and_clone() {
  local remote clone
  remote="$(mktemp -d)/origin.git"
  clone="$(mktemp -d)/clone"
  git init -q --bare -b main "$remote"
  git clone -q "$remote" "$clone"
  (
    cd "$clone" || exit 1
    git config user.email test@example.com
    git config user.name "Test"
    git commit -q --allow-empty -m "chore(ci): first"
    git push -q origin main
  )
  printf '%s %s' "$clone" "$remote"
}

# advance_main <clone> [file] — add a commit to main on the remote, via a
# second clone, so the branch under test is genuinely behind.
advance_main() {
  local remote_url other
  remote_url="$(git -C "$1" remote get-url origin)"
  other="$(mktemp -d)/other"
  git clone -q "$remote_url" "$other"
  (
    cd "$other" || exit 1
    git config user.email other@example.com
    git config user.name "Other"
    printf 'from main\n' > "${2:-main-file.txt}"
    git add .
    git commit -q -m "chore(ci): main moved on"
    git push -q origin main
  )
  rm -rf "$other"
}

# feature_branch <clone> [file] — a one-commit feature branch, pushed.
feature_branch() {
  (
    cd "$1" || exit 1
    git checkout -q -b feature
    printf 'from feature\n' > "${2:-feature-file.txt}"
    git add .
    git commit -q -m "feat(dpe-web): the feature"
    git push -q --set-upstream origin feature
  )
}

# run_script <clone> — run the script as a subprocess in that clone.
run_script() {
  ( cd "$1" && bash "$SCRIPT" )
}

# remote_log <clone> <ref> — the remote's history for a ref, oldest last.
remote_log() {
  git -C "$1" fetch -q origin
  git -C "$1" log --format=%s "origin/$2"
}

# --- Unit: preconditions --------------------------------------------------

test_detached_head_is_refused() {
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  ( cd "$clone" && git checkout -q --detach HEAD && bash "$SCRIPT" )
  local rc=$?; rm -rf "$clone"; return $rc
}
assert_fail "refuses a detached HEAD" test_detached_head_is_refused

test_the_base_branch_itself_is_refused() {
  # The catastrophic case: rebasing main onto origin/main and force-pushing it.
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  advance_main "$clone"
  run_script "$clone"
  local rc=$?; rm -rf "$clone"; return $rc
}
assert_fail "refuses to run on the base branch" test_the_base_branch_itself_is_refused

test_uncommitted_changes_are_refused() {
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  advance_main "$clone"
  ( cd "$clone" && printf 'edited\n' >> feature-file.txt )
  run_script "$clone"
  local rc=$?; rm -rf "$clone"; return $rc
}
assert_fail "refuses uncommitted changes" test_uncommitted_changes_are_refused

test_untracked_files_are_tolerated() {
  # Rebase does not care about them, and refusing would reject every working
  # tree holding a scratch file.
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  advance_main "$clone"
  ( cd "$clone" && printf 'scratch\n' > untracked.txt )
  run_script "$clone"
  local rc=$?; rm -rf "$clone"; return $rc
}
assert_ok "tolerates untracked files" test_untracked_files_are_tolerated

test_a_rebase_in_progress_is_refused() {
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  # Both sides touch one path, so the rebase stops.
  feature_branch "$clone" shared.txt
  advance_main "$clone" shared.txt
  run_script "$clone" >/dev/null 2>&1   # leaves the rebase in progress
  run_script "$clone"                   # the run under test
  local rc=$?
  ( cd "$clone" && git rebase --abort >/dev/null 2>&1 )
  rm -rf "$clone"; return $rc
}
assert_fail "refuses while a rebase is in progress" test_a_rebase_in_progress_is_refused

# --- Integration: the happy path ------------------------------------------

test_rebases_and_pushes() {
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  advance_main "$clone"
  run_script "$clone" >/dev/null 2>&1 || { rm -rf "$clone"; return 1; }

  # What the remote holds is the assertion: the feature commit replayed on top
  # of main's, and no merge commit anywhere.
  local log merges
  log="$(remote_log "$clone" feature | tr '\n' '|')"
  merges="$(git -C "$clone" log --merges --oneline origin/main..origin/feature | wc -l | tr -d ' ')"
  rm -rf "$clone"
  assert_eq "remote history is the feature commit on top of main's" \
    "feat(dpe-web): the feature|chore(ci): main moved on|chore(ci): first|" "$log"
  assert_eq "no merge commits on the pushed branch" "0" "$merges"
}
test_rebases_and_pushes

test_running_twice_is_a_no_op() {
  # Without the up-to-date check the second run force-pushes an identical
  # branch and re-triggers every required check for nothing.
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  advance_main "$clone"
  run_script "$clone" >/dev/null 2>&1
  local before after
  before="$(git -C "$clone" rev-parse HEAD)"
  run_script "$clone" >/dev/null 2>&1
  local rc=$?
  after="$(git -C "$clone" rev-parse HEAD)"
  rm -rf "$clone"
  assert_eq "the second run succeeds and changes nothing" "$before" "$after"
  assert_zero "the second run exits 0" "$rc"
}
test_running_twice_is_a_no_op

test_a_branch_carrying_a_merge_commit_comes_out_linear() {
  # The shape the "Update branch" button produces, which is the whole reason
  # this script exists.
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  advance_main "$clone"
  ( cd "$clone" && git fetch -q origin && git merge -q --no-ff origin/main -m "Merge branch 'main' into feature" \
      && git push -q origin feature )
  local before_merges
  before_merges="$(git -C "$clone" log --merges --oneline origin/main..HEAD | wc -l | tr -d ' ')"
  run_script "$clone" >/dev/null 2>&1
  local after_merges
  after_merges="$(git -C "$clone" log --merges --oneline origin/main..origin/feature | wc -l | tr -d ' ')"
  rm -rf "$clone"
  assert_eq "the fixture really had a merge commit" "1" "$before_merges"
  assert_eq "the pushed branch has none" "0" "$after_merges"
}
test_a_branch_carrying_a_merge_commit_comes_out_linear

test_an_unpushed_branch_is_pushed_without_a_force() {
  # `--force-with-lease` has nothing to compare against for a branch the remote
  # has never seen, and would refuse it.
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  advance_main "$clone"
  (
    cd "$clone" || exit 1
    git checkout -q -b local-only
    printf 'local\n' > local-file.txt
    git add . && git commit -q -m "feat(dpe-web): never pushed"
  )
  run_script "$clone" >/dev/null 2>&1 || { rm -rf "$clone"; return 1; }
  local log; log="$(remote_log "$clone" local-only | head -1)"
  rm -rf "$clone"
  assert_eq "the branch reaches the remote" "feat(dpe-web): never pushed" "$log"
}
test_an_unpushed_branch_is_pushed_without_a_force

test_a_conflict_stops_and_pushes_nothing() {
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone" shared.txt
  advance_main "$clone" shared.txt
  local before; before="$(git -C "$clone" rev-parse origin/feature)"
  run_script "$clone" >/dev/null 2>&1
  local rc=$?
  git -C "$clone" fetch -q origin
  local after; after="$(git -C "$clone" rev-parse origin/feature)"
  local in_progress=1
  ( cd "$clone" && rebase_in_progress ) && in_progress=0
  ( cd "$clone" && git rebase --abort >/dev/null 2>&1 )
  rm -rf "$clone"
  assert_nonzero "a conflict exits non-zero" "$rc"
  assert_eq "the remote branch is untouched by a failed rebase" "$before" "$after"
  assert_eq "the rebase is left in progress for the author to finish" "0" "$in_progress"
}
test_a_conflict_stops_and_pushes_nothing

test_a_lease_refusal_does_not_overwrite_somebody_else() {
  # Somebody pushes to the branch after our fetch. `--force-with-lease` has to
  # refuse; a plain `--force` would destroy their commit.
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  advance_main "$clone"
  # Rebase locally, then let somebody else push before we do.
  ( cd "$clone" && git fetch -q origin && git rebase -q origin/main )
  local remote_url other
  remote_url="$(git -C "$clone" remote get-url origin)"
  other="$(mktemp -d)/other"
  git clone -q -b feature "$remote_url" "$other"
  (
    cd "$other" || exit 1
    git config user.email other@example.com
    git config user.name "Other"
    printf 'theirs\n' > theirs.txt
    git add . && git commit -q -m "feat(dpe-web): somebody else's work"
    git push -q origin feature
  )
  # The script's own fetch will see their commit, so make the branch behind
  # again to get past the up-to-date check, then run it.
  advance_main "$clone" second-main-file.txt
  run_script "$clone" >/dev/null 2>&1
  local rc=$?
  git -C "$clone" fetch -q origin
  local survived; survived="$(git -C "$clone" log --format=%s origin/feature | grep -c "somebody else")"
  rm -rf "$clone" "$other"
  assert_nonzero "the push is refused" "$rc"
  assert_eq "their commit survives on the remote" "1" "$survived"
}
test_a_lease_refusal_does_not_overwrite_somebody_else

test_progress_messages_name_the_remote_and_the_base() {
  # `$REMOTE…` reads the ellipsis's first byte as part of the variable name, so
  # bash expands an unset variable and prints "Fetching " plus two stray bytes
  # with the remote name gone. It is a message-only fault, which is exactly why
  # it shipped once — every functional test still passed.
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  advance_main "$clone"
  local out; out="$(run_script "$clone" 2>&1)"
  rm -rf "$clone"
  case "$out" in
    *"Fetching origin"*) PASS=$((PASS + 1)) ;;
    *) FAIL=$((FAIL + 1)); echo "FAIL: the fetch line should name the remote, got: $out" ;;
  esac
  case "$out" in
    *"onto origin/main"*) PASS=$((PASS + 1)) ;;
    *) FAIL=$((FAIL + 1)); echo "FAIL: the rebase line should name the base, got: $out" ;;
  esac
}
test_progress_messages_name_the_remote_and_the_base

# --- Integration: the merged-branch cases -----------------------------------
#
# Both were found by running the script against a real remote after the branch
# it was on had been merged and deleted. The unit fixtures could not reach
# either, and the script's behaviour in both was wrong: it flattened the branch
# to the base and then reported "somebody else pushed to this branch".

test_a_branch_deleted_upstream_is_reported_and_left_alone() {
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  advance_main "$clone"
  # What a rebase-merge plus "delete branch" leaves behind: the commits are on
  # main under new SHAs, and the branch is gone from the remote.
  local remote_url other
  remote_url="$(git -C "$clone" remote get-url origin)"
  other="$(mktemp -d)/other"
  git clone -q "$remote_url" "$other"
  (
    cd "$other" || exit 1
    git config user.email other@example.com
    git config user.name "Other"
    # `git cherry-pick` has no -q; the patch has to land for real, or main does
    # not actually carry the branch's work and the fixture proves nothing.
    git cherry-pick "$(git -C "$clone" rev-parse feature)" >/dev/null
    git push -q origin main
    git push -q origin --delete feature
  )
  rm -rf "$other"

  local before; before="$(git -C "$clone" rev-parse feature)"
  run_script "$clone" >/dev/null 2>&1
  local rc=$?
  local after; after="$(git -C "$clone" rev-parse feature)"
  rm -rf "$clone"
  assert_nonzero "a branch deleted upstream exits non-zero" "$rc"
  assert_eq "the local branch is left exactly as it was" "$before" "$after"
}
test_a_branch_deleted_upstream_is_reported_and_left_alone

test_a_rebase_that_drops_every_commit_pushes_nothing() {
  # The branch is still on the remote, but its commits are already on main —
  # a merge that kept the branch. Force-pushing the flattened result would
  # replace the branch's history with nothing.
  local dirs clone; dirs="$(make_remote_and_clone)"; clone="${dirs%% *}"
  feature_branch "$clone"
  local remote_url other
  remote_url="$(git -C "$clone" remote get-url origin)"
  other="$(mktemp -d)/other"
  git clone -q "$remote_url" "$other"
  (
    cd "$other" || exit 1
    git config user.email other@example.com
    git config user.name "Other"
    git cherry-pick "$(git -C "$clone" rev-parse feature)" >/dev/null
    git commit -q --allow-empty -m "chore(ci): main moved on"
    git push -q origin main
  )
  rm -rf "$other"

  local before; before="$(git -C "$clone" rev-parse origin/feature)"
  run_script "$clone" >/dev/null 2>&1
  local rc=$?
  git -C "$clone" fetch -q origin
  local after; after="$(git -C "$clone" rev-parse origin/feature)"
  rm -rf "$clone"
  assert_nonzero "an emptied rebase exits non-zero" "$rc"
  assert_eq "the remote branch is not replaced with nothing" "$before" "$after"
}
test_a_rebase_that_drops_every_commit_pushes_nothing

# --- Summary --------------------------------------------------------------

echo
echo "update-branch tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
