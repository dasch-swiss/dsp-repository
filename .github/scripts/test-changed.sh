#!/usr/bin/env bash
#
# Usage: test-changed.sh [base]    (default origin/main; `just test-changed`)
#        test-changed.sh --audit
#
# Runs `just test`'s commands restricted to the packages changed since
# merge-base(base, HEAD) (committed, staged, unstaged and untracked) and their
# reverse dependencies over every path-dependency kind. Anything it cannot map
# runs the full `just test`: unknown means full.
#
# Each changed path is classified by the first rule that matches:
#   1. .github/scripts/<name>.sh or <name>.test.sh selects <name>.test.sh; any
#      other path under .github/scripts/ selects every script test.
#   2. EXTRA_EDGES: adds packages that read the path without a Cargo edge to it.
#      Checked for every path outside rule 1; the path is then still classified
#      by 3 to 6.
#   3. The package whose manifest directory is the longest prefix of the path.
#   4. UNOWNED_TO_PACKAGE: a path outside every package that one serves.
#   5. UNOWNED_NO_TESTS: a path outside every package that no test reads.
#   6. Anything else: full run. Workspace-wide inputs (Cargo.lock, the root
#      Cargo.toml, rust-toolchain.toml, .cargo/, flake.*, justfile) land here
#      by being unowned.
# Table entries are bash `case` patterns over repo-root-relative paths: a
# directory is `dir/*`, a file its exact path.
#
# --audit lists the whole-file reads in *.rs (include_*!, CARGO_MANIFEST_DIR
# joins, repo-root path literals) that rules 2 to 5 would not connect to the
# reader, i.e. what EXTRA_EDGES is missing; exit 1 when it prints any. Run it
# after adding a read.
#
# The commands mirror the `test` recipe in the justfile: keep the two in step.
# TEST_CHANGED_DRY_RUN=1 prints the commands instead of running them.
#
# Safe to `source`. Dependencies: bash (3.2 compatible), git, cargo, jq, awk,
# and perl for --audit.

# "<path pattern> <package>...": packages that read the path without a Cargo edge.
EXTRA_EDGES=(
  "areas/access/dpe/server/data/* dpe-core dpe-api-oai dpe-web editor-core"
  "areas/access/cpe/server/tests/fixtures/* cpe-core"
  "areas/access/cpe/store/schema.sql cpe-store-build"
  "docs/src/cpe/config-language.md cpe-store-build"
)
# "<path pattern> <package>": a path outside every package that the package serves.
UNOWNED_TO_PACKAGE=(
  "areas/deposit/editor/public/* deposit-server"
  "areas/deposit/editor/style/* deposit-server"
)
# Paths outside every package that `just test` never reads.
# Documentation is matched by name, not `*.md`: data markdown that tests read
# (areas/access/media/data) must fall to rule 6.
UNOWNED_NO_TESTS=(
  'docs/*' '*/docs/*' '.github/*.md' '.github/workflows/*' '.claude/*'
  'README.md' '*/README.md' 'CLAUDE.md' '*/CLAUDE.md' 'CONTEXT.md' '*/CONTEXT.md'
  'ARCH-MAP.md' 'CHANGELOG.md' 'CONVENTIONS.md' 'REVIEW.md'
  '*/web-e2e-tests/*' 'modules/mosaic/playground-e2e-tests/*'
)

run() {
  if [ -n "${TEST_CHANGED_DRY_RUN:-}" ]; then
    printf 'run: %s\n' "$*"
  else
    "$@"
  fi
}

full_run() {
  echo "full run: $1"
  run just test
}

# The functions below set main's locals (roots, script_tests, all_scripts, full)
# through bash's dynamic scoping.

# A name the workspace lacks would reach cargo as a bad -p, so it forces a full run.
add_root() {
  case $'\n'"$names"$'\n' in
    *$'\n'"$1"$'\n'*) roots="${roots}$1"$'\t'"$2"$'\n' ;;
    *) full="$2 (a rule table names $1, which is no workspace package)" ;;
  esac
}

