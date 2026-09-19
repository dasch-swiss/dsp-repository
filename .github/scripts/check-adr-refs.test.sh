#!/usr/bin/env bash
#
# Tests for check-adr-refs.sh. Dependency-free: bash and git.
#
# Fixtures build throwaway git repos because the gate reads the tree out of
# the index via git grep and git ls-files, and stubbing git would test the
# stub. mktemp names an explicit TMPDIR template: a bare mktemp resolves to a
# path a sandboxed shell may not be allowed to write.
#
# No component in this repository has its own docs/adr/ yet, so the
# temporary tree built here is the only place qualified resolution is
# exercised.
#
# This file is itself a tracked text file the gate scans, so it never writes
# a literal four-digit ADR token: every fixture number is built with a printf
# split (printf 'ADR-%s' 0009), so no "ADR-NNNN" shape appears in this
# file's own source for the gate to trip over.
#
# Run: bash .github/scripts/check-adr-refs.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-adr-refs.sh disable=SC1091
source "$SCRIPT_DIR/check-adr-refs.sh"

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

# make_repo: throwaway repo with a root ADR series and a component series
# (docs/adr/0001-x.md at the root, comp/docs/adr/0003-x.md under a
# component), all clean. Echoes its path.
make_repo() {
  local dir
  dir="$(mktemp -d "${TMPDIR:-/tmp}/adr-refs.XXXXXX")"
  mkdir -p "$dir/docs/adr" "$dir/comp/docs/adr"
  printf -- '---\nstatus: accepted\n---\n\n# x\n' >"$dir/docs/adr/0001-x.md"
  printf -- '---\nstatus: accepted\n---\n\n# x\n' >"$dir/comp/docs/adr/0003-x.md"
  ( cd "$dir" && git init -q -b main && git config user.email t@example.com \
      && git config user.name t && git add -A && git commit -qm init )
  printf '%s' "$dir"
}

# gate_rc <file> <line...>: fresh repo with this file committed, echo the
# gate's exit status.
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

# 1. A resolving bare reference passes.
check "a resolving bare reference passes" 0 \
  "$(gate_rc README.md "$(printf 'see ADR-%s for the rule' 0001)")"

# 2. A dangling bare reference fails.
check "a dangling bare reference fails" 1 \
  "$(gate_rc README.md "$(printf 'see ADR-%s for the rule' 0009)")"

# 3. A resolving qualified reference passes. This is also what proves a
#    qualified reference is not counted as a bare one as well: the root series
#    has no matching number, so a double-counted bare reading would fail here.
check "a resolving qualified reference passes" 0 \
  "$(gate_rc README.md "$(printf 'see comp/ADR-%s for the rule' 0003)")"

# 4. A qualified reference to a missing component file fails.
check "a qualified reference to a missing component file fails" 1 \
  "$(gate_rc README.md "$(printf 'see comp/ADR-%s for the rule' 0009)")"

# 5. The other half of the same rule: a qualified reference never falls back to
#    the root series. The number exists at the root and not in the component,
#    so resolving it against either series in turn would wrongly pass.
check "a qualified reference does not fall back to the root series" 1 \
  "$(gate_rc README.md "$(printf 'see comp/ADR-%s for the rule' 0001)")"

# 6. A dangling reference under docs/specs/ is not reported.
check "a dangling reference under docs/specs/ is not reported" 0 \
  "$(gate_rc docs/specs/2026-01-01-x/01-x.md "$(printf 'future work will add ADR-%s' 0009)")"

# 7. Absence is an error, not zero work.
repo="$(make_repo)"
( cd "$repo" && git rm -rq docs/adr && git commit -qm drop && main >/dev/null 2>&1 )
check "no root ADRs at all fails" 1 "$?"
rm -rf "$repo"

echo
echo "check-adr-refs tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
