#!/usr/bin/env bash
#
# Tests for check-stand-in-fixture.sh. Dependency-free: bash and coreutils.
#
# Fixtures build throwaway directory trees in the repository's layout (no git
# needed: the gate reads the working tree, as the bridge and the readers do).
#
# Run: bash .github/scripts/check-stand-in-fixture.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-stand-in-fixture.sh disable=SC1091
source "$SCRIPT_DIR/check-stand-in-fixture.sh"

PASS=0
FAIL=0

GENERATION=20261007T000000Z-00000000
DIGEST="sha256:$(printf 'ab%.0s' {1..32})"

# check <desc> <expected> <actual>
check() {
  if [ "$2" = "$3" ]; then
    PASS=$((PASS + 1))
  else
    FAIL=$((FAIL + 1))
    echo "FAIL: $1 (expected $2, got $3)"
  fi
}

# check_has <desc> <message> <text>: the message holds the text.
check_has() {
  case "$2" in
  *"$3"*) PASS=$((PASS + 1)) ;;
  *)
    FAIL=$((FAIL + 1))
    echo "FAIL: $1 (no \"$3\" in: $2)"
    ;;
  esac
}

# token <generation> <digest>: a Ready token's bytes.
token() {
  printf '{"token_version":1,"generation":"%s","previous_generation":null,"inputs_digest":"%s"}\n' "$1" "$2"
}

# make_tree: throwaway repository root whose shipped inputs and fixture
# generation hold the same two files, both tokens naming the generation and
# recording the digest the test file pins. Echoes its path.
make_tree() {
  local dir side
  dir="$(mktemp -d "${TMPDIR:-/tmp}/stand-in-fixture.XXXXXX")"
  for side in "$dir/$INPUTS" "$dir/$FIXTURE/$GENERATION"; do
    mkdir -p "$side/narrative"
    printf 'project "0803"\n' >"$side/config.kdl"
    printf '# Home\n' >"$side/narrative/home.en.md"
  done
  mkdir -p "$(dirname "$dir/$SYNC_TOKEN")" "$(dirname "$dir/$PINNED")"
  token "$GENERATION" "$DIGEST" >"$dir/$FIXTURE/.ready"
  token "$GENERATION" "$DIGEST" >"$dir/$SYNC_TOKEN"
  printf 'const INPUTS_DIGEST_0803: &str = "%s";\n' "$DIGEST" >"$dir/$PINNED"
  printf '%s' "$dir"
}

# gate_run <dir>: run the gate in <dir>, then remove it; sets RC and MSG (stderr).
gate_run() {
  MSG="$(cd "$1" && main 2>&1 1>/dev/null)"
  RC=$?
  rm -rf "$1"
}

# 1. Identical trees pass.
gate_run "$(make_tree)"
check "identical trees pass" "0" "$RC"

# 2. One differing byte fails and names the file.
dir="$(make_tree)"
printf '# Hume\n' >"$dir/$FIXTURE/$GENERATION/narrative/home.en.md"
gate_run "$dir"
check "a differing byte fails" "1" "$RC"
check_has "the differing file is named" "$MSG" "$FIXTURE/$GENERATION/narrative/home.en.md: differs"

# 3. A file the fixture has and the inputs lack fails and is named.
dir="$(make_tree)"
printf 'extra\n' >"$dir/$FIXTURE/$GENERATION/narrative/extra.de.md"
gate_run "$dir"
check "an extra fixture file fails" "1" "$RC"
check_has "the extra file is named" "$MSG" "$FIXTURE/$GENERATION/narrative/extra.de.md: not in"

# 4. A file the inputs have and the fixture lacks fails and is named.
dir="$(make_tree)"
rm "$dir/$FIXTURE/$GENERATION/config.kdl"
gate_run "$dir"
check "a missing fixture file fails" "1" "$RC"
check_has "the missing file is named" "$MSG" "$FIXTURE/$GENERATION/config.kdl: missing"

# 5. A symbolic link in place of a file fails, even to identical bytes: the readers refuse it.
dir="$(make_tree)"
rm "$dir/$FIXTURE/$GENERATION/config.kdl"
ln -s "$dir/$INPUTS/config.kdl" "$dir/$FIXTURE/$GENERATION/config.kdl"
gate_run "$dir"
check "a symbolic link fails" "1" "$RC"
check_has "the symbolic link is named" "$MSG" "$FIXTURE/$GENERATION/config.kdl: not a regular file"

# 6. Only the generation the token names is compared.
dir="$(make_tree)"
mkdir -p "$dir/$FIXTURE/20261001T000000Z-0000000a"
printf 'older\n' >"$dir/$FIXTURE/20261001T000000Z-0000000a/config.kdl"
gate_run "$dir"
check "another generation is not compared" "0" "$RC"

# 7. A fixture without a token fails.
dir="$(make_tree)"
rm "$dir/$FIXTURE/.ready"
gate_run "$dir"
check "a missing token fails" "1" "$RC"
check_has "the missing token is named" "$MSG" "$FIXTURE/.ready is not a regular file"

# 8. A token naming a generation that is not there fails.
dir="$(make_tree)"
token 20261008T000000Z-00000000 "$DIGEST" | tee "$dir/$SYNC_TOKEN" >"$dir/$FIXTURE/.ready"
gate_run "$dir"
check "a token naming no generation directory fails" "1" "$RC"
check_has "the missing generation is named" "$MSG" "20261008T000000Z-00000000, the generation"

# 9. Absence is an error, not zero work: empty inputs fail.
dir="$(make_tree)"
rm -r "${dir:?}/$INPUTS" "${dir:?}/$FIXTURE/$GENERATION"
mkdir -p "$dir/$INPUTS" "$dir/$FIXTURE/$GENERATION"
gate_run "$dir"
check "empty shipped inputs fail" "1" "$RC"
check_has "the empty inputs are named" "$MSG" "$INPUTS holds no file"

# 10. Tokens that are not the same bytes fail.
dir="$(make_tree)"
token "$GENERATION" "sha256:$(printf 'cd%.0s' {1..32})" >"$dir/$SYNC_TOKEN"
gate_run "$dir"
check "differing tokens fail" "1" "$RC"
check_has "the sync token is named" "$MSG" "$SYNC_TOKEN is not the same bytes"

# 11. Tokens recording another digest than the pinned one fail, naming both.
dir="$(make_tree)"
printf 'const INPUTS_DIGEST_0803: &str = "sha256:%s";\n' "$(printf 'ef%.0s' {1..32})" >"$dir/$PINNED"
gate_run "$dir"
check "a stale digest fails" "1" "$RC"
check_has "the recorded digest is named" "$MSG" "record inputs_digest $DIGEST"
check_has "the pin is named" "$MSG" "$PINNED pins sha256:ef"

# 12. A .DS_Store on either side is ignored: Finder writes one, and it is never committed.
dir="$(make_tree)"
printf 'finder\n' >"$dir/$INPUTS/narrative/.DS_Store"
printf 'other\n' >"$dir/$FIXTURE/$GENERATION/.DS_Store"
gate_run "$dir"
check "a .DS_Store is ignored" "0" "$RC"

echo
echo "check-stand-in-fixture tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
