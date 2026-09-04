#!/usr/bin/env bash
#
# Bring the current branch up to date with the base by REBASING, then push.
#
# `main`'s ruleset requires a branch to be up to date before merging, and it
# also requires linear history — so a branch has to be updated, and the only
# update that can then land is a rebase. GitHub's "Update branch" button
# defaults to merging the base in instead, there is no repository setting that
# changes that default, and the commit gate therefore rejects the result. This
# is the correct update in one command, so nobody has to remember the chevron
# next to that button.
#
# It refuses rather than guesses. Every precondition below is a state where a
# rebase-and-force-push would either fail confusingly or destroy something:
#
#   • detached HEAD          — there is no branch to push
#   • the base branch itself — rebasing `main` onto itself then force-pushing it is catastrophic
#   • a rebase in progress   — the tree is already half-rewritten
#   • uncommitted changes    — a rebase cannot run, and this is checked before anything is fetched
#   • the branch is gone from the remote — it has been merged and deleted, so rebasing it would
#     flatten it to the base and the push would fail on a lease against a ref that no longer exists
#   • the rebase left no commits         — every one is already on the base, so there is nothing to push
#
# A conflict leaves the rebase in progress on purpose. It means the branch and
# the base genuinely disagree, which is work only the author can do; aborting
# for them would throw away a resolution already half-made and hide the
# disagreement. Nothing is pushed in that case.
#
# Configuration (environment variables, with defaults):
#   BASE_REF   the base to rebase onto      (default: origin/main)
#   REMOTE     the remote to push to        (default: origin)
#
# A variable immediately followed by a multi-byte character is always braced —
# `${REMOTE}…`, never `$REMOTE…`. Bash reads the ellipsis's first byte as part
# of the name, expands the unset variable that results, and prints the
# remaining bytes as mojibake: "Fetching " and two stray bytes, with the remote
# name gone. It is a message-only fault, which is what makes it easy to ship.
#
# The file is safe to `source` (for unit testing): it defines functions and
# only runs `main` when executed directly.

: "${BASE_REF:=origin/main}"
: "${REMOTE:=origin}"

# current_branch — echo the checked-out branch, or fail on a detached HEAD.
current_branch() {
  git symbolic-ref --quiet --short HEAD
}

# base_branch — the branch name inside BASE_REF: `main` from `origin/main`.
base_branch() {
  printf '%s' "${BASE_REF#"$REMOTE"/}"
}

# has_tracked_changes — true when a rebase could not run.
#
# Tracked modifications only, staged or not. Untracked files are deliberately
# tolerated: `git rebase` does not care about them unless a replayed commit
# needs the same path, and refusing on them would reject every working tree
# holding a scratch file or an unignored build artifact.
has_tracked_changes() {
  ! git diff --quiet || ! git diff --cached --quiet
}

# rebase_in_progress — true while a rebase is stopped part-way.
rebase_in_progress() {
  local dir
  dir="$(git rev-parse --git-path rebase-merge)"
  [ -d "$dir" ] && return 0
  dir="$(git rev-parse --git-path rebase-apply)"
  [ -d "$dir" ]
}

# up_to_date — true when the base is already an ancestor of the branch.
up_to_date() {
  git merge-base --is-ancestor "$BASE_REF" HEAD
}

# has_merge_commits — true when the branch carries a merge, which is what the
# "Update branch" button leaves behind.
has_merge_commits() {
  [ -n "$(git rev-list --merges "$BASE_REF..HEAD")" ]
}

# nothing_to_do — up to date AND linear.
#
# Both halves, and the second is the one that matters: a branch the button has
# already been pressed on *is* up to date — the base is an ancestor, through
# the merge commit — so an up-to-date check alone reports "nothing to do" on
# exactly the branch this script exists to fix. Being up to date is also not
# enough on its own to skip the push: without the linearity half, running this
# after the button leaves the merge commit in place and the gate still red.
#
# The linear half alone would be wrong the other way: without the up-to-date
# check, a second run force-pushes an identical branch and re-triggers every
# required check for nothing.
nothing_to_do() {
  up_to_date && ! has_merge_commits
}

# remote_has_branch <branch> — whether the branch exists on the remote, so the
# push knows whether it has a lease to compare against.
remote_has_branch() {
  git rev-parse --verify --quiet "refs/remotes/$REMOTE/$1" >/dev/null
}

