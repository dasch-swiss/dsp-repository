#!/usr/bin/env bash
#
# CPE's dependency seam (ADR-0007, amendment of 2026-10-11; ARCH-MAP.md,
# areas/access/cpe). A grep until Bazel `visibility` lands. Three rules over the
# tracked Cargo.toml files:
#
#   1. Nothing outside areas/access/cpe/ depends on a CPE crate, except on the
#      crates in PORT_EXCEPTIONS: sync-store implements the port, so cpe-ports
#      is the one crate another capability may depend on. DEV-7465 adds the
#      host-facing crates (cpe-server, cpe-store) there when access-server
#      mounts CPE.
#   2. No manifest under areas/access/cpe/ depends on a Mosaic crate.
#   3. The crates in LEAVES depend on no CPE crate, the port included.
#
# Forbidden names are the [package] names of areas/access/cpe/*/Cargo.toml and
# modules/mosaic/*/Cargo.toml, so rules 1 and 2 cover a new crate without an
# edit to this script. Only PORT_EXCEPTIONS and LEAVES are by path: a new crate
# that must stay a leaf is added to LEAVES.
#
# A CPE crate sits directly in areas/access/cpe/<dir>/. A manifest nested deeper
# is not taken for a CPE crate, so its name is not forbidden outside; rule 2
# still applies to it.
#
# The manifests are those in the git index: a new Cargo.toml is seen locally
# only once it is staged.
#
# Safe to `source`. Dependencies: bash (3.2 and later), git, awk, grep.

PORT_EXCEPTIONS=('areas/access/cpe/ports/Cargo.toml')

LEAVES=('areas/access/cpe/core/Cargo.toml' 'areas/access/cpe/ports/Cargo.toml' 'areas/access/cpe/image-viewer/Cargo.toml')

