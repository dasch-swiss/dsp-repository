# The services' version, which release-please updates; the dev recipes report it as `service.version`.

VERSION := trim(read("version.txt"))

# DPE's published data set, which the image recipes below package and the editor recipes read from.

DPE_DATA_DIR := "areas/access/dpe/server/data"

# Projects whose OAI records `just fetch-records` refreshes. Add a shortcode here to track a new project.

RECORD_SHORTCODES := "081C 0868 0803"

# Pinned nightly rustfmt, shared with CI and flake.nix. Bumping it: see CONVENTIONS.md → Formatting.

RUSTFMT_NIGHTLY := trim(read(".rustfmt-nightly"))

# List all recipes
default:
    just --list --unsorted

# Install all requirements
install-requirements: install-e2e-requirements
    #!/usr/bin/env sh
    rustup show
    # Nightly rustfmt: .rustfmt.toml sets nightly-only options and `just fmt` runs the pinned nightly.
    # rust-toolchain.toml pins stable, so `rustup show` alone never provisions it. Bazel builds with its
    # own toolchain (MODULE.bazel); the rustup toolchains serve rustfmt, `cargo binstall` and dsp-cli's
    # publish.
    rustup toolchain install {{ RUSTFMT_NIGHTLY }} --component rustfmt
    brew install cargo-binstall
    # Native build deps the Nix devShell provides via flake.nix buildInputs. No cmake: `ring` is the
    # one TLS crypto provider and builds with the C compiler alone, where `aws-lc-sys` needed cmake.
    # `jq` is what `just fair-check` reads F-UJI's JSON result with.
    brew install pkg-config jq
    # Bazelisk provides `bazel` and reads .bazelversion. osv-scanner and cargo-audit are the two
    # halves of `just audit`.
    brew install bazelisk osv-scanner
    # commitlint-rs powers the commit-message gate in `just commit-lint`
    cargo binstall -y commitlint-rs@0.2.4
    cargo binstall -y mdbook@0.4.52
    cargo binstall -y mdbook-alerts@0.8.0
    cargo binstall -y mdbook-mermaid@0.16.2
    cargo binstall -y bacon@3.23.0
    cargo binstall -y maudfmt@0.1.8
    cargo binstall -y cargo-audit@0.22.2

# Install Playwright browsers for E2E tests
install-e2e-requirements: _check-node
    # `npm ci` first: the suites run via `npx playwright test`, which resolves the runner from
    # node_modules. With none, npx re-resolves the `^1.44.1` range on every run and can land on a
    # version that mismatches the installed browsers ("Executable doesn't exist"). Both packages
    # track a package-lock.json, so `ci` pins the runner to the same version the browsers match.
    cd modules/mosaic/playground-e2e-tests && npm ci && npx playwright install
    cd areas/access/dpe/web-e2e-tests && npm ci && npx playwright install
    cd areas/deposit/editor/web-e2e-tests && npm ci && npx playwright install

# Verify Node is on PATH. just runs recipes in sh, which does NOT see shell-function version managers (e.g. lazy nvm) — only real binaries on PATH. (DEV-6642)
[private]
_check-node:
    @command -v node >/dev/null 2>&1 || { echo >&2 "error: 'node' not on PATH. just runs recipes in sh, which can't see nvm's lazy shell functions — expose your default node bin on PATH for all shells (eager-load it in your shell rc, or use brew/volta/asdf). See docs/src/fundamentals/onboarding.md."; exit 1; }

# Each modules/*/public/vendor/README.md and areas/*/*/public/vendor/README.md
# table against the files it describes.

# Verify the third-party bytes we ship or execute. Run by `just check`. (DEV-7126, DEV-6727)
verify-checksums:
    bash .github/scripts/verify-checksums.sh

# A shared crate must not know one service's directory layout. It compiles
# either way, so only a grep catches it.

# Verify no shared crate hardcodes a path into a service module. Run by `just check`. (DEV-7046)
check-shared-paths:
    bash .github/scripts/check-shared-paths.sh

# `@crates` targets are public, so a role's third-party bans cannot be a
# `visibility` list; first-party edges are (ADR-0002, ADR-0003, ADR-0007).

# Verify each Deposit and Access Area role's BUILD.bazel uses only the third-party crates its role allows, and that no first-party target is public. Run by `just check`. (DEV-7375, DEV-7527, ADR-0003)
check-capability-deps:
    bash .github/scripts/check-capability-deps.sh

# editor-web and identity-web depend on axum for their handlers, so a view that
# uses it compiles fine and only a grep over the sources catches it.

# Verify only handlers/, routes.rs, shell.rs, lib.rs and fixtures.rs of editor-web and identity-web use axum. Run by `just check`. (DEV-7527)
check-views-no-axum:
    bash .github/scripts/check-views-no-axum.sh

# `data-on-`, `data-attr-`, `data-class-`, `data-style-`: the attribute renders
# fine and the control is inert, so only a grep or a browser catches it.

# Verify Datastar attribute names fit the whitelist grammar and no template spells an action out. Run by `just check`. (DEV-6920, DEV-7369)
check-datastar-attributes:
    bash .github/scripts/check-datastar-attributes.sh

# A wrong ADR citation reads fine in prose and compiles fine as code, so only
# a grep over the whole tree catches a renumbered or never-written record.

# Verify every ADR reference resolves to a record. Run by `just check`. (DEV-7330)
check-adr-refs:
    bash .github/scripts/check-adr-refs.sh

# A live test needs a DSP stack, and without #[ignore] it runs — and passes
# vacuously via its own early-return skip — under a plain test invocation.

