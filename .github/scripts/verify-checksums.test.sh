#!/usr/bin/env bash
#
# Tests for verify-checksums.sh.
#
# Dependency-free: bash, awk and git. The fixtures build throwaway git repos
# because the "file on disk with no table row" direction is answered by
# `git ls-files`, and stubbing that would test the stub.
#
# What is covered is what carries risk: that drift is caught in both
# directions, and that a row which *looks* like an entry but carries a
# malformed digest is reported rather than silently skipped, because a skipped row is
# an unverified file, which is the failure the gate exists to prevent.
#
# Fixtures name an explicit template under TMPDIR: a bare `mktemp` resolves to
# the platform default, which a sandboxed shell may not be allowed to write.
#
# Run: bash .github/scripts/verify-checksums.test.sh   (or `just test`)

set -uo pipefail

# Fixtures build throwaway repositories with `git init`, which honours an
# inherited GIT_DIR over the directory it is run in: without this, a run under
# `git rebase --exec` or a git hook re-initializes the caller's repository.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./verify-checksums.sh disable=SC1091
source "$SCRIPT_DIR/verify-checksums.sh"

PASS=0
FAIL=0

# assert_ok "desc" cmd...   : expects the command to exit 0
assert_ok() {
  local desc="$1"; shift
  if "$@" >/dev/null 2>&1; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected success)"
  fi
}

# assert_fail "desc" cmd... : expects the command to exit non-zero
assert_fail() {
  local desc="$1"; shift
  if "$@" >/dev/null 2>&1; then
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected failure, got success)"
  else
    PASS=$((PASS + 1))
  fi
}

