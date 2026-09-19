#!/usr/bin/env bash
#
# A shared crate must not hardcode a path into a service module; see
# shared/README.md for the rule. Only this half needs a check: a service
# dependency is a Cargo cycle, but a hardcoded path compiles cleanly.
#
# The ../ or modules/ prefix is load-bearing. It matches a path but not a
# "/dpe/projects" route, a docs/src/dpe/... path or `dpe-core` prose, all of
# which are real code in shared/. Widening it would flag those.
#
# Safe to `source`. Dependencies: bash, git, awk.

# Must stay a quoted array. As an unquoted string, bash expands the glob first,
# and bash's * does not cross / where a git pathspec's does, so git would see
# shallow filenames and every nested file would go unread while the gate passed.
# Test fixtures and the scripts that refresh them count: they are shared-crate
# files as able to name a service path as a source file is. Widen this again
# when a shared crate grows a directory neither pathspec reaches.
SHARED_PATHSPECS=('shared/*/src/*.rs' 'shared/*/testdata/**')

# Every module under modules/. From the index, so an untracked file cannot
# widen the rule. Wider than "service": mosaic is the design system and is
# equally off-limits.
non_shared_modules() {
  git ls-files -- modules | awk -F/ 'NF > 2 { print $2 }' | sort -u
}

non_shared_path_pattern() {
  local alternation
  alternation="$(non_shared_modules | paste -sd'|' -)"
  [ -n "$alternation" ] || return 0
  printf '(\\.\\./|modules/)(%s)/' "$alternation"
}

main() {
  local count pattern violations rc

  count="$(git ls-files -- "${SHARED_PATHSPECS[@]}" | wc -l | tr -d '[:space:]')"
  pattern="$(non_shared_path_pattern)"

  # Absence is an error, not zero work: a gate that reads nothing, or forbids
  # nothing, would pass while enforcing nothing.
  [ "$count" -gt 0 ] || { echo "✗ no shared sources matched (${SHARED_PATHSPECS[*]}). Run from the repo root" >&2; return 1; }
  [ -n "$pattern" ] || { echo "✗ no modules found under modules/. Run from the repo root" >&2; return 1; }

  violations="$(git grep -nE "$pattern" -- "${SHARED_PATHSPECS[@]}")" && rc=0 || rc=$?
  # git grep exits 1 for no matches, >1 for a real failure. Only the latter must
  # not be reported as a clean tree.
  [ "$rc" -le 1 ] || { echo "✗ git grep failed; the gate could not run" >&2; return 1; }

  if [ -n "$violations" ]; then
    printf '%s\n\n' "$violations" >&2
    echo "✗ a shared crate hardcodes a path into a service module. Take the path as" >&2
    echo "  a parameter (see load_from(data_dir) in shared-metadata), or move the" >&2
    echo "  code to the crate that owns the data. See shared/README.md." >&2
    return 1
  fi
  echo "✓ shared crates: no hardcoded paths into service modules ($count files checked)"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
