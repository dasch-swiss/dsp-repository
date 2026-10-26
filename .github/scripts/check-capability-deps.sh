#!/usr/bin/env bash
#
# Every crate of an area in CHECKED_AREAS depends, among this workspace's
# crates, only on what its role allows (ADR-0003; the Deposit Area's roles:
# areas/deposit/ADR-0004, "The shape"; CPE's components, image-viewer and
# store-build: ADR-0007).
#
# A crate at areas/<area>/<capability>/<role>/Cargo.toml may depend on:
#   every role            its own capability's crates and shared/* crates;
#   store                 also another capability's ports crate (it answers that port);
#   web, server           also the area's shell and mosaic-tiles;
#   ports, image-viewer   shared/* crates only, not even its own capability's;
#   api-*                 shared/* crates and its own capability's core crate only;
#   core, pull-requests,
#   components,
#   store-build           nothing more.
# core, store, ports, web, server and api-* are ADR-0003's and open to every
# capability; another role is open only to the capability CAPABILITY_ROLES
# names it for, so a new role is an edit here, beside its record (ADR-0003).
# areas/<area>/shell may depend on shared/* crates and mosaic-tiles only.
# A crate of another capability of the same area may name a crate in DEV_ONLY,
# and only as a dev-dependency (ADR-0008, ADR-0012): its tests build that
# adapter over the committed data. A ports, image-viewer or api-* crate may
# not, nor may a crate of a capability that provides a DEV_ONLY crate.
# Third-party crates: a ports crate may use none; a core crate no web framework,
# Maud or database crate; components and image-viewer no web framework or
# database crate; a web crate beside its capability's server crate, which holds
# the handlers, no web framework; the shell no web framework. Other roles are
# not restricted. areas/<area>/server is check-composition-root-deps.sh's.
# Every dependency kind counts, keyed on cargo's .name, the real package even
# when the manifest renames it. Only workspace members are seen: a crate with a
# [workspace] of its own (areas/access/dpe/server/fuzz) is review.
# CPE's crates are held to check-cpe-seam.sh as well: no Mosaic crate, cpe-core
# a leaf, the CPE crates CPE_EDGES lists.
#
# Safe to `source`. Dependencies: bash, cargo, jq.

CHECKED_AREAS="deposit access"
# Crate directories another crate of their area may depend on as a
# dev-dependency only: the adapters of a reading capability's ports.
DEV_ONLY="areas/access/sync/store areas/access/media/store"
# <area>/<capability>/<role> for each role outside ADR-0003's anatomy.
CAPABILITY_ROLES="deposit/editor/pull-requests access/cpe/components access/cpe/image-viewer access/cpe/store-build"
# Glob patterns, matched against a dependency's package name.
WEB_FRAMEWORK="axum axum-* hyper hyper-* tower tower-* tower_governor"
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

# allowed <crate path> <dependency name> <dependency path> <kind>: true if the
# edge is in the list above. The shell's path reads as capability "shell", no
# role.
allowed() {
  local from="$1" dep="$2" to="$3" kind="$4" area cap role rest
  IFS=/ read -r _ area cap role <<<"$from"
  case "$to" in shared/*) return 0 ;; esac
  if [ -z "$role" ]; then
    [ "$dep" = mosaic-tiles ]
    return
  fi
  case "$role" in
    ports | image-viewer) return 1 ;;
    api-*)
      [ "$to" = "areas/$area/$cap/core" ]
      return
      ;;
  esac
  if [ "$kind" = dev ] && dev_only "$to" && ! provides_dev_only "areas/$area/$cap"; then
    case "$to" in "areas/$area/"*) return 0 ;; esac
  fi
  case "$to" in "areas/$area/$cap/"*) return 0 ;; esac
  case "$role" in
    store)
      rest="${to#"areas/$area/"}"
      [ "$rest" != "$to" ] && [ "${rest#*/}" = ports ] && return 0
      ;;
    web | server)
      [ "$to" = "areas/$area/shell" ] && return 0
      [ "$dep" = mosaic-tiles ] && return 0
      ;;
  esac
  return 1
}

