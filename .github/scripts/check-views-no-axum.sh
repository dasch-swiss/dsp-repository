#!/usr/bin/env bash
#
# In a web crate that also holds its capability's handlers, only handlers/, the
# route table (routes.rs), shell.rs, lib.rs and the handler tests' request
# harness (fixtures.rs, cfg(test)) use axum; the views (pages/,
# form/, entity/ and any module still to come) are `fn -> Markup` (ARCH-MAP.md,
# areas/deposit/editor and areas/deposit/identity). The crate depends on axum for
# its handlers, so check-capability-deps.sh cannot tell a view's use of it from a
# handler's: only a grep over the sources can. Everything but those is
# checked, so a new view directory is covered without an edit here. A web crate
# without handlers, beside a `server` crate, is held to no web framework at all
# by check-capability-deps.sh.
#
# The pattern matches a path into axum or an axum_* crate (`axum::`,
# `axum_extra::`), in a `use` or inline, and not a name that merely ends in
# `axum`. A comment that spells such a path is reported too; reword it.
#
# Safe to `source`. Dependencies: bash, git.

# The src/ directories of the web crates that hold handlers. Each must hold a
# tracked file: a moved crate would otherwise drop out of the gate unnoticed.
WEB_SOURCES=(
  'areas/deposit/editor/web/src'
  'areas/deposit/identity/web/src'
)

# The files and directories under each of WEB_SOURCES that may use axum.
AXUM_ALLOWED=('handlers' 'routes.rs' 'shell.rs' 'lib.rs' 'fixtures.rs')

AXUM_PATTERN='(^|[^[:alnum:]_])axum(_[[:alnum:]_]+)?::'

main() {
  local src allowed specs=() count=0 violations rc

  for src in "${WEB_SOURCES[@]}"; do
    if [ -z "$(git ls-files -- "$src")" ]; then
      echo "✗ no tracked file under $src. Run from the repo root, or update WEB_SOURCES in check-views-no-axum.sh" >&2
      return 1
    fi
    specs+=("$src")
    for allowed in "${AXUM_ALLOWED[@]}"; do
      specs+=(":(exclude)$src/$allowed")
    done
  done
  count="$(git ls-files -- "${specs[@]}" | wc -l | tr -d '[:space:]')"

  violations="$(git grep -nE "$AXUM_PATTERN" -- "${specs[@]}")" && rc=0 || rc=$?
  # git grep exits 1 for no matches, >1 for a real failure. Only the latter must
  # not be reported as a clean tree.
  [ "$rc" -le 1 ] || { echo "✗ git grep failed; the gate could not run" >&2; return 1; }

  if [ -n "$violations" ]; then
    printf '%s\n\n' "$violations" >&2
    echo "✗ a module outside handlers/, routes.rs, shell.rs, lib.rs and fixtures.rs uses axum. Views are" >&2
    echo "  \`fn -> Markup\`; the handler turns the Markup into a response. Move the axum code" >&2
    echo "  to handlers/. See ARCH-MAP.md, areas/deposit/editor." >&2
    return 1
  fi
  echo "✓ views: no axum in ${WEB_SOURCES[*]} outside ${AXUM_ALLOWED[*]} ($count files checked)"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -euo pipefail
  main "$@"
fi
