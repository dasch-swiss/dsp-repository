#!/usr/bin/env bash
#
# Tests for check-datastar-attributes.sh. Dependency-free: bash, git and awk.
#
# Fixtures build throwaway git repos because the gate lists files through git,
# and stubbing git would test the stub. mktemp names an explicit TMPDIR
# template: a bare mktemp resolves to a path a sandboxed shell may not write.
#
# The no-fire cases carry the weight. This repo already contains
# `assert!(!out.contains("data-on-submit"))` in page tests, the assertions that
# pin the rule, and a gate that flagged them would be "fixed" by deleting the
# tests that prevent the bug.
#
# Run: bash .github/scripts/check-datastar-attributes.test.sh   (or `just test`)

set -uo pipefail

# Fixtures build throwaway repositories with `git init`, which honours an
# inherited GIT_DIR over the directory it is run in: without this, a run under
# `git rebase --exec` or a git hook re-initializes the caller's repository.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-datastar-attributes.sh disable=SC1091
source "$SCRIPT_DIR/check-datastar-attributes.sh"

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
  dir="$(mktemp -d "${TMPDIR:-/tmp}/datastar-attrs.XXXXXX")"
  for f in areas/deposit/editor/web areas/access/dpe/web; do
    mkdir -p "$dir/$f/src"
    printf 'pub fn a() {}\n' >"$dir/$f/src/lib.rs"
  done
  ( cd "$dir" && git init -q -b main && git config user.email t@example.com \
      && git config user.name t && git config commit.gpgsign false \
      && git add -A && git commit -qm init )
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

SRC=areas/deposit/editor/web/src/x.rs

# 1. A clean tree passes.
repo="$(make_repo)"
( cd "$repo" && main >/dev/null 2>&1 )
check "a clean Maud tree passes" 0 "$?"
rm -rf "$repo"

# 2. Rule 1: the pre-RC.6 hyphen forms render fine and do nothing.
for attr in data-on-click data-attr-hidden data-class-x data-style-x; do
  check "$attr= fails" 1 \
    "$(gate_rc "$SRC" "html! { div $attr=\"x\" {} }")"
done
check "a quoted dotted hyphen attribute fails" 1 \
  "$(gate_rc "$SRC" 'html! { input "data-on-change__debounce.1s"={ "x" } {} }')"

# 3. Rule 1: other names outside the grammar.
check "a key-less data-on fails" 1 \
  "$(gate_rc "$SRC" 'html! { span data-on="true" {} }')"
check "an unknown modifier fails" 1 \
  "$(gate_rc "$SRC" 'html! { a data-on:click__once=(x) {} }')"
check "data-onn:click fails" 1 \
  "$(gate_rc "$SRC" 'html! { a data-onn:click=(x) {} }')"
check "a custom attribute starting with a plugin name and a hyphen fails" 1 \
  "$(gate_rc "$SRC" 'html! { span data-text-size="12" {} }')"
check "data-style:color fails" 1 \
  "$(gate_rc "$SRC" 'html! { span data-style:color=(x) {} }')"

# 4. Rule 1: names inside the grammar.
check "a chained window and throttle.trailing modifier passes" 0 \
  "$(gate_rc "$SRC" 'html! { div data-on:resize__window__throttle.200ms.trailing=(x) {} }')"
check "a quoted debounce modifier passes" 0 \
  "$(gate_rc "$SRC" 'html! { input "data-on:input__debounce.300ms"=(x) {} }')"
check "data-indicator:_tab_loading passes" 0 \
  "$(gate_rc "$SRC" 'html! { div data-indicator:_tab_loading {} }')"
check "data-attr:aria-expanded passes" 0 \
  "$(gate_rc "$SRC" 'html! { div data-attr:aria-expanded=(x) {} }')"
check "prevent and outside modifiers pass" 0 \
  "$(gate_rc "$SRC" 'html! { a data-on:click__prevent__outside=(x) {} }')"
for attr in data-signals data-show data-text data-effect data-init; do
  check "key-less $attr passes" 0 \
    "$(gate_rc "$SRC" "html! { div $attr=\"x\" {} }")"
done

# 5. Rule 1: custom attributes are not Datastar-shaped.
for attr in data-only data-frame data-testid data-bool; do
  check "custom $attr passes" 0 \
    "$(gate_rc "$SRC" "html! { div $attr=\"x\" {} }")"
