#!/usr/bin/env bash
#
# CPE's dependency seam (ADR-0007, amendments of 2026-10-11 and, for cpe-server
# in rules 1 and 5, of 2026-10-12; ADR-0008 and ADR-0012 for rule 4;
# ARCH-MAP.md, areas/access/cpe). A grep until Bazel `visibility` lands. Five
# rules over the tracked Cargo.toml files:
#
#   1. Nothing outside areas/access/cpe/ depends on a CPE crate, except on the
#      crates in PORT_EXCEPTIONS: cpe-ports, which sync-store and media-store
#      implement and the composition root hands over, and cpe-server, which
#      the composition root mounts. cpe-store is not among them: the adapters
#      of the ports are sync-store's and media-store's, so no host names the
#      store.
#   2. No manifest under areas/access/cpe/ depends on a Mosaic crate.
#   3. The crates in LEAVES depend on no CPE crate, the port included.
#   4. No manifest under areas/access/cpe/ names a crate of areas/access/sync/
#      or areas/access/media/ outside a dev-dependencies table. CPE's only feeds
#      are its ports (ADR-0008, ADR-0012); only its tests may build an adapter,
#      to read the committed snapshot or the committed deposited files.
#      Exempt are the lines below a [dev-dependencies] header, a
#      target-specific one, and [dev-dependencies.<name>]. An inline
#      `dev-dependencies = { ... }`, a dotted `dev-dependencies.<name>` key
#      and a [dev_dependencies] header are not exempt: the line is not parsed,
#      so they are reported.
#   5. A crate in CPE_EDGES depends on no CPE crate but those listed for it, in
#      any table, dev-dependencies included: cpe-store on cpe-core alone,
#      cpe-store-build on cpe-core and cpe-ports, cpe-server on every CPE crate
#      but cpe-store. The builder does not depend on the store it fills, whose
#      schema it includes as a file, and the store does not depend on the
#      builder. The host crate reaches the store through cpe-web alone, so a
#      second way to open one cannot be added there unnoticed. A crate that is
#      not in the list is not held to the rule: cpe-web depends on the store.
#
# Forbidden names are the [package] names of areas/access/cpe/*/Cargo.toml,
# modules/mosaic/*/Cargo.toml, areas/access/sync/*/Cargo.toml and
# areas/access/media/*/Cargo.toml, so rules 1, 2 and 4 cover a new crate
# without an edit to this script. PORT_EXCEPTIONS, LEAVES and CPE_EDGES are by
# path: a new crate that must stay a leaf is added to LEAVES, and one whose CPE
# dependencies are to be held is added to CPE_EDGES.
#
# A manifest without [package] outside areas/access/cpe/ (the workspace root)
# is held to its renames alone: it renames no CPE, Mosaic, sync or media
# crate. The inheriting `x.workspace = true` names no forbidden crate, so a
# rename is reported where it is declared, whichever crate inherits it: a CPE
# crate's cannot be told from another's. The port is no exception there, since
# a leaf or a crate in CPE_EDGES could inherit it (reported as rule 3).
# Declaring the entry without the rename ends the report.
#
# The lines are matched, not parsed. A key that is no dependency is reported
# when it is spelled like one: a feature named as a forbidden crate
# (`sync-store = []` under [features]), or such a key under
# [package.metadata.*]. Renaming the key ends the report.
#
# A CPE crate sits directly in areas/access/cpe/<dir>/. A manifest nested deeper
# is not taken for a CPE crate, so its name is not forbidden outside; rules 2
# and 4 still apply to it.
#
# The manifests are those in the git index: a new Cargo.toml is seen locally
# only once it is staged.
#
# Safe to `source`. Dependencies: bash (3.2 and later), git, awk, grep.

PORT_EXCEPTIONS=('areas/access/cpe/ports/Cargo.toml' 'areas/access/cpe/server/Cargo.toml')

LEAVES=('areas/access/cpe/core/Cargo.toml' 'areas/access/cpe/ports/Cargo.toml' 'areas/access/cpe/image-viewer/Cargo.toml')

