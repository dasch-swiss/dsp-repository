#!/usr/bin/env bash
#
# Integrity gate for the third-party bytes this repo ships to browsers or
# executes during a build. Two artifact kinds, one file so there is one place
# to look:
#
#   Vendored JS (DEV-7126) lives in modules/*/public/vendor/. Each directory's
#   README.md opens with "Do not edit these files directly" and records a
#   SHA-256 per file. Nothing recomputed those hashes, so the table attested
#   files it could not vouch for: a mistyped 64-character digest is exactly the
#   value diff review skims past. verify_vendor_readme recomputes them.
#
#   The Tailwind standalone CLI (DEV-6727) is downloaded at build time and
#   executed by every `just css*` recipe, both CI build actions and the mosaic
#   image build. tailwind.pins records the expected SHA-256 per release asset.
#
# Run directly (`just verify-checksums`) this checks everything checkable
# without a network: the vendor tables, and that tailwind.pins is complete and
# well-formed. The pinned binaries are verified where they are downloaded:
# `_tailwind-bin` in the justfile and modules/mosaic/playground/Dockerfile both
# source this file for verify_file and tailwind_pin, and fail closed.
#
# Parsing is awk, not `grep -E`: the negated-bracket patterns this needs are
# mis-evaluated by at least one drop-in grep replacement, and a table row that
# silently fails to match is a row that silently goes unverified.
#
# Dependencies: bash, awk, git, and one of sha256sum/shasum. Paths are relative
# to the repo root, which is where `just` runs recipes from.
#
# The file is safe to `source`: it defines functions and only runs `main` when
# executed directly.

: "${TAILWIND_PINS:=tailwind.pins}"

# Every release asset the download sites can resolve to, across the os/arch
# pairs they support. A version bump that refreshes only the developer's own
# architecture leaves a hole here, and verify_tailwind_pins reports it at
# `just check` rather than halfway through a build on another one.
: "${TAILWIND_ASSETS:=tailwindcss-linux-arm64 tailwindcss-linux-x64 tailwindcss-macos-arm64 tailwindcss-macos-x64}"

# is_sha256 <string>: true for a 64-character lowercase hex digest.
is_sha256() {
  case "$1" in
    "" | *[!0-9a-f]*) return 1 ;;
  esac
  [ "${#1}" -eq 64 ]
}

# sha256_of <file>: the bare digest. coreutils on Linux, perl shasum on macOS;
# both print "<digest>  <path>".
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | cut -d' ' -f1
  else
    shasum -a 256 "$1" | cut -d' ' -f1
  fi
}

# verify_file <file> <expected-sha256>: the one assertion every caller makes.
verify_file() {
  local file="$1" want="$2" got
  if [ ! -f "$file" ]; then
    echo "✗ listed but missing: $file" >&2
    return 1
  fi
  got="$(sha256_of "$file")"
  if [ "$got" != "$want" ]; then
    {
      echo "✗ checksum mismatch: $file"
      echo "    expected $want"
      echo "    actual   $got"
    } >&2
    return 1
  fi
  return 0
}

# vendor_rows <readme>: emit "<filename>\t<sha256>" per table row.
#
# Only the table whose header names SHA-256 is read. A README may hold other
# tables (the editor's documents Datastar behaviour), and their rows also start
# with a backticked cell, so an unanchored parse would read them as vendor
# entries and fail the build over a "malformed digest" that was never one.
#
# Within that table a row is a pipe line whose first cell is a backticked
# filename; the |---|---| separator is not. Splitting on "|" rather than
# matching the whole line keeps this indifferent to column count and spacing.
# The last cell must be `sha256:<64 lowercase hex>`; a row that looks like an
# entry but carries a malformed digest is reported and fails the file, because
# skipping it would quietly drop that file from verification altogether. Such a
# row is still emitted, with an empty digest, so the caller counts the file as
# listed and does not also report it as an undeclared file.
vendor_rows() {
  awk '
    # The header opens the table; the first non-row line closes it.
    /^[|]/ && /SHA-256/ { in_table = 1; next }
    !/^[|]/ { in_table = 0; next }

    in_table {
      n = split($0, f, "|")
      if (n < 3) next
      name = f[2]
      gsub(/^[ \t]+|[ \t]+$/, "", name)
      if (name !~ /^`.+`$/) next
      gsub(/^`|`$/, "", name)

      cell = f[n - 1]
      gsub(/^[ \t]+|[ \t]+$/, "", cell)
      hash = cell
      if (sub(/^`sha256:/, "", hash) && sub(/`$/, "", hash) &&
        length(hash) == 64 && hash !~ /[^0-9a-f]/) {
        print name "\t" hash
        next
      }
      printf "✗ %s:%d: row for `%s` has no well-formed `sha256:<64 hex>` cell (got %s)\n", \
        FILENAME, FNR, name, cell > "/dev/stderr"
      print name "\t"
      rc = 1
    }
    END { exit rc }
  ' "$1"
}

