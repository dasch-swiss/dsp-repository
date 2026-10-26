#!/usr/bin/env bash
#
# Tests for check-capability-deps.sh. Needs bash, cargo and jq.
#
# Fixtures are throwaway Cargo workspaces, not hand-written metadata JSON: a
# renamed or dev-only edge is whatever cargo reports.
#
# Run: bash .github/scripts/check-capability-deps.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-capability-deps.sh disable=SC1091
source "$SCRIPT_DIR/check-capability-deps.sh"

PASS=0
FAIL=0

# check <desc> <expected-rc> <actual-rc>
check() {
  if [ "$2" = "$3" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $1 (expected rc=$2, got rc=$3)"
  fi
}

# fails_with <desc> <expected stderr line>: run the gate in $WS and pass if it
# fails with that line, so a fixture that breaks cargo cannot pass a case.
fails_with() {
  local out
  out="$(run_gate)"
  if [ "${out%%$'\n'*}" = 1 ] && grep -qxF "$2" <<<"${out#*$'\n'}"; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $1 (expected rc=1 and: $2)"
    printf '%s\n' "$out" | sed 's/^/  /'
  fi
}

WS=""

# crate <dir> <name>: a library crate at $WS/<dir>, registered as a member.
crate() {
  mkdir -p "$WS/$1/src"
  : >"$WS/$1/src/lib.rs"
  printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\n' "$2" >"$WS/$1/Cargo.toml"
  printf '%s\n' "$1" >>"$WS/members"
}

# dep <dir> <line>: append a line to <dir>'s Cargo.toml (its [dependencies]
# unless the line opens another table). @ in the line becomes $WS.
dep() {
  printf '%s\n' "${2//@/$WS}" >>"$WS/$1/Cargo.toml"
}

# new_ws: a workspace shaped like the Deposit Area, every allowed edge present
# once. Sets WS, and CHECKED_AREAS, CAPABILITY_ROLES and DEV_ONLY for it.
new_ws() {
  WS="$(mktemp -d "${TMPDIR:-/tmp}/capability-deps.XXXXXX")" || exit 1
  CHECKED_AREAS=deposit
  CAPABILITY_ROLES=deposit/alpha/pull-requests
  DEV_ONLY="areas/access/sync/store areas/access/media/store"
  local cap
  for cap in alpha beta; do
    crate "areas/deposit/$cap/core" "$cap-core"
    crate "areas/deposit/$cap/store" "$cap-store"
    crate "areas/deposit/$cap/ports" "$cap-ports"
    crate "areas/deposit/$cap/web" "$cap-web"
  done
  crate areas/deposit/alpha/pull-requests alpha-pull-requests
  crate areas/deposit/shell deposit-shell
  crate areas/deposit/server deposit-server
  crate shared/metadata shared-metadata
  crate modules/mosaic/tiles mosaic-tiles

  dep areas/deposit/alpha/core 'shared-metadata = { path = "@/shared/metadata" }'
  dep areas/deposit/alpha/store 'alpha-core = { path = "@/areas/deposit/alpha/core" }'
  dep areas/deposit/alpha/store 'beta-ports = { path = "@/areas/deposit/beta/ports" }'
  dep areas/deposit/alpha/ports 'shared-metadata = { path = "@/shared/metadata" }'
  dep areas/deposit/alpha/web 'alpha-core = { path = "@/areas/deposit/alpha/core" }'
  dep areas/deposit/alpha/web 'alpha-store = { path = "@/areas/deposit/alpha/store" }'
  dep areas/deposit/alpha/web 'deposit-shell = { path = "@/areas/deposit/shell" }'
  dep areas/deposit/alpha/web 'mosaic-tiles = { path = "@/modules/mosaic/tiles" }'
  dep areas/deposit/alpha/pull-requests 'alpha-core = { path = "@/areas/deposit/alpha/core" }'
  dep areas/deposit/alpha/pull-requests 'shared-metadata = { path = "@/shared/metadata" }'
  dep areas/deposit/shell 'mosaic-tiles = { path = "@/modules/mosaic/tiles" }'
  dep areas/deposit/shell 'shared-metadata = { path = "@/shared/metadata" }'
  # The root's edges are check-composition-root-deps.sh's.
  dep areas/deposit/server 'alpha-core = { path = "@/areas/deposit/alpha/core" }'
}

# new_access_ws: a workspace shaped like the Access Area, every allowed edge
# present once: a capability with every role a server crate brings, a provider
# answering its port, and the two DEV_ONLY adapters as its tests use them. Sets
# WS, and CHECKED_AREAS, CAPABILITY_ROLES and DEV_ONLY for it.
new_access_ws() {
  WS="$(mktemp -d "${TMPDIR:-/tmp}/capability-deps.XXXXXX")" || exit 1
  CHECKED_AREAS=access
  CAPABILITY_ROLES="access/alpha/components access/alpha/image-viewer access/alpha/store-build"
  DEV_ONLY="areas/access/sync/store areas/access/media/store"
  local role
  for role in core ports store web server api-oai components image-viewer store-build; do
    crate "areas/access/alpha/$role" "alpha-$role"
  done
  crate areas/access/sync/store sync-store
  crate areas/access/media/store media-store
  crate areas/access/server access-server
  crate shared/metadata shared-metadata
  crate modules/mosaic/tiles mosaic-tiles

  dep areas/access/alpha/api-oai 'alpha-core = { path = "@/areas/access/alpha/core" }'
  dep areas/access/alpha/api-oai 'shared-metadata = { path = "@/shared/metadata" }'
  dep areas/access/alpha/api-oai 'axum = "0.8"'
  dep areas/access/alpha/image-viewer 'shared-metadata = { path = "@/shared/metadata" }'
  dep areas/access/alpha/image-viewer 'maud = "0.27"'
  dep areas/access/alpha/components 'alpha-core = { path = "@/areas/access/alpha/core" }'
  dep areas/access/alpha/components 'alpha-image-viewer = { path = "@/areas/access/alpha/image-viewer" }'
  dep areas/access/alpha/components 'maud = "0.27"'
  dep areas/access/alpha/store 'rusqlite = "0.37"'
  dep areas/access/alpha/web 'alpha-components = { path = "@/areas/access/alpha/components" }'
  dep areas/access/alpha/web 'alpha-store = { path = "@/areas/access/alpha/store" }'
  dep areas/access/alpha/web 'mosaic-tiles = { path = "@/modules/mosaic/tiles" }'
  dep areas/access/alpha/store-build 'alpha-core = { path = "@/areas/access/alpha/core" }'
  dep areas/access/alpha/store-build 'alpha-ports = { path = "@/areas/access/alpha/ports" }'
  dep areas/access/alpha/server 'alpha-api-oai = { path = "@/areas/access/alpha/api-oai" }'
  dep areas/access/alpha/server 'alpha-store-build = { path = "@/areas/access/alpha/store-build" }'
  dep areas/access/alpha/server 'alpha-web = { path = "@/areas/access/alpha/web" }'
  dep areas/access/alpha/server 'mosaic-tiles = { path = "@/modules/mosaic/tiles" }'
  dep areas/access/alpha/server 'axum = "0.8"'
  dep areas/access/sync/store 'alpha-ports = { path = "@/areas/access/alpha/ports" }'
  dep areas/access/media/store 'alpha-ports = { path = "@/areas/access/alpha/ports" }'
  dep areas/access/alpha/store-build '[dev-dependencies]'
  dep areas/access/alpha/store-build 'sync-store = { path = "@/areas/access/sync/store" }'
  dep areas/access/alpha/server '[dev-dependencies]'
  dep areas/access/alpha/server 'sync-store = { path = "@/areas/access/sync/store" }'
  dep areas/access/alpha/server 'media-store = { path = "@/areas/access/media/store" }'
}

# run_gate: write the workspace manifest, run the gate in it, remove it;
# echo its exit status, then its stderr.
run_gate() {
  local members err status
  members="$(sed 's/.*/"&",/' "$WS/members")"
  printf '[workspace]\nresolver = "2"\nmembers = [\n%s\n]\n' "$members" >"$WS/Cargo.toml"
  err="$( cd "$WS" && main 2>&1 >/dev/null )"
  status=$?
  rm -rf "$WS"
  printf '%s\n%s\n' "$status" "$err"
}

rc() {
  local out
  out="$(run_gate)"
  echo "${out%%$'\n'*}"
}

new_ws
dep areas/deposit/alpha/core 'serde = "1"'
dep areas/deposit/alpha/web 'axum = "0.8"'
check "every allowed edge, and third-party crates no role denies, pass" 0 "$(rc)"

new_ws
dep areas/deposit/alpha/core 'beta-core = { path = "@/areas/deposit/beta/core" }'
fails_with "a core crate on another capability's core fails" "areas/deposit/alpha/core depends on beta-core (areas/deposit/beta/core, normal), an edge its role does not allow"
new_ws
dep areas/deposit/alpha/core 'mosaic-tiles = { path = "@/modules/mosaic/tiles" }'
fails_with "a core crate on mosaic-tiles fails" "areas/deposit/alpha/core depends on mosaic-tiles (modules/mosaic/tiles, normal), an edge its role does not allow"

new_ws
dep areas/deposit/alpha/store 'beta-core = { path = "@/areas/deposit/beta/core" }'
fails_with "a store crate on another capability's core fails" "areas/deposit/alpha/store depends on beta-core (areas/deposit/beta/core, normal), an edge its role does not allow"

new_ws
dep areas/deposit/alpha/web 'beta-web = { path = "@/areas/deposit/beta/web" }'
fails_with "a web crate on another capability's web fails" "areas/deposit/alpha/web depends on beta-web (areas/deposit/beta/web, normal), an edge its role does not allow"
new_ws
dep areas/deposit/alpha/web 'beta-ports = { path = "@/areas/deposit/beta/ports" }'
fails_with "a web crate on another capability's ports fails" "areas/deposit/alpha/web depends on beta-ports (areas/deposit/beta/ports, normal), an edge its role does not allow"

new_ws
dep areas/deposit/alpha/ports 'alpha-core = { path = "@/areas/deposit/alpha/core" }'
fails_with "a ports crate on its own capability's core fails" "areas/deposit/alpha/ports depends on alpha-core (areas/deposit/alpha/core, normal), an edge its role does not allow"

new_ws
dep areas/deposit/alpha/pull-requests 'beta-ports = { path = "@/areas/deposit/beta/ports" }'
fails_with "a pull-requests crate on another capability's ports fails" "areas/deposit/alpha/pull-requests depends on beta-ports (areas/deposit/beta/ports, normal), an edge its role does not allow"

new_ws
dep areas/deposit/shell 'alpha-web = { path = "@/areas/deposit/alpha/web" }'
fails_with "the shell on a capability crate fails" "areas/deposit/shell depends on alpha-web (areas/deposit/alpha/web, normal), an edge its role does not allow"

new_ws
crate areas/access/dpe/core dpe-core
dep areas/deposit/alpha/core 'dpe-core = { path = "@/areas/access/dpe/core" }'
fails_with "a capability crate on another area's crate fails" "areas/deposit/alpha/core depends on dpe-core (areas/access/dpe/core, normal), an edge its role does not allow"

new_ws
dep areas/deposit/alpha/web 'other = { package = "beta-core", path = "@/areas/deposit/beta/core" }'
fails_with "a renamed dependency on another capability's core fails" "areas/deposit/alpha/web depends on beta-core (areas/deposit/beta/core, normal), an edge its role does not allow"

new_ws
dep areas/deposit/alpha/web '[dev-dependencies]'
dep areas/deposit/alpha/web 'beta-store = { path = "@/areas/deposit/beta/store" }'
fails_with "a dev-dependency on another capability's store fails" "areas/deposit/alpha/web depends on beta-store (areas/deposit/beta/store, dev), an edge its role does not allow (across capabilities, a dev-dependency only on a crate in DEV_ONLY, ADR-0008)"

new_ws
crate areas/deposit/alpha/cli alpha-cli
fails_with "a crate in an unknown role fails" "alpha-cli (areas/deposit/alpha/cli) is in no role this gate knows: core, store, ports, web, server or api-* under a capability, a role CAPABILITY_ROLES gives its capability, or the area's shell or server"

new_ws
dep areas/deposit/alpha/ports 'serde = "1"'
fails_with "a ports crate on any third-party crate fails" "areas/deposit/alpha/ports depends on serde (third-party, normal), a crate its role may not use"

new_ws
dep areas/deposit/alpha/core 'axum = "0.8"'
fails_with "a core crate on a web framework fails" "areas/deposit/alpha/core depends on axum (third-party, normal), a crate its role may not use"
new_ws
dep areas/deposit/alpha/core 'html = { package = "maud", version = "0.27" }'
fails_with "a core crate on Maud under another name fails" "areas/deposit/alpha/core depends on maud (third-party, normal), a crate its role may not use"
new_ws
dep areas/deposit/alpha/core '[dev-dependencies]'
dep areas/deposit/alpha/core 'rusqlite = "0.37"'
fails_with "a core crate on a database crate, dev-only, fails" "areas/deposit/alpha/core depends on rusqlite (third-party, dev), a crate its role may not use"

new_ws
dep areas/deposit/shell 'axum = "0.8"'
fails_with "the shell on a web framework fails" "areas/deposit/shell depends on axum (third-party, normal), a crate its role may not use"

new_access_ws
check "every allowed Access Area edge, the DEV_ONLY dev-dependencies included, passes" 0 "$(rc)"

new_access_ws
dep areas/access/alpha/api-oai 'alpha-web = { path = "@/areas/access/alpha/web" }'
fails_with "an api-* crate on its own capability's web crate fails" "areas/access/alpha/api-oai depends on alpha-web (areas/access/alpha/web, normal), an edge its role does not allow"

new_access_ws
dep areas/access/alpha/image-viewer 'alpha-core = { path = "@/areas/access/alpha/core" }'
fails_with "an image-viewer crate on its own capability's core fails" "areas/access/alpha/image-viewer depends on alpha-core (areas/access/alpha/core, normal), an edge its role does not allow"

new_access_ws
dep areas/access/alpha/web 'sync-store = { path = "@/areas/access/sync/store" }'
fails_with "a DEV_ONLY crate as a normal dependency fails" "areas/access/alpha/web depends on sync-store (areas/access/sync/store, normal), an edge its role does not allow"

new_access_ws
dep areas/access/alpha/components '[dev-dependencies]'
dep areas/access/alpha/components 'rusqlite = "0.37"'
fails_with "a components crate on a database crate fails" "areas/access/alpha/components depends on rusqlite (third-party, dev), a crate its role may not use"

new_access_ws
dep areas/access/alpha/components 'tower = "0.5"'
fails_with "a components crate on a web framework fails" "areas/access/alpha/components depends on tower (third-party, normal), a crate its role may not use"

new_access_ws
dep areas/access/alpha/web 'axum = "0.8"'
fails_with "a web crate beside a server crate on a web framework fails" "areas/access/alpha/web depends on axum (third-party, normal), a crate its role may not use"

new_access_ws
dep areas/access/alpha/server "[target.'cfg(unix)'.dependencies]"
dep areas/access/alpha/server 'beta = { package = "media-store", path = "@/areas/access/media/store" }'
fails_with "a server crate on another capability's store fails" "areas/access/alpha/server depends on media-store (areas/access/media/store, normal), an edge its role does not allow"

new_access_ws
CHECKED_AREAS="deposit access"
crate areas/deposit/beta/core beta-core
dep areas/deposit/beta/core '[dev-dependencies]'
dep areas/deposit/beta/core 'sync-store = { path = "@/areas/access/sync/store" }'
fails_with "another area's DEV_ONLY crate as a dev-dependency fails" "areas/deposit/beta/core depends on sync-store (areas/access/sync/store, dev), an edge its role does not allow (across capabilities, a dev-dependency only on a crate in DEV_ONLY, ADR-0008)"

new_access_ws
dep areas/access/alpha/ports '[dev-dependencies]'
dep areas/access/alpha/ports 'sync-store = { path = "@/areas/access/sync/store" }'
fails_with "a ports crate on a DEV_ONLY crate, dev-only, fails" "areas/access/alpha/ports depends on sync-store (areas/access/sync/store, dev), an edge its role does not allow (across capabilities, a dev-dependency only on a crate in DEV_ONLY, ADR-0008)"

new_access_ws
dep areas/access/media/store '[dev-dependencies]'
dep areas/access/media/store 'sync-store = { path = "@/areas/access/sync/store" }'
fails_with "a DEV_ONLY provider on another DEV_ONLY crate, dev-only, fails" "areas/access/media/store depends on sync-store (areas/access/sync/store, dev), an edge its role does not allow (across capabilities, a dev-dependency only on a crate in DEV_ONLY, ADR-0008)"

new_access_ws
crate areas/access/beta/components beta-components
fails_with "a role CAPABILITY_ROLES gives another capability fails" "beta-components (areas/access/beta/components) is in no role this gate knows: core, store, ports, web, server or api-* under a capability, a role CAPABILITY_ROLES gives its capability, or the area's shell or server"

new_access_ws
dep areas/access/alpha/web 'axum-extra = "0.10"'
fails_with "a web crate beside a server crate on an axum-* crate fails" "areas/access/alpha/web depends on axum-extra (third-party, normal), a crate its role may not use"

new_access_ws
DEV_ONLY="areas/access/sync/store areas/access/gone/store"
fails_with "a DEV_ONLY path that is no workspace crate fails" "DEV_ONLY names areas/access/gone/store, which is no workspace crate"

WS="$(mktemp -d "${TMPDIR:-/tmp}/capability-deps.XXXXXX")" || exit 1
CHECKED_AREAS=deposit
crate areas/deposit/shell deposit-shell
crate areas/deposit/server deposit-server
fails_with "an area with no capability crate fails" "areas/deposit has no capability crate (areas/deposit/<capability>/<role>/Cargo.toml)"

echo
echo "check-capability-deps tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