# Rule 5, one entry per crate: `<manifest>:<the CPE crates it may depend on>`,
# the crates by [package] name with a space between them.
CPE_EDGES=(
  'areas/access/cpe/store/Cargo.toml:cpe-core'
  'areas/access/cpe/store-build/Cargo.toml:cpe-core cpe-ports'
  'areas/access/cpe/server/Cargo.toml:cpe-core cpe-ports cpe-store-build cpe-web cpe-components cpe-image-viewer'
)

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
#   - a dotted key that starts with the table (`dependencies.name.path = ...`,
#     `build-dependencies.name = ...`), at the top of the file or below
#     [target.'cfg(unix)'], or with the target before it
#     (`target.'cfg(unix)'.dependencies.name.path = ...`). The table is read in
#     the underscore spelling Cargo also accepts (`build_dependencies`,
#     `dev_dependencies`). `dev-dependencies.name` is matched too, so rule 4
#     reports that spelling: its exemption is by table header;
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
  grep -qE "$(dependency_pattern "$2")" "$1"
}

# ERE for a line that names dependency $1, in the forms depends_on lists.
dependency_pattern() {
  local name="$1" key dotted inline header
  key="^[[:space:]]*[\"']?${name}[\"']?[[:space:]]*[=.]"
  dotted="^[[:space:]]*([\"']?target[\"']?[[:space:]]*\\.[^#]*\\.[[:space:]]*)?[\"']?[A-Za-z_-]*dependencies[\"']?[[:space:]]*\\.[[:space:]]*[\"']?${name}[\"']?[[:space:]]*[=.]"
  inline="^[^#]*[{,][[:space:]]*[\"']?${name}[\"']?[[:space:]]*[=.]"
  header="^[[:space:]]*\\[([^]]*dependencies|[[:space:]]*patch[[:space:]]*\\.[^]]*)[[:space:]]*\\.[[:space:]]*[\"']?${name}[\"']?[[:space:]]*\\][[:space:]]*(#.*)?\$"
  printf '%s' "${key}|${dotted}|${inline}|${header}|$(rename_pattern "$name")"
}

# Cargo.toml $1 without its dev-dependencies tables: each header that is
# [dev-dependencies], [dev-dependencies.<name>] or either below [target.<cfg>.],
# and the lines below it up to the next header. The header is read without its
# trailing comment and from its first segment, so neither a comment that ends in
# `[dev-dependencies]` nor a dependency named `dev-dependencies` or
# `x-dev-dependencies` exempts a table. The underscore spelling,
# [dev_dependencies], exempts none either: only the one spelling is read. Not
# told apart: a dependency named
# `dev-dependencies` below [target.<cfg>.dependencies]. Any line that starts
# with `[` counts as a header, a line of a multi-line array too. \047 is a
# single quote.
without_dev_dependencies() {
  awk '
    /^[[:space:]]*\[/ {
      header = $0
      sub(/\][[:space:]]*#.*$/, "]", header)
      skip = (header ~ /^[[:space:]]*\[[[:space:]]*(target[[:space:]]*\..*\.[[:space:]]*)?["\047]?dev-dependencies["\047]?[[:space:]]*(\.[^.]*)?\][[:space:]]*$/)
    }
    !skip { print }
  ' "$1"
}

# True if Cargo.toml $1 names dependency $2 outside its dev-dependencies tables.
# The filtered text goes to grep as a here-string, not through a pipe: grep -q
# leaves at its first match, and under `pipefail` a writer cut off by that would
# turn the match into a failure.
depends_on_outside_dev() {
  local body
  body="$(without_dev_dependencies "$1")" || return 1
  grep -qE "$(dependency_pattern "$2")" <<<"$body"
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

# The names in $1 (one per line) that are not among the names in $2 (with a
# space between them).
names_except() {
  local name
  while IFS= read -r name; do
    case " $2 " in
      *" $name "*) ;;
      *) [ -z "$name" ] || printf '%s\n' "$name" ;;
    esac
  done <<<"$1"
}

