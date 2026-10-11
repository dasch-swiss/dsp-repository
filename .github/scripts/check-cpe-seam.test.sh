#!/usr/bin/env bash
#
# Tests for check-cpe-seam.sh. Dependency-free: bash, git, awk, grep.
#
# Fixtures build throwaway git repos because the gate reads its forbidden sets
# out of the index (the [package] names of areas/access/cpe/*/Cargo.toml,
# modules/mosaic/*/Cargo.toml and areas/access/sync/*/Cargo.toml), so stubbing git would test the stub, not the
# derivation. mktemp names an explicit TMPDIR template: a bare mktemp resolves to
# a path a sandboxed shell may not be allowed to write.
#
# Run: bash .github/scripts/check-cpe-seam.test.sh   (or `just test`)

set -uo pipefail

# See check-shared-paths.test.sh for why this is unset: git init honours an
# inherited GIT_DIR over the directory it is run in.
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_OBJECT_DIRECTORY GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_NAMESPACE

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./check-cpe-seam.sh disable=SC1091
source "$SCRIPT_DIR/check-cpe-seam.sh"

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

# check_msg <desc> <needle> <haystack>: the gate's output contains the needle.
check_msg() {
  case "$3" in
    *"$2"*) PASS=$((PASS + 1)) ;;
    *)
      FAIL=$((FAIL + 1))
      echo "FAIL: $1 (output lacks: $2)"
      printf '%s\n' "$3" | sed 's/^/    | /'
      ;;
  esac
}

# crate <dir> <name>: a minimal crate Cargo.toml at $dir/Cargo.toml.
crate() {
  mkdir -p "$1"
  printf '[package]\nname = "%s"\nversion = "0.1.0"\n\n[dependencies]\n' "$2" >"$1/Cargo.toml"
}

# base_repo: an uncommitted throwaway tree with six CPE crates, two Mosaic
# crates, a DPE crate, sync-store and a workspace root without [package]. It
# passes the gate as it is. Echoes the path; stage_repo finishes it.
base_repo() {
  local dir
  dir="$(mktemp -d "${TMPDIR:-/tmp}/cpe-seam.XXXXXX")"
  crate "$dir/areas/access/cpe/core" cpe-core
  crate "$dir/areas/access/cpe/ports" cpe-ports
  crate "$dir/areas/access/cpe/components" cpe-components
  crate "$dir/areas/access/cpe/image-viewer" cpe-image-viewer
  crate "$dir/areas/access/cpe/store" cpe-store
  crate "$dir/areas/access/cpe/store-build" cpe-store-build
  crate "$dir/modules/mosaic/tiles" mosaic-tiles
  crate "$dir/modules/mosaic/playground" mosaic-playground
  crate "$dir/areas/access/dpe/web" dpe-web
  crate "$dir/areas/access/sync/store" sync-store
  printf '[workspace]\nmembers = ["areas/access/dpe/web"]\n\n[workspace.dependencies]\nmaud = "0.27"\n' >"$dir/Cargo.toml"
  printf '%s' "$dir"
}

# stage_repo <repo>: the gate reads the index, so the fixture is staged and
# never committed: a commit would run the developer's hooks and signing.
stage_repo() {
  ( cd "$1" && git init -q -b main && git add -A )
}

# gate_in <repo>: run the gate there; stdout and stderr in $OUT, status in $RC.
gate_in() {
  OUT="$( cd "$1" && main 2>&1 )"
  RC=$?
}

# exec_in <repo>: as gate_in, but executes the script, as `just check` does,
# so its own `set -euo pipefail` applies.
exec_in() {
  OUT="$( cd "$1" && bash "$SCRIPT_DIR/check-cpe-seam.sh" 2>&1 )"
  RC=$?
}

# case_run <crate-dir> <text>: a fresh repo with <text> appended to that crate's
# manifest (a new crate named fixture-<dir> if the base has none there), then
# the gate. Sets $OUT and $RC.
case_run() {
  local repo
  repo="$(base_repo)"
  [ -f "$repo/$1/Cargo.toml" ] || crate "$repo/$1" "fixture-$(basename "$1")"
  printf '%s\n' "$2" >>"$repo/$1/Cargo.toml"
  stage_repo "$repo"
  gate_in "$repo"
  rm -rf "$repo"
}

# dropped_run <path>: the base repo with <path> removed from the index, then
# the gate. Sets $OUT and $RC.
dropped_run() {
  local repo
  repo="$(base_repo)"
  stage_repo "$repo"
  ( cd "$repo" && git rm -rqf "$1" )
  gate_in "$repo"
  rm -rf "$repo"
}

CPE=areas/access/cpe
DPE=areas/access/dpe/web

# --- The base fixture passes, so every failure below is its own edit's.
repo="$(base_repo)"
stage_repo "$repo"
gate_in "$repo"
check "the base fixture passes" 0 "$RC"
check_msg "the success line counts what it checked" "✓ CPE seam: 10 manifests checked against 6 CPE and 2 Mosaic crates" "$OUT"
exec_in "$repo"
check "the executed script passes on the base fixture" 0 "$RC"
rm -rf "$repo"