# verify_vendor_readme <readme>: every row matches the file on disk, and every
# committed file in the directory has a row.
verify_vendor_readme() {
  local readme="$1" dir rows name want listed="" file base rc=0

  dir="${readme%/*}"

  rows="$(vendor_rows "$readme")" || rc=1

  while IFS="$(printf '\t')" read -r name want; do
    [ -n "$name" ] || continue
    listed="$listed $name "
    # An empty digest is a malformed row vendor_rows already reported.
    [ -n "$want" ] || continue
    verify_file "$dir/$name" "$want" || rc=1
  done <<<"$rows"

  # The other direction is a file the repo commits here that no row claims:
  # a renamed or dropped table row. Committed files only, so a developer's
  # untracked scratch in a vendor directory does not fail everyone's build.
  #
  # The listing is captured before the loop rather than substituted into the
  # herestring: a substitution that fails there yields zero lines and no
  # error, so this whole direction would silently pass.
  local tracked
  if ! tracked="$(git ls-files -- "$dir")"; then
    echo "✗ $readme: cannot list committed files in $dir (git ls-files failed)" >&2
    return 1
  fi
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    base="${file##*/}"
    [ "$base" != "README.md" ] || continue
    case "$listed" in
      *" $base "*) ;;
      *)
        echo "✗ $readme: $base is committed here but has no row in the table" >&2
        rc=1
        ;;
    esac
  done <<<"$tracked"

  if [ "$rc" -eq 0 ]; then
    echo "✓ $readme"
  fi
  return "$rc"
}

# tailwind_version: the version recorded in tailwind.pins.
tailwind_version() {
  awk '$1 == "version" { print $2; found = 1; exit } END { exit !found }' "$TAILWIND_PINS"
}

# tailwind_pin <release-asset-name>: the pinned digest for one release asset.
tailwind_pin() {
  awk -v want="$1" '$2 == want { print $1; found = 1; exit } END { exit !found }' "$TAILWIND_PINS" || {
    echo "✗ no pin for '$1' in $TAILWIND_PINS. Run 'just tailwind-pins-refresh'" >&2
    return 1
  }
}

# verify_tailwind_pins: tailwind.pins is present, versioned, and pins every
# asset the build can ask for. It cannot verify the binaries themselves; those
# are ~100 MB downloads, verified at the point of download instead.
verify_tailwind_pins() {
  local ver asset pin rc=0

  if [ ! -f "$TAILWIND_PINS" ]; then
    echo "✗ missing $TAILWIND_PINS" >&2
    return 1
  fi
  if ! ver="$(tailwind_version)" || [ -z "$ver" ]; then
    echo "✗ $TAILWIND_PINS: no 'version <x.y.z>' line" >&2
    return 1
  fi
  for asset in $TAILWIND_ASSETS; do
    pin="$(tailwind_pin "$asset")" || { rc=1; continue; }
    if ! is_sha256 "$pin"; then
      echo "✗ $TAILWIND_PINS: pin for $asset is not a 64-character lowercase hex digest: $pin" >&2
      rc=1
    fi
  done

  if [ "$rc" -eq 0 ]; then
    echo "✓ $TAILWIND_PINS (Tailwind $ver)"
  fi
  return "$rc"
}

main() {
  local dir tracked found=0 rc=0

  for dir in modules/*/public/vendor; do
    [ -d "$dir" ] || continue
    found=1
    if [ -f "$dir/README.md" ]; then
      verify_vendor_readme "$dir/README.md" || rc=1
    elif ! tracked="$(git ls-files -- "$dir")"; then
      echo "✗ cannot list committed files in $dir (git ls-files failed)" >&2
      rc=1
    elif [ -n "$tracked" ]; then
      # Globbing the READMEs alone would miss this: a vendor directory
      # with none ships third-party files carrying no record whatsoever,
      # which is the same hole one directory wider.
      echo "✗ $dir commits files but has no README.md recording their digests" >&2
      rc=1
    fi
  done
  # An unmatched glob would make this gate a silent no-op, which is the
  # failure it exists to prevent, so absence is an error, not zero work.
  if [ "$found" -eq 0 ]; then
    echo "✗ no modules/*/public/vendor directory found. Run from the repo root" >&2
    rc=1
  fi

  verify_tailwind_pins || rc=1

  [ "$rc" -eq 0 ] || echo "✗ checksum verification failed (see above)" >&2
  return "$rc"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
