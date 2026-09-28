#!/usr/bin/env bash
#
# Tests for check-composition-root-deps.sh. Dependency-free: bash, git, awk, grep.
#
# Fixtures build throwaway git repos because the gate reads its forbidden set
# out of the index (every areas/<area>/<capability>/{core,web,api-*}/Cargo.toml's
# [package] name), so stubbing git would test the stub, not the derivation.
# mktemp names an explicit TMPDIR template: a bare mktemp resolves to a path a
# sandboxed shell may not be allowed to write.
#
# Run: bash .github/scripts/check-composition-root-deps.test.sh   (or `just test`)

set -uo pipefail

# See check-shared-paths.test.sh for why this is unset: git init honours an
# inherited GIT_DIR over the directory it is run in.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-composition-root-deps.sh disable=SC1091
source "$SCRIPT_DIR/check-composition-root-deps.sh"

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

# crate <dir> <name>: a minimal crate Cargo.toml at $dir/Cargo.toml.
crate() {
  mkdir -p "$1"
  printf '[package]\nname = "%s"\nversion = "0.1.0"\n' "$2" >"$1/Cargo.toml"
}

# make_repo: throwaway repo shaped like this one — one area (access), one
# capability (dpe) with core/web/api-oai crates and a server crate, plus a
# composition root at areas/access/server whose Cargo.toml is $1's contents.
# Echoes the repo path.
make_repo() {
  local dir
  dir="$(mktemp -d "${TMPDIR:-/tmp}/comp-root-deps.XXXXXX")"
  crate "$dir/areas/access/dpe/core" dpe-core
  crate "$dir/areas/access/dpe/web" dpe-web
  crate "$dir/areas/access/dpe/api-oai" dpe-api-oai
  crate "$dir/areas/access/dpe/server" dpe-server
  mkdir -p "$dir/shared/telemetry"
  printf '[package]\nname = "shared-telemetry"\nversion = "0.1.0"\n' >"$dir/shared/telemetry/Cargo.toml"
  mkdir -p "$dir/areas/access/server"
  printf '%s\n' "$1" >"$dir/areas/access/server/Cargo.toml"
  ( cd "$dir" && git init -q -b main && git config user.email t@example.com \
      && git config user.name t && git add -A && git commit -qm init )
  printf '%s' "$dir"
}

# gate_rc <server-cargo-toml-content>: fresh repo with that content at
# areas/access/server/Cargo.toml, echo the gate's exit status.
gate_rc() {
  local repo rc
  repo="$(make_repo "$1")"
  ( cd "$repo" && main >/dev/null 2>&1 )
  rc=$?
  rm -rf "$repo"
  echo "$rc"
}

DEPS_HEADER='[package]
name = "access-server"
version = "0.1.0"

[dependencies]'

# 1. A dependency on a capability's core crate fails.
check "a dependency on a capability's core crate fails" 1 \
  "$(gate_rc "$DEPS_HEADER
dpe-core = { path = \"../dpe/core\" }")"

# 2. Depending only on the server crate and a shared crate passes.
check "server + shared dependencies pass" 0 \
  "$(gate_rc "$DEPS_HEADER
dpe-server = { path = \"../dpe/server\" }
shared-telemetry = { path = \"../../../shared/telemetry\" }")"

# 3. A dependency on a capability's web crate fails.
check "a dependency on a capability's web crate fails" 1 \
  "$(gate_rc "$DEPS_HEADER
dpe-web = { path = \"../dpe/web\" }")"

# 4. A dependency on a capability's api-* crate fails.
check "a dependency on a capability's api-* crate fails" 1 \
  "$(gate_rc "$DEPS_HEADER
dpe-api-oai = { path = \"../dpe/api-oai\" }")"

# 5. A third-party crate is unrestricted.
check "a third-party dependency passes" 0 \
  "$(gate_rc "$DEPS_HEADER
dpe-server = { path = \"../dpe/server\" }
axum = \"0.8\"")"

# 6. The `.workspace = true` dependency form is caught too, not just the
#    inline-table form — the derivation reads [package] name, not a fixed
#    shape of the dependent's line.
check "a .workspace-form dependency on a core crate fails" 1 \
  "$(gate_rc "$DEPS_HEADER
dpe-core.workspace = true")"

# 7. A crate whose name shares a forbidden crate's prefix is not a false
#    positive — the match is anchored, not substring.
check "a same-prefix crate name does not false-positive" 0 \
  "$(gate_rc "$DEPS_HEADER
dpe-server = { path = \"../dpe/server\" }
dpe-core-fixtures = { path = \"../../../shared/dpe-core-fixtures\" }")"

# 8. A renamed dependency is caught by its `package = "..."`, in the inline
#    form and in the table form.
check "a renamed inline dependency on a core crate fails" 1 \
  "$(gate_rc "$DEPS_HEADER
core = { package = \"dpe-core\", path = \"../dpe/core\" }")"
check "a renamed table-form dependency on a web crate fails" 1 \
  "$(gate_rc "$DEPS_HEADER
[dependencies.views]
package = \"dpe-web\"
path = \"../dpe/web\"")"

# 9. Absence of any areas/*/server/Cargo.toml is an error, not zero work.
repo="$(make_repo "$DEPS_HEADER
dpe-server = { path = \"../dpe/server\" }")"
( cd "$repo" && git rm -rqf areas/access/server/Cargo.toml && git commit -qm drop && main >/dev/null 2>&1 )
check "no areas/*/server/Cargo.toml at all fails" 1 "$?"
rm -rf "$repo"

echo
echo "check-composition-root-deps tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
