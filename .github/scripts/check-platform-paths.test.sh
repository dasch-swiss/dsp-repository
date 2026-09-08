#!/usr/bin/env bash
#
# Tests for check-platform-paths.sh. Dependency-free: bash, git and awk.
#
# Fixtures build throwaway git repos because the gate reads both halves of the
# rule out of the index, and stubbing git would test the stub. mktemp names an
# explicit TMPDIR template: a bare mktemp resolves to a path a sandboxed shell
# may not be allowed to write.
#
# Six cases, one per distinct failure mode. The no-fire case matters most: a
# false positive there would be "fixed" by deleting documentation. The nested
# case is not redundant with the shallow ones, because the pathspecs only
# recurse while they reach git unexpanded.
#
# Run: bash .github/scripts/check-platform-paths.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-platform-paths.sh disable=SC1091
source "$SCRIPT_DIR/check-platform-paths.sh"

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

# make_repo: throwaway repo shaped like this one, two service modules and two
# platform crates, all clean. Echoes its path.
make_repo() {
  local dir f
  dir="$(mktemp -d "${TMPDIR:-/tmp}/platform-paths.XXXXXX")"
  mkdir -p "$dir/modules/dpe/server/data"
  printf '{}\n' >"$dir/modules/dpe/server/data/x.json"
  for f in editor/core platform/metadata platform/telemetry; do
    mkdir -p "$dir/modules/$f/src"
    printf 'pub fn a() {}\n' >"$dir/modules/$f/src/lib.rs"
  done
  ( cd "$dir" && git init -q -b main && git config user.email t@example.com \
      && git config user.name t && git add -A && git commit -qm init )
  printf '%s' "$dir"
}

# gate_rc <file> <line...>: fresh repo with this file committed, echo the gate's
# exit status.
gate_rc() {
  local file="$1"; shift
  local repo rc
  repo="$(make_repo)"
  mkdir -p "$repo/$(dirname "$file")"
  printf '%s\n' "$@" >"$repo/$file"
  ( cd "$repo" && git add -A && git commit -qm add && main >/dev/null 2>&1 )
  rc=$?
  rm -rf "$repo"
  echo "$rc"
}

# 1. A clean tree passes.
repo="$(make_repo)"
( cd "$repo" && main >/dev/null 2>&1 )
check "a clean platform tree passes" 0 "$?"
rm -rf "$repo"

# 2. A relative path into a service module fails.
check "a relative include_str! into a service fails" 1 \
  "$(gate_rc modules/platform/metadata/src/bad.rs \
     'const X: &str = include_str!("../../../dpe/server/data/x.json");')"

# 3. Same, one directory deeper. Fails the moment PLATFORM_PATHSPECS stops
#    being a quoted array, which is how that bug was caught.
check "a violation in a nested src/ subdirectory fails" 1 \
  "$(gate_rc modules/platform/metadata/src/validators/deep.rs \
     'const X: &str = include_str!("../../../../dpe/server/data/x.json");')"

# 4. The three shapes that must not fire, all real code in modules/platform.
check "prose, routes and docs paths do not fire" 0 \
  "$(gate_rc modules/platform/telemetry/src/ok.rs \
     '//! Page-URL normalization stays in `dpe-server`; cf. dpe_core::utils.' \
     '/// see `docs/src/dpe/oai-pmh.md` for why' \
     'const ROUTE: &str = "/dpe/projects";')"

# 5. The forbidden set is derived: a module that did not exist when this gate
#    was written is covered without editing it. mosaic arrived that way.
repo="$(make_repo)"
mkdir -p "$repo/modules/mosaic/tiles/src" "$repo/modules/platform/metadata/src"
printf 'pub fn d() {}\n' >"$repo/modules/mosaic/tiles/src/lib.rs"
printf 'const X: &str = include_str!("../../../mosaic/tiles/src/lib.rs");\n' \
  >"$repo/modules/platform/metadata/src/bad.rs"
( cd "$repo" && git add -A && git commit -qm mosaic && main >/dev/null 2>&1 )
check "a path into a module added later fails" 1 "$?"
rm -rf "$repo"

# 6. Absence is an error, not zero work.
repo="$(make_repo)"
( cd "$repo" && git rm -rq modules/platform && git commit -qm drop && main >/dev/null 2>&1 )
check "no platform sources at all fails" 1 "$?"
rm -rf "$repo"

echo
echo "check-platform-paths tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
