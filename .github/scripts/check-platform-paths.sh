#!/usr/bin/env bash
#
# A platform crate must not hardcode a path into another module; see
# modules/platform/README.md for the rule. Only this half needs a check: a
# service dependency is a Cargo cycle, but a hardcoded path compiles cleanly.
#
# The ../ or modules/ prefix is load-bearing. It matches a path but not a
# "/dpe/projects" route, a docs/src/dpe/... path or `dpe-core` prose, all of
# which are real code in modules/platform. Widening it would flag those.
#
# Safe to `source`. Dependencies: bash, git, awk.

# Must stay a quoted array. As an unquoted string, bash expands the glob first,
# and bash's * does not cross / where a git pathspec's does, so git would see
# shallow filenames and every nested file would go unread while the gate passed.
# No platform crate has a tests/ directory yet; widen this when one does.
PLATFORM_PATHSPECS=('modules/platform/*/src/*.rs')

# Every module under modules/ except platform: wider than "service" because
# mosaic is the design system and is equally off-limits. From the index, so an
# untracked file cannot widen the rule.
non_platform_modules() {
  git ls-files -- modules | awk -F/ 'NF > 2 && $2 != "platform" { print $2 }' | sort -u
}

non_platform_path_pattern() {
  local alternation
  alternation="$(non_platform_modules | paste -sd'|' -)"
  [ -n "$alternation" ] || return 0
  printf '(\\.\\./|modules/)(%s)/' "$alternation"
}

main() {
  local count pattern violations rc

  count="$(git ls-files -- "${PLATFORM_PATHSPECS[@]}" | wc -l | tr -d '[:space:]')"
  pattern="$(non_platform_path_pattern)"

  # Absence is an error, not zero work: a gate that reads nothing, or forbids
  # nothing, would pass while enforcing nothing.
  [ "$count" -gt 0 ] || { echo "✗ no platform sources matched (${PLATFORM_PATHSPECS[*]}). Run from the repo root" >&2; return 1; }
  [ -n "$pattern" ] || { echo "✗ no non-platform modules found under modules/. Run from the repo root" >&2; return 1; }

  violations="$(git grep -nE "$pattern" -- "${PLATFORM_PATHSPECS[@]}")" && rc=0 || rc=$?
  # git grep exits 1 for no matches, >1 for a real failure. Only the latter must
  # not be reported as a clean tree.
  [ "$rc" -le 1 ] || { echo "✗ git grep failed; the gate could not run" >&2; return 1; }

  if [ -n "$violations" ]; then
    printf '%s\n\n' "$violations" >&2
    echo "✗ a platform crate hardcodes a path into another module. Take the path as" >&2
    echo "  a parameter (see load_from(data_dir) in platform-metadata), or move the" >&2
    echo "  code to the crate that owns the data. See modules/platform/README.md." >&2
    return 1
  fi
  echo "✓ platform crates: no hardcoded paths into other modules ($count files checked)"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