# Verify every dsp-cli live test is #[ignore]d. Run by `just check`. (DEV-7330)
check-live-tests-ignored:
    bash .github/scripts/check-live-tests-ignored.sh

# The pinned nightly rustfmt: `$RUSTFMT` when set (the Nix shell exports it), else rustup's. A bare `rustfmt` would be the stable one, which ignores .rustfmt.toml's nightly options. It formats files, so the edition is passed: 2024 for dsp-cli, 2021 for the rest of the tree.

[private]
_rustfmt *ARGS:
    #!/usr/bin/env bash
    set -euo pipefail
    rustfmt="${RUSTFMT:-$(rustup which --toolchain {{ RUSTFMT_NIGHTLY }} rustfmt)}"
    git ls-files -z '*.rs' ':!dsp-cli/**' | xargs -0 "$rustfmt" --edition 2021 {{ ARGS }}
    git ls-files -z 'dsp-cli/**.rs' | xargs -0 "$rustfmt" --edition 2024 {{ ARGS }}

# Run every gate script, the format checks and clippy over every Bazel target, tests included, with `-Dwarnings` (set in .bazelrc). FLAGS: extra Bazel flags, such as CI's `--config=ci` and remote flags.
check *FLAGS: verify-checksums check-shared-paths check-capability-deps check-views-no-axum check-datastar-attributes check-adr-refs check-live-tests-ignored
    #!/usr/bin/env bash
    set -euo pipefail
    just --check --fmt --unstable
    # maudfmt 0.1.8 has no --check mode, so verify it is a no-op on the `html!` macros by
    # formatting a throwaway copy of each tracked .rs file and diffing (non-mutating). maudfmt
    # leaves files without `html!` byte-identical, so iterating all .rs is safe.
    tmp="$(mktemp "${TMPDIR:-/tmp}/maudfmt.XXXXXX")"
    trap 'rm -f "$tmp"' EXIT
    rc=0
    while IFS= read -r -d '' f; do
        cp "$f" "$tmp"
        maudfmt "$tmp" >/dev/null 2>&1
        if ! diff -q "$f" "$tmp" >/dev/null; then echo "maudfmt would reformat: $f" >&2; rc=1; fi
    done < <(git ls-files -z '*.rs')
    [ "$rc" -eq 0 ] || { echo "run 'just fmt' to fix Maud formatting" >&2; exit 1; }
    just _rustfmt --check
    # `--keep_going` reports every failing target in one run. The dsp-cli live tests are `manual`,
    # outside `//...`, and named here so they are compiled and linted too.
    live="$(bazel query 'attr(tags, manual, kind(rust_test, //dsp-cli:all))')"
    bazel build {{ FLAGS }} --keep_going --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect --output_groups=clippy_checks //... $live

# Format all code: maudfmt for the `html!` Maud macros, then the pinned nightly rustfmt for the rest.
fmt:
    #!/usr/bin/env bash
    set -euo pipefail
    git ls-files -z '*.rs' | xargs -0 maudfmt --
    just _rustfmt

# Fix justfile formatting. Warning: will change existing file. Please first use check.
fix:
    just --fmt --unstable

# Build every Bazel target. FLAGS: extra Bazel flags, such as CI's `--config=ci` and remote flags.
build *FLAGS:
    bazel build {{ FLAGS }} //...

# Run the server from the repository root, a release build with the release stylesheet, as the image runs it. CPE is mounted only with a base, so it is set here.
run *FLAGS: css-release
    CPE_BASE=/cpe bazel run --config=release --run_in_cwd {{ FLAGS }} //areas/access/server:access_server -- serve

# Validate all data files in the default data directory
validate-data:
    bazel run --run_in_cwd //areas/access/server:access_server -- validate {{ DPE_DATA_DIR }}

# Re-download and re-patch the DataCite JSON schema the DataCite JSON writer's output is shape-checked against. The script takes the XSD directory as an argument because a shared crate may not know a module's layout; this justfile may, and passes it. (DEV-7268)
refresh-datacite-schema:
    bash shared/fair/testdata/schemas/download-schemas.sh \
        areas/access/dpe/api-oai/src/handlers/testdata/schemas/include