done

# 6. Rule 2: a raw action in a template.
check "a raw @get( fails" 1 \
  "$(gate_rc "$SRC" 'let x = format!("@get(/a)");')"
check "a raw @post( fails" 1 \
  "$(gate_rc "$SRC" 'let x = format!("@post(/a)");')"

# 7. Production region: the same violations in test code do not fire.
check "violations inside a #[cfg(test)] mod do not fire" 0 \
  "$(gate_rc "$SRC" 'pub fn a() {}' '#[cfg(test)]' 'mod tests {' \
     '    const A: &str = "data-on-click=x @get(/a) data-on:click__once=y";' '}')"
check "an attribute between #[cfg(test)] and mod still ends the region" 0 \
  "$(gate_rc "$SRC" 'pub fn a() {}' '#[cfg(test)]' '#[allow(clippy::x)]' 'mod tests {' \
     '    const A: &str = "@post(/a)";' '}')"
check "a violation before the #[cfg(test)] mod fires" 1 \
  "$(gate_rc "$SRC" 'let x = format!("@get(/a)");' '#[cfg(test)]' 'mod tests {}')"
check "#[cfg(test)] on a use does not end the region" 1 \
  "$(gate_rc "$SRC" '#[cfg(test)]' 'use foo;' 'html! { div data-on-click="x" {} }')"
check "#[cfg(test)] on an indented item does not end the region" 1 \
  "$(gate_rc "$SRC" '    #[cfg(test)]' '    mod tests {}' 'let x = format!("@get(/a)");')"
check "#[cfg(test)] on an out-of-line mod does not end the region" 1 \
  "$(gate_rc "$SRC" '#[cfg(test)]' 'mod fixtures;' 'let x = format!("@get(/a)");')"
check "#[cfg(test)] mod x; on one line does not end the region" 1 \
  "$(gate_rc "$SRC" '#[cfg(test)] mod fixtures;' 'let x = format!("@get(/a)");')"
# The region is per file: a test mod in one file leaves the next file scanned.
repo="$(make_repo)"
printf '%s\n' '#[cfg(test)]' 'mod tests {}' >"$repo/areas/deposit/editor/web/src/a.rs"
printf '%s\n' 'let x = format!("@get(/a)");' >"$repo/areas/deposit/editor/web/src/b.rs"
( cd "$repo" && git add -A && git commit -qm add && main >/dev/null 2>&1 )
check "a test mod in one file does not end the next file's region" 1 "$?"
rm -rf "$repo"
for name in tests.rs page_tests.rs test_support.rs test_utils.rs; do
  check "violations in $name do not fire" 0 \
    "$(gate_rc "areas/deposit/editor/web/src/$name" \
       'html! { div data-on-click="x" data-on:click__once=(y) {} }' 'let x = "@get(/a)";')"
done
check "a file whose name merely ends in tests.rs is scanned" 1 \
  "$(gate_rc areas/deposit/editor/web/src/contests.rs 'let x = "@get(/a)";')"
check "violations on // /// //! lines do not fire" 0 \
  "$(gate_rc "$SRC" '// data-on-click=x @get(/a)' '    /// data-attr-hidden=x @post(/a)' \
     '//! data-on:click__once=x')"

# 8. Pathspecs: nested, area-level and module paths are scanned.
check "a violation in a nested src/ subdirectory fails" 1 \
  "$(gate_rc areas/access/cpe/components/src/component/x.rs 'html! { a data-on-click="x" {} }')"
check "a violation in an area-level crate fails" 1 \
  "$(gate_rc areas/access/server/src/x.rs 'html! { a data-on-click="x" {} }')"
check "a violation in a module crate fails" 1 \
  "$(gate_rc modules/mosaic/tiles/src/x.rs 'html! { a data-on-click="x" {} }')"

# 9. A tree with no Maud sources is an error, not a pass: a gate that reads
#    nothing enforces nothing.
repo="$(mktemp -d "${TMPDIR:-/tmp}/datastar-attrs.XXXXXX")"
( cd "$repo" && git init -q -b main && git config user.email t@example.com \
    && git config user.name t && git commit -q --allow-empty -m init && main >/dev/null 2>&1 )
check "an empty tree is an error" 1 "$?"
rm -rf "$repo"

echo "check-datastar-attributes: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