# Executed, with two violations: both are printed before the script exits.
repo="$(base_repo)"
printf 'cpe-core = "1"\n' >>"$repo/$DPE/Cargo.toml"
printf 'mosaic-tiles = "1"\n' >>"$repo/$CPE/components/Cargo.toml"
stage_repo "$repo"
exec_in "$repo"
check "the executed script fails on a violation" 1 "$RC"
check_msg "the executed script prints the rule 1 violation" "$DPE/Cargo.toml depends on cpe-core (rule 1" "$OUT"
check_msg "the executed script prints the rule 2 violation" "$CPE/components/Cargo.toml depends on mosaic-tiles (rule 2" "$OUT"
rm -rf "$repo"

# --- Rule 1: nothing outside the capability depends on a CPE crate.
case_run "$DPE" 'cpe-components = { path = "../../cpe/components" }'
check "rule 1: a DPE crate depending on cpe-components fails" 1 "$RC"
check_msg "rule 1: the violation names manifest, dependency and rule" "$DPE/Cargo.toml depends on cpe-components (rule 1" "$OUT"
check_msg "rule 1: the guidance points at the exception list" "PORT_EXCEPTIONS" "$OUT"

case_run "$DPE" 'cpe-core = { path = "../../cpe/core" }'
check "rule 1: a DPE crate depending on cpe-core fails" 1 "$RC"

case_run "$DPE" 'cpe-image-viewer = { path = "../../cpe/image-viewer" }'
check "rule 1: a DPE crate depending on cpe-image-viewer fails" 1 "$RC"

case_run areas/access/sync/store 'cpe-ports = { path = "../../cpe/ports" }'
check "rule 1: sync-store depending on cpe-ports passes" 0 "$RC"

# --- The matcher: every spelling of a dependency, in every table.
case_run "$DPE" 'cpe-core.workspace = true'
check "matcher: the .workspace = true form is caught" 1 "$RC"
case_run "$DPE" 'cpe-core.path = "../../cpe/core"'
check "matcher: the dotted key is caught" 1 "$RC"
case_run "$DPE" 'cpe-core = "0.1"'
check "matcher: the bare version string is caught" 1 "$RC"
case_run "$DPE" '"cpe-core" = { path = "../../cpe/core" }'
check "matcher: a quoted key is caught" 1 "$RC"
case_run "$DPE" '  cpe-core = { path = "../../cpe/core" }'
check "matcher: an indented key is caught" 1 "$RC"
case_run "$DPE" '[dev-dependencies.cpe-core]
path = "../../cpe/core"'
check "matcher: a [dev-dependencies.name] header is caught" 1 "$RC"
case_run "$DPE" '[build-dependencies.cpe-core]
path = "../../cpe/core"'
check "matcher: a [build-dependencies.name] header is caught" 1 "$RC"
case_run "$DPE" "[target.'cfg(unix)'.dependencies.cpe-core]
path = \"../../cpe/core\""
check "matcher: a target-specific table header is caught" 1 "$RC"
case_run "$DPE" '[dependencies."cpe-core"] # the domain crate
path = "../../cpe/core"'
check "matcher: a header with a quoted name and a trailing comment is caught" 1 "$RC"
case_run "$DPE" '[dependencies.cpe-core ]
path = "../../cpe/core"'
check "matcher: a header with a space before the bracket is caught" 1 "$RC"
case_run "$DPE" '[ dependencies . cpe-core ]
path = "../../cpe/core"'
check "matcher: a header with spaces around the dot is caught" 1 "$RC"
case_run "$DPE" 'core = { package = "cpe-core", path = "../../cpe/core" }'
check "matcher: a package = rename is caught" 1 "$RC"
case_run "$DPE" '[dependencies.views]
"package" = "cpe-core"
path = "../../cpe/core"'
check "matcher: a quoted package key is caught" 1 "$RC"
case_run "$DPE" '[dependencies.views]
package = "cpe-components"
path = "../../cpe/components"'
check "matcher: a package = line in a dependency table is caught" 1 "$RC"

case_run "$DPE" '# core = { package = "cpe-core", path = "../../cpe/core" }'
check "matcher: a commented package = line does not match" 0 "$RC"
case_run "$DPE" 'x = { subpackage = "cpe-core" }'
check "matcher: subpackage = does not match" 0 "$RC"
# Cargo does not resolve `cpe_core` to the package `cpe-core`.
case_run "$DPE" 'cpe_core = "1"'
check "matcher: the underscore spelling does not match" 0 "$RC"
case_run "$DPE" 'cpe-core-extras = "1"'
check "matcher: a longer name with the same prefix does not match" 0 "$RC"
case_run "$DPE" 'x = { package = "cpe-core-extras", version = "1" }'
check "matcher: a renamed longer name does not match" 0 "$RC"
case_run "$DPE" 'foo = ["cpe-core/bar"]'
check "matcher: a feature list naming a crate's feature does not match" 0 "$RC"

