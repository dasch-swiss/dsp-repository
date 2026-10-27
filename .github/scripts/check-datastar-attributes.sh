#!/usr/bin/env bash
#
# Datastar reads a plugin's name, key and modifiers out of the attribute name,
# and Maud only takes literal attribute names, so a name cannot be typed in
# Rust. Two failures render fine and do nothing: the pre-RC.6 hyphen delimiter
# (`data-on-click`) and a name the bundle does not register or a modifier it
# does not know. The browser logs a console error, the element still renders,
# and a snapshot asserting the attribute is present still passes.
#
# Rule 1 holds every Datastar-shaped `data-*` token to a whitelist grammar
# (admits what the code uses today, grows with the change that first needs
# more). Rule 2 keeps actions out of templates: they come from the
# `shared-datastar` crate, which sits outside the pathspecs, so a raw
# `@get(` in a production region is a bypass.
#
# Only the production region of a file is read: the lines before the first
# column-0 `#[cfg(test)]` that guards an inline `mod … {`. Tests name the
# forbidden forms in negative assertions (`assert!(!out.contains("data-on-submit"))`)
# that pin these very rules; a gate that flagged them would be "fixed" by
# deleting them. What the gate does not catch is listed in ADR-0004.
#
# Safe to `source`. Dependencies: bash, git, awk.

# Must stay a quoted array, so git expands the glob rather than bash — bash's *
# does not cross `/` where a git pathspec's does, and an expanded-too-early glob
# silently reduces the gate to top-level files. The areas/ one needs no third
# `*` for the capability level: a pathspec `*` spans it, and one more `*` would
# miss an area-level composition root (`areas/<area>/server/src`, ADR-0002).
MAUD_PATHSPECS=('modules/*/*/src/*.rs' 'areas/*/*/src/*.rs')

# Output lines are `R1|file:line: text` or `R2|file:line: text`. The awk body
# holds no single quote, which would end the shell string around it.
# shellcheck disable=SC2016
DATASTAR_AWK='
# The grammar. `plugins` decides what is Datastar-shaped, the `re_*` patterns
# what is valid: a new plugin or modifier is added to both, with a case in
# check-datastar-attributes.test.sh, in the change that first uses it.
BEGIN {
  nplugins = split("on bind attr class signals indicator show text style effect init ref computed", plugins, " ")
  mod = "(prevent|window|outside|debounce\\.[0-9]+(ms|s)|throttle\\.[0-9]+(ms|s)(\\.trailing)?)"
  re_on = "^data-on:[a-z][a-z0-9-]*(__" mod ")*$"
  re_keyed = "^data-(attr|class|signals|bind|indicator):[A-Za-z_][A-Za-z0-9_-]*$"
  re_keyless = "^data-(attr|class|signals|bind|indicator|show|text|effect|init)$"
}

function shaped(rest,   i, p) {
  if (rest ~ /^[A-Za-z][A-Za-z0-9_-]*:/) return 1
  for (i = 1; i <= nplugins; i++) {
    p = plugins[i]
    if (rest == p) return 1
    if (index(rest, p) == 1 && substr(rest, length(p) + 1, 1) ~ /[:_-]/) return 1
  }
  return 0
}

function valid(tok) {
  return (tok ~ re_on || tok ~ re_keyed || tok ~ re_keyless)
}

FNR == 1 { skip = 0; pending = 0 }
skip { next }

