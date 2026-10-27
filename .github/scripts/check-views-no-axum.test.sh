#!/usr/bin/env bash
#
# Tests for check-views-no-axum.sh. Dependency-free: bash and git.
#
# Fixtures build throwaway git repos because the gate reads the tree out of the
# index, and stubbing git would test the stub. mktemp names an explicit TMPDIR
# template: a bare mktemp resolves to a path a sandboxed shell may not write.
#
# Run: bash .github/scripts/check-views-no-axum.test.sh   (or `just test`)

set -uo pipefail

# See check-datastar-attributes.test.sh: git init honours an inherited GIT_DIR
# over the directory it is run in.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-views-no-axum.sh disable=SC1091
source "$SCRIPT_DIR/check-views-no-axum.sh"

PASS=0
FAIL=0

check() {
  if [ "$2" = "$3" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $1 (expected rc=$2, got rc=$3)"
  fi
}

# make_repo: throwaway repo with the two web crates, each with a clean view and
# axum in every place that may use it.
make_repo() {
  local dir c
  dir="$(mktemp -d "${TMPDIR:-/tmp}/views-no-axum.XXXXXX")"
  for c in editor identity; do
    mkdir -p "$dir/areas/deposit/$c/web/src/pages" "$dir/areas/deposit/$c/web/src/handlers"
    printf 'pub fn render() -> maud::Markup { maud::html! {} }\n' >"$dir/areas/deposit/$c/web/src/pages/mod.rs"
    printf 'use axum::response::IntoResponse;\n' >"$dir/areas/deposit/$c/web/src/handlers/mod.rs"
    printf 'use axum::Router;\n' >"$dir/areas/deposit/$c/web/src/routes.rs"
    printf 'use axum::http::HeaderMap;\n' >"$dir/areas/deposit/$c/web/src/shell.rs"
    printf 'pub use axum::Router;\n' >"$dir/areas/deposit/$c/web/src/lib.rs"
    printf 'use axum::body::Body;\n' >"$dir/areas/deposit/$c/web/src/fixtures.rs"
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

repo="$(make_repo)"
( cd "$repo" && main >/dev/null 2>&1 )
check "clean views beside axum in handlers/, routes.rs, shell.rs, lib.rs and fixtures.rs pass" 0 "$?"
rm -rf "$repo"

check "a use of axum in the editor's pages/ fails" 1 \
  "$(gate_rc areas/deposit/editor/web/src/pages/section.rs 'use axum::response::Html;')"
check "an inline axum path in the editor's form/ fails" 1 \
  "$(gate_rc areas/deposit/editor/web/src/form/widgets.rs '    let s = axum::http::StatusCode::OK;')"
check "an axum_extra path in the editor's entity/ fails" 1 \
  "$(gate_rc areas/deposit/editor/web/src/entity/mod.rs 'use axum_extra::extract::Form;')"
check "a use of axum in identity's pages/ fails" 1 \
  "$(gate_rc areas/deposit/identity/web/src/pages/login.rs 'use axum::Json;')"
check "a use of axum in a nested view directory fails" 1 \
  "$(gate_rc areas/deposit/editor/web/src/pages/review/rows.rs 'use axum::Form;')"
check "a use of axum in a view directory still to come fails" 1 \
  "$(gate_rc areas/deposit/editor/web/src/components/row.rs 'use axum::Json;')"
check "a use of axum in another top-level module fails" 1 \
  "$(gate_rc areas/deposit/identity/web/src/startup.rs 'use axum::Router;')"
check "a file named like an allowed one, deeper down, is checked" 1 \
  "$(gate_rc areas/deposit/editor/web/src/pages/lib.rs 'use axum::Router;')"
check "a name that ends in axum is no axum path" 0 \
  "$(gate_rc areas/deposit/editor/web/src/pages/section.rs 'use crate::not_axum::Thing;')"
check "axum in a handler stays allowed" 0 \
  "$(gate_rc areas/deposit/identity/web/src/handlers/login.rs 'use axum::Form;')"

repo="$(make_repo)"
( cd "$repo" && git rm -rq areas/deposit/identity/web/src && git commit -qm drop && main >/dev/null 2>&1 )
check "a web crate with no tracked source fails" 1 "$?"
rm -rf "$repo"

echo
echo "check-views-no-axum tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
