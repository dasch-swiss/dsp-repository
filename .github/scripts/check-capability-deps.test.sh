#!/usr/bin/env bash
#
# Tests for check-capability-deps.sh. Needs bash.
#
# Fixtures are throwaway trees of hand-written BUILD.bazel files; the gate runs
# with the tree as its working directory.
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
# fails with that line, so a fixture that breaks the gate cannot pass a case.
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

# build <dir> <line>...: append lines to $WS/<dir>/BUILD.bazel.
build() {
  local dir="$1"
  shift
  mkdir -p "$WS/$dir"
  printf '%s\n' "$@" >>"$WS/$dir/BUILD.bazel"
}

# new_ws: a tree shaped like the Deposit Area with every role present and
# third-party crates its roles allow. Sets WS and CHECKED_AREAS.
new_ws() {
  WS="$(mktemp -d "${TMPDIR:-/tmp}/capability-deps.XXXXXX")" || exit 1
  # shellcheck disable=SC2034 # read by main in the sourced script
  CHECKED_AREAS=deposit
  build areas/deposit/alpha/core 'rust_library(name = "core", deps = ["@crates//:serde"])'
  build areas/deposit/alpha/ports 'rust_library(name = "ports")'
  build areas/deposit/alpha/store 'rust_library(name = "store", deps = ["@crates//:rusqlite", "@crates//:axum"])'
  build areas/deposit/alpha/web 'rust_library(name = "web", deps = ["@crates//:axum", "@crates//:maud"])'
  build areas/deposit/alpha/components 'rust_library(name = "components", deps = ["@crates//:maud"])'
  build areas/deposit/alpha/image-viewer 'rust_library(name = "image-viewer", deps = ["@crates//:maud"])'
  build areas/deposit/alpha/lift 'rust_library(name = "lift", deps = ["@crates//:oxigraph"])'
  build areas/deposit/alpha/client 'rust_library(name = "client", deps = ["@crates//:reqwest", "@crates//:tokio"])'
  build areas/deposit/shell 'rust_library(name = "shell", deps = ["@crates//:maud"])'
  build areas/deposit/server 'rust_binary(name = "server", deps = ["@crates//:axum"])'
  build shared/metadata 'rust_library(name = "metadata", visibility = ["//areas:__subpackages__"])'
}