count_lines() {
  printf '%s\n' "$1" | wc -l | tr -d '[:space:]'
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
  local all_files cpe_files mosaic_files sync_files media_files cpe_names mosaic_names sync_names media_names
  local feed_names rule1_names port_names
  local file entry count=0 violations="" out rule4_out
  local rule1='1: nothing outside areas/access/cpe/ depends on a CPE crate'
  local rule2='2: no CPE crate depends on Mosaic'
  local rule3='3: a leaf depends on no CPE crate'
  local rule4='4: a CPE crate names a sync or media crate under dev-dependencies only'
  local rule5='5: a crate in CPE_EDGES depends on the CPE crates listed for it only'

  cpe_files="$(manifests_at_depth 'areas/access/cpe/*/Cargo.toml' 5)"
  mosaic_files="$(manifests_at_depth 'modules/mosaic/*/Cargo.toml' 4)"
  sync_files="$(manifests_at_depth 'areas/access/sync/*/Cargo.toml' 5)"
  media_files="$(manifests_at_depth 'areas/access/media/*/Cargo.toml' 5)"
  # Absence is an error, not zero work: a gate that reads nothing, or forbids
  # nothing, would pass while enforcing nothing.
  [ -n "$cpe_files" ] || { echo "✗ no areas/access/cpe/*/Cargo.toml found. Run from the repo root" >&2; return 1; }
  [ -n "$mosaic_files" ] || { echo "✗ no modules/mosaic/*/Cargo.toml found. Run from the repo root" >&2; return 1; }
  [ -n "$sync_files" ] || { echo "✗ no areas/access/sync/*/Cargo.toml found. Run from the repo root" >&2; return 1; }
  [ -n "$media_files" ] || { echo "✗ no areas/access/media/*/Cargo.toml found. Run from the repo root" >&2; return 1; }
  # A renamed crate directory would otherwise drop out of rule 3 or 5
  # unnoticed.
  for file in "${PORT_EXCEPTIONS[@]}" "${LEAVES[@]}" "${CPE_EDGES[@]%%:*}"; do
    if ! git ls-files --error-unmatch -- "$file" >/dev/null 2>&1; then
      echo "✗ $file is not tracked. Update PORT_EXCEPTIONS, LEAVES or CPE_EDGES in check-cpe-seam.sh" >&2
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
  done <<<"$cpe_files"$'\n'"$mosaic_files"$'\n'"$sync_files"$'\n'"$media_files"

  cpe_names="$(printf '%s\n' "$cpe_files" | package_names)"
  mosaic_names="$(printf '%s\n' "$mosaic_files" | package_names)"
  sync_names="$(printf '%s\n' "$sync_files" | package_names)"
  media_names="$(printf '%s\n' "$media_files" | package_names)"
  # Rule 4's forbidden set: the crates that feed CPE through a port.
  feed_names="${sync_names}"$'\n'"${media_names}"
  rule1_names="$(
    printf '%s\n' "$cpe_files" | while IFS= read -r file; do
      if ! in_list "$file" "${PORT_EXCEPTIONS[@]}"; then
        printf '%s\n' "$file"
      fi
    done | package_names
  )"
  port_names="$(printf '%s\n' "${PORT_EXCEPTIONS[@]}" | package_names)"
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
        rule4_out="$(report "$file" "$feed_names" "$rule4" depends_on_outside_dev)"
        if [ -n "$rule4_out" ]; then
          out="${out}${out:+$'\n'}${rule4_out}"
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
          # forbidden crate, so it has to be caught where it is declared. That
          # holds for what a CPE crate would inherit as well: a Mosaic, sync or
          # media crate, and the port, which rules 3 and 5 forbid to some.
          out="$(
            report "$file" "$rule1_names" "$rule1" renames_to
            report "$file" "$port_names" "$rule3" renames_to
            report "$file" "$mosaic_names" "$rule2" renames_to
            report "$file" "$feed_names" "$rule4" renames_to
          )"
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

  for entry in "${CPE_EDGES[@]}"; do
    file="${entry%%:*}"
    out="$(report "$file" "$(names_except "$cpe_names" "${entry#*:}")" "$rule5")"
    if [ -n "$out" ]; then
      violations="${violations}${violations:+$'\n'}${out}"
    fi
  done

  if [ -n "$violations" ]; then
    printf '%s\n\n' "$violations" >&2
    echo "✗ CPE's dependency seam is broken (ADR-0007, ADR-0008, ADR-0012)." >&2
    echo "  Outside areas/access/cpe/, depend only on a crate in PORT_EXCEPTIONS (check-cpe-seam.sh)." >&2
    echo "  Inside it, depend on no Mosaic crate, from a crate in LEAVES on no CPE crate," >&2
    echo "  and on a crate of areas/access/sync/ or areas/access/media/ under [dev-dependencies] only." >&2
    echo "  From a crate in CPE_EDGES, depend on the CPE crates listed for it only." >&2
    echo "  In the workspace root, rename no CPE, Mosaic, sync or media crate (package = ...)." >&2
    echo "  See ARCH-MAP.md, areas/access/cpe." >&2
    return 1
  fi
  echo "✓ CPE seam: $count manifests checked against $(count_lines "$cpe_names") CPE, $(count_lines "$mosaic_names") Mosaic, $(count_lines "$sync_names") sync and $(count_lines "$media_names") media crates (${#PORT_EXCEPTIONS[@]} excepted)"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