# dev_only <crate path>: true if the crate is in DEV_ONLY.
dev_only() {
  case " $DEV_ONLY " in *" $1 "*) return 0 ;; esac
  return 1
}

# provides_dev_only <capability path>: true if a crate in DEV_ONLY is the
# capability's.
provides_dev_only() {
  case " $DEV_ONLY " in *" $1/"*) return 0 ;; esac
  return 1
}

# denied <dependency name> <patterns>: true if the name matches one of the
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

# third_party_allowed <crate path> <dependency name> <server crate paths>: true
# unless the crate's role denies that third-party crate (the list above). The
# third argument lists every capability's server crate, one per line.
third_party_allowed() {
  local role denied
  IFS=/ read -r _ _ _ role <<<"$1"
  case "$role" in
    ports) return 1 ;;
    core) denied="$WEB_FRAMEWORK maud $DATABASE" ;;
    components | image-viewer) denied="$WEB_FRAMEWORK $DATABASE" ;;
    web)
      grep -qx "${1%/web}/server" <<<"$3" || return 0
      denied="$WEB_FRAMEWORK"
      ;;
    "") denied="$WEB_FRAMEWORK" ;;
    *) return 0 ;;
  esac
  ! denied "$2" "$denied"
}

main() {
  local graph kind name path dep to dep_kind parts area dir hint capabilities="" servers="" problems=""

  [ -n "${CHECKED_AREAS// /}" ] || { echo "✗ CHECKED_AREAS is empty: the gate would check nothing" >&2; return 1; }
  graph="$(crate_graph)" || { echo "✗ cargo metadata or jq failed" >&2; return 1; }

  while IFS=$'\t' read -r kind name path; do
    [ "$kind" = P ] || continue
    IFS=/ read -r -a parts <<<"$path"
    # <segment count>:<role>:<capability or root-level name>
    case "${#parts[@]}:${parts[3]:-}:${parts[2]:-}" in
      4:server:*)
        capabilities="${capabilities}${parts[1]}"$'\n'
        servers="${servers}${path}"$'\n'
        ;;
      4:core:* | 4:store:* | 4:ports:* | 4:web:* | 4:api-*:*)
        capabilities="${capabilities}${parts[1]}"$'\n'
        ;;
      3::shell | 3::server) ;;
      *)
        if [ "${#parts[@]}" = 4 ] && case " $CAPABILITY_ROLES " in *" ${path#areas/} "*) true ;; *) false ;; esac; then
          capabilities="${capabilities}${parts[1]}"$'\n'
        else
          problems="${problems}$name ($path) is in no role this gate knows: core, store, ports, web, server or api-* under a capability, a role CAPABILITY_ROLES gives its capability, or the area's shell or server"$'\n'
        fi
        ;;
    esac
  done <<<"$graph"

  # A moved adapter would otherwise leave a stale path that lets whatever crate
  # comes to sit there be a dev-dependency.
  for dir in $DEV_ONLY; do
    case " $CHECKED_AREAS " in *" $(cut -d/ -f2 <<<"$dir") "*) ;; *) continue ;; esac
    grep -q "^P"$'\t'"[^"$'\t'"]*"$'\t'"$dir\$" <<<"$graph" \
      || problems="${problems}DEV_ONLY names $dir, which is no workspace crate"$'\n'
  done

  for area in $CHECKED_AREAS; do
    grep -qx "$area" <<<"$capabilities" \
      || problems="${problems}areas/$area has no capability crate (areas/$area/<capability>/<role>/Cargo.toml)"$'\n'
  done

  while IFS=$'\t' read -r kind path dep to dep_kind; do
    [ "${path#areas/*/}" = server ] && continue
    case "$kind" in
      E)
        hint=""
        [ "$dep_kind" = dev ] && hint=" (across capabilities, a dev-dependency only on a crate in DEV_ONLY, ADR-0008)"
        allowed "$path" "$dep" "$to" "$dep_kind" \
          || problems="${problems}$path depends on $dep ($to, $dep_kind), an edge its role does not allow$hint"$'\n'
        ;;
      T)
        third_party_allowed "$path" "$dep" "$servers" \
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