# A column-0 #[cfg(test)] ends the production region only when the next
# non-attribute line opens an inline `mod … {`. On a `use`, `fn`, indented item
# or out-of-line `mod x;` (its body is another file) it does not.
/^#\[cfg\(test\)\][ \t]*mod [A-Za-z0-9_]+[ \t]*\{/ { skip = 1; next }
/^#\[cfg\(test\)\]/ { pending = 1; next }
pending && /^#\[/ { next }
pending && /^[ \t]*(\/\/.*)?$/ { next }
pending { if ($0 ~ /^mod [A-Za-z0-9_]+[ \t]*\{/) { skip = 1; next } pending = 0 }

/^[ \t]*\/\// { next }

{
  line = $0
  shown = line
  sub(/^[ \t]+/, "", shown)

  if (line ~ /@(get|post|put|patch|delete)\(/) {
    print "R2|" FILENAME ":" FNR ": " shown
  }

  n = length(line)
  pos = 1
  bad = 0
  while (pos <= n && !bad) {
    rest = substr(line, pos)
    if (!match(rest, /data-/)) break
    start = pos + RSTART - 1
    pre = (start > 1) ? substr(line, start - 1, 1) : ""
    pos = start + 5
    if (pre != "" && pre ~ /[A-Za-z0-9_-]/) continue
    tail = substr(line, start)
    match(tail, /^data-[A-Za-z0-9_:.-]*/)
    tok = substr(tail, 1, RLENGTH)
    sub(/\.+$/, "", tok)
    if (shaped(substr(tok, 6)) && !valid(tok)) {
      print "R1|" FILENAME ":" FNR ": " shown
      bad = 1
    }
  }
}
'

main() {
  local listed count f out rc r1 r2 failed=0
  local files=()

  # core.quotePath=false: a non-ASCII path comes back as-is, not C-quoted (and dropped below).
  listed="$(git -c core.quotePath=false ls-files -- "${MAUD_PATHSPECS[@]}")" || { echo "✗ git ls-files failed; the gate could not run" >&2; return 1; }

  while IFS= read -r f; do
    [ -n "$f" ] || continue
    # Whole-file test helpers: every line of these is test code.
    case "${f##*/}" in
      tests.rs | *_tests.rs | test_support.rs | *_test_support.rs | test_utils.rs | *_test_utils.rs) continue ;;
    esac
    [ -f "$f" ] && files+=("$f")
  done <<<"$listed"
  count="${#files[@]}"

  # Absence is an error, not zero work: a gate that reads nothing would pass
  # while enforcing nothing.
  [ "$count" -gt 0 ] || { echo "✗ no Maud sources matched (${MAUD_PATHSPECS[*]}). Run from the repo root" >&2; return 1; }

  out="$(awk "$DATASTAR_AWK" "${files[@]}")" && rc=0 || rc=$?
  [ "$rc" -eq 0 ] || { echo "✗ awk failed; the gate could not run" >&2; return 1; }

  r1="$(printf '%s\n' "$out" | sed -n 's/^R1|//p')"
  r2="$(printf '%s\n' "$out" | sed -n 's/^R2|//p')"

  if [ -n "$r1" ]; then
    failed=1
    printf '%s\n\n' "$r1" >&2
    echo "✗ a Maud template has a Datastar-shaped attribute name outside the whitelist" >&2
    echo "  grammar. Datastar ignores it: the element renders and the control does nothing." >&2
    echo "  Use the colon form with a registered plugin and known modifiers (data-on:click," >&2
    echo "  data-attr:disabled); rename a custom attribute that starts with a plugin name and" >&2
    echo "  a hyphen. A valid form the grammar lacks is added to DATASTAR_AWK in" >&2
    echo "  .github/scripts/check-datastar-attributes.sh, with a test case. See" >&2
    echo "  docs/src/dpe/architecture.md → Datastar Attribute Conventions." >&2
  else
    echo "✓ Datastar attribute names: every Datastar-shaped token fits the grammar ($count files checked)"
  fi

  if [ -n "$r2" ]; then
    failed=1
    printf '%s\n\n' "$r2" >&2
    echo "✗ a Maud template spells a Datastar action out (@get(, @post(, …). Build it with" >&2
    echo "  shared_datastar::get / post so retry and request options stay typed; every state" >&2
    echo "  change is a POST (ADR-0004). A missing option is a method in" >&2
    echo "  shared/datastar/src/lib.rs, with a test of its output." >&2
  else
    echo "✓ Datastar actions: no raw @get/@post/@put/@patch/@delete in production code ($count files checked)"
  fi

  return "$failed"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