# run_gate: run the gate in $WS, remove it; echo its exit status, then its
# stderr.
run_gate() {
  local err status
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
check "every role present, with the third-party crates its role allows, passes" 0 "$(rc)"

new_ws
build areas/deposit/alpha/ports 'rust_test(name = "t", deps = ["@crates//:serde"])'
fails_with "a ports crate on any third-party crate fails" "areas/deposit/alpha/ports/BUILD.bazel uses serde (third-party), a crate its role may not use"

new_ws
build areas/deposit/alpha/core 'rust_library(name = "x", deps = ["@crates//:axum"])'
fails_with "a core crate on a web framework fails" "areas/deposit/alpha/core/BUILD.bazel uses axum (third-party), a crate its role may not use"
new_ws
build areas/deposit/alpha/core 'rust_library(name = "x", deps = ["@crates//:maud"])'
fails_with "a core crate on Maud fails" "areas/deposit/alpha/core/BUILD.bazel uses maud (third-party), a crate its role may not use"
new_ws
build areas/deposit/alpha/core 'rust_test(name = "t", deps = ["@crates//:rusqlite"])'
fails_with "a core crate on a database crate, in a test, fails" "areas/deposit/alpha/core/BUILD.bazel uses rusqlite (third-party), a crate its role may not use"

new_ws
build areas/deposit/alpha/components 'rust_library(name = "x", deps = ["@crates//:axum"])'
fails_with "a components crate on a web framework fails" "areas/deposit/alpha/components/BUILD.bazel uses axum (third-party), a crate its role may not use"
new_ws
build areas/deposit/alpha/image-viewer 'rust_library(name = "x", deps = ["@crates//:tower"])'
fails_with "an image-viewer crate on a web framework fails" "areas/deposit/alpha/image-viewer/BUILD.bazel uses tower (third-party), a crate its role may not use"

new_ws
build areas/deposit/alpha/server 'rust_library(name = "server")'
fails_with "a web crate beside a server crate on a web framework fails" "areas/deposit/alpha/web/BUILD.bazel uses axum (third-party), a crate its role may not use"

new_ws
build areas/deposit/beta/web 'rust_library(name = "web", deps = ["@crates//:axum"])'
check "a web crate with no server crate beside it may use a web framework" 0 "$(rc)"

new_ws
build areas/deposit/alpha/lift 'rust_library(name = "x", deps = ["@crates//:reqwest"])'
fails_with "a lift crate on an HTTP client fails" "areas/deposit/alpha/lift/BUILD.bazel uses reqwest (third-party), a crate its role may not use"

new_ws
build areas/deposit/alpha/client 'rust_test(name = "t", deps = ["@crates//:axum"])'
fails_with "a client crate on a web framework, in a test, fails" "areas/deposit/alpha/client/BUILD.bazel uses axum (third-party), a crate its role may not use"
new_ws
build areas/deposit/alpha/client 'rust_library(name = "x", deps = ["@crates//:maud"])'
fails_with "a client crate on Maud fails" "areas/deposit/alpha/client/BUILD.bazel uses maud (third-party), a crate its role may not use"
new_ws
build areas/deposit/alpha/client 'rust_library(name = "x", deps = ["@crates//:deadpool-sqlite"])'
fails_with "a client crate on a database crate fails" "areas/deposit/alpha/client/BUILD.bazel uses deadpool-sqlite (third-party), a crate its role may not use"

new_ws
build areas/deposit/shell 'rust_library(name = "x", deps = ["@crates//:tower-http"])'
fails_with "the shell on a tower-* crate fails" "areas/deposit/shell/BUILD.bazel uses tower-http (third-party), a crate its role may not use"

new_ws
build areas/deposit/alpha/store 'rust_library(name = "x", deps = ["@crates//:hyper"])'
check "a store crate may use a web framework" 0 "$(rc)"
new_ws
build areas/deposit/alpha/store 'rust_library(name = "x", deps = ["@crates//:notify"])'
fails_with "a store crate on a file-notification crate fails" "areas/deposit/alpha/store/BUILD.bazel uses notify (third-party), a crate its role may not use"
new_ws
build areas/deposit/alpha/store 'rust_test(name = "t", deps = ["@crates//:inotify-sys"])'
fails_with "a store crate on an inotify-* crate, in a test, fails" "areas/deposit/alpha/store/BUILD.bazel uses inotify-sys (third-party), a crate its role may not use"

new_ws
build areas/deposit/alpha/core '# no @crates//:axum here, only a comment' 'rust_library(name = "x") # @crates//:rusqlite'
check "a ban named only in a comment passes" 0 "$(rc)"

new_ws
build areas/deposit/alpha/core/sub 'rust_library(name = "sub", deps = ["@crates//:axum"])'
check "a nested package below a role is not checked" 0 "$(rc)"

for dir in areas/deposit/other shared/other modules/mosaic dsp-cli; do
  new_ws
  build "$dir" 'rust_library(name = "pub", visibility = ["//visibility:public"])'
  fails_with "a public target under ${dir%%/*}/ fails" "$dir/BUILD.bazel:1 makes a target public (//visibility:public)"
done

new_ws
build shared/metadata '# widening to //visibility:public would undo the boundary'
check "//visibility:public in a comment passes" 0 "$(rc)"

new_ws
build tools/release 'rust_binary(name = "release", visibility = ["//visibility:public"])'
check "a public target outside areas, shared, modules and dsp-cli passes" 0 "$(rc)"

new_ws
build shared/metadata 'package(default_visibility = ["//visibility:public"])'
fails_with "a public default_visibility fails" "shared/metadata/BUILD.bazel:2 makes a target public (//visibility:public)"

echo
echo "check-capability-deps tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