# An inline table of tables: the name is a key after `{` or `,`, not at the
# start of a line.
case_run "$DPE" "[target.'cfg(unix)']
dependencies = { cpe-core = { path = \"../../cpe/core\" } }"
check "matcher: a name in an inline dependencies table is caught" 1 "$RC"
check_msg "matcher: the inline violation names manifest, dependency and rule" "$DPE/Cargo.toml depends on cpe-core (rule 1" "$OUT"
case_run "$DPE" "[target.'cfg(unix)']
dependencies = { maud = \"0.27\", cpe-core = { path = \"../../cpe/core\" } }"
check "matcher: a name after a comma in an inline table is caught" 1 "$RC"
case_run "$DPE" "[target.'cfg(unix)']
dependencies = { \"cpe-core\" = \"1\" }"
check "matcher: a quoted name in an inline table is caught" 1 "$RC"
case_run "$DPE" "[target.'cfg(unix)']
dependencies = {cpe-core.path = \"../../cpe/core\"}"
check "matcher: a dotted name in an inline table is caught" 1 "$RC"
case_run "$DPE" "[target.'cfg(unix)']
dependencies = { cpe-core-extras = \"1\", my-cpe-core = \"1\" }"
check "matcher: a longer name in an inline table does not match" 0 "$RC"
case_run "$DPE" 'x = { version = "1", features = ["a", "cpe-core"] }'
check "matcher: a feature named like a crate in an inline table does not match" 0 "$RC"
case_run "$DPE" "# dependencies = { cpe-core = { path = \"../../cpe/core\" } }"
check "matcher: a commented inline table does not match" 0 "$RC"
# The line is not parsed, so prose in a string value that reads like an inline
# key is reported. Kept: narrowing the matcher would also miss
# `{ cpe-core.path = ... }`.
case_run "$DPE" 'description = "a, cpe-core. b"'
check "matcher: a string value reading like an inline key is reported" 1 "$RC"

# A patch table's header names the crate with no `dependencies` before it.
case_run "$DPE" '[patch.crates-io.cpe-core]
path = "../../cpe/core"'
check "matcher: a [patch.<registry>.name] header is caught" 1 "$RC"
case_run "$DPE" '[patch."https://example.org/cpe.git".cpe-core]
path = "../../cpe/core"'
check "matcher: a [patch.\"<url>\".name] header is caught" 1 "$RC"
case_run "$DPE" '[patch.crates-io.cpe-core-extras]
path = "../extras"'
check "matcher: a patch header for a longer name does not match" 0 "$RC"
case_run "$DPE" '[package.metadata.cpe-core]
note = "x"'
check "matcher: a header that is neither a dependency nor a patch table does not match" 0 "$RC"

# --- Rule 2: no CPE crate depends on Mosaic.
case_run "$CPE/components" 'mosaic-tiles = { path = "../../../../modules/mosaic/tiles" }'
check "rule 2: a CPE crate depending on mosaic-tiles fails" 1 "$RC"
check_msg "rule 2: the violation names manifest, dependency and rule" "$CPE/components/Cargo.toml depends on mosaic-tiles (rule 2" "$OUT"
case_run "$CPE/components" 'maud = { workspace = true }'
check "rule 2: a CPE crate depending on maud passes" 0 "$RC"
case_run "$DPE" 'mosaic-tiles = { path = "../../../../modules/mosaic/tiles" }'
check "rule 2: a DPE crate depending on mosaic-tiles passes" 0 "$RC"
case_run "$CPE/components" "[target.'cfg(unix)']
dependencies = { mosaic-tiles = \"1\" }"
check "rule 2: a Mosaic crate in an inline dependencies table fails" 1 "$RC"

# A manifest without [package] under the capability can only hide Mosaic
# behind a rename, as the root can hide a CPE crate.
repo="$(base_repo)"
printf '[workspace.dependencies]\nm = { package = "mosaic-tiles", path = "../../../modules/mosaic/tiles" }\n' >"$repo/$CPE/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "rule 2: a rename of a Mosaic crate in a manifest without [package] fails" 1 "$RC"
check_msg "rule 2: the rename violation names manifest, dependency and rule" "$CPE/Cargo.toml depends on mosaic-tiles (rule 2" "$OUT"
rm -rf "$repo"

# --- Rule 3: the leaves stay leaves.
case_run "$CPE/core" 'cpe-components = { path = "../components" }'
check "rule 3: cpe-core depending on cpe-components fails" 1 "$RC"
check_msg "rule 3: the violation names manifest, dependency and rule" "$CPE/core/Cargo.toml depends on cpe-components (rule 3" "$OUT"
case_run "$CPE/image-viewer" 'cpe-core = { path = "../core" }'
check "rule 3: cpe-image-viewer depending on cpe-core fails" 1 "$RC"
case_run "$CPE/ports" 'cpe-core = { path = "../core" }'
check "rule 3: cpe-ports depending on cpe-core fails" 1 "$RC"
case_run "$CPE/core" 'cpe-ports = { path = "../ports" }'
check "rule 3: cpe-core depending on cpe-ports fails" 1 "$RC"
case_run "$CPE/core" "[target.'cfg(unix)']
dependencies = { cpe-ports = { path = \"../ports\" } }"
check "rule 3: a leaf depending on a CPE crate in an inline table fails" 1 "$RC"
case_run "$CPE/components" 'cpe-core = { path = "../core" }
cpe-image-viewer = { path = "../image-viewer" }'
check "rule 3: cpe-components depending on cpe-core and cpe-image-viewer passes" 0 "$RC"
# The base fixture already shows that a leaf's own [package] name is not a
# dependency; a [lib] name is a `name` line as well.
case_run "$CPE/core" '[lib]
name = "cpe_core"'
check "rule 3: a leaf's [lib] name does not trip it" 0 "$RC"

# --- Rule 4: a CPE crate names a sync crate under dev-dependencies only.
# case_run appends below the fixture crate's [dependencies] header.

# fails_rule4 <desc>: the gate failed, and by rule 4. A case that guards the
# edge of the exemption would otherwise pass on any other rule's failure.
fails_rule4() {
  check "$1" 1 "$RC"
  check_msg "$1: by rule 4" "(rule 4" "$OUT"
}

case_run "$CPE/store-build" 'sync-store = { path = "../../sync/store" }'
check "rule 4: a CPE crate depending on sync-store fails" 1 "$RC"
check_msg "rule 4: the violation names manifest, dependency and rule" "$CPE/store-build/Cargo.toml depends on sync-store (rule 4" "$OUT"
check_msg "rule 4: the guidance names the table" "[dev-dependencies] only" "$OUT"
case_run "$CPE/store-build" '[dev-dependencies]
sync-store = { path = "../../sync/store" }'
check "rule 4: sync-store under [dev-dependencies] passes" 0 "$RC"
case_run "$CPE/store-build" '  [ dev-dependencies ] # the adapter, for the tests
sync-store.path = "../../sync/store"'
check "rule 4: a [dev-dependencies] header with spaces and a comment passes" 0 "$RC"
case_run "$CPE/store-build" '[dev-dependencies.sync-store]
path = "../../sync/store"'
check "rule 4: a [dev-dependencies.name] header passes" 0 "$RC"
case_run "$CPE/store-build" "[target.'cfg(unix)'.dev-dependencies]
sync-store = { path = \"../../sync/store\" }"
check "rule 4: a target-specific dev-dependencies table passes" 0 "$RC"
case_run "$CPE/store-build" '[dev-dependencies]
adapter = { package = "sync-store", path = "../../sync/store" }'
check "rule 4: a rename under [dev-dependencies] passes" 0 "$RC"
case_run "$CPE/store-build" 'adapter = { package = "sync-store", path = "../../sync/store" }'
fails_rule4 "rule 4: a rename under [dependencies] fails"
case_run "$CPE/store-build" '[build-dependencies]
sync-store = { path = "../../sync/store" }'
fails_rule4 "rule 4: sync-store under [build-dependencies] fails"
case_run "$CPE/store-build" "[target.'cfg(unix)'.dependencies.sync-store]
path = \"../../sync/store\""
fails_rule4 "rule 4: a target-specific dependencies header fails"
# The exemption ends at the next header.
case_run "$CPE/store-build" '[dev-dependencies]
tempfile = "3"

[dependencies.sync-store]
path = "../../sync/store"'
fails_rule4 "rule 4: a dependencies table after the dev-dependencies table fails"
# `dev-dependencies` is a whole segment of the header, not the tail of a name.
case_run "$CPE/store-build" '[dependencies.x-dev-dependencies]
package = "sync-store"'
fails_rule4 "rule 4: a dependency whose name ends in dev-dependencies is not exempt"
case_run "$CPE/store-build" '[dependencies.dev-dependencies]
package = "sync-store"'
fails_rule4 "rule 4: a dependency named dev-dependencies is not exempt"
# A header's trailing comment is not part of the header.
case_run "$CPE/store-build" '[dependencies.sync-store] # moved out of [dev-dependencies]
path = "../../sync/store"'
fails_rule4 "rule 4: a comment that ends in [dev-dependencies] exempts nothing"
case_run "$CPE/store-build" "[target.'cfg(any(unix, target_os = \"x\"))'.dev-dependencies] # tests
sync-store = { path = \"../../sync/store\" }"
check "rule 4: a dev-dependencies table below a cfg with dots, quotes and a comment passes" 0 "$RC"
case_run "$CPE/store-build" "[target.x.dev-dependencies.sync-store]
path = \"../../sync/store\""
check "rule 4: a target-specific [dev-dependencies.name] header passes" 0 "$RC"
# The line is not parsed, so the inline form and the dotted key are reported.
case_run "$CPE/store-build" "[target.'cfg(unix)']
dev-dependencies = { sync-store = { path = \"../../sync/store\" } }"
fails_rule4 "rule 4: an inline dev-dependencies table is reported"
case_run "$CPE/store-build" "[target.'cfg(unix)']
dev-dependencies.sync-store.path = \"../../sync/store\""
fails_rule4 "rule 4: a dotted dev-dependencies key is reported"
case_run "$CPE/store-build" "[target.'cfg(unix)']
dependencies.sync-store.path = \"../../sync/store\""
fails_rule4 "rule 4: a dotted dependencies key fails"
case_run "$DPE" "[target.'cfg(unix)']
build-dependencies.cpe-core = { path = \"../../cpe/core\" }"
check "matcher: a dotted key that starts with the table is caught" 1 "$RC"
# The target may stand before the table in the key itself, which TOML allows
# only above the first header.
repo="$(base_repo)"
printf "target.'cfg(unix)'.dependencies.sync-store.path = \"../../sync/store\"\n\n[package]\nname = \"cpe-store-build\"\nversion = \"0.1.0\"\n" >"$repo/$CPE/store-build/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
fails_rule4 "rule 4: a dotted key with the target before the table, above the first header, fails"
rm -rf "$repo"
# The cases below append the key under [dependencies], where TOML would read
# it otherwise: they hold the pattern to the line, which is all the gate reads.
case_run "$CPE/store-build" "target.'cfg(target_os = \"linux\")'.dependencies.sync-store = { path = \"../../sync/store\" }"
fails_rule4 "rule 4: a dotted key below a cfg that holds an equals sign fails"
case_run "$CPE/store-build" "target . \"cfg(unix)\" . build-dependencies . sync-store.path = \"../../sync/store\""
fails_rule4 "rule 4: a dotted target key with spaces around the dots fails"
case_run "$CPE/store-build" "target.'cfg(unix)'.dev-dependencies.sync-store.path = \"../../sync/store\""
fails_rule4 "rule 4: a dotted target key into dev-dependencies is reported"
case_run "$CPE/store-build" "# target.'cfg(unix)'.dependencies.sync-store.path = \"../../sync/store\""
check "rule 4: a commented dotted target key does not match" 0 "$RC"
case_run "$CPE/store-build" "target.'cfg(unix)'.dependencies.sync-store-extras.path = \"../extras\""
check "rule 4: a dotted target key for a longer name does not match" 0 "$RC"
case_run "$DPE" "target.'cfg(unix)'.dependencies.cpe-core.path = \"../../cpe/core\""
check "matcher: a dotted key with the target before the table is caught" 1 "$RC"
# Cargo also reads the tables in an underscore spelling.
case_run "$CPE/store-build" "[target.'cfg(unix)']
build_dependencies.sync-store.path = \"../../sync/store\""
fails_rule4 "rule 4: a dotted build_dependencies key fails"
case_run "$CPE/store-build" "[target.'cfg(unix)']
dev_dependencies.sync-store.path = \"../../sync/store\""
fails_rule4 "rule 4: a dotted dev_dependencies key is reported"
case_run "$CPE/store-build" '[dev_dependencies]
sync-store = { path = "../../sync/store" }'
fails_rule4 "rule 4: a [dev_dependencies] header is not exempt"
# The line is not parsed, so a key that is no dependency and is spelled like
# one is reported. Kept: the report is seen, and renaming the key ends it.
case_run "$CPE/store-build" '[features]
sync-store = []'
fails_rule4 "rule 4: a feature named like a sync crate is reported"
case_run "$CPE/store-build" '[package.metadata.docs]
sync-store = "see the adapter"'
fails_rule4 "rule 4: a metadata key named like a sync crate is reported"
# A match in the first lines of a long manifest: grep leaves early, and the
# executed script runs under pipefail.
repo="$(base_repo)"
crate "$repo/$CPE/store-build" cpe-store-build
{
  printf 'sync-store = { path = "../../sync/store" }\n'
  i=0
  while [ "$i" -lt 4000 ]; do
    printf '# a line of padding to make the manifest longer than a pipe holds\n'
    i=$((i + 1))
  done
} >>"$repo/$CPE/store-build/Cargo.toml"
stage_repo "$repo"
exec_in "$repo"
check "rule 4: the executed script fails on a match at the top of a long manifest" 1 "$RC"
check_msg "rule 4: the executed script prints the violation" "$CPE/store-build/Cargo.toml depends on sync-store (rule 4" "$OUT"
rm -rf "$repo"
# A manifest nested below a crate is held to the rule too.
case_run "$CPE/core/fuzz" 'sync-store = { path = "../../../sync/store" }'
fails_rule4 "rule 4: a nested manifest under the capability fails"
# The rule is the capability's: another capability may depend on sync-store.
case_run "$DPE" 'sync-store = { path = "../../sync/store" }'
check "rule 4: a DPE crate depending on sync-store passes" 0 "$RC"
# A second sync crate is forbidden by its [package] name, with no edit here.
repo="$(base_repo)"
crate "$repo/areas/access/sync/replay" sync-replay
printf 'sync-replay = { path = "../../sync/replay" }\n' >>"$repo/$CPE/components/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "rule 4: a new sync crate is forbidden without an edit to the script" 1 "$RC"
check_msg "rule 4: the violation names the new crate" "$CPE/components/Cargo.toml depends on sync-replay (rule 4" "$OUT"
rm -rf "$repo"

# --- Rule 5: a crate in CPE_EDGES depends on the CPE crates listed for it only.
case_run "$CPE/store-build" 'cpe-store = { path = "../store" }'
check "rule 5: cpe-store-build depending on cpe-store fails" 1 "$RC"
check_msg "rule 5: the violation names manifest, dependency and rule" "$CPE/store-build/Cargo.toml depends on cpe-store (rule 5" "$OUT"
check_msg "rule 5: the guidance names the list" "From a crate in CPE_EDGES" "$OUT"
case_run "$CPE/store" 'cpe-store-build = { path = "../store-build" }'
check "rule 5: cpe-store depending on cpe-store-build fails" 1 "$RC"
check_msg "rule 5: the violation names the builder" "$CPE/store/Cargo.toml depends on cpe-store-build (rule 5" "$OUT"
case_run "$CPE/store" 'cpe-components = { path = "../components" }'
check "rule 5: cpe-store depending on cpe-components fails" 1 "$RC"
check_msg "rule 5: the violation names the component library" "$CPE/store/Cargo.toml depends on cpe-components (rule 5" "$OUT"
# The lists differ: the port is the builder's, not the store's.
case_run "$CPE/store" 'cpe-ports = { path = "../ports" }'
check "rule 5: cpe-store depending on cpe-ports fails" 1 "$RC"
check_msg "rule 5: the violation names the port" "$CPE/store/Cargo.toml depends on cpe-ports (rule 5" "$OUT"
case_run "$CPE/store" 'cpe-core = { path = "../core" }'
check "rule 5: cpe-store depending on cpe-core passes" 0 "$RC"
case_run "$CPE/store-build" 'cpe-core = { path = "../core" }
cpe-ports = { path = "../ports" }'
check "rule 5: cpe-store-build depending on cpe-core and cpe-ports passes" 0 "$RC"
# No table is exempt: the builder's tests read the schema as a file too.
case_run "$CPE/store-build" '[dev-dependencies]
cpe-store = { path = "../store" }'
check "rule 5: cpe-store under the builder's [dev-dependencies] fails" 1 "$RC"
check_msg "rule 5: the dev-dependency violation names the rule" "$CPE/store-build/Cargo.toml depends on cpe-store (rule 5" "$OUT"
case_run "$CPE/store-build" 's = { package = "cpe-store", path = "../store" }'
check "rule 5: a rename of cpe-store in the builder fails" 1 "$RC"
check_msg "rule 5: the rename violation names the rule" "$CPE/store-build/Cargo.toml depends on cpe-store (rule 5" "$OUT"
# A crate that is not in the list is free: the page assembly and the host
# depend on the store, and the host on the builder.
case_run "$CPE/web" 'cpe-store = { path = "../store" }
cpe-store-build = { path = "../store-build" }
cpe-components = { path = "../components" }'
check "rule 5: a CPE crate that is not in CPE_EDGES may depend on the store and the builder" 0 "$RC"
dropped_run areas/access/cpe/store-build
check "rule 5: a CPE_EDGES path that is not tracked fails" 1 "$RC"
check_msg "rule 5: the message names the path and the list" "areas/access/cpe/store-build/Cargo.toml is not tracked. Update PORT_EXCEPTIONS, LEAVES or CPE_EDGES" "$OUT"

# --- Which manifests are CPE's and Mosaic's crates.
# A manifest nested below a crate is not one: its name is not forbidden.
repo="$(base_repo)"
crate "$repo/$CPE/components/fuzz" cpe-components-fuzz
printf 'cpe-components-fuzz = "1"\n' >>"$repo/$DPE/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a manifest nested below a CPE crate is not taken for a CPE crate" 0 "$RC"
rm -rf "$repo"
repo="$(base_repo)"
crate "$repo/modules/mosaic/tiles/fuzz" mosaic-tiles-fuzz
printf 'mosaic-tiles-fuzz = "1"\n' >>"$repo/$CPE/components/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a manifest nested below a Mosaic crate is not taken for a Mosaic crate" 0 "$RC"
rm -rf "$repo"
case_run "$CPE/components/fuzz" 'cpe-core = { path = "../../core" }'
check "a nested manifest under the capability may depend on a CPE crate" 0 "$RC"
case_run "$CPE/components/fuzz" 'mosaic-tiles = "1"'
check "a nested manifest under the capability may not depend on Mosaic" 1 "$RC"

# The first `name` line of this manifest is a [[bin]]'s, not the package's.
repo="$(base_repo)"
printf '[[bin]]\nname = "decoy"\n\n[package]\nname = "cpe-components"\nversion = "0.1.0"\n' >"$repo/$CPE/components/Cargo.toml"
printf 'cpe-components = "1"\n' >>"$repo/$DPE/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "the crate's name is read from [package], not from an earlier [[bin]]" 1 "$RC"
check_msg "the violation carries the [package] name" "depends on cpe-components (rule 1" "$OUT"
rm -rf "$repo"

# named_run <crate-dir> <manifest text>: the base repo with that crate's
# manifest replaced by <text>, and a DPE dependency on cpe-components and on
# mosaic-tiles from a CPE crate, so a name that drops out of a forbidden set
# shows as a pass. Sets $OUT and $RC.
named_run() {
  local repo
  repo="$(base_repo)"
  printf '%s\n' "$2" >"$repo/$1/Cargo.toml"
  printf 'cpe-components = "1"\n' >>"$repo/$DPE/Cargo.toml"
  printf 'mosaic-tiles = "1"\n' >>"$repo/$CPE/core/Cargo.toml"
  stage_repo "$repo"
  gate_in "$repo"
  rm -rf "$repo"
}

# Every spelling of the [package] name that TOML allows is read.
named_run "$CPE/components" '[package]
  name = "cpe-components"
version = "0.1.0"'
check "an indented name line is read" 1 "$RC"
check_msg "the indented name is forbidden outside" "$DPE/Cargo.toml depends on cpe-components (rule 1" "$OUT"
named_run "$CPE/components" '[ package ] # the library
"name" = "cpe-components"
version = "0.1.0"'
check_msg "a [ package ] header with spaces and a quoted name key is read" "$DPE/Cargo.toml depends on cpe-components (rule 1" "$OUT"
named_run "$CPE/components" 'package.name = "cpe-components"
package.version = "0.1.0"'
check_msg "a dotted package.name key is read" "$DPE/Cargo.toml depends on cpe-components (rule 1" "$OUT"
named_run modules/mosaic/tiles '  [package]
  name = "mosaic-tiles"
  version = "0.1.0"'
check_msg "an indented [package] table of a Mosaic crate is read" "$CPE/core/Cargo.toml depends on mosaic-tiles (rule 2" "$OUT"

# A name that cannot be read is an error: the crate would otherwise drop out of
# the forbidden sets and the gate would pass.
named_run "$CPE/components" 'package = { name = "cpe-components", version = "0.1.0" }'
check "a CPE crate whose [package] name cannot be read fails" 1 "$RC"
check_msg "the message names the manifest" "✗ $CPE/components/Cargo.toml: cannot read the [package] name" "$OUT"
named_run modules/mosaic/tiles '[package]
version = "0.1.0"'
check "a Mosaic crate whose [package] name cannot be read fails" 1 "$RC"
check_msg "the message names the manifest" "✗ modules/mosaic/tiles/Cargo.toml: cannot read the [package] name" "$OUT"
named_run "$CPE/components" '[workspace]
members = []'
check "a manifest without [package] where a CPE crate is expected fails" 1 "$RC"
check_msg "the message names the manifest" "✗ $CPE/components/Cargo.toml: cannot read the [package] name" "$OUT"

# Outside the capability, a [package] table in another spelling still makes the
# manifest a crate, whose every dependency form is read.
repo="$(base_repo)"
printf '[ package ]\nname = "dpe-web"\nversion = "0.1.0"\n\n[dependencies]\ncpe-core = "1"\n' >"$repo/$DPE/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a crate outside with a [ package ] header is checked as a crate" 1 "$RC"
check_msg "its violation names manifest, dependency and rule" "$DPE/Cargo.toml depends on cpe-core (rule 1" "$OUT"
rm -rf "$repo"
# A top-level `package.` key makes it a crate too. The CPE dependency is a
# plain entry under [workspace.dependencies], which only a crate is held to.
repo="$(base_repo)"
printf 'package.name = "dpe-web"\npackage.version = "0.1.0"\n\n[workspace.dependencies]\ncpe-core = "1"\n' >"$repo/$DPE/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a crate outside with top-level package. keys is checked as a crate" 1 "$RC"
check_msg "it is counted as a crate" "$DPE/Cargo.toml depends on cpe-core (rule 1" "$OUT"
rm -rf "$repo"
case_run "$DPE" '[dependencies.foo]
path = "../../cpe/core"
package = "cpe-core"'
check "a crate outside renaming a CPE crate in a dependency table fails" 1 "$RC"
check_msg "its violation names manifest, dependency and rule" "$DPE/Cargo.toml depends on cpe-core (rule 1" "$OUT"

# --- The workspace root has no [package] table.
repo="$(base_repo)"
printf 'cpe-core = { path = "areas/access/cpe/core" }\nmosaic-tiles = { path = "modules/mosaic/tiles" }\n' >>"$repo/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a plain [workspace.dependencies] entry is not a dependency" 0 "$RC"
rm -rf "$repo"
# A `package =` key in a table body is a dependency's rename, not a package
# table: the root stays a manifest without [package].
repo="$(base_repo)"
printf 'cpe-core = { path = "areas/access/cpe/core" }\n\n[workspace.dependencies.foo]\npath = "vendor/foo"\npackage = "x"\n' >>"$repo/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a package = key in a root dependency table does not make the root a crate" 0 "$RC"
check_msg "the root is not counted as a crate" "✓ CPE seam: 10 manifests checked" "$OUT"
rm -rf "$repo"
repo="$(base_repo)"
printf '\n[workspace.dependencies.foo]\npath = "areas/access/cpe/core"\npackage = "cpe-core"\n' >>"$repo/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a rename in a root dependency table fails" 1 "$RC"
check_msg "the table rename violation names the root manifest, the dependency and the rule" "Cargo.toml depends on cpe-core (rule 1" "$OUT"
rm -rf "$repo"
# The member's `x.workspace = true` names no CPE crate, so only the root's line
# shows the rename.
repo="$(base_repo)"
printf 'x = { package = "cpe-core", path = "areas/access/cpe/core" }\n' >>"$repo/Cargo.toml"
printf 'x.workspace = true\n' >>"$repo/$DPE/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a rename declared in the root's [workspace.dependencies] fails" 1 "$RC"
check_msg "the rename violation names the root manifest, the dependency and the rule" "Cargo.toml depends on cpe-core (rule 1" "$OUT"
rm -rf "$repo"
# The port is no exception: a leaf, or the store, would inherit the rename
# unseen.
repo="$(base_repo)"
printf 'x = { package = "cpe-ports", path = "areas/access/cpe/ports" }\n' >>"$repo/Cargo.toml"
printf 'x.workspace = true\n' >>"$repo/$CPE/store/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a rename of the port in the root's [workspace.dependencies] fails" 1 "$RC"
check_msg "the port's rename violation names the root manifest, the dependency and the rule" "Cargo.toml depends on cpe-ports (rule 3" "$OUT"
rm -rf "$repo"
# Without the rename the port stays what another capability may depend on.
repo="$(base_repo)"
printf 'cpe-ports = { path = "areas/access/cpe/ports" }\n' >>"$repo/Cargo.toml"
printf 'cpe-ports.workspace = true\n' >>"$repo/areas/access/sync/store/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "a plain root entry for the port, inherited by sync-store, passes" 0 "$RC"
rm -rf "$repo"
# A CPE crate inherits a renamed Mosaic or sync crate as `x.workspace = true`
# as well, so the root's renames are held to rules 2 and 4 too.
repo="$(base_repo)"
printf 'm = { package = "mosaic-tiles", path = "modules/mosaic/tiles" }\n' >>"$repo/Cargo.toml"
printf 'm.workspace = true\n' >>"$repo/$CPE/components/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "rule 2: a rename of a Mosaic crate declared in the root fails" 1 "$RC"
check_msg "rule 2: the rename violation names the root manifest, the dependency and the rule" "Cargo.toml depends on mosaic-tiles (rule 2" "$OUT"
rm -rf "$repo"
repo="$(base_repo)"
printf 'adapter = { package = "sync-store", path = "areas/access/sync/store" }\n' >>"$repo/Cargo.toml"
printf 'adapter.workspace = true\n' >>"$repo/$CPE/store-build/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "rule 4: a rename of a sync crate declared in the root fails" 1 "$RC"
check_msg "rule 4: the rename violation names the root manifest, the dependency and the rule" "Cargo.toml depends on sync-store (rule 4" "$OUT"
check_msg "rule 4: the guidance names the root's renames" "In the workspace root, rename no" "$OUT"
rm -rf "$repo"
repo="$(base_repo)"
printf '\n[workspace.dependencies.adapter]\npath = "areas/access/sync/store"\npackage = "sync-store"\n' >>"$repo/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "rule 4: a rename of a sync crate in a root dependency table fails" 1 "$RC"
check_msg "rule 4: the table rename violation names the root manifest and the rule" "Cargo.toml depends on sync-store (rule 4" "$OUT"
rm -rf "$repo"
# Without a rename the inheriting line names the crate, and its own table
# decides.
repo="$(base_repo)"
printf 'sync-store = { path = "areas/access/sync/store" }\n' >>"$repo/Cargo.toml"
printf '\n[dev-dependencies]\nsync-store.workspace = true\n' >>"$repo/$CPE/store-build/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
check "rule 4: a plain root entry inherited under [dev-dependencies] passes" 0 "$RC"
rm -rf "$repo"
# With the rename the inheriting table cannot be read, so the root's line is
# reported even for a dev-dependency. Kept: declaring it plainly ends the report.
repo="$(base_repo)"
printf 'adapter = { package = "sync-store", path = "areas/access/sync/store" }\n' >>"$repo/Cargo.toml"
printf '\n[dev-dependencies]\nadapter.workspace = true\n' >>"$repo/$CPE/store-build/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
fails_rule4 "rule 4: a renamed root entry inherited under [dev-dependencies] is reported"
rm -rf "$repo"
repo="$(base_repo)"
printf 'sync-store = { path = "areas/access/sync/store" }\n' >>"$repo/Cargo.toml"
printf 'sync-store.workspace = true\n' >>"$repo/$CPE/store-build/Cargo.toml"
stage_repo "$repo"
gate_in "$repo"
fails_rule4 "rule 4: a plain root entry inherited under [dependencies] fails"
check_msg "rule 4: the violation is the inheriting crate's" "$CPE/store-build/Cargo.toml depends on sync-store (rule 4" "$OUT"
rm -rf "$repo"

# --- Absence is an error.
dropped_run areas/access/cpe
check "a repository without areas/access/cpe/ fails" 1 "$RC"
check_msg "the message says what is missing" "no areas/access/cpe/*/Cargo.toml" "$OUT"
dropped_run modules/mosaic
check "a repository without modules/mosaic/ fails" 1 "$RC"
check_msg "the message says what is missing" "no modules/mosaic/*/Cargo.toml" "$OUT"
dropped_run areas/access/sync
check "a repository without areas/access/sync/ fails" 1 "$RC"
check_msg "the message says what is missing" "no areas/access/sync/*/Cargo.toml" "$OUT"
dropped_run areas/access/cpe/image-viewer
check "a leaf path that is not tracked fails" 1 "$RC"
check_msg "the message names the path" "areas/access/cpe/image-viewer/Cargo.toml" "$OUT"
dropped_run areas/access/cpe/ports
check "an exception path that is not tracked fails" 1 "$RC"
check_msg "the message names the path" "areas/access/cpe/ports/Cargo.toml" "$OUT"
# A manifest in the index and not on disk cannot be read: without this the
# gate would skip it and pass.
for missing in "$CPE/core" "$DPE"; do
  repo="$(base_repo)"
  stage_repo "$repo"
  rm "$repo/$missing/Cargo.toml"
  gate_in "$repo"
  check "a tracked manifest missing from the work tree fails ($missing)" 1 "$RC"
  check_msg "the message names the missing manifest" "$missing/Cargo.toml" "$OUT"
  rm -rf "$repo"
done

echo
echo "check-cpe-seam tests: $PASS passed, $FAIL failed"
[ "$FAIL" -eq 0 ]
