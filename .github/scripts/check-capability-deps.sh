#!/usr/bin/env bash
#
# Two rules over the BUILD.bazel files under the working directory (the
# repository root), read as text with `#` comments stripped.
#
# First-party edges are not checked here: each provider's `visibility` list is
# the rule (ADR-0002, ADR-0003, ADR-0007).
#
# 1. Third-party bans per role (ADR-0003). A role's BUILD file is
#    areas/<area>/<capability>/<role>/BUILD.bazel (exactly that depth; a nested
#    package such as areas/access/dpe/server/fuzz is review) or
#    areas/<area>/shell/BUILD.bazel (role shell), for each area in CHECKED_AREAS.
#    Every `@crates//:<name>` in the file counts, whatever the target or
#    attribute. A ports crate may use none; a core crate no web framework, Maud
#    or database crate; components and image-viewer no web framework or
#    database crate; a web crate beside its capability's server crate, which
#    holds the handlers, no web framework; the shell no web framework. Other
#    roles, the composition root areas/<area>/server among them, are not
#    restricted.
#
# 2. No public first-party target: no `//visibility:public` in any BUILD.bazel
#    under areas/, shared/, modules/ or dsp-cli/. Widening a target to public
#    is the quickest way to silence a visibility error, and it would undo every
#    visibility boundary without touching a rule.
#
# Safe to `source`. Dependencies: bash, grep, sed.

CHECKED_AREAS="deposit access"
PUBLIC_CHECKED_DIRS="areas shared modules dsp-cli"
# Glob patterns, matched against a crate name.
WEB_FRAMEWORK="axum axum-* hyper hyper-* tower tower-* tower_governor"
DATABASE="rusqlite deadpool-sqlite libsqlite3-sys sqlx"

# denied <crate name> <patterns>: true if the name matches one of the
# space-separated glob patterns. `read -a` splits them without expanding a glob
# against the working directory.
denied() {
  local pattern patterns
  read -r -a patterns <<<"$2"
  for pattern in "${patterns[@]}"; do
    # shellcheck disable=SC2254
    case "$1" in $pattern) return 0 ;; esac
  done
  return 1
}

# third_party_allowed <role> <crate name> <has server>: true unless the role
# denies that crate. <has server> is "yes" when the capability has a server crate.
third_party_allowed() {
  local denied_patterns
  case "$1" in
    ports) return 1 ;;
    core) denied_patterns="$WEB_FRAMEWORK maud $DATABASE" ;;
    components | image-viewer) denied_patterns="$WEB_FRAMEWORK $DATABASE" ;;
    web)
      [ "$3" = yes ] || return 0
      denied_patterns="$WEB_FRAMEWORK"
      ;;
    shell) denied_patterns="$WEB_FRAMEWORK" ;;
    *) return 0 ;;
  esac
  ! denied "$2" "$denied_patterns"
}

# crates_of <BUILD file>: the distinct crate names it takes from @crates.
crates_of() {
  sed 's/#.*//' "$1" | grep -o '@crates//:[A-Za-z0-9_.-]*' | sed 's|^@crates//:||' | sort -u
}

main() {
  local area file cap role has_server name dir hit problems=""

  [ -n "${CHECKED_AREAS// /}" ] || { echo "✗ CHECKED_AREAS is empty: the gate would check nothing" >&2; return 1; }

  for area in $CHECKED_AREAS; do
    for file in "areas/$area"/*/BUILD.bazel "areas/$area"/*/*/BUILD.bazel; do
      [ -f "$file" ] || continue
      IFS=/ read -r _ _ cap role _ <<<"$file"
      if [ "$role" = BUILD.bazel ]; then
        [ "$cap" = shell ] || continue
        role=shell
        has_server=no
      else
        has_server=no
        [ -f "areas/$area/$cap/server/BUILD.bazel" ] && has_server=yes
      fi
      while IFS= read -r name; do
        [ -n "$name" ] || continue
        third_party_allowed "$role" "$name" "$has_server" \
          || problems="${problems}$file uses $name (third-party), a crate its role may not use"$'\n'
      done < <(crates_of "$file")
    done
  done

  for dir in $PUBLIC_CHECKED_DIRS; do
    [ -d "$dir" ] || continue
    while IFS= read -r file; do
      while IFS= read -r hit; do
        [ -n "$hit" ] || continue
        problems="${problems}$file:${hit%%:*} makes a target public (//visibility:public)"$'\n'
      done < <(sed 's/#.*//' "$file" | grep -n '//visibility:public')
    done < <(find "$dir" -name BUILD.bazel -type f -not -path '*/node_modules/*' | sort)
  done

  if [ -n "$problems" ]; then
    printf '%s' "$problems" >&2
    echo "✗ a BUILD.bazel uses a third-party crate its role may not use, or makes a target public." >&2
    echo "  First-party edges are each provider's visibility list; bans are in the header of this script (ADR-0003)." >&2
    return 1
  fi
  echo "✓ capability deps: no role uses a third-party crate it may not, and no target is public"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