script_path() {
  local path="$1" file="${1#.github/scripts/}" test_file
  case "$file" in
    */*) all_scripts=1 ;;
    *.test.sh)
      [ -f "$path" ] && script_tests="${script_tests}${file}"$'\n'
      ;;
    *.sh)
      test_file="${file%.sh}.test.sh"
      if [ -f ".github/scripts/$test_file" ]; then
        script_tests="${script_tests}${test_file}"$'\n'
      else
        echo "warning: $path has no $test_file, nothing tests it" >&2
      fi
      ;;
    *) all_scripts=1 ;;
  esac
}

# package_dirs <metadata>: <name><TAB><manifest directory relative to the workspace root>
package_dirs() {
  jq -r '.workspace_root as $root | .packages[]
    | "\(.name)\t\(.manifest_path | ltrimstr($root + "/") | rtrimstr("/Cargo.toml"))"' <<<"$1"
}

# owner_of <path>: sets the caller's locals owner and owner_dir to
# the package whose manifest directory is the longest prefix of the path, or to
# "" when no package contains it.
owner_of() {
  local name dir
  owner="" owner_dir=""
  while IFS=$'\t' read -r name dir; do
    case "$1" in
      "$dir"/*)
        if [ "${#dir}" -gt "${#owner_dir}" ]; then
          owner="$name"
          owner_dir="$dir"
        fi
        ;;
    esac
  done <<<"$packages"
}

classify() {
  local path="$1" entry pat pkgs pkg owner owner_dir

  case "$path" in
    .github/scripts/*)
      script_path "$path"
      return
      ;;
  esac

  for entry in ${EXTRA_EDGES[@]+"${EXTRA_EDGES[@]}"}; do
    read -r pat pkgs <<<"$entry"
    # shellcheck disable=SC2254
    case "$path" in
      $pat) for pkg in $pkgs; do add_root "$pkg" "$path"; done ;;
    esac
  done

  owner_of "$path"
  if [ -n "$owner" ]; then
    add_root "$owner" "$path"
    return
  fi

  for entry in ${UNOWNED_TO_PACKAGE[@]+"${UNOWNED_TO_PACKAGE[@]}"}; do
    read -r pat pkg <<<"$entry"
    # shellcheck disable=SC2254
    case "$path" in
      $pat)
        add_root "$pkg" "$path"
        return
        ;;
    esac
  done

  for pat in ${UNOWNED_NO_TESTS[@]+"${UNOWNED_NO_TESTS[@]}"}; do
    # shellcheck disable=SC2254
    case "$path" in $pat) return ;; esac
  done

  full="$path"
}

# path_matches <path> <pattern>: a rule-table pattern matches the path, or the
# path read as a directory.
path_matches() {
  local form
  for form in "$1" "$1/"; do
    # shellcheck disable=SC2254
    case "$form" in $2) return 0 ;; esac
  done
  return 1
}

# normalize <relative path>: resolve . and .. segments; fails when the path
# leaves the repository.
normalize() {
  local IFS=/ seg out=""
  set -f
  for seg in $1; do
    case "$seg" in
      '' | .) ;;
      ..)
        if [ -z "$out" ]; then
          set +f
          return 1
        fi
        case "$out" in
          */*) out="${out%/*}" ;;
          *) out="" ;;
        esac
        ;;
      *) out="${out:+$out/}$seg" ;;
    esac
  done
  set +f
  printf '%s\n' "$out"
}

# audit_reads: one line per whole-file read in the tracked *.rs files,
# "<file><TAB><line><TAB><kind><TAB><literal>". kind is rel (an include_*! path,
# relative to the file), manifest (a CARGO_MANIFEST_DIR join, relative to the
# crate) or root (a repo-root-relative literal, whitespace-free so that
# messages naming a path are not reads). perl -0777 sees reads that span lines.
audit_reads() {
  # shellcheck disable=SC2016
  git ls-files -z '*.rs' | xargs -0 perl -0777 -ne '
    my $f = $ARGV;
    my @spans;
    sub ln { my ($t, $p) = @_; 1 + (substr($t, 0, $p) =~ tr/\n//) }
    sub out { print join("\t", @_), "\n" }
    while (/include_(?:str|bytes)!\s*\(\s*"([^"\\\n]*)"/g) {
      push @spans, [$-[0], $+[0]];
      out($f, ln($_, $-[0]), "rel", $1);
    }
    while (/env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)((?:\s*\))?(?:\s*\.\s*join\s*\(\s*"[^"\\\n]*"\s*\))+|(?:\s*,\s*"[^"\\\n]*")+)/g) {
      my $tail = $1;
      push @spans, [$-[0], $+[0]];
      out($f, ln($_, $-[0]), "manifest", join($tail =~ /join/ ? "/" : "", $tail =~ /"([^"]*)"/g));
    }
    while (/format!\s*\(\s*"\{\}([^"\\\n]*)"\s*,\s*env!\s*\(\s*"CARGO_MANIFEST_DIR"\s*\)/g) {
      push @spans, [$-[0], $+[0]];
      out($f, ln($_, $-[0]), "manifest", $1);
    }
    while (/"((?:areas|modules|shared|docs)\/[^"\s{}\\:]*)"/g) {
      my $at = $-[1];
      next if grep { $at >= $_->[0] && $at < $_->[1] } @spans;
      out($f, ln($_, $-[0]), "root", $1);
    }
  '
}

