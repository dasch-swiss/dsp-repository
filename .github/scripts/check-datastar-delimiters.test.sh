#!/usr/bin/env bash
#
# Tests for check-datastar-delimiters.sh. Dependency-free: bash and git.
#
# Fixtures build throwaway git repos because the gate reads the tree out of the
# index, and stubbing git would test the stub. mktemp names an explicit TMPDIR
# template: a bare mktemp resolves to a path a sandboxed shell may not write.
#
# The no-fire cases carry the weight. This repo already contains
# `assert!(!out.contains("data-on-submit"))` in two page tests — the assertions
# that pin the rule — and a gate that flagged them would be "fixed" by deleting
# the tests that prevent the bug.
#
# Run: bash .github/scripts/check-datastar-delimiters.test.sh   (or `just test`)

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-datastar-delimiters.sh disable=SC1091
source "$SCRIPT_DIR/check-datastar-delimiters.sh"

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

# make_repo: throwaway repo shaped like this one, with a clean Maud template.
make_repo() {
  local dir f
  dir="$(mktemp -d "${TMPDIR:-/tmp}/datastar-delims.XXXXXX")"
  for f in editor/web dpe/web; do
    mkdir -p "$dir/modules/$f/src"
    printf 'pub fn a() {}\n' >"$dir/modules/$f/src/lib.rs"
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

# 1. A clean tree passes.
repo="$(make_repo)"
( cd "$repo" && main >/dev/null 2>&1 )
check "a clean Maud tree passes" 0 "$?"
rm -rf "$repo"

# 2. The bare hyphen form fails — the shape that renders fine and does nothing.
check "a bare data-on- attribute fails" 1 \
  "$(gate_rc modules/editor/web/src/bad.rs \
     'html! { form data-on-submit={ "@post()" } {} }')"

# 3. Every prefix the rename covers, not just data-on-.
for prefix in attr class style; do
  check "a data-$prefix- attribute fails" 1 \
    "$(gate_rc modules/editor/web/src/bad.rs \
       "html! { div data-$prefix-disabled={ \"\$x\" } {} }")"
done

# 4. The quoted form Maud requires for dotted names. Missed by a pattern that
#    only accepts a bare attribute name, and the likeliest place for a slip.
check "a quoted dotted hyphen attribute fails" 1 \
  "$(gate_rc modules/editor/web/src/bad.rs \
     'html! { input "data-on-change__debounce.1s"={ "@post()" } {} }')"

# 5. The false positive that matters: the negative assertions already in this
#    repo, which exist to pin exactly this rule.
check "a negative assertion naming the hyphen form does not fire" 0 \
  "$(gate_rc modules/editor/web/src/ok.rs \
     'assert!(!out.contains("data-on-submit"), "{out}");' \
     'assert!(!out.contains("data-attr-disabled"), "{out}");')"

# 6. The correct colon spelling, bare and quoted-dotted.
check "colon-delimited attributes do not fire" 0 \
  "$(gate_rc modules/editor/web/src/ok.rs \
     'html! { form data-on:submit={ "@post()" } {} }' \
     'html! { input "data-on:change__debounce.1s"={ "@post()" } {} }')"

# 7. Prose naming the old delimiter must not fire, or the gate is "fixed" by
#    deleting the comment that explains it — including this module's own docs.
check "prose mentioning data-on- does not fire" 0 \
  "$(gate_rc modules/editor/web/src/ok.rs \
     '//! The pre-RC.6 forms (data-on-, data-attr-) are inert.' \
     '/// Write data-on:click, never data-on-click.')"

# 8. Nested src/ subdirectory. Fails the moment MAUD_PATHSPECS stops being a
#    quoted array, since bash would expand the glob before git sees it.
check "a violation in a nested src/ subdirectory fails" 1 \
  "$(gate_rc modules/editor/web/src/pages/deep/bad.rs \
     'html! { form data-on-submit={ "@post()" } {} }')"

# 9. A tree with no Maud sources is an error, not a pass: a gate that reads
#    nothing enforces nothing.
repo="$(mktemp -d "${TMPDIR:-/tmp}/datastar-delims.XXXXXX")"
( cd "$repo" && git init -q -b main && git config user.email t@example.com \
    && git config user.name t && git commit -q --allow-empty -m init && main >/dev/null 2>&1 )
check "an empty tree is an error" 1 "$?"
rm -rf "$repo"

echo "check-datastar-delimiters: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
