#!/usr/bin/env bash
#
# Every ADR reference in the repository must resolve to a record. A bare
# `ADR-NNNN` names a root ADR (`docs/adr/NNNN-*.md`); a qualified
# `<component>/ADR-NNNN` names a component ADR (`<component>/docs/adr/NNNN-*.md`).
# See docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md for
# the rule this enforces. A wrong citation reads fine in prose and compiles
# fine as code, so only a grep over the whole tree catches a renumbered or
# never-written record.
#
# docs/specs/** is excluded: specs and journals are point-in-time records that
# cite intended future state by design, including decision records that do
# not exist yet, so a dangling reference there is correct rather than broken
# (ADR-0006's second Consequences clause).
#
# `git grep -P` is used deliberately: it is available on macOS and Linux
# alike, where BSD `grep -P` is not. `-I` skips binary files; `git ls-files`
# semantics keep `target/` out because it is untracked.
#
# Safe to `source`. Dependencies: bash, git.

# The negative lookbehind is what keeps a qualified reference's trailing
# "ADR-NNNN" from also being counted as a bare one: without it, the bare
# pattern would match that tail on every qualified reference too, and a
# qualified citation with an unresolvable bare reading would double-report or,
# worse, resolve against the wrong series.
BARE_PATTERN='(?<![/\w])ADR-\d{4}\b'
QUALIFIED_PATTERN='(?<![\w/.-])[\w.-]+(?:/[\w.-]+)*/ADR-\d{4}\b'

# resolves <glob>: true if <glob> matches at least one file in the working
# tree. New files must be staged for git grep to see them at all, so the
# working-tree glob and git grep agree once the change is staged.
resolves() {
  local -a matches
  shopt -s nullglob
  matches=($1)
  shopt -u nullglob
  [ "${#matches[@]}" -gt 0 ]
}

main() {
  local bare_matches qualified_matches bare_count=0 qualified_count=0
  local entry rest file line ref num dir unresolved=0 rc

  # Absence is an error, not zero work: a gate that finds no root ADR at all
  # would pass on a repository where the convention was never followed.
  [ -n "$(git ls-files -- 'docs/adr/*.md')" ] || {
    echo "✗ no root ADRs found under docs/adr/. Run from the repo root" >&2
    return 1
  }

  # git grep exits 1 for "no matches" and >1 for a real failure. Only the
  # latter must not be reported as a clean tree.
  bare_matches="$(git grep -InoP "$BARE_PATTERN" -- . ':!docs/specs')" && rc=0 || rc=$?
  [ "$rc" -le 1 ] || { echo "✗ git grep failed; the gate could not run" >&2; return 1; }

  qualified_matches="$(git grep -InoP "$QUALIFIED_PATTERN" -- . ':!docs/specs')" && rc=0 || rc=$?
  [ "$rc" -le 1 ] || { echo "✗ git grep failed; the gate could not run" >&2; return 1; }

  if [ -n "$bare_matches" ]; then
    while IFS= read -r entry; do
      [ -n "$entry" ] || continue
      bare_count=$((bare_count + 1))
      file="${entry%%:*}"
      rest="${entry#*:}"
      line="${rest%%:*}"
      ref="${rest#*:}"
      num="${ref#ADR-}"
      resolves "docs/adr/${num}-*.md" || {
        echo "$file:$line: $ref -- no docs/adr/${num}-*.md at the repository root" >&2
        unresolved=$((unresolved + 1))
      }
    done <<<"$bare_matches"
  fi

  if [ -n "$qualified_matches" ]; then
    while IFS= read -r entry; do
      [ -n "$entry" ] || continue
      qualified_count=$((qualified_count + 1))
      file="${entry%%:*}"
      rest="${entry#*:}"
      line="${rest%%:*}"
      ref="${rest#*:}"
      dir="${ref%/ADR-*}"
      num="${ref##*ADR-}"
      resolves "${dir}/docs/adr/${num}-*.md" || {
        echo "$file:$line: $ref -- no ${dir}/docs/adr/${num}-*.md" >&2
        unresolved=$((unresolved + 1))
      }
    done <<<"$qualified_matches"
  fi

  if [ "$unresolved" -gt 0 ]; then
    echo >&2
    echo "✗ $unresolved ADR reference(s) do not resolve to a record. See docs/adr/0006-decision-records-are-colocated-and-cited-qualified.md." >&2
    return 1
  fi

  echo "✓ ADR references: $bare_count bare, $qualified_count qualified, all resolve"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