# unread_reason <target> <reader package>: prints why the read of a repo-root
# relative target reaches no test of the reader and returns 0; returns 1 when
# a Cargo edge, an EXTRA_EDGES line or rule 6 covers it.
unread_reason() {
  local target="$1" reader="$2" owner owner_dir entry pat pkgs pkg="" swallowed="" deps
  owner_of "$target/"
  pkg="$owner"
  if [ -z "$pkg" ]; then
    for entry in ${UNOWNED_TO_PACKAGE[@]+"${UNOWNED_TO_PACKAGE[@]}"}; do
      read -r pat owner <<<"$entry"
      if path_matches "$target" "$pat"; then
        pkg="$owner"
        break
      fi
    done
  fi
  if [ -z "$pkg" ]; then
    for pat in ${UNOWNED_NO_TESTS[@]+"${UNOWNED_NO_TESTS[@]}"}; do
      path_matches "$target" "$pat" && swallowed=1
    done
    [ -n "$swallowed" ] || return 1
  else
    [ "$pkg" = "$reader" ] && return 1
    deps="$(awk -F'\t' -v r="$reader" '$1 == r { print $2 }' <<<"$reach")"
    case " $deps " in *" $pkg "*) return 1 ;; esac
  fi
  for entry in ${EXTRA_EDGES[@]+"${EXTRA_EDGES[@]}"}; do
    read -r pat pkgs <<<"$entry"
    if path_matches "$target" "$pat"; then
      case " $pkgs " in *" $reader "*) return 1 ;; esac
    fi
  done
  if [ -n "$pkg" ]; then
    echo "package $pkg, not reached from ${reader:-any package}"
  else
    echo "unowned, rule 5 selects no test"
  fi
}

# Sets packages and reach (every package's dependency closure over path
# dependencies, itself included) for unread_reason.
audit() {
  local metadata packages reach file line kind lit target reason found=0
  local owner owner_dir reads_file

  if ! command -v perl >/dev/null 2>&1; then
    echo "test-changed.sh --audit needs perl" >&2
    return 2
  fi
  cd "$(git rev-parse --show-toplevel)" || return 2
  metadata="$(cargo metadata --format-version=1 --no-deps)" || return 2
  packages="$(package_dirs "$metadata")"
  # <package><TAB><space-separated closure>
  reach="$(jq -r '
    [.packages[].name] as $names
    | [.packages[] | .name as $from | .dependencies[]
       | select(.path != null and (.name | IN($names[]))) | {from: $from, to: .name}] | unique as $edges
    | def step($s): ($s + [$edges[] | select(.from | IN($s[])) | .to]) | unique;
      $names[] as $n
    | ([$n] | until(step(.) == .; step(.))) as $c
    | "\($n)\t\($c | join(" "))"' <<<"$metadata")"

  reads_file="$(mktemp "${TMPDIR:-/tmp}/test-changed.XXXXXX")" || return 2
  if ! audit_reads >"$reads_file"; then
    rm -f "$reads_file"
    echo "test-changed.sh --audit: scanning the *.rs files failed" >&2
    return 2
  fi

  while IFS=$'\t' read -r file line kind lit; do
    owner_of "$file"
    case "$kind" in
      rel) target="$(normalize "$(dirname "$file")/$lit")" || continue ;;
      manifest)
        [ -n "$owner_dir" ] || continue
        target="$(normalize "$owner_dir/$lit")" || continue
        ;;
      *) target="$(normalize "$lit")" || continue ;;
    esac
    reason="$(unread_reason "$target" "$owner")" || continue
    echo "$file:$line: \"$lit\" -> $target ($reason)"
    found=1
  done <"$reads_file"
  rm -f "$reads_file"
  [ "$found" = 0 ]
}

