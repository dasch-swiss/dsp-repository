#!/usr/bin/env bash
#
# areas/<area>/server/Cargo.toml must not depend on a capability's core, web or
# api-* crate (ADR-0007; the rule and why store/ports stay allowed: ARCH-MAP.md
# "Composition root"). A grep until Bazel `visibility` lands.
#
# Forbidden names are the [package] names of every
# areas/<area>/<capability>/{core,web,api-*}/Cargo.toml, so a new capability is
# covered without editing this script. A new forbidden role goes in the
# pathspecs of forbidden_crates.
#
# Safe to `source`. Dependencies: bash, git, awk, grep.

# The [package] name of a crate's Cargo.toml, or nothing if it has none.
crate_name() {
  awk -F'"' '/^name[[:space:]]*=/ { print $2; exit }' "$1"
}

# Forbidden crate names for one area: the [package] name of every
# areas/<area>/<capability>/{core,web,api-*}/Cargo.toml under it. A git
# pathspec's `*` crosses `/` (unlike bash's), so `areas/$area/*/core/Cargo.toml`
# alone would also match a deeper accidental core/ — the NF==5 filter pins the
# match to exactly areas/<area>/<capability>/core/Cargo.toml.
forbidden_crates() {
  local area="$1" f
  git ls-files -- "areas/$area/*/core/Cargo.toml" "areas/$area/*/web/Cargo.toml" "areas/$area/*/api-*/Cargo.toml" \
    | awk -F/ 'NF == 5' \
    | while IFS= read -r f; do crate_name "$f"; done
}

# True if Cargo.toml $1 names dependency $2: a `name = ...` line, a
# `name.workspace = true` line or a `[dependencies.name]` table header, all
# anchored at line start (Cargo.toml keys are not indented under
# [dependencies]) so this cannot match a longer crate name sharing the prefix;
# or a renamed dependency's `package = "name"`, quoted so it matches the whole
# name.
depends_on() {
  local file="$1" name="$2"
  grep -qE "^${name}[[:space:]]*(=|\\.workspace)|^\\[dependencies\\.${name}\\]|package[[:space:]]*=[[:space:]]*\"${name}\"" "$file"
}

main() {
  local server_files count=0 file area forbidden name violations=""

  # Same NF filter as forbidden_crates: a composition root is exactly
  # areas/<area>/server/Cargo.toml (NF==4), not a capability's
  # areas/<area>/<capability>/server/Cargo.toml, which the crossing `*` would
  # also match.
  server_files="$(git ls-files -- 'areas/*/server/Cargo.toml' | awk -F/ 'NF == 4')"
  # Absence is an error, not zero work: a gate that reads nothing would pass
  # while enforcing nothing. See check-shared-paths.sh's rule of the same name.
  [ -n "$server_files" ] || { echo "✗ no areas/*/server/Cargo.toml found. Run from the repo root" >&2; return 1; }

  while IFS= read -r file; do
    count=$((count + 1))
    area="$(printf '%s\n' "$file" | awk -F/ '{print $2}')"
    forbidden="$(forbidden_crates "$area")"
    [ -n "$forbidden" ] || continue
    while IFS= read -r name; do
      [ -n "$name" ] || continue
      if depends_on "$file" "$name"; then
        violations="${violations}${violations:+$'\n'}$file depends on $name, a $area capability's core/web/api-* crate"
      fi
    done <<<"$forbidden"
  done <<<"$server_files"

  if [ -n "$violations" ]; then
    printf '%s\n\n' "$violations" >&2
    echo "✗ a composition root depends directly on a capability's core, web or api-* crate." >&2
    echo "  Depend on the capability's server crate instead; see ADR-0007." >&2
    return 1
  fi
  echo "✓ composition roots: no direct core/web/api-* dependency ($count areas/*/server/Cargo.toml checked)"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