# True if Cargo.toml $1 has a package table, in any spelling TOML allows: a
# [package] header, indented or with spaces inside the brackets, or a top-level
# `package.` or `package =` key. The workspace root has none. A `package` key
# counts only before the first table header: below one it is a dependency's
# rename (`[workspace.dependencies.x]` then `package = "y"`), and the root must
# not be taken for a crate because of it.
has_package() {
  awk '
    /^[[:space:]]*\[/ {
      if ($0 ~ /^[[:space:]]*\[[[:space:]]*package[[:space:]]*\][[:space:]]*(#.*)?$/) found = 1
      seen_header = 1
      next
    }
    !seen_header && /^[[:space:]]*package[[:space:]]*[.=]/ { found = 1 }
    END { exit !found }
  ' "$1"
}

# The name in the [package] table of Cargo.toml $1, or nothing if it cannot be
# read. Not the first `name` line of the file: [lib] and [[bin]] tables have one
# too, and may come first. Reads `name = "x"` under the header, indented or with
# a quoted key, and a top-level `package.name = "x"`. \047 is a single quote.
crate_name() {
  awk '
    function value(line) {
      if (line !~ /=[[:space:]]*["\047]/) return ""
      sub(/^[^=]*=[[:space:]]*["\047]/, "", line)
      sub(/["\047].*$/, "", line)
      return line
    }
    /^[[:space:]]*\[/ {
      seen_header = 1
      in_package = ($0 ~ /^[[:space:]]*\[[[:space:]]*package[[:space:]]*\][[:space:]]*(#.*)?$/)
      next
    }
    in_package && /^[[:space:]]*["\047]?name["\047]?[[:space:]]*=/ { print value($0); exit }
    !seen_header && /^[[:space:]]*package[[:space:]]*\.[[:space:]]*name[[:space:]]*=/ { print value($0); exit }
  ' "$1"
}

# Tracked files matching pathspec $1 with exactly $2 path segments. A git
# pathspec's `*` crosses `/` (unlike bash's), so without the filter a manifest
# nested below a crate would be taken for a crate.
manifests_at_depth() {
  git ls-files -- "$1" | awk -F/ -v n="$2" 'NF == n'
}

# The [package] names of the manifests listed on stdin. main has checked that
# each has one.
package_names() {
  local file
  while IFS= read -r file; do
    if [ -n "$file" ]; then
      crate_name "$file"
    fi
  done
}

# ERE for a dependency renamed to package $1: a `package = "$1"` whose key,
# bare or quoted, is not the tail of a longer key and not in a comment.
rename_pattern() {
  printf '%s' "^([^#]*[^[:alnum:]_#-])?[\"']?package[\"']?[[:space:]]*=[[:space:]]*[\"']$1[\"']"
}

# True if Cargo.toml $1 renames a dependency to package $2.
renames_to() {
  grep -qE "$(rename_pattern "$2")" "$1"
}

# True if Cargo.toml $1 names dependency $2, in any table:
#   - a key: the name, bare or quoted, indented or not, then `=` or `.`
#     (`name = ...`, `name.workspace = true`, `name.path = ...`);
#   - a key inside an inline table, so after `{` or `,` and not in a comment
#     (`dependencies = { name = ... }` under [target.'cfg(unix)']). The line
#     is not parsed, so a string value containing `, name.` or `, name =` is
#     reported too. That side is chosen: a false report is seen and reworded,
#     a missed `{ name.path = ... }` is not seen;
#   - a table header ending in `dependencies.name]`, so [dev-dependencies.name],
#     [build-dependencies.name] and [target.'cfg(unix)'.dependencies.name] too,
#     or a [patch.<source>.name] header, with the spaces TOML allows around the
#     dot and inside the brackets;
#   - a rename, see rename_pattern.
# Each closes on the name, so `name-extras` does not match. Only the exact
# spelling counts: Cargo does not resolve `cpe_core` to the package `cpe-core`.
#
# check-composition-root-deps.sh has a narrower matcher of the same name. They
# are separate on purpose: a change to one is not a change to the other.
depends_on() {
  local file="$1" name="$2" key inline header
  key="^[[:space:]]*[\"']?${name}[\"']?[[:space:]]*[=.]"
  inline="^[^#]*[{,][[:space:]]*[\"']?${name}[\"']?[[:space:]]*[=.]"
  header="^[[:space:]]*\\[([^]]*dependencies|[[:space:]]*patch[[:space:]]*\\.[^]]*)[[:space:]]*\\.[[:space:]]*[\"']?${name}[\"']?[[:space:]]*\\][[:space:]]*(#.*)?\$"
  grep -qE "${key}|${inline}|${header}|$(rename_pattern "$name")" "$file"
}

# Print one violation line per name in $2 (one per line) that manifest $1
# depends on, as $4 (default depends_on) decides. $3 is the rule broken.
report() {
  local file="$1" forbidden="$2" rule="$3" matcher="${4:-depends_on}" name
  while IFS= read -r name; do
    if [ -n "$name" ] && "$matcher" "$file" "$name"; then
      printf '%s depends on %s (rule %s)\n' "$file" "$name" "$rule"
    fi
  done <<<"$forbidden"
}

# True if $1 is one of the remaining arguments.
in_list() {
  local needle="$1" item
  shift
  for item in "$@"; do
    if [ "$item" = "$needle" ]; then
      return 0
    fi
  done
  return 1
}

main() {
  local all_files cpe_files mosaic_files cpe_names mosaic_names rule1_names
  local file count=0 violations="" out
  local rule1='1: nothing outside areas/access/cpe/ depends on a CPE crate'
  local rule2='2: no CPE crate depends on Mosaic'
  local rule3='3: a leaf depends on no CPE crate'

  cpe_files="$(manifests_at_depth 'areas/access/cpe/*/Cargo.toml' 5)"
  mosaic_files="$(manifests_at_depth 'modules/mosaic/*/Cargo.toml' 4)"
  # Absence is an error, not zero work: a gate that reads nothing, or forbids
  # nothing, would pass while enforcing nothing.
  [ -n "$cpe_files" ] || { echo "✗ no areas/access/cpe/*/Cargo.toml found. Run from the repo root" >&2; return 1; }
  [ -n "$mosaic_files" ] || { echo "✗ no modules/mosaic/*/Cargo.toml found. Run from the repo root" >&2; return 1; }
  # A renamed leaf directory would otherwise drop out of rule 3 unnoticed.
  for file in "${PORT_EXCEPTIONS[@]}" "${LEAVES[@]}"; do
    if ! git ls-files --error-unmatch -- "$file" >/dev/null 2>&1; then
      echo "✗ $file is not tracked. Update PORT_EXCEPTIONS or LEAVES in check-cpe-seam.sh" >&2
      return 1
    fi
  done

  all_files="$(git ls-files -- 'Cargo.toml' '*/Cargo.toml')" || { echo "✗ git ls-files failed; the gate could not run" >&2; return 1; }
  # A manifest in the index and not on disk cannot be read, and an unread
  # manifest would count as one without a dependency.
  while IFS= read -r file; do
    if [ ! -f "$file" ]; then
      echo "✗ $file is tracked and missing from the work tree; the gate could not read it" >&2
      return 1
    fi
  done <<<"$all_files"

  # A crate whose name cannot be read would drop out of the forbidden sets,
  # and the gate would pass without it.
  while IFS= read -r file; do
    if [ -z "$(crate_name "$file")" ]; then
      echo "✗ $file: cannot read the [package] name" >&2
      return 1
    fi
  done <<<"$cpe_files"$'\n'"$mosaic_files"

  cpe_names="$(printf '%s\n' "$cpe_files" | package_names)"
  mosaic_names="$(printf '%s\n' "$mosaic_files" | package_names)"
  rule1_names="$(
    printf '%s\n' "$cpe_files" | while IFS= read -r file; do
      if ! in_list "$file" "${PORT_EXCEPTIONS[@]}"; then
        printf '%s\n' "$file"
      fi
    done | package_names
  )"
  [ -n "$cpe_names" ] || { echo "✗ no [package] name found in areas/access/cpe/*/Cargo.toml" >&2; return 1; }
  [ -n "$mosaic_names" ] || { echo "✗ no [package] name found in modules/mosaic/*/Cargo.toml" >&2; return 1; }

  while IFS= read -r file; do
    out=""
    case "$file" in
      areas/access/cpe/*)
        if has_package "$file"; then
          count=$((count + 1))
          out="$(report "$file" "$mosaic_names" "$rule2")"
        else
          out="$(report "$file" "$mosaic_names" "$rule2" renames_to)"
        fi
        ;;
      *)
        if has_package "$file"; then
          count=$((count + 1))
          out="$(report "$file" "$rule1_names" "$rule1")"
        else
          # A plain [workspace.dependencies] entry is not a dependency, and the
          # member inheriting it is caught by its own `name.workspace = true`.
          # A renamed entry is inherited as `x.workspace = true`, which names no
          # forbidden crate, so it has to be caught where it is declared.
          out="$(report "$file" "$rule1_names" "$rule1" renames_to)"
        fi
        ;;
    esac
    if [ -n "$out" ]; then
      violations="${violations}${violations:+$'\n'}${out}"
    fi
  done <<<"$all_files"

  for file in "${LEAVES[@]}"; do
    out="$(report "$file" "$cpe_names" "$rule3")"
    if [ -n "$out" ]; then
      violations="${violations}${violations:+$'\n'}${out}"
    fi
  done

  if [ -n "$violations" ]; then
    printf '%s\n\n' "$violations" >&2
    echo "✗ CPE's dependency seam is broken (ADR-0007)." >&2
    echo "  Outside areas/access/cpe/, depend only on a crate in PORT_EXCEPTIONS (check-cpe-seam.sh)." >&2
    echo "  Inside it, depend on no Mosaic crate, and from a crate in LEAVES on no CPE crate." >&2
    echo "  See ARCH-MAP.md, areas/access/cpe." >&2
    return 1
  fi
  echo "✓ CPE seam: $count manifests checked against $(printf '%s\n' "$cpe_names" | wc -l | tr -d '[:space:]') CPE and $(printf '%s\n' "$mosaic_names" | wc -l | tr -d '[:space:]') Mosaic crates (${#PORT_EXCEPTIONS[@]} excepted)"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