# assert_eq "desc" <expected> <actual>
assert_eq() {
  local desc="$1" want="$2" got="$3"
  if [ "$want" = "$got" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $desc (expected '$want', got '$got')"
  fi
}

# --- Fixtures -------------------------------------------------------------

VENDOR_DIR="modules/demo/public/vendor"

PIN_A="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"

# make_vendor_repo: throwaway repo with one vendor directory holding a.js and
# b.js and a README whose table records their true digests. Tests mutate one
# side or the other and assert the mismatch is caught.
make_vendor_repo() {
  local dir a b
  dir="$(mktemp -d "${TMPDIR:-/tmp}/vendor-fixture.XXXXXX")"
  mkdir -p "$dir/$VENDOR_DIR"
  printf 'console.log("a");\n' >"$dir/$VENDOR_DIR/a.js"
  printf 'console.log("b");\n' >"$dir/$VENDOR_DIR/b.js"
  a="$(sha256_of "$dir/$VENDOR_DIR/a.js")"
  b="$(sha256_of "$dir/$VENDOR_DIR/b.js")"
  {
    echo '# Vendored JavaScript Dependencies'
    echo
    echo '| File | Package | Version | SHA-256 |'
    echo '|------|---------|---------|---------|'
    echo "| \`a.js\` | demo-a | 1.0.0 | \`sha256:$a\` |"
    echo "| \`b.js\` | demo-b | 2.0.0 | \`sha256:$b\` |"
  } >"$dir/$VENDOR_DIR/README.md"
  (
    cd "$dir" || exit 1
    git init -q -b main
    git config user.email test@example.com
    git config user.name "Test"
    git add -A
    git commit -q -m "chore(ci): fixture"
  )
  printf '%s' "$dir"
}

# --- Unit: verify_file ----------------------------------------------------

test_verify_file_match() {
  local f; f="$(mktemp "${TMPDIR:-/tmp}/verify-file.XXXXXX")"; printf 'payload\n' >"$f"
  verify_file "$f" "$(sha256_of "$f")"
  local rc=$?; rm -f "$f"; return $rc
}
test_verify_file_one_byte_changed() {
  local f want; f="$(mktemp "${TMPDIR:-/tmp}/verify-file.XXXXXX")"; printf 'payload\n' >"$f"
  want="$(sha256_of "$f")"
  printf 'x' >>"$f"
  verify_file "$f" "$want"
  local rc=$?; rm -f "$f"; return $rc
}
assert_ok   "verify_file: digest matches"          test_verify_file_match
assert_fail "verify_file: one appended byte fails" test_verify_file_one_byte_changed
assert_fail "verify_file: absent file fails"       verify_file "/nonexistent/none.js" "$PIN_A"

# --- Unit: vendor_rows ----------------------------------------------------

test_rows_parses_both() {
  local repo out
  repo="$(make_vendor_repo)"
  out="$(vendor_rows "$repo/$VENDOR_DIR/README.md" 2>/dev/null | cut -f1 | tr '\n' ',')"
  rm -rf "$repo"
  [ "$out" = "a.js,b.js," ]
}
assert_ok "vendor_rows: reads every row, not just the last" test_rows_parses_both

# A digest one character short is the shape a hand-edited table actually
# produces. It must be an error, not a skipped row.
test_rows_rejects_short_digest() {
  local repo rc
  repo="$(make_vendor_repo)"
  # Greedy [0-9a-f]* then one more backtracks off exactly the last hex digit.
  sed 's/\(sha256:[0-9a-f]*\)[0-9a-f]/\1/' "$repo/$VENDOR_DIR/README.md" >"$repo/r.md"
  vendor_rows "$repo/r.md" >/dev/null 2>&1
  rc=$?; rm -rf "$repo"; return $rc
}
assert_fail "vendor_rows: a 63-hex digest is reported, not skipped" test_rows_rejects_short_digest

# --- Integration: verify_vendor_readme ------------------------------------

test_vendor_clean() {
  local repo rc
  repo="$(make_vendor_repo)"
  ( cd "$repo" && verify_vendor_readme "$VENDOR_DIR/README.md" )
  rc=$?; rm -rf "$repo"; return $rc
}
test_vendor_file_byte_changed() {
  local repo rc
  repo="$(make_vendor_repo)"
  printf '\n' >>"$repo/$VENDOR_DIR/b.js"
  ( cd "$repo" && verify_vendor_readme "$VENDOR_DIR/README.md" )
  rc=$?; rm -rf "$repo"; return $rc
}
test_vendor_row_dropped() {
  local repo rc
  repo="$(make_vendor_repo)"
  sed '/b\.js/d' "$repo/$VENDOR_DIR/README.md" >"$repo/tmp" && mv "$repo/tmp" "$repo/$VENDOR_DIR/README.md"
  ( cd "$repo" && verify_vendor_readme "$VENDOR_DIR/README.md" )
  rc=$?; rm -rf "$repo"; return $rc
}
test_vendor_file_listed_but_absent() {
  local repo rc
  repo="$(make_vendor_repo)"
  rm -f "$repo/$VENDOR_DIR/b.js"
  ( cd "$repo" && verify_vendor_readme "$VENDOR_DIR/README.md" )
  rc=$?; rm -rf "$repo"; return $rc
}
# An untracked scratch file is not a vendored artifact, and failing every
# developer's `just check` over one would be a false alarm.
test_vendor_untracked_file_ignored() {
  local repo rc
  repo="$(make_vendor_repo)"
  printf 'scratch\n' >"$repo/$VENDOR_DIR/notes.txt"
  ( cd "$repo" && verify_vendor_readme "$VENDOR_DIR/README.md" )
  rc=$?; rm -rf "$repo"; return $rc
}
# A vendor README may document other things in tables of their own, and their
# rows also open with a backticked cell. Reading those as vendor entries would
# fail the build over a "malformed digest" that was never a digest.
test_vendor_ignores_a_second_table() {
  local repo rc
  repo="$(make_vendor_repo)"
  {
    echo
    echo '## Events'
    echo
    echo '| Event | Description |'
    echo '|-------|-------------|'
    # The backticks are literal markdown, and deliberate: a backticked first
    # cell is exactly what made an unanchored parse read this as an entry.
    # shellcheck disable=SC2016
    echo '| `datastar-fetch` | fires on every fetch |'
  } >>"$repo/$VENDOR_DIR/README.md"
  ( cd "$repo" && verify_vendor_readme "$VENDOR_DIR/README.md" )
  rc=$?; rm -rf "$repo"; return $rc
}
assert_ok   "vendor: a second, non-checksum table is ignored" test_vendor_ignores_a_second_table
assert_ok   "vendor: table and files agree"                  test_vendor_clean
assert_fail "vendor: a changed byte fails"                   test_vendor_file_byte_changed
assert_fail "vendor: committed file with no row fails"       test_vendor_row_dropped
assert_fail "vendor: row whose file is gone fails"           test_vendor_file_listed_but_absent
assert_ok   "vendor: untracked file in the directory is not an error" test_vendor_untracked_file_ignored

# A failing `git ls-files` must be an error, not an empty listing: reading it
# as "no committed files" would pass the undeclared-file direction silently,
# which is the half of the check that catches a dropped table row.
test_vendor_git_failure_is_an_error() {
  local repo stub rc
  repo="$(make_vendor_repo)"
  stub="$(mktemp -d "${TMPDIR:-/tmp}/gitstub.XXXXXX")"
  printf '#!/bin/sh\nexit 128\n' >"$stub/git"
  chmod +x "$stub/git"
  ( cd "$repo" && PATH="$stub:$PATH" verify_vendor_readme "$VENDOR_DIR/README.md" )
  rc=$?; rm -rf "$repo" "$stub"; return $rc
}
assert_fail "vendor: a failing git ls-files is an error, not a pass" test_vendor_git_failure_is_an_error

# --- Integration: main (end to end) ---------------------------------------

test_main_clean() {
  local repo rc
  repo="$(make_vendor_repo)"
  ( cd "$repo" && main )
  rc=$?; rm -rf "$repo"; return $rc
}
# A vendor directory carrying committed files but no README has no integrity
# record at all, and a glob over READMEs cannot see it.
test_main_vendor_dir_without_readme() {
  local repo rc
  repo="$(make_vendor_repo)"
  (
    cd "$repo" \
      && git rm -q --cached "$VENDOR_DIR/README.md" \
      && rm -f "$VENDOR_DIR/README.md" \
      && main
  )
  rc=$?; rm -rf "$repo"; return $rc
}
# An unmatched glob must be an error, not zero work: a gate that silently
# checks nothing is the failure this script exists to prevent.
test_main_no_vendor_tree() {
  local repo rc
  repo="$(make_vendor_repo)"
  ( cd "$repo" && rm -rf modules && main )
  rc=$?; rm -rf "$repo"; return $rc
}
# A vendor directory inside an area is checked like one under modules/. The
# clean modules/ copy stays, so only the areas/ glob can make this fail.
test_main_area_vendor_dir_tampered() {
  local repo rc
  repo="$(make_vendor_repo)"
  (
    cd "$repo" \
      && mkdir -p areas/deposit \
      && cp -R modules/demo areas/deposit/demo \
      && printf '\n' >>areas/deposit/demo/public/vendor/b.js \
      && git add areas \
      && main
  )
  rc=$?; rm -rf "$repo"; return $rc
}
assert_ok   "main: a consistent repo passes"                     test_main_clean
assert_fail "main: vendor directory with files but no README fails" test_main_vendor_dir_without_readme
assert_fail "main: no vendor directory at all fails"             test_main_no_vendor_tree
assert_fail "main: a tampered file in an area's vendor directory fails" test_main_area_vendor_dir_tampered

# --- Summary --------------------------------------------------------------

echo
echo "verify-checksums tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
