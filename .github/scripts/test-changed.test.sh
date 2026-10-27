#!/usr/bin/env bash
#
# Tests for test-changed.sh. Needs bash, git, awk, perl, cargo and jq.
#
# Fixtures are throwaway git repositories holding a Cargo workspace, not
# hand-written metadata JSON: a dev-only edge is whatever cargo reports. The
# script runs under TEST_CHANGED_DRY_RUN, so the assertions are on the selection
# and the commands it prints.
#
# Run: bash .github/scripts/test-changed.test.sh   (or `just test`)

set -uo pipefail

# Fixtures build throwaway repositories with `git init`, which honours an
# inherited GIT_DIR over the directory it is run in: without this, a run under
# `git rebase --exec` or a git hook re-initializes the caller's repository.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./test-changed.sh disable=SC1091
source "$SCRIPT_DIR/test-changed.sh"

# The fixture's rule tables, not the repository's.
EXTRA_EDGES=("docs/src/a-data.md a")
UNOWNED_TO_PACKAGE=("assets/* b")
UNOWNED_NO_TESTS=('docs/*' 'README.md' '*/README.md' '.github/workflows/*')

export TEST_CHANGED_DRY_RUN=1

PASS=0
FAIL=0

WS=""
OUT=""
RC=0

git_ws() {
  git -C "$WS" -c user.name=test -c user.email=test@example.com -c commit.gpgsign=false "$@"
}

crate() {
  local dir="$1" name="$2"
  shift 2
  mkdir -p "$WS/$dir/src"
  : >"$WS/$dir/src/lib.rs"
  printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2021"\n\n' "$name" >"$WS/$dir/Cargo.toml"
  if [ "$#" -gt 0 ]; then printf '%s\n' "$@" >>"$WS/$dir/Cargo.toml"; fi
}

# new_ws: a committed git repository on `main` with the packages
#   a (pkgs/a), a/inner (pkgs/a/inner), b (depends on a), c (dev-depends on a),
# a script x.sh with its test, and a docs/ tree.
new_ws() {
  WS="$(mktemp -d "${TMPDIR:-/tmp}/test-changed.XXXXXX")" || exit 1
  git init -q -b main "$WS"
  printf '[workspace]\nresolver = "2"\nmembers = ["pkgs/*", "pkgs/a/inner"]\n' >"$WS/Cargo.toml"
  : >"$WS/Cargo.lock"
  crate pkgs/a a
  crate pkgs/a/inner inner
  crate pkgs/b b '[dependencies]' 'a = { path = "../a" }'
  crate pkgs/c c '[dev-dependencies]' 'a = { path = "../a" }'
  mkdir -p "$WS/.github/scripts" "$WS/docs/src" "$WS/assets"
  : >"$WS/.github/scripts/x.sh"
  : >"$WS/.github/scripts/x.test.sh"
  : >"$WS/.github/scripts/y.sh"
  : >"$WS/docs/src/a-data.md"
  : >"$WS/docs/src/other.md"
  : >"$WS/assets/logo.svg"
  git_ws add -A
  git_ws commit -q -m fixture
}

touch_path() {
  mkdir -p "$(dirname "$WS/$1")"
  echo "# change" >>"$WS/$1"
}

run_main() {
  OUT="$(cd "$WS/${2:-.}" && main "$1" 2>&1)"
  RC=$?
}

run_audit() {
  OUT="$(cd "$WS" && audit 2>&1)"
  RC=$?
}

ok() {
  PASS=$((PASS + 1))
}

bad() {
  FAIL=$((FAIL + 1))
  echo "FAIL: $1"
  printf '%s\n' "$OUT" | sed 's/^/  /'
}

expect_line() {
  if grep -qxF -- "$2" <<<"$OUT"; then ok; else bad "$1 (expected line: $2)"; fi
}

expect_no() {
  if grep -qF -- "$2" <<<"$OUT"; then bad "$1 (unexpected: $2)"; else ok; fi
}

expect_rc() {
  if [ "$RC" = "$2" ]; then ok; else bad "$1 (expected rc=$2, got rc=$RC)"; fi
}

new_ws
touch_path pkgs/a/src/lib.rs
run_main main
expect_line "a change in a: selection with reasons" "a (pkgs/a/src/lib.rs) + rdeps: b, c"
expect_line "a change in a: runs a and its dependents, dev ones included" "run: cargo test --tests -p a -p b -p c"
rm -rf "$WS"

new_ws
touch_path pkgs/b/src/lib.rs
run_main main
expect_line "a change in b: runs b only" "run: cargo test --tests -p b"
rm -rf "$WS"

new_ws
touch_path pkgs/a/inner/src/lib.rs
run_main main
expect_line "a change in a/inner: runs inner, not a" "run: cargo test --tests -p inner"
rm -rf "$WS"

# EXTRA_EDGES is additive: the path is still classified by the later rules.
new_ws
touch_path docs/src/a-data.md
run_main main
expect_line "extra edge under docs/: selects the edge's package and its rdeps" "a (docs/src/a-data.md) + rdeps: b, c"
rm -rf "$WS"

new_ws
EXTRA_EDGES=("pkgs/b/src/* a")
touch_path pkgs/b/src/lib.rs
run_main main
expect_line "extra edge on an owned path: owner and edge package" "run: cargo test --tests -p a -p b -p c"
EXTRA_EDGES=("docs/src/a-data.md a")
rm -rf "$WS"

new_ws
touch_path assets/logo.svg
run_main main
expect_line "unowned path served by a package" "b (assets/logo.svg)"
rm -rf "$WS"

new_ws
touch_path Cargo.lock
run_main main
expect_line "Cargo.lock: full run naming it" "full run: Cargo.lock"
rm -rf "$WS"