# Refresh the tracked OAI record dumps. Needs `bearer` in the environment (see README).
[group('dpe')]
fetch-records:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${bearer:?export bearer=\"Bearer eyJ...\"}"
    for sc in {{ RECORD_SHORTCODES }}; do
        out=areas/access/dpe/server/data/records/$sc-records.json
        curl -fsS -H "Authorization: $bearer" 'https://api.dasch.swiss/v3/export/resources/oai' \
            -d '{"shortcode": "'"$sc"'"}' -o "$out" -w "$out %{http_code}\n" >&2
    done

# Run every Bazel test, then the gate scripts' tests. FLAGS: extra Bazel flags, such as CI's.
test *FLAGS:
    bazel test {{ FLAGS }} //...
    just test-scripts

# Run the tests of every .github/scripts/*.sh gate. Needs bash, git, awk, perl and jq.
test-scripts:
    #!/usr/bin/env bash
    set -uo pipefail
    # Every file, not a list: a new gate's tests run without an edit here. All
    # run, so one report names every failing gate.
    rc=0
    for t in .github/scripts/*.test.sh; do
        bash "$t" || rc=1
    done
    exit "$rc"

# Build the libFuzzer binaries of the DPE fuzz targets (`--config=fuzz`) and print their paths. Run one directly with its corpus directory, not through `bazel run`, whose sandbox would discard the corpus it grows (docs/src/bazel.md, *Fuzz targets*). `--remote_download_toplevel` keeps the binaries local under a remote cache.
[group('dpe')]
build-fuzz *FLAGS:
    #!/usr/bin/env bash
    set -euo pipefail
    targets=$(bazel query --output=label 'attr(name, "_bin$", //areas/access/dpe/server/fuzz:*)')
    bazel build --config=fuzz --remote_download_toplevel {{ FLAGS }} $targets
    # Not `bazel-bin`: it follows whichever configuration was built last.
    bazel cquery --config=fuzz {{ FLAGS }} --output=files "set($targets)"

# Accept the `insta` snapshots of one test target: runs it from the checkout with `INSTA_UPDATE=always`, so `insta` writes the snapshot files (never rename a `.snap.new` by hand); review with `git diff`. Example: `just snap-accept //areas/access/dpe/server:dpe_server_test`.
[group('bazel')]
snap-accept target *FLAGS:
    #!/usr/bin/env bash
    set -euo pipefail
    root="$PWD"
    # `bazel run` starts a test in its runfiles tree, through test-setup.sh, which runs the
    # `--run_under` words as one command: a `cd … && ` prefix would reach `cd` as arguments. The
    # wrapper resolves the binary's runfiles-relative path, then runs it from the checkout.
    INSTA_UPDATE=always INSTA_WORKSPACE_ROOT="$root" bazel run {{ FLAGS }} \
        --run_under="/bin/bash -c 'b=\$PWD/\$0; cd \"$root\" && exec \"\$b\" \"\$@\"'" {{ target }}

# Rewrite the committed project files with what the editor's canonical writer produces: runs the round-trip test from the checkout with `CANONICALIZE_PROJECT_FILES=1`; review with `git diff`.
[group('bazel')]
canonicalize-projects *FLAGS:
    #!/usr/bin/env bash
    set -euo pipefail
    root="$PWD"
    # The same `--run_under` wrapper as `snap-accept`: the test then resolves the corpus against
    # the checkout, not its runfiles.
    CANONICALIZE_PROJECT_FILES=1 bazel run {{ FLAGS }} \
        --run_under="/bin/bash -c 'b=\$PWD/\$0; cd \"$root\" && exec \"\$b\" \"\$@\"'" \
        //areas/deposit/editor/core:editor_core_integration_tests/canonical_round_trip_test

# Re-resolve the crate universe after a `crate.spec` change, and refresh both locks. With a crate name, move only that crate, as for a security bump of a transitive crate. Never `bazel mod tidy`.
[group('bazel')]
repin crate="":
    #!/usr/bin/env bash
    set -euo pipefail
    # `1` re-resolves new and changed specs and leaves every other locked version where it is.
    CARGO_BAZEL_REPIN="{{ if crate == "" { "1" } else { crate } }}" bazel fetch @crates//:all
    # Again without the variable: Bazel records it in MODULE.bazel.lock, and CI's
    # `--lockfile_mode=error` rejects a lock recorded with it.
    bazel mod deps --lockfile_mode=update

# Scan the crates Bazel builds for advisories. An ignored advisory is listed here and in osv-scanner.toml, each with its reason.
[group('bazel')]
audit:
    #!/usr/bin/env bash
    set -euo pipefail
    ignores=(
        # rsa Marvin timing side channel, no fixed release. The private-key operation is the DKIM
        # signature of outgoing mail (identity-store), whose timing no attacker observes;
        # jsonwebtoken's RSA support is unused (dsp-cli decodes without verifying).
        --ignore RUSTSEC-2023-0071
        # crypto-bigint 0.5 timing; the fix needs elliptic-curve 0.14, which jsonwebtoken 10 does not
        # take. Reached only through jsonwebtoken's p256/p384; dsp-cli uses no ECDSA algorithm.
        --ignore RUSTSEC-2026-0387
        # quick-xml 0.37 reader issues, fixed in 0.41, a breaking upgrade of its own. dpe-api-oai
        # only writes XML with `quick_xml::Writer`; nothing here reads XML with quick-xml.
        --ignore RUSTSEC-2026-0194
        --ignore RUSTSEC-2026-0195
        # ruzstd decompression bomb, fixed in 0.9. Reached through pyroscope's unwinder, which parses
        # the objects mapped into the server's own process, not outside input.
        --ignore RUSTSEC-2026-0344
        # Bundled SQLite before 3.54.0 reading a database with a crafted `-wal` file. Fixed in
        # libsqlite3-sys 0.39, held back by the rusqlite 0.38 pin (MODULE.bazel). The servers open
        # only database files they create in their own data directories.
        --ignore RUSTSEC-2026-0459
    )
    # `Cargo.Bazel.lock` is what Bazel builds; `dsp-cli/Cargo.lock` is what `cargo install dsp-cli` gets.
    cargo audit --file Cargo.Bazel.lock "${ignores[@]}"
    cargo audit --file dsp-cli/Cargo.lock "${ignores[@]}"
    osv-scanner scan source --lockfile=Cargo.lock:Cargo.Bazel.lock --lockfile=Cargo.lock:dsp-cli/Cargo.lock --config=osv-scanner.toml

# The images' build flags: a release build, stamped (the editor's footer version). The targets select the musl platform themselves (`//tools/image:defs.bzl`).

IMAGE_FLAGS := "--config=release --stamp"

# Build the DPE image and load it into Docker as `local-scan/dpe:latest`. FLAGS: extra Bazel flags, such as CI's.
[group('dpe')]
build-docker-dpe *FLAGS:
    bazel run {{ FLAGS }} {{ IMAGE_FLAGS }} //areas/access/server:image_load

# Build the editor image and load it into Docker as `local-scan/editor:latest`. FLAGS: extra Bazel flags, such as CI's.
[group('editor')]
build-docker-editor *FLAGS:
    bazel run {{ FLAGS }} {{ IMAGE_FLAGS }} //areas/deposit/server:image_load

# Build the Mosaic playground image and load it into Docker as `local-scan/mosaic-playground:latest`. FLAGS: extra Bazel flags, such as CI's.
[group('mosaic')]
build-docker-mosaic-playground *FLAGS:
    bazel run {{ FLAGS }} {{ IMAGE_FLAGS }} //modules/mosaic/playground:image_load

# Build an image and push it with its `oci_push` target, such as `//areas/access/server:image_push`. ARGS: Bazel flags, then `--` and the push's own, `--tag <tag>` and `--repository <repository>` (default: the target's). From CI only: the data layers take untracked files (`app_image`).
[group('bazel')]
bazel-docker-push TARGET *ARGS:
    bazel run {{ IMAGE_FLAGS }} {{ TARGET }} {{ ARGS }}

# Build what `bazel-docker-push` runs, without pushing. CI builds first and logs in to the registry afterwards, so the build's code never runs beside the credentials. FLAGS: extra Bazel flags, such as CI's.
[group('bazel')]
bazel-docker-push-build TARGET *FLAGS:
    bazel build {{ IMAGE_FLAGS }} {{ TARGET }} {{ FLAGS }}

# Build and load the three images, start each, and expect 200 from `/healthz` (the playground: `/`) and exit 0 from the server's `healthcheck` subcommand. FLAGS: extra Bazel flags, such as CI's.
image-smoke *FLAGS:
    #!/usr/bin/env bash
    set -euo pipefail
    just build-docker-dpe {{ FLAGS }}
    just build-docker-editor {{ FLAGS }}
    just build-docker-mosaic-playground {{ FLAGS }}
    containers=()
    trap 'for c in ${containers[@]+"${containers[@]}"}; do docker rm -f "$c" >/dev/null 2>&1 || true; done' EXIT
    # usage: smoke <image> <path> <healthcheck binary, or empty> [docker run arguments]
    smoke() {
        local image=$1 path=$2 bin=$3 name="image-smoke-$$-${#containers[@]}"
        shift 3
        containers+=("$name")
        # Loopback only, on a port Docker picks. The images are amd64 only (emulated on arm64 hosts).
        docker run -d --platform linux/amd64 --name "$name" -p 127.0.0.1::8080 "$@" "$image" >/dev/null
        fail() { echo >&2 "error: $image: $1"; docker logs "$name" >&2 || true; exit 1; }
        local port code=""
        # Fails when the container has already exited.
        port="$(docker port "$name" 8080/tcp | head -n1 | sed 's/.*://')" || fail "not running"
        for _ in $(seq 60); do
            [ "$(docker inspect -f '{{{{.State.Running}}' "$name")" = true ] || fail "exited"
            code="$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$port$path" || true)"
            [ "$code" = 200 ] && break
            sleep 1
        done
        [ "$code" = 200 ] || fail "answered $path with '$code', not 200"
        # distroless `nonroot`'s user; a base digest of the root variant would leave it empty.
        user="$(docker inspect -f '{{{{.Config.User}}' "$image")"
        [ "$user" = 65532 ] || fail "runs as user '$user', not 65532"
        # distroless has no shell: the binary is executed directly, from the image's working directory.
        if [ -n "$bin" ]; then
            docker exec "$name" "./$bin" healthcheck
        fi
        echo "ok: $image"
    }
    smoke local-scan/dpe:latest /healthz access-server
    # DEV: PROD refuses to start without an SMTP relay (`run-docker-editor`).
    smoke local-scan/editor:latest /healthz deposit-server -e EDITOR_ENV=DEV
    smoke local-scan/mosaic-playground:latest / ""

# Generate rust-project.json for rust-analyzer from the Bazel graph. Re-run after adding or moving a target.
[group('bazel')]
rust-project:
    bazel run @rules_rust//tools/rust_analyzer:gen_rust_project

# Run the commit gate over `<base>..HEAD`: message rules, then the one-commit cap
commit-lint base="origin/main":
    #!/usr/bin/env bash
    # Enforces the type allowlist + mandatory scope from `.commitlintrc.yml`, then
    # the one-commit-per-PR cap. Same checks the CI `gate` job runs. See
    # `docs/src/git-conventions.md`.
    #
    # We feed one message at a time on stdin instead of commitlint's `--from/--to`
    # range mode: commitlint-rs reads stdin whenever stdin is not a TTY (always
    # true in CI and under `just`), so range mode is unreachable in automation.
    # The loop also attributes each failure to its commit, and is a no-op on an
    # empty range. `--no-merges` skips merge commits, so neither the synthetic
    # `refs/pull/N/merge` commit nor a local merge is message-checked: message
    # rules do not apply to merges, and `required_linear_history` in the ruleset
    # is what actually blocks a merge commit from landing on `main`.
    #
    # Both checks always run, so a branch with two problems reports both. The
    # count check reads PR_BODY from the environment for the `allow-many-commits`
    # override — CI passes the PR description; locally it is empty, so the cap
    # applies strictly.
    set -uo pipefail
    merge_base="$(git merge-base "{{ base }}" HEAD)"
    fail=0
    while IFS= read -r sha; do
        [ -n "$sha" ] || continue
        if ! git log -1 --pretty=%B "$sha" | commitlint; then
            echo "  ↳ offending commit: $(git log -1 --pretty='%h %s' "$sha")" >&2
            fail=1
        fi
    done < <(git rev-list --no-merges --reverse "$merge_base..HEAD")
    if [ "$fail" -eq 0 ]; then
        echo "✓ commit messages: all commits in $merge_base..HEAD OK"
    else
        echo "✗ commit messages: one or more commits violate the convention (see above)" >&2
    fi
    BASE_REF="{{ base }}" bash .github/scripts/check-commit-count.sh || fail=1
    exit "$fail"

# Clean all build artifacts
clean:
    bazel clean

[group('docs')]
docs-install-requirements:
    cargo install mdbook

# Generate mdbook-mermaid runtime assets (gitignored). Idempotent; offline.
[group('docs')]
docs-mermaid-assets:
    mdbook-mermaid install docs

[group('docs')]
docs-build: docs-mermaid-assets
    mdbook build docs

[group('docs')]
docs-serve: docs-mermaid-assets
    mdbook serve docs

[group('docs')]
docs-clean:
    mdbook clean docs

[group('docs')]
docs-test:
    mdbook test docs

###################
# Mosaic targets
###################

# Build the playground stylesheet with Bazel (`//modules/mosaic/playground:css`) and copy it to playground/public/assets/app.css (gitignored), where the dev binary serves it from. (DEV-6642)
[group('mosaic')]
css-mosaic *FLAGS:
    just _css-copy //modules/mosaic/playground:css modules/mosaic/playground/public/assets dev {{ FLAGS }}

# Dev loop: Tailwind --watch + bacon (kill_then_restart) serving the playground's dev binary. (DEV-6642)
[group('mosaic')]
watch-mosaic-playground:
    #!/usr/bin/env bash
    set -euo pipefail
    tw="$(just -q _tailwind-cli)"
    "$tw" -i modules/mosaic/playground/style/main.css -o modules/mosaic/playground/public/assets/app.css --watch &
    pid=$!
    trap 'kill $pid 2>/dev/null || true' EXIT
    bacon playground

# Run the Mosaic playground image on port 8080 (`just build-docker-mosaic-playground` first)
[group('mosaic')]
run-docker-mosaic-playground:
    docker run --rm --platform linux/amd64 -p 8080:8080 local-scan/mosaic-playground:latest

###################
# DPE targets
###################

# The Tailwind standalone CLI Bazel pins (`//tools/tailwind`, four `http_file`s with `sha256` in MODULE.bazel), fetched for this machine; echoes its path. The dev recipes' `--watch` runs it, so the watch and the build use one binary. (DEV-6642, DEV-6727)
[private]
_tailwind-cli:
    #!/usr/bin/env bash
    set -euo pipefail
    bazel build //tools/tailwind
    echo "$(bazel info output_base 2>/dev/null)/$(bazel cquery --output=files //tools/tailwind 2>/dev/null)"

# usage: _css-copy <target> <asset dir> dev|release [Bazel flags]. Builds a stylesheet target and copies its file into the asset directory the server reads in the checkout: `dev` writes `app.css`, `release` writes `app.<hash8>.css` and removes the older hashed files and `app.css`. The DPE and editor targets output a directory holding `app.<hash8>.css`, the playground's `app.css`; minified either way, so the dev and release files hold the same bytes.
[private]
_css-copy target out mode *FLAGS:
    #!/usr/bin/env bash
    set -euo pipefail
    # `.bazelrc` sets `--remote_download_minimal`: under a remote cache the output would stay remote.
    bazel build --remote_download_toplevel {{ FLAGS }} {{ target }}
    built="$(bazel info {{ FLAGS }} execution_root 2>/dev/null)/$(bazel cquery {{ FLAGS }} --output=files {{ target }} 2>/dev/null)"
    if [ -d "$built" ]; then src="$(find "$built" -maxdepth 1 -name 'app*.css' | head -n1)"; else src="$built"; fi
    [ -f "$src" ] || { echo >&2 "error: no stylesheet in $built"; exit 1; }
    mkdir -p "{{ out }}"
    if [ "{{ mode }}" = dev ]; then
        install -m 0644 "$src" "{{ out }}/app.css"
        echo "built {{ out }}/app.css"
        exit 0
    fi
    name="$(basename "$src")"
    # Write the hashed file first, then drop the stale hashed files and the unhashed one. If the
    # copy fails, the previous hashed stylesheet is still in place.
    install -m 0644 "$src" "{{ out }}/$name"
    find "{{ out }}" -maxdepth 1 -name 'app.[0-9a-f]*.css' ! -name "$name" -delete
    rm -f "{{ out }}/app.css"
    echo "built {{ out }}/$name"

# Build the DPE stylesheet with Bazel (`//areas/access/dpe:css`) → public/assets/app.css, which a dev build links. main.css is the single Tailwind entry. (DEV-6642)
[group('dpe')]
css *FLAGS:
    just _css-copy //areas/access/dpe:css areas/access/dpe/public/assets dev {{ FLAGS }}

# Build the release stylesheet → public/assets/app.<hash8>.css, which a release build finds by scanning the directory at startup. (DEV-6642)
[group('dpe')]
css-release *FLAGS:
    just _css-copy //areas/access/dpe:css areas/access/dpe/public/assets release {{ FLAGS }}

# Start the DPE with hot reload: Tailwind --watch + bacon (kill_then_restart) serving access-server.
[group('dpe')]
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    tw="$(just -q _tailwind-cli)"
    "$tw" -i areas/access/dpe/style/main.css -o areas/access/dpe/public/assets/app.css --watch &
    pid=$!
    trap 'kill $pid 2>/dev/null || true' EXIT
    bacon serve

# Score a landing page's FAIRness with F-UJI. Usage: just fair-check <url> [min_score]. Exits non-zero when the total is below min_score. (DEV-7268)
[group('dpe')]
fair-check url min_score="0":
    #!/usr/bin/env bash
    set -euo pipefail

    # F-UJI 3.5.0, pinned by digest and not by `:latest`: the score is only
    # comparable against the rows in docs/src/dpe/machine-readable-metadata.md
    # if the assessor is the same build, and 3.5.0 is the version the
    # 2026-09-15 baseline was taken with. Bumping this is a reviewed change,
    # made when F-UJI publishes a release and checked at least with every
    # change to a landing page, as for the Tailwind CLI pin in
    # docs/src/security.md. There is no refresh recipe because this never
    # runs in CI.
    #
    # Not `:latest` for a second reason: `:latest` is 4.0.0, which does not
    # start in its published image. Verify a new digest starts before pinning
    # it.
    image="ghcr.io/pangaea-data-publisher/fuji@sha256:3cde9d30bc148798a512b9e3a8a9ee6e63c4d09a6a33bb7651ac007c5824c687"

    # A scratch Docker config with no credential store. The developer's own
    # ~/.docker/config.json may set `credsStore: desktop` while the active
    # context is colima, and `docker pull` then dies on a missing
    # docker-credential-desktop. The image is public, so no credentials are
    # wanted at all.
    DOCKER_CONFIG="$(mktemp -d)"
    export DOCKER_CONFIG
    echo '{}' > "$DOCKER_CONFIG/config.json"

    name="fuji-fair-check-$$"
    # Stops the container on any exit, including a failed curl or a Ctrl-C.
    trap 'docker rm -f "$name" >/dev/null 2>&1 || true; rm -rf "$DOCKER_CONFIG"' EXIT

    # Loopback only: F-UJI's API takes a URL and fetches it, so a container
    # published on 0.0.0.0 would be an open fetcher on the developer's network.
    # --add-host is what lets the container reach a server on the host's port
    # 4000 on Linux, where host.docker.internal is not resolved for it.
    docker run -d --rm --name "$name" \
      -p 127.0.0.1:1071:1071 \
      --add-host=host.docker.internal:host-gateway \
      "$image" >/dev/null

    # Not `curl -f`: the endpoint answers 401 without credentials, and any
    # HTTP response at all means the server is up. 180s because F-UJI refreshes
    # its re3data DOI table before it starts listening, which on a first run
    # takes well over a minute.
    up=0
    for _ in $(seq 180); do
      if curl -s -o /dev/null "http://localhost:1071/fuji/api/v1/metrics"; then up=1; break; fi
      sleep 1
    done
    # The loop's own success is the verdict. Probing a second time would let a
    # single blip after a successful wait be reported as "F-UJI did not answer".
    [ "$up" = 1 ] \
      || { echo >&2 "error: F-UJI did not answer on port 1071 within 180s"; docker logs "$name" >&2; exit 1; }

    # F-UJI's own published defaults, from fuji_server/config/users.py. They
    # are not a secret: the container is reachable on loopback only and is
    # destroyed when this recipe returns.
    # 1800s, not 600s, and the reason is not slowness in general. Since the
    # landing page advertises `schema:distribution`, an assessment of a project
    # whose records carry files *downloads those files*: F-UJI collects every
    # advertised distribution, takes up to five per MIME type, fetches each one
    # and starts a Tika server per worker thread to sniff it. Runtime therefore
    # scales with how many distinct file types a project has, where it used to
    # be roughly constant per page. Project 0868 took 558s on 2026-09-18 and
    # the recipe failed at the old cap with curl's exit 28. A project with no
    # file still answers in well under a minute. Do not trim this back.
    result="$(curl -s --max-time 1800 -u marvel:wonderwoman \
      -H 'Content-Type: application/json' \
      -d "$(jq -nc --arg url '{{ url }}' '{object_identifier: $url, test_debug: true, use_datacite: true}')" \
      "http://localhost:1071/fuji/api/v1/evaluate")"

    echo "$result" | jq -e 'has("summary")' >/dev/null \
      || { echo >&2 "error: F-UJI returned no result"; echo "$result" | head -c 2000 >&2; exit 1; }

    # The table below drops `test_debug`, which is where an assessor says *why*
    # a sub-test failed and which file it read — the evidence every attribution
    # in the residuals ledger rests on. A run takes minutes now, so throwing it
    # away and re-running to get it back is the wrong trade. `.claude/tmp/` is
    # gitignored scratch, and one file per run keeps two runs comparable.
    mkdir -p .claude/tmp
    raw=".claude/tmp/fair-check-$(date -u +%Y%m%dT%H%M%SZ).json"
    printf '%s' "$result" > "$raw"

    echo "url:              {{ url }}"
    echo "software_version: $(echo "$result" | jq -r '.software_version')"
    echo "image digest:     ${image#*@}"
    echo "full result:      $raw"
    echo
    printf '%-14s %-6s %s\n' METRIC SCORE OUTCOME
    echo "$result" | jq -r '.results[] | [.metric_identifier, "\(.score.earned)/\(.score.total)", .test_status] | @tsv' \
      | awk -F'\t' '{printf "%-14s %-6s %s\n", $1, $2, $3}'
    echo

    earned="$(echo "$result" | jq -r '.summary.score_earned.FAIR')"
    total="$(echo "$result" | jq -r '.summary.score_total.FAIR')"
    echo "total: $earned/$total (target {{ min_score }})"

    # The exit code is the check, so an orchestrator or a reviewer gets a
    # mechanical pass/fail rather than a table to read.
    awk -v e="$earned" -v m="{{ min_score }}" 'BEGIN { exit !(e >= m) }' \
      || { echo >&2 "error: F-UJI scored $earned, below the target of {{ min_score }}"; exit 1; }

# Start the Grafana LGTM (Loki, Grafana, Tempo, Mimir) all-in-one container for local observability
[group('dpe')]
lgtm-up:
    docker run --rm -p 3000:3000 -p 4317:4317 -p 4318:4318 -p 4040:4040 grafana/otel-lgtm

# Start the DPE with hot reload, exporting traces/metrics/logs to a local LGTM stack (run `just lgtm-up` in another terminal first)
[group('dpe')]
dev-otel:
    #!/usr/bin/env bash
    set -euo pipefail
    tw="$(just -q _tailwind-cli)"
    "$tw" -i areas/access/dpe/style/main.css -o areas/access/dpe/public/assets/app.css --watch &
    pid=$!
    trap 'kill $pid 2>/dev/null || true' EXIT
    OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=dpe \
    OTEL_RESOURCE_ATTRIBUTES="service.namespace=dpe,service.version={{ VERSION }},deployment.environment=dev" \
    PYROSCOPE_ENDPOINT=http://localhost:4040 \
    bacon serve

# Run the DPE image on port 8080 (`just build-docker-dpe` first)
[group('dpe')]
run-docker-dpe:
    docker run --rm --platform linux/amd64 -p 8080:8080 local-scan/dpe:latest

# usage: _release-bin <target> [Bazel flags]. Builds a server for this machine with `--config=release`, the profile the images use, and echoes the binary's absolute path. Not `bazel-bin`: it follows whichever configuration was built last.
[private]
_release-bin target *FLAGS:
    #!/usr/bin/env bash
    set -euo pipefail
    # `--remote_download_toplevel`: under a remote cache `.bazelrc`'s `--remote_download_minimal`
    # would leave the binary remote.
    bazel build --config=release --remote_download_toplevel {{ FLAGS }} {{ target }} >&2
    echo "$(bazel info --config=release {{ FLAGS }} execution_root 2>/dev/null)/$(bazel cquery --config=release {{ FLAGS }} --output=files {{ target }} 2>/dev/null)"

# Run the DPE accessibility E2E tests against a release build with the release stylesheet; Playwright starts the server on port 4000. FLAGS: extra Bazel flags, such as CI's.
[group('dpe')]
test-a11y-dpe *FLAGS: _check-node
    #!/usr/bin/env bash
    set -euo pipefail
    just css-release {{ FLAGS }}
    ACCESS_SERVER_BIN="$(just -q _release-bin //areas/access/server:access_server {{ FLAGS }})"
    export ACCESS_SERVER_BIN
    cd areas/access/dpe/web-e2e-tests && npx playwright test tests/accessibility.spec.ts --project=chromium

###################
# Editor targets
###################

# Build the editor stylesheet with Bazel (`//areas/deposit/editor:css`) → public/assets/app.css, which a dev build links. main.css is the single Tailwind entry.
[group('editor')]
css-editor *FLAGS:
    just _css-copy //areas/deposit/editor:css areas/deposit/editor/public/assets dev {{ FLAGS }}

# Build the release stylesheet → public/assets/app.<hash8>.css, which a release build finds by scanning the directory at startup. Mirrors `just css-release`.
[group('editor')]
css-editor-release *FLAGS:
    just _css-copy //areas/deposit/editor:css areas/deposit/editor/public/assets release {{ FLAGS }}

# Start the editor with hot reload: Tailwind --watch + bacon (kill_then_restart) serving deposit-server. Binds 127.0.0.1:4100, so `just dev` can run alongside it.
[group('editor')]
dev-editor:
    #!/usr/bin/env bash
    set -euo pipefail
    tw="$(just -q _tailwind-cli)"
    "$tw" -i areas/deposit/editor/style/main.css -o areas/deposit/editor/public/assets/app.css --watch &
    pid=$!
    trap 'kill $pid 2>/dev/null || true' EXIT
    # The editor has no default data directory: the published set is DPE's
    # content, consumed through EDITOR_DATA_DIR. Locally that is DPE's
    # checked-out data; the image bakes a snapshot at /app/server/data.
    EDITOR_DATA_DIR={{ DPE_DATA_DIR }} \
    bacon serve-editor

# Start the editor with hot reload, exporting traces/metrics/logs to a local LGTM stack (run `just lgtm-up` in another terminal first)
[group('editor')]
dev-editor-otel:
    #!/usr/bin/env bash
    set -euo pipefail
    tw="$(just -q _tailwind-cli)"
    "$tw" -i areas/deposit/editor/style/main.css -o areas/deposit/editor/public/assets/app.css --watch &
    pid=$!
    trap 'kill $pid 2>/dev/null || true' EXIT
    EDITOR_DATA_DIR={{ DPE_DATA_DIR }} \
    OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=editor \
    OTEL_RESOURCE_ATTRIBUTES="service.namespace=editor,service.version={{ VERSION }},deployment.environment=dev" \
    PYROSCOPE_ENDPOINT=http://localhost:4040 \
    bacon serve-editor

# Run the editor E2E suite, the JavaScript-enabled and the javaScriptEnabled=false pass, against a release build with the release stylesheet; Playwright starts the server. FLAGS: extra Bazel flags, such as CI's.
[group('editor')]
test-e2e-editor *FLAGS: _check-node
    #!/usr/bin/env bash
    set -euo pipefail
    just css-editor-release {{ FLAGS }}
    DEPOSIT_SERVER_BIN="$(just -q _release-bin //areas/deposit/server:deposit_server {{ FLAGS }})"
    export DEPOSIT_SERVER_BIN
    cd areas/deposit/editor/web-e2e-tests && npx playwright test

# Run the editor accessibility E2E tests (WCAG 2.1 AA via axe-core), built as for `test-e2e-editor`. FLAGS: extra Bazel flags, such as CI's.
[group('editor')]
test-a11y-editor *FLAGS: _check-node
    #!/usr/bin/env bash
    set -euo pipefail
    just css-editor-release {{ FLAGS }}
    DEPOSIT_SERVER_BIN="$(just -q _release-bin //areas/deposit/server:deposit_server {{ FLAGS }})"
    export DEPOSIT_SERVER_BIN
    cd areas/deposit/editor/web-e2e-tests && npx playwright test tests/accessibility.spec.ts --project=chromium-js

# Run the editor image on port 8080 (`just build-docker-editor` first)
[group('editor')]
run-docker-editor:
    # DEV overrides the image's PROD default: startup refuses PROD without an
    # SMTP relay, because that combination writes every login code to the log
    # while the service looks healthy. Locally the console transport is the point.
    docker run --rm --platform linux/amd64 -p 8080:8080 -e EDITOR_ENV=DEV local-scan/editor:latest

# Lint E2E test TypeScript with Biome
lint-e2e: _check-node
    cd areas/access/dpe/web-e2e-tests && npx @biomejs/biome check .
    cd modules/mosaic/playground-e2e-tests && npx @biomejs/biome check .
    cd areas/deposit/editor/web-e2e-tests && npx @biomejs/biome check .

###################
# dsp-cli targets
###################

# Run the CLI with the given arguments (debug build).
[group('dsp-cli')]
dsp-cli-run *args:
    bazel run --run_in_cwd //dsp-cli:dsp -- {{ args }}

# Needs a reachable DSP stack and the environment variables the live tests read
# (DSP_TEST_SERVER, DSP_TEST_USER, DSP_TEST_PASSWORD, DSP_TEST_PROJECT,
# DSP_TEST_CLASS_IRI, DSP_TEST_VOCAB_PROJECT, DSP_TEST_NON_ADMIN_TOKEN, DSP_TOKEN, and optionally
# DSP_TEST_ORDER_BY_IRI). DSP_LIVE_STRICT=1 makes a missing required variable
# panic instead of the test quietly skipping. `live_update_check` is excluded:
# it calls out to crates.io to check for a newer dsp-cli release, not a DSP
# stack, so it does not belong in a run gated on stack reachability.

# Run the dsp-cli live tests (layer 5, dsp-cli/ADR-0009) with Bazel. Each variable is passed by name, because Bazel's strict test environment drops what is not named; the targets set `DSP_LIVE_STRICT=1` themselves. FLAGS: extra Bazel flags, such as CI's.
[group('dsp-cli')]
dsp-cli-test-live *FLAGS:
    bazel test {{ FLAGS }} \
        --test_env=DSP_TEST_SERVER --test_env=DSP_TEST_USER --test_env=DSP_TEST_PASSWORD \
        --test_env=DSP_TEST_PROJECT --test_env=DSP_TEST_CLASS_IRI --test_env=DSP_TEST_VOCAB_PROJECT \
        --test_env=DSP_TEST_NON_ADMIN_TOKEN --test_env=DSP_TEST_ORDER_BY_IRI --test_env=DSP_TOKEN \
        $(bazel query 'filter("^//dsp-cli:live_[a-z_]+_test$", //dsp-cli:all) except //dsp-cli:live_update_check_test')

# Verify Docker is on PATH and its daemon answers. just runs recipes in sh, which does NOT see shell-function version managers — only real binaries on PATH.
[private]
_check-docker:
    @command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1 || { echo >&2 "error: 'docker' not on PATH or its daemon is not running. Start Docker (or Colima/OrbStack) and make sure 'docker' is on PATH for all shells."; exit 1; }

# Start the dsp-cli test stack (Fuseki + knora-api) and load fixtures.
[group('dsp-cli')]
dsp-cli-stack-up: _check-docker
    docker compose --env-file dsp-cli/ci/stack/stack.env -f dsp-cli/ci/stack/docker-compose.yml up -d --wait db
    bash dsp-cli/ci/stack/load-fixtures.sh
    docker compose --env-file dsp-cli/ci/stack/stack.env -f dsp-cli/ci/stack/docker-compose.yml up -d --wait api
    bash dsp-cli/ci/stack/wait-for-api.sh

# Tear down the dsp-cli test stack and remove its volumes.
[group('dsp-cli')]
dsp-cli-stack-down: _check-docker
    docker compose --env-file dsp-cli/ci/stack/stack.env -f dsp-cli/ci/stack/docker-compose.yml down -v

# Load fixtures into an already-running dsp-cli test stack.
[group('dsp-cli')]
dsp-cli-stack-fixtures: _check-docker
    bash dsp-cli/ci/stack/load-fixtures.sh
