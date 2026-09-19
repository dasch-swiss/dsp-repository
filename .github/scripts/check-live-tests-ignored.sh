#!/usr/bin/env bash
#
# Every #[test] in dsp-cli/tests/live_*.rs must carry an #[ignore] attribute:
# these tests need a live DSP stack, and without #[ignore] they
# run — and pass vacuously via their own early-return skip — under a plain
# `cargo nextest run --all-features`. That "passes" for the wrong reason, so
# only a scan of the attribute list per test catches a test that lost its
# #[ignore].
#
# A per-file line-by-line scan (not "the line after #[test]") is required
# because #[ignore] may appear before #[test], and doc comments may sit
# between the two attributes; only a small state machine handles both.
#
# Safe to `source`. Dependencies: bash.

main() {
  local file line lineno saw_test saw_ignore offender_count=0 test_count=0 file_count=0
  local -a files

  shopt -s nullglob
  files=(dsp-cli/tests/live_*.rs)
  shopt -u nullglob

  # Absence is an error, not zero work: a gate that finds no live test file at
  # all would pass on a repository where none was ever committed.
  [ "${#files[@]}" -gt 0 ] || {
    echo "✗ no live test files under dsp-cli/tests/. Run from the repo root" >&2
    return 1
  }

  for file in "${files[@]}"; do
    file_count=$((file_count + 1))
    saw_test=0
    saw_ignore=0
    lineno=0

    while IFS= read -r line || [ -n "$line" ]; do
      lineno=$((lineno + 1))

      case "$line" in
      '#[test]'*) saw_test=1 ;;
      '#[ignore'*) saw_ignore=1 ;;
      esac

      if [[ "$line" =~ ^[[:space:]]*(pub[[:space:]]+)?fn[[:space:]] ]]; then
        if [ "$saw_test" -eq 1 ]; then
          test_count=$((test_count + 1))
          if [ "$saw_ignore" -eq 0 ]; then
            echo "$file:$lineno: #[test] without #[ignore]" >&2
            offender_count=$((offender_count + 1))
          fi
        fi
        saw_test=0
        saw_ignore=0
      fi
    done <"$file"
  done

  if [ "$offender_count" -gt 0 ]; then
    echo >&2
    echo "✗ $offender_count live test(s) missing #[ignore]. Every dsp-cli/tests/live_*.rs test needs a DSP stack and must not run under a plain test invocation." >&2
    return 1
  fi

  echo "✓ live tests: $test_count #[test] across $file_count file(s), all #[ignore]"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