main() {
  local base="${1:-origin/main}" merge_base metadata tmp_file path
  local packages names roots="" script_tests="" all_scripts="" full=""
  local closure root_names selected="" p paths n shown rd line t
  local pkg_args=() dev_args=() dev_features="" rc=0

  cd "$(git rev-parse --show-toplevel)" || return 1
  merge_base="$(git merge-base "$base" HEAD)" || { full_run "no merge-base with $base"; return; }

  tmp_file="$(mktemp "${TMPDIR:-/tmp}/test-changed.XXXXXX")" || return 1
  if ! metadata="$(cargo metadata --format-version=1 --no-deps 2>"$tmp_file")"; then
    cat "$tmp_file" >&2
    rm -f "$tmp_file"
    full_run "cargo metadata failed"
    return
  fi

  packages="$(package_dirs "$metadata")" || {
    rm -f "$tmp_file"
    full_run "reading the package list failed"
    return
  }
  names="$(cut -f1 <<<"$packages")"

  # A file, not a substitution: the list is NUL-separated.
  if ! { git diff -z --name-only --no-renames "$merge_base" -- &&
    git ls-files -z --others --exclude-standard; } >"$tmp_file"; then
    rm -f "$tmp_file"
    full_run "listing the changed paths failed"
    return
  fi
  while IFS= read -r -d '' path; do
    classify "$path"
    [ -n "$full" ] && break
  done <"$tmp_file"
  rm -f "$tmp_file"

  if [ -n "$full" ]; then
    full_run "$full"
    return
  fi

  root_names="$(cut -f1 <<<"$roots" | sort -u | grep -v '^$')"
  if [ -n "$root_names" ]; then
    # <root><TAB><one of its reverse dependencies>
    closure="$(jq -r --arg roots "$root_names" '
      [.packages[].name] as $names
      | [.packages[] | .name as $from | .dependencies[]
         | select(.path != null and (.name | IN($names[]))) | {from: $from, to: .name}] | unique as $edges
      | def step($s): ($s + [$edges[] | select(.to | IN($s[])) | .from]) | unique;
        ($roots | split("\n") | map(select(. != ""))[]) as $r
      | ([$r] | until(step(.) == .; step(.))) - [$r] | .[] | "\($r)\t\(.)"' <<<"$metadata")" ||
      { full_run "computing the reverse dependencies failed"; return; }
    selected="$(printf '%s\n%s\n' "$root_names" "$(cut -f2 <<<"$closure")" | grep -v '^$' | sort -u | tr '\n' ' ')"
  fi

  if [ -n "$all_scripts" ]; then
    script_tests="$(for t in .github/scripts/*.test.sh; do [ -f "$t" ] && echo "${t#.github/scripts/}"; done)"
  fi
  script_tests="$(printf '%s' "$script_tests" | grep -v '^$' | sort -u)"

  if [ -z "$selected" ] && [ -z "$script_tests" ]; then
    echo "nothing to test since $base"
    return 0
  fi

  for p in $root_names; do
    paths="$(awk -F'\t' -v p="$p" '$1 == p { print $2 }' <<<"$roots" | sort -u)"
    n="$(wc -l <<<"$paths" | tr -d ' ')"
    shown="$(head -n 3 <<<"$paths" | paste -sd, - )"
    shown="${shown//,/, }"
    [ "$n" -gt 3 ] && shown="$shown, +$((n - 3)) more"
    line="$p ($shown)"
    rd="$(awk -F'\t' -v p="$p" '$1 == p { print $2 }' <<<"$closure" | paste -sd, - )"
    [ -n "$rd" ] && line="$line + rdeps: ${rd//,/, }"
    echo "$line"
  done
  if [ -n "$all_scripts" ]; then
    echo "script tests: all"
  elif [ -n "$script_tests" ]; then
    echo "script tests: $(paste -sd, - <<<"$script_tests" | sed 's/,/, /g')"
  fi

  for p in $selected; do
    pkg_args=(${pkg_args[@]+"${pkg_args[@]}"} -p "$p")
    case "$p" in
      dpe-server | mosaic-playground)
        dev_args=(${dev_args[@]+"${dev_args[@]}"} -p "$p")
        dev_features="${dev_features:+$dev_features,}$p/dev"
        ;;
    esac
  done
  if [ "${#pkg_args[@]}" -gt 0 ]; then
    run cargo test --tests "${pkg_args[@]}" || return $?
  fi
  if [ "${#dev_args[@]}" -gt 0 ]; then
    run cargo test "${dev_args[@]}" --features "$dev_features" --tests || return $?
  fi
  for t in $script_tests; do
    run bash ".github/scripts/$t" || rc=1
  done
  return "$rc"
}

if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
  set -uo pipefail
  case "${1:-}" in
    --audit) audit ;;
    *) main "$@" ;;
  esac
fi