new_ws
touch_path Cargo.toml
run_main main
expect_line "root Cargo.toml: full run, owned by no package" "full run: Cargo.toml"
rm -rf "$WS"

# Only documentation by name runs nothing: data markdown that tests read falls
# to a full run.
new_ws
touch_path data/x.md
run_main main
expect_line "unowned data markdown: full run naming it" "full run: data/x.md"
rm -rf "$WS"

new_ws
rm -rf "$WS/pkgs/c"
run_main main
expect_line "a deleted package directory: full run naming its path" "full run: pkgs/c/Cargo.toml"
rm -rf "$WS"

new_ws
touch_path pkgs/a/src/lib.rs
run_main no-such-ref
expect_line "an unknown base: full run naming it" "full run: no merge-base with no-such-ref"
expect_line "an unknown base: runs just test" "run: just test"
rm -rf "$WS"

new_ws
EXTRA_EDGES=("docs/src/a-data.md missing")
touch_path docs/src/a-data.md
run_main main
expect_line "a table naming no workspace package: full run" "full run: docs/src/a-data.md (a rule table names missing, which is no workspace package)"
EXTRA_EDGES=("docs/src/a-data.md a")
rm -rf "$WS"

new_ws
touch_path docs/src/other.md
run_main main
expect_line "a docs-only change: nothing to test" "nothing to test since main"
expect_rc "a docs-only change: exits 0" 0
rm -rf "$WS"

new_ws
touch_path .github/scripts/x.sh
run_main main
expect_line "x.sh: runs x.test.sh" "run: bash .github/scripts/x.test.sh"
expect_no "x.sh: no cargo command" "cargo"
rm -rf "$WS"

new_ws
touch_path .github/scripts/x.test.sh
run_main main
expect_line "x.test.sh: selects itself" "run: bash .github/scripts/x.test.sh"
rm -rf "$WS"

new_ws
touch_path .github/scripts/y.sh
run_main main
expect_line "a script without a test: warns" "warning: .github/scripts/y.sh has no y.test.sh, nothing tests it"
rm -rf "$WS"

new_ws
touch_path .github/scripts/README
run_main main
expect_line "another path under .github/scripts: all script tests" "run: bash .github/scripts/x.test.sh"
rm -rf "$WS"

new_ws
touch_path pkgs/b/src/new.rs
run_main main pkgs/a/src
expect_line "an untracked file, run from a subdirectory" "b (pkgs/b/src/new.rs)"
rm -rf "$WS"

new_ws
git_ws checkout -q -b topic
touch_path pkgs/b/src/lib.rs
git_ws add -A
git_ws commit -q -m change
run_main main
expect_line "a committed change since the merge-base" "b (pkgs/b/src/lib.rs)"
rm -rf "$WS"

new_ws
crate pkgs/dpe-server dpe-server
crate pkgs/mosaic-playground mosaic-playground
git_ws add -A
git_ws commit -q -m more
touch_path pkgs/dpe-server/src/lib.rs
run_main main
expect_line "dpe-server selected: its dev feature only" "run: cargo test -p dpe-server --features dpe-server/dev --tests"
touch_path pkgs/mosaic-playground/src/lib.rs
run_main main
expect_line "both selected: one dev-feature run" "run: cargo test -p dpe-server -p mosaic-playground --features dpe-server/dev,mosaic-playground/dev --tests"
rm -rf "$WS"

# --audit fixture: a multi-line CARGO_MANIFEST_DIR read from a into d, with no
# Cargo edge between them.
new_audit_ws() {
  new_ws
  crate pkgs/d d
  : >"$WS/pkgs/d/f"
  printf '%s\n' 'pub const F: &str = include_str!(concat!(' '    env!("CARGO_MANIFEST_DIR"),' '    "/../d/f"' '));' >"$WS/pkgs/a/src/lib.rs"
  git_ws add -A
  git_ws commit -q -m reader
}

new_audit_ws
run_audit
expect_line "audit: a multi-line manifest-dir read into an unconnected sibling" 'pkgs/a/src/lib.rs:2: "/../d/f" -> pkgs/d/f (package d, not reached from a)'
expect_rc "audit: exits 1 on a finding" 1
rm -rf "$WS"

new_audit_ws
EXTRA_EDGES=("pkgs/d/f a")
run_audit
expect_rc "audit: silent once an EXTRA_EDGES line covers the read" 0
EXTRA_EDGES=("docs/src/a-data.md a")
rm -rf "$WS"

new_audit_ws
crate pkgs/e e '[dependencies]' 'd = { path = "../d" }'
printf '[dev-dependencies]\ne = { path = "../e" }\n' >>"$WS/pkgs/a/Cargo.toml"
run_audit
expect_rc "audit: silent once a transitive dev dependency reaches the sibling" 0
rm -rf "$WS"

new_ws
crate pkgs/d d
: >"$WS/pkgs/d/f"
printf '%s\n' 'pub const F: &str = include_str!("../../d/f");' >"$WS/pkgs/a/src/lib.rs"
git_ws add -A
git_ws commit -q -m reader
run_audit
expect_line "audit: a relative include_str! into an unconnected sibling" 'pkgs/a/src/lib.rs:1: "../../d/f" -> pkgs/d/f (package d, not reached from a)'
rm -rf "$WS"

new_ws
printf '%s\n' 'pub const P: &str = "docs/src/other.md";' >"$WS/pkgs/b/src/lib.rs"
git_ws add -A
git_ws commit -q -m reader
run_audit
expect_line "audit: a docs/ literal no test reads" 'pkgs/b/src/lib.rs:1: "docs/src/other.md" -> docs/src/other.md (unowned, rule 5 selects no test)'
rm -rf "$WS"

echo
echo "test-changed tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