main() {
  local branch
  if ! branch="$(current_branch)"; then
    echo "✗ HEAD is detached, so there is no branch to update." >&2
    return 1
  fi
  if [ "$branch" = "$(base_branch)" ]; then
    echo "✗ Refusing to run on '$branch', which is the base branch itself." >&2
    echo "  Check out the branch you want to update first." >&2
    return 1
  fi
  if rebase_in_progress; then
    echo "✗ A rebase is already in progress." >&2
    echo "  Finish it with 'git rebase --continue' or drop it with 'git rebase --abort'." >&2
    return 1
  fi
  # Before the fetch, so a refusal leaves the repository exactly as it was.
  if has_tracked_changes; then
    echo "✗ There are uncommitted changes, and a rebase cannot run over them." >&2
    echo "  Commit or stash them first." >&2
    return 1
  fi

  # Captured BEFORE the fetch, because that is what the lease has to be taken
  # against — see the push below.
  local seen=""
  if remote_has_branch "$branch"; then
    seen="$(git rev-parse "refs/remotes/$REMOTE/$branch")"
  fi

  # `--prune` so a branch deleted on the remote stops looking present: the
  # check below is what turns a merged-and-deleted branch into a clear message
  # instead of a rebase that flattens it and a lease failure nobody can read.
  echo "Fetching ${REMOTE}…"
  if ! git fetch --quiet --prune "$REMOTE"; then
    echo "✗ Could not fetch from '$REMOTE'." >&2
    return 1
  fi
  if ! git rev-parse --verify --quiet "$BASE_REF" >/dev/null; then
    echo "✗ '$BASE_REF' does not exist." >&2
    return 1
  fi

  # After the fetch, so it reflects the remote as it is now. A branch we had
  # seen and can no longer see was deleted upstream — almost always because the
  # PR was merged. Stopping here matters: the rebase would drop every commit as
  # "previously applied", leaving the branch silently equal to the base.
  if [ -n "$seen" ] && ! remote_has_branch "$branch"; then
    echo "✗ '$REMOTE/$branch' no longer exists — the branch was deleted upstream," >&2
    echo "  which usually means the pull request was merged. Nothing was changed." >&2
    echo "  If you are done with it: git checkout $(base_branch) && git branch -D $branch" >&2
    return 1
  fi

  if nothing_to_do; then
    echo "✓ '$branch' is already up to date with $BASE_REF and linear — nothing to do."
    return 0
  fi

  echo "Rebasing '$branch' onto ${BASE_REF}…"
  if ! git rebase "$BASE_REF"; then
    echo >&2
    echo "✗ The rebase stopped, so nothing was pushed." >&2
    echo "  Resolve the conflicts, 'git add' them and run 'git rebase --continue'," >&2
    echo "  then run this again. To give up entirely: 'git rebase --abort'." >&2
    return 1
  fi

  # A rebase that dropped everything: every commit was already on the base, so
  # the branch now *is* the base. Pushing that would replace the branch's
  # history on the remote with nothing, and there is nothing to review either.
  if [ "$(git rev-list --count "$BASE_REF..HEAD")" -eq 0 ]; then
    echo "✗ The rebase left no commits: every one is already on $BASE_REF." >&2
    echo "  Nothing was pushed. The branch has most likely been merged." >&2
    return 1
  fi

  # A branch that has never been pushed has no lease to compare against, so
  # `--force-with-lease` would refuse it. It also needs no force.
  if ! remote_has_branch "$branch"; then
    echo "Pushing '$branch' to $REMOTE for the first time…"
    git push --quiet --set-upstream "$REMOTE" "$branch" || return 1
    echo "✓ '$branch' is rebased onto $BASE_REF and pushed."
    return 0
  fi

  # The lease is taken against the commit this repository had seen for the
  # branch **before** the fetch above, never against the bare form.
  #
  # Bare `--force-with-lease` compares against the remote-tracking ref, and this
  # script refreshes that ref two lines earlier — so the lease would be taken
  # against a state nobody has looked at, and a colleague's push to the same
  # branch would be overwritten silently. That is the whole failure the lease
  # exists to prevent, and it is what the bare form does here.
  echo "Pushing '$branch' to ${REMOTE}…"
  if ! git push --quiet --force-with-lease="$branch:$seen" "$REMOTE" "$branch"; then
    echo >&2
    echo "✗ The push was refused: '$REMOTE/$branch' has moved since this" >&2
    echo "  repository last saw it. Somebody else pushed to this branch — look" >&2
    echo "  at what they did before overwriting it." >&2
    return 1
  fi
  echo "✓ '$branch' is rebased onto $BASE_REF and force-pushed."
}

# Only run when executed, so the tests can source the functions.
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  main "$@"
fi
