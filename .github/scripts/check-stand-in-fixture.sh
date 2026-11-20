#!/usr/bin/env bash
#
# The VRE bridge's shipped inputs of project 0803 are the one authoritative
# copy of the files `media` serves for it; the committed `media` fixture holds
# the same files for the tests and for access-server's image. The generation
# the fixture's Ready token names must hold exactly the files of
# areas/deposit/vre-bridge/inputs/0803/, byte for byte, both ways. The
# fixture's two Ready tokens, `media`'s and `sync`'s, must be the same bytes
# and record the inputs digest the bridge's refresh_0803 test pins: no reader
# checks that digest, so nothing else would notice a stale one.
#
# The trees sit in different areas, and no Bazel target may cross them
# (ADR-0002): this script is the only code that reads both. It reads the
# working tree, as the bridge and the readers read their directories, so a
# symbolic link counts as a difference: the readers refuse one.
#
# Safe to `source`. Dependencies: bash, find, sort, cmp, mktemp.

INPUTS=areas/deposit/vre-bridge/inputs/0803
FIXTURE=areas/access/media/data/0803
SYNC_TOKEN=areas/access/sync/data/0803/.ready
PINNED=areas/deposit/vre-bridge/server/tests/refresh_0803.rs

# files_below <dir> <out>: every entry below <dir> but its directories and
# macOS's .DS_Store files (never committed, never in the image: the inputs'
# glob excludes them), as paths relative to it, NUL-separated and sorted, into
# the file <out>. Fails when <dir> cannot be listed whole, so an unreadable
# directory is no pass.
files_below() {
  (cd "$1" && find . -mindepth 1 ! -type d ! -name .DS_Store -print0) >"$2" || {
    echo "✗ $1 cannot be listed" >&2
    return 1
  }
  LC_ALL=C sort -z -o "$2" "$2"
}

check_tokens() {
  local token pinned recorded
  [ -f "$SYNC_TOKEN" ] && [ ! -L "$SYNC_TOKEN" ] && cmp -s "$FIXTURE/.ready" "$SYNC_TOKEN" || {
    echo "✗ $SYNC_TOKEN is not the same bytes as $FIXTURE/.ready: the two tokens hold equal provenance" >&2
    return 1
  }
  if [[ "$(<"$PINNED")" =~ INPUTS_DIGEST_0803:\ \&str\ =\ \"(sha256:[0-9a-f]{64})\" ]]; then
    pinned="${BASH_REMATCH[1]}"
  else
    echo "✗ $PINNED pins no INPUTS_DIGEST_0803" >&2
    return 1
  fi
  token="$(<"$FIXTURE/.ready")"
  [[ "$token" =~ \"inputs_digest\":\"([^\"]*)\" ]] && recorded="${BASH_REMATCH[1]}"
  [ "${recorded:-}" = "$pinned" ] || {
    echo "✗ $FIXTURE/.ready and $SYNC_TOKEN record inputs_digest ${recorded:-(none)}; $PINNED pins $pinned. Write the pinned digest into both tokens" >&2
    return 1
  }
}

main() {
  local token generation fixture rel lists count=0 offender_count=0

  [ -d "$INPUTS" ] || {
    echo "✗ $INPUTS is not a directory. Run from the repo root" >&2
    return 1
  }
  [ -f "$FIXTURE/.ready" ] && [ ! -L "$FIXTURE/.ready" ] || {
    echo "✗ $FIXTURE/.ready is not a regular file" >&2
    return 1
  }
  token="$(<"$FIXTURE/.ready")"
  if [[ "$token" =~ \"generation\":\"([0-9]{8}T[0-9]{6}Z-[0-9a-f]{8})\" ]]; then
    generation="${BASH_REMATCH[1]}"
  else
    echo "✗ $FIXTURE/.ready names no generation" >&2
    return 1
  fi
  fixture="$FIXTURE/$generation"
  [ -d "$fixture" ] && [ ! -L "$fixture" ] || {
    echo "✗ $fixture, the generation $FIXTURE/.ready names, is not a directory" >&2
    return 1
  }
  check_tokens || return 1

  lists="$(mktemp -d "${TMPDIR:-/tmp}/stand-in-fixture.XXXXXX")" || return 1
  files_below "$INPUTS" "$lists/inputs" && files_below "$fixture" "$lists/fixture" || {
    rm -rf "$lists"
    return 1
  }

  while IFS= read -r -d '' rel; do
    rel="${rel#./}"
    count=$((count + 1))
    if [ ! -e "$fixture/$rel" ] && [ ! -L "$fixture/$rel" ]; then
      echo "$fixture/$rel: missing; $INPUTS/$rel has it" >&2
      offender_count=$((offender_count + 1))
    elif [ -L "$INPUTS/$rel" ] || [ ! -f "$INPUTS/$rel" ]; then
      echo "$INPUTS/$rel: not a regular file" >&2
      offender_count=$((offender_count + 1))
    elif [ -L "$fixture/$rel" ] || [ ! -f "$fixture/$rel" ]; then
      echo "$fixture/$rel: not a regular file" >&2
      offender_count=$((offender_count + 1))
    elif ! cmp -s "$INPUTS/$rel" "$fixture/$rel"; then
      echo "$fixture/$rel: differs from $INPUTS/$rel" >&2
      offender_count=$((offender_count + 1))
    fi
  done <"$lists/inputs"

  while IFS= read -r -d '' rel; do
    rel="${rel#./}"
    if [ ! -e "$INPUTS/$rel" ] && [ ! -L "$INPUTS/$rel" ]; then
      echo "$fixture/$rel: not in $INPUTS" >&2
      offender_count=$((offender_count + 1))
    fi
  done <"$lists/fixture"
  rm -rf "$lists"

  # Absence is an error, not zero work: two empty trees are equal and pin nothing.
  [ "$count" -gt 0 ] || {
    echo "✗ $INPUTS holds no file" >&2
    return 1
  }

  if [ "$offender_count" -gt 0 ]; then
    echo >&2
    echo "✗ $offender_count difference(s). $INPUTS is the authoritative copy: edit it, then copy it to $fixture." >&2
    return 1
  fi

  echo "✓ stand-in fixture: the $count file(s) of $fixture equal the shipped inputs, and both tokens record their pinned digest"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
