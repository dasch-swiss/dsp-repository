#!/usr/bin/env bash
#
# Every crate of an area in CHECKED_AREAS depends, among this workspace's
# crates, only on what its role allows (ADR-0003; the Deposit Area's roles:
# areas/deposit/ADR-0004, "The shape").
#
# A crate at areas/<area>/<capability>/<role>/Cargo.toml may depend on:
#   every role            its own capability's crates and shared/* crates;
#   store                 also another capability's ports crate (it answers that port);
#   web                   also the area's shell and mosaic-tiles;
#   ports                 shared/* crates only, not even its own capability's;
#   core, pull-requests   nothing more.
# areas/<area>/shell may depend on shared/* crates and mosaic-tiles only.
# Third-party crates: a ports crate may use none, a core crate no web framework,
# Maud or database crate, the shell no web framework; other roles are not
# restricted. areas/<area>/server is check-composition-root-deps.sh's. Every
# dependency kind counts, keyed on cargo's .name, the real package even when
# the manifest renames it.
#
# Safe to `source`. Dependencies: bash, cargo, jq.

CHECKED_AREAS="deposit"
WEB_FRAMEWORK="axum axum-core hyper tower tower-http tower_governor"
DATABASE="rusqlite deadpool-sqlite libsqlite3-sys sqlx"

# One line per workspace package under a checked area, tab-separated:
#   P <name> <path>
# one per path dependency of it on a workspace package:
#   E <path> <dependency name> <dependency path> <kind>
# and one per dependency on anything else:
#   T <path> <dependency name> - <kind>
# Paths are crate directories relative to the workspace root.
crate_graph() {
  local metadata
  metadata="$(cargo metadata --no-deps --format-version 1)" || return 1
  jq -r --arg areas "$CHECKED_AREAS" '
    .workspace_root as $root
    | def rel: ltrimstr($root + "/") | rtrimstr("/Cargo.toml");
    (.packages | map({key: .name, value: (.manifest_path | rel)}) | from_entries) as $path_of
    | ($areas | split(" ") | map("areas/" + . + "/")) as $prefixes
    | .packages[]
    | (.manifest_path | rel) as $p
    | select(any($prefixes[]; . as $pre | $p | startswith($pre)))
    | "P\t\(.name)\t\($p)",
      (.dependencies[]
        | if .path != null and $path_of[.name] != null
          then "E\t\($p)\t\(.name)\t\($path_of[.name])\t\(.kind // "normal")"
          else "T\t\($p)\t\(.name)\t-\t\(.kind // "normal")" end)
  ' <<<"$metadata"
}

# allowed <crate path> <dependency name> <dependency path>: true if the edge is
# in the list above. The shell's path reads as capability "shell", no role.
allowed() {
  local from="$1" dep="$2" to="$3" area cap role rest
  IFS=/ read -r _ area cap role <<<"$from"
  case "$to" in shared/*) return 0 ;; esac
  if [ -z "$role" ]; then
    [ "$dep" = mosaic-tiles ]
    return
  fi
  [ "$role" = ports ] && return 1
  case "$to" in "areas/$area/$cap/"*) return 0 ;; esac
  case "$role" in
    store)
      rest="${to#"areas/$area/"}"
      [ "$rest" != "$to" ] && [ "${rest#*/}" = ports ] && return 0
      ;;
    web)
      [ "$to" = "areas/$area/shell" ] && return 0
      [ "$dep" = mosaic-tiles ] && return 0
      ;;
  esac
  return 1
}

# third_party_allowed <crate path> <dependency name>: true unless the crate's
# role denies that third-party crate (the list above).
third_party_allowed() {
  local role denied
  IFS=/ read -r _ _ _ role <<<"$1"
  case "$role" in
    ports) return 1 ;;
    core) denied="$WEB_FRAMEWORK maud $DATABASE" ;;
    "") denied="$WEB_FRAMEWORK" ;;
    *) return 0 ;;
  esac
  case " $denied " in *" $2 "*) return 1 ;; esac
}

main() {
  local graph kind name path dep to dep_kind parts area capabilities="" problems=""

  [ -n "${CHECKED_AREAS// /}" ] || { echo "✗ CHECKED_AREAS is empty: the gate would check nothing" >&2; return 1; }
  graph="$(crate_graph)" || { echo "✗ cargo metadata or jq failed" >&2; return 1; }

  while IFS=$'\t' read -r kind name path; do
    [ "$kind" = P ] || continue
    IFS=/ read -r -a parts <<<"$path"
    # <segment count>:<role>:<capability or root-level name>
    case "${#parts[@]}:${parts[3]:-}:${parts[2]:-}" in
      4:core:* | 4:store:* | 4:ports:* | 4:web:* | 4:pull-requests:*)
        capabilities="${capabilities}${parts[1]}"$'\n'
        ;;
      3::shell | 3::server) ;;
      *)
        problems="${problems}$name ($path) is in no role this gate knows: core, store, ports, web or pull-requests under a capability, or the area's shell or server"$'\n'
        ;;
    esac
  done <<<"$graph"

  for area in $CHECKED_AREAS; do
    grep -qx "$area" <<<"$capabilities" \
      || problems="${problems}areas/$area has no capability crate (areas/$area/<capability>/<role>/Cargo.toml)"$'\n'
  done

  while IFS=$'\t' read -r kind path dep to dep_kind; do
    [ "${path#areas/*/}" = server ] && continue
    case "$kind" in
      E)
        allowed "$path" "$dep" "$to" \
          || problems="${problems}$path depends on $dep ($to, $dep_kind), an edge its role does not allow"$'\n'
        ;;
      T)
        third_party_allowed "$path" "$dep" \
          || problems="${problems}$path depends on $dep (third-party, $dep_kind), a crate its role may not use"$'\n'
        ;;
    esac
  done <<<"$graph"

  if [ -n "$problems" ]; then
    printf '%s\n' "$problems" >&2
    echo "✗ a crate in ${CHECKED_AREAS} depends on a crate its role may not use, or sits in no known role." >&2
    echo "  Each role's edges are listed at the top of this script. One capability needing another declares a port in its ports crate (ADR-0003)." >&2
    return 1
  fi
  echo "✓ capability edges: every crate in $CHECKED_AREAS depends only on what its role allows"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
