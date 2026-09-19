#!/usr/bin/env bash
#
# Tests for check-live-tests-ignored.sh. Dependency-free: bash only.
#
# Fixtures build throwaway directory trees (no git needed — the gate reads
# dsp-cli/tests/live_*.rs directly off the filesystem, unlike check-adr-refs.sh
# which reads the git index).
#
# Run: bash .github/scripts/check-live-tests-ignored.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-live-tests-ignored.sh disable=SC1091
source "$SCRIPT_DIR/check-live-tests-ignored.sh"

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

# make_tree: throwaway dir with an empty dsp-cli/tests/ so live_*.rs can be
# dropped in. Echoes its path.
make_tree() {
  local dir
  dir="$(mktemp -d "${TMPDIR:-/tmp}/live-ignored.XXXXXX")"
  mkdir -p "$dir/dsp-cli/tests"
  printf '%s' "$dir"
}

# gate_run <file> <content>: fresh tree with this file written under
# dsp-cli/tests/, echo "<rc> <stderr>" (stderr on a single joined line so the
# caller can grep it).
gate_run() {
  local file="$1" content="$2"
  local dir rc out
  dir="$(make_tree)"
  printf '%s' "$content" >"$dir/dsp-cli/tests/$file"
  out="$(cd "$dir" && main 2>&1 1>/dev/null)"
  rc=$?
  rm -rf "$dir"
  printf '%s\x1f%s' "$rc" "$out"
}

# 1. An ignored test passes.
result="$(gate_run live_x.rs '#[test]
#[ignore = "needs a DSP stack"]
fn t() {}
')"
check "an ignored test passes" "0" "${result%%$'\x1f'*}"

# 2. A non-ignored test fails and is named (file:line in the message).
result="$(gate_run live_y.rs '#[test]
fn not_ignored() {}
')"
rc="${result%%$'\x1f'*}"
msg="${result#*$'\x1f'}"
check "a non-ignored test fails" "1" "$rc"
check "the offender is named with file:line" "0" "$(case "$msg" in *"live_y.rs:2:"*) echo 0 ;; *) echo 1 ;; esac)"

# 3. A file with no tests passes.
result="$(gate_run live_z.rs 'fn helper() {}
')"
check "a file with no tests passes" "0" "${result%%$'\x1f'*}"

# 4. #[ignore] on the wrong item does not satisfy a bare #[test].
result="$(gate_run live_w.rs '#[ignore = "not a test"]
fn helper() {}

#[test]
fn real_test() {}
')"
rc="${result%%$'\x1f'*}"
msg="${result#*$'\x1f'}"
check "ignore on the wrong item still fails the real test" "1" "$rc"
check "the real test (not the helper) is named" "0" "$(case "$msg" in *"live_w.rs:5:"*) echo 0 ;; *) echo 1 ;; esac)"

# 5. Ignore-before-test (legal attribute order) passes.
result="$(gate_run live_v.rs '#[ignore = "needs a DSP stack"]
#[test]
fn t() {}
')"
check "ignore-before-test passes" "0" "${result%%$'\x1f'*}"

# 6. Absence is an error, not zero work.
dir="$(make_tree)"
rc_out="$(cd "$dir" && main >/dev/null 2>&1; echo $?)"
rm -rf "$dir"
check "no live test files at all fails" "1" "$rc_out"

echo
echo "check-live-tests-ignored tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
