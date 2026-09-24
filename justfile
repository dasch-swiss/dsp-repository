DOCKER_REPO := "daschswiss/dsp-repository"
CARGO_VERSION := `cargo metadata --format-version=1 --no-deps | jq --raw-output '.packages[].version'`
COMMIT_HASH := `git log --pretty=format:'%h' -n 1`
GIT_TAG := `git describe --tags --exact-match 2>/dev/null || true`
IMAGE_TAG := if GIT_TAG == "" { CARGO_VERSION + "-" + COMMIT_HASH } else { CARGO_VERSION }
DOCKER_IMAGE := DOCKER_REPO + ":" + IMAGE_TAG

# DPE's published data set, which the editor recipes below read from and package.

DPE_DATA_DIR := "modules/dpe/server/data"

# Projects whose OAI records `just fetch-records` refreshes. Add a shortcode here to track a new project.

RECORD_SHORTCODES := "081C 0868 0803"

# List all recipes
default:
    just --list --unsorted

# Install all requirements
install-requirements: install-e2e-requirements
    #!/usr/bin/env sh
    rustup show
    # Nightly rustfmt: .rustfmt.toml sets nightly-only options and `just fmt` runs `cargo +nightly fmt`.
    # rust-toolchain.toml pins stable, so `rustup show` alone never provisions it.
    rustup toolchain install nightly --component rustfmt
    brew install cargo-binstall
    # Native build deps the Nix devShell provides via flake.nix buildInputs; without them this path
    # has no cmake for aws-lc-sys. `jq` is what `just fair-check` reads F-UJI's JSON result with.
    brew install cmake pkg-config jq
    # commitlint-rs powers the commit-message gate in `just commit-lint`
    cargo binstall -y commitlint-rs@0.2.4
    cargo binstall -y cargo-watch@8.5.3
    cargo binstall -y mdbook@0.4.52
    cargo binstall -y mdbook-alerts@0.8.0
    cargo binstall -y mdbook-mermaid@0.16.2
    cargo binstall -y bacon@3.23.0
    cargo binstall -y maudfmt@0.1.8
    cargo binstall -y cargo-machete@0.9.2
    cargo binstall -y cargo-nextest@0.9.144
    cargo binstall -y cargo-deny@0.20.2

# Install Playwright browsers for E2E tests
install-e2e-requirements: _check-node
    # `npm ci` first: the suites run via `npx playwright test`, which resolves the runner from
    # node_modules. With none, npx re-resolves the `^1.44.1` range on every run and can land on a
    # version that mismatches the installed browsers ("Executable doesn't exist"). Both packages
    # track a package-lock.json, so `ci` pins the runner to the same version the browsers match.
    cd modules/mosaic/playground-e2e-tests && npm ci && npx playwright install
    cd modules/dpe/web-e2e-tests && npm ci && npx playwright install
    cd areas/deposit/editor/web-e2e-tests && npm ci && npx playwright install

# Verify Node is on PATH. just runs recipes in sh, which does NOT see shell-function version managers (e.g. lazy nvm) — only real binaries on PATH. (DEV-6642)
[private]
_check-node:
    @command -v node >/dev/null 2>&1 || { echo >&2 "error: 'node' not on PATH. just runs recipes in sh, which can't see nvm's lazy shell functions — expose your default node bin on PATH for all shells (eager-load it in your shell rc, or use brew/volta/asdf). See docs/src/fundamentals/onboarding.md."; exit 1; }

# Each modules/*/public/vendor/README.md table against the files it describes,
# plus tailwind.pins for completeness.

# Verify the third-party bytes we ship or execute. Run by `just check`. (DEV-7126, DEV-6727)
verify-checksums:
    bash .github/scripts/verify-checksums.sh

# A shared crate must not know one service's directory layout. It compiles
# either way, so only a grep catches it.

# Verify no shared crate hardcodes a path into a service module. Run by `just check`. (DEV-7046)
check-shared-paths:
    bash .github/scripts/check-shared-paths.sh

# `data-on-`, `data-attr-`, `data-class-`, `data-style-`: the attribute renders
# fine and the control is inert, so only a grep or a browser catches it.

# Verify no Maud template uses Datastar's pre-RC.6 hyphen delimiter. Run by `just check`. (DEV-6920)
check-datastar-delimiters:
    bash .github/scripts/check-datastar-delimiters.sh

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

# Run all fmt and clippy checks
check: verify-checksums check-shared-paths check-datastar-delimiters check-adr-refs check-live-tests-ignored
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
    cargo +nightly fmt --check --all
    cargo clippy --all-features -- -D warnings
    # Fail on declared-but-unconsumed dependencies (a dep added "for later wiring" that never
    # got wired survives on the strength of a comment otherwise).
    cargo machete

# Format all code: maudfmt for the `html!` Maud macros, then cargo +nightly fmt for the rest.
fmt:
    #!/usr/bin/env bash
    set -euo pipefail
    git ls-files -z '*.rs' | xargs -0 maudfmt --
    cargo +nightly fmt --all

# Fix justfile formatting. Warning: will change existing file. Please first use check.
fix:
    just --fmt --unstable

# Run all rust builds
build:
    cargo build --all-targets

# Run server
run:
    cargo run --bin dpe-server --release -- serve

# Validate all data files in the default data directory
validate-data:
    cargo run --bin dpe-server -- validate modules/dpe/server/data

# Re-download and re-patch the DataCite JSON schema the DataCite JSON writer's output is shape-checked against. The script takes the XSD directory as an argument because a shared crate may not know a module's layout; this justfile may, and passes it. (DEV-7268)
refresh-datacite-schema:
    bash shared/fair/testdata/schemas/download-schemas.sh \
        modules/dpe/api-oai/src/handlers/testdata/schemas/include

# Refresh the tracked OAI record dumps. Needs `bearer` in the environment (see README).
[group('dpe')]
fetch-records:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${bearer:?export bearer=\"Bearer eyJ...\"}"
    for sc in {{ RECORD_SHORTCODES }}; do
        out=modules/dpe/server/data/records/$sc-records.json
        curl -fsS -H "Authorization: $bearer" 'https://api.dasch.swiss/v3/export/resources/oai' \
            -d '{"shortcode": "'"$sc"'"}' -o "$out" -w "$out %{http_code}\n" >&2
    done

# Run all tests
test:
    cargo test --tests
    # The dev-only live-reload code is feature-gated, so its tests need the feature enabled.
    cargo test -p dpe-server -p mosaic-playground --features dpe-server/dev,mosaic-playground/dev --tests
    bash .github/scripts/check-commit-count.test.sh
    # Commit-advisory helpers (deterministic parts only; needs jq)
    bash .github/scripts/commit-advisory.test.sh
    bash .github/scripts/verify-checksums.test.sh
    bash .github/scripts/check-shared-paths.test.sh
    bash .github/scripts/check-datastar-delimiters.test.sh
    bash .github/scripts/check-adr-refs.test.sh
    bash .github/scripts/check-live-tests-ignored.test.sh

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
    cargo clean

# Build linux/amd64 Docker image locally
docker-build-amd64:
    docker buildx build --platform linux/amd64 -t {{ DOCKER_IMAGE }}-amd64 --load .

# Push previously build linux/amd64 image to Docker hub
docker-push-amd64:
    docker push {{ DOCKER_IMAGE }}-amd64

# Build linux/arm64 Docker image locally
docker-build-arm64:
    docker buildx build --platform linux/arm64 -t {{ DOCKER_IMAGE }}-arm64 --load .

# Push previously build linux/arm64 image to Docker hub
docker-push-arm64:
    docker push {{ DOCKER_IMAGE }}-arm64

# Publish Docker manifest combining aarch64 and x86 published images
docker-publish-manifest:
    docker manifest create {{ DOCKER_IMAGE }} --amend {{ DOCKER_IMAGE }}-amd64 --amend {{ DOCKER_IMAGE }}-arm64
    docker manifest annotate --arch amd64 --os linux {{ DOCKER_IMAGE }} {{ DOCKER_IMAGE }}-amd64
    docker manifest annotate --arch arm64 --os linux {{ DOCKER_IMAGE }} {{ DOCKER_IMAGE }}-arm64
    docker manifest inspect {{ DOCKER_IMAGE }}
    docker manifest push {{ DOCKER_IMAGE }}

# Output the BUILD_TAG
docker-image-tag:
    @echo {{ IMAGE_TAG }}

# Watch for changes and run tests
watch:
    cargo watch -x test

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

# Build the playground stylesheet → playground/public/assets/app.css (gitignored). Standalone Tailwind CLI off the canonical tokens.css, same mechanism as `just css`. (DEV-6642)
[group('mosaic')]
css-mosaic:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i modules/mosaic/playground/style/main.css -o modules/mosaic/playground/public/assets/app.css --minify

# Dev loop: Tailwind --watch + cargo-watch rebuilding the plain Axum playground binary. (DEV-6642)
[group('mosaic')]
watch-mosaic-playground:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i modules/mosaic/playground/style/main.css -o modules/mosaic/playground/public/assets/app.css --watch &
    tw=$!
    trap 'kill $tw 2>/dev/null || true' EXIT
    MOSAIC_PUBLIC_DIR=modules/mosaic/playground/public cargo watch -x 'run -p mosaic-playground --features dev'

# Build Docker image for mosaic playground
[group('mosaic')]
build-docker-mosaic-playground:
    docker build -f modules/mosaic/playground/Dockerfile -t mosaic-playground .

# Run mosaic playground Docker container on port 8080
[group('mosaic')]
run-docker-mosaic-playground:
    docker run --rm -p 8080:8080 mosaic-playground

###################
# DPE targets
###################

# Rewrite tailwind.pins for a Tailwind release, from the sha256sums.txt published alongside it. (DEV-6727)
tailwind-pins-refresh version:
    #!/usr/bin/env bash
    set -euo pipefail
    # That file is read once, here, and never at build time, where an attacker
    # able to swap a release asset could swap its checksum file too. Pinning buys
    # a fixed reference point and an audit trail, not authenticity: see
    # docs/src/security.md.
    . .github/scripts/verify-checksums.sh
    ver="{{ version }}"
    sums="$(curl -fsSL --connect-timeout 10 --max-time 60 \
        "https://github.com/tailwindlabs/tailwindcss/releases/download/v$ver/sha256sums.txt")"
    tmp="$(mktemp "${TMPDIR:-/tmp}/tailwind-pins.XXXXXX")"
    # Keep everything above the version line; rewrite from there down.
    awk '$1 == "version" { exit } { print }' tailwind.pins >"$tmp"
    printf 'version %s\n\n' "$ver" >>"$tmp"
    for asset in $TAILWIND_ASSETS; do
        pin="$(printf '%s\n' "$sums" | awk -v a="$asset" '$2 == a || $2 == "./" a { print $1; exit }')"
        [ -n "$pin" ] || { echo "✗ $asset is not listed in v$ver's sha256sums.txt" >&2; exit 1; }
        printf '%s  %s\n' "$pin" "$asset" >>"$tmp"
    done
    mv "$tmp" tailwind.pins
    TAILWIND_PINS=tailwind.pins verify_tailwind_pins
    git --no-pager diff -- tailwind.pins

# The version and the SHA-256 of every release asset both come from tailwind.pins,
# which modules/mosaic/playground/Dockerfile reads too, so neither file can drift
# onto a version the other has not seen; bump with `just tailwind-pins-refresh
# <version>`. The binary is verified before it is handed to a caller.

# Resolve the pinned Tailwind v4 standalone CLI (download + cache under target/, gitignored); echoes its path. Bundles plugins incl. typography, so no Node/npm is needed. (DEV-6642, DEV-6727)
[private]
_tailwind-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    . .github/scripts/verify-checksums.sh
    ver="$(tailwind_version)"
    case "$(uname -s)" in Darwin) os=macos ;; Linux) os=linux ;; *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;; esac
    case "$(uname -m)" in arm64|aarch64) arch=arm64 ;; x86_64) arch=x64 ;; *) echo "unsupported arch: $(uname -m)" >&2; exit 1 ;; esac
    asset="tailwindcss-$os-$arch"
    bin="target/tailwind/tailwindcss-$ver-$os-$arch"
    want="$(tailwind_pin "$asset")"
    if [ ! -x "$bin" ]; then
        mkdir -p target/tailwind
        url="https://github.com/tailwindlabs/tailwindcss/releases/download/v$ver/$asset"
        echo "fetching Tailwind standalone CLI: $url" >&2
        curl -fsSL --connect-timeout 10 --max-time 60 --retry 3 -o "$bin" "$url"
        chmod +x "$bin"
    fi
    # Checked on every resolve, not only after a download: a cache left by an
    # earlier build, or a truncated one, is precisely what must not be executed.
    if ! verify_file "$bin" "$want"; then
        rm -f "$bin"
        echo "removed the unverified binary. If this Tailwind bump is intentional, run 'just tailwind-pins-refresh $ver'" >&2
        exit 1
    fi
    echo "$bin"

# Build the unified DPE stylesheet → public/assets/app.css (dev, unhashed). main.css is the single Tailwind entry. (DEV-6642)
[group('dpe')]
css:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i modules/dpe/style/main.css -o modules/dpe/public/assets/app.css --minify

# Build the release stylesheet with a content-hashed filename (app.<hash>.css). (DEV-6642)
[group('dpe')]
css-release:
    #!/usr/bin/env bash
    set -euo pipefail
    # The server discovers the name by scanning the asset dir at startup, so there
    # is no build.rs and no tracked-source edit: `git diff --exit-code` stays clean.
    bin="$(just -q _tailwind-bin)"
    out=modules/dpe/public/assets
    "$bin" -i modules/dpe/style/main.css -o "$out/app.css" --minify
    if command -v sha256sum >/dev/null 2>&1; then h=$(sha256sum "$out/app.css" | cut -c1-8); else h=$(shasum -a 256 "$out/app.css" | cut -c1-8); fi
    # Write the hashed file first, then drop the stale hashed files + the unhashed
    # temp. If the copy fails, the previous hashed CSS is still in place.
    cp "$out/app.css" "$out/app.$h.css"
    find "$out" -maxdepth 1 -name 'app.[0-9a-f]*.css' ! -name "app.$h.css" -delete
    rm -f "$out/app.css"
    echo "built $out/app.$h.css"

# Start the DPE with hot reload: Tailwind --watch + bacon (kill_then_restart) serving dpe-server.
[group('dpe')]
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i modules/dpe/style/main.css -o modules/dpe/public/assets/app.css --watch &
    tw=$!
    trap 'kill $tw 2>/dev/null || true' EXIT
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
    # docs/src/security.md. There is no `tailwind-pins-refresh`-style recipe
    # because this never runs in CI.
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
    bin="$(just -q _tailwind-bin)"
    "$bin" -i modules/dpe/style/main.css -o modules/dpe/public/assets/app.css --watch &
    tw=$!
    trap 'kill $tw 2>/dev/null || true' EXIT
    OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=dpe \
    OTEL_RESOURCE_ATTRIBUTES="service.namespace=dpe,service.version={{ CARGO_VERSION }},deployment.environment=dev" \
    PYROSCOPE_ENDPOINT=http://localhost:4040 \
    bacon serve

# Build Docker image for DPE
[group('dpe')]
build-docker-dpe:
    docker build -f modules/dpe/Dockerfile -t dpe .

# Run DPE Docker container on port 8080
[group('dpe')]
run-docker-dpe:
    docker run --rm -p 8080:8080 dpe

# Run accessibility E2E tests for the DPE (requires running server on port 4000)
[group('dpe')]
test-a11y-dpe: _check-node
    cd modules/dpe/web-e2e-tests && npx playwright test tests/accessibility.spec.ts --project=chromium

###################
# Editor targets
###################

# Build the unified editor stylesheet → public/assets/app.css (dev, unhashed). main.css is the single Tailwind entry.
[group('editor')]
css-editor:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i areas/deposit/editor/style/main.css -o areas/deposit/editor/public/assets/app.css --minify

# Build the release stylesheet with a content-hashed filename (app.<hash>.css); the server discovers it by scanning the asset dir at startup. Mirrors `just css-release`.
[group('editor')]
css-editor-release:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    out=areas/deposit/editor/public/assets
    "$bin" -i areas/deposit/editor/style/main.css -o "$out/app.css" --minify
    if command -v sha256sum >/dev/null 2>&1; then h=$(sha256sum "$out/app.css" | cut -c1-8); else h=$(shasum -a 256 "$out/app.css" | cut -c1-8); fi
    # Write the hashed file first, then drop the stale hashed files + the unhashed
    # temp. If the copy fails, the previous hashed CSS is still in place.
    cp "$out/app.css" "$out/app.$h.css"
    find "$out" -maxdepth 1 -name 'app.[0-9a-f]*.css' ! -name "app.$h.css" -delete
    rm -f "$out/app.css"
    echo "built $out/app.$h.css"

# Start the editor with hot reload: Tailwind --watch + bacon (kill_then_restart) serving editor-server. Binds 127.0.0.1:4100, so `just dev` can run alongside it.
[group('editor')]
dev-editor:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i areas/deposit/editor/style/main.css -o areas/deposit/editor/public/assets/app.css --watch &
    tw=$!
    trap 'kill $tw 2>/dev/null || true' EXIT
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
    bin="$(just -q _tailwind-bin)"
    "$bin" -i areas/deposit/editor/style/main.css -o areas/deposit/editor/public/assets/app.css --watch &
    tw=$!
    trap 'kill $tw 2>/dev/null || true' EXIT
    EDITOR_DATA_DIR={{ DPE_DATA_DIR }} \
    OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=editor \
    OTEL_RESOURCE_ATTRIBUTES="service.namespace=editor,service.version={{ CARGO_VERSION }},deployment.environment=dev" \
    PYROSCOPE_ENDPOINT=http://localhost:4040 \
    bacon serve-editor

# Needs `cargo build -p editor-server --release` and `just css-editor-release`
# first; Playwright starts the binary itself.

# Run the editor E2E suite: the JavaScript-enabled and the javaScriptEnabled=false pass.
[group('editor')]
test-e2e-editor: _check-node
    cd areas/deposit/editor/web-e2e-tests && npx playwright test

# Run the editor accessibility E2E tests (WCAG 2.1 AA via axe-core). Same build prerequisites as `test-e2e-editor`.
[group('editor')]
test-a11y-editor: _check-node
    cd areas/deposit/editor/web-e2e-tests && npx playwright test tests/accessibility.spec.ts --project=chromium-js

# Build the editor Docker image locally; `arch=x86_64` reproduces the one CI publishes.
[group('editor')]
build-docker-editor arch="": css-editor-release
    #!/usr/bin/env bash
    set -euo pipefail
    # The image runs a *static musl* binary. A host `cargo build --target
    # *-unknown-linux-musl` needs a musl cross-linker, which macOS does not have —
    # so the compile happens in the same container image CI's runner provides,
    # installing musl-tools + clang exactly as `.github/actions/build-editor` does.
    arch="{{ arch }}"
    if [ -z "$arch" ]; then
        # Default to the host arch: cross-building amd64 on arm64 works but is
        # emulated, and this release profile (lto, codegen-units=1) is slow enough
        # already. CI publishes amd64; `arch=x86_64` reproduces it.
        case "$(uname -m)" in
            arm64 | aarch64) arch=aarch64 ;;
            x86_64) arch=x86_64 ;;
            *) echo "unsupported host arch $(uname -m) — pass arch=aarch64 or arch=x86_64" >&2; exit 1 ;;
        esac
    fi
    case "$arch" in
        aarch64) platform=linux/arm64 ;;
        x86_64) platform=linux/amd64 ;;
        *) echo "arch must be aarch64 or x86_64, got '$arch'" >&2; exit 1 ;;
    esac
    target="$arch-unknown-linux-musl"
    # Track rust-toolchain.toml rather than hardcoding, so a channel bump cannot
    # leave this recipe building with a different compiler than everything else.
    rust_version="$(sed -n 's/^channel = "\(.*\)"/\1/p' rust-toolchain.toml)"
    stage=target/editor-staging
    # Container artifacts live outside target/{debug,release} so they never mix
    # with host builds of the same profile.
    docker run --rm --platform "$platform" \
        -v "$PWD":/work -w /work \
        -e CARGO_TARGET_DIR=/work/target/container \
        -e CARGO_HOME=/work/target/container-cargo-home \
        "rust:$rust_version-bookworm" \
        bash -eu -c "
            apt-get update -qq
            apt-get install -y -qq --no-install-recommends musl-tools clang >/dev/null
            rustup target add $target
            cargo build -p editor-server --release --target $target
        "
    rm -rf "$stage"
    mkdir -p "$stage"
    cp "target/container/$target/release/editor-server" "$stage/"
    cp -r areas/deposit/editor/public "$stage/"
    # DPE's published set, copied in deliberately: git stays the source of truth
    # and the editor reads an image-baked snapshot via EDITOR_DATA_DIR, so a data
    # change reaches the editor by rebuilding the image, not at runtime.
    cp -r {{ DPE_DATA_DIR }} "$stage/"
    cp areas/deposit/editor/Dockerfile "$stage/"
    docker build --platform "$platform" -f "$stage/Dockerfile" -t metadata-editor "$stage"

# Run the editor Docker container on port 8080
[group('editor')]
run-docker-editor:
    # DEV overrides the image's PROD default: startup refuses PROD without an
    # SMTP relay, because that combination writes every login code to the log
    # while the service looks healthy. Locally the console transport is the point.
    docker run --rm -p 8080:8080 -e EDITOR_ENV=DEV metadata-editor

# Lint E2E test TypeScript with Biome
lint-e2e: _check-node
    cd modules/dpe/web-e2e-tests && npx @biomejs/biome check .
    cd modules/mosaic/playground-e2e-tests && npx @biomejs/biome check .
    cd areas/deposit/editor/web-e2e-tests && npx @biomejs/biome check .

###################
# dsp-cli targets
###################

# Run the CLI with the given arguments (debug build).
[group('dsp-cli')]
dsp-cli-run *args:
    cargo run -p dsp-cli --bin dsp -- {{ args }}

# Needs a reachable DSP stack and the environment variables the live tests read
# (DSP_TEST_SERVER, DSP_TEST_USER, DSP_TEST_PASSWORD, DSP_TEST_PROJECT,
# DSP_TEST_CLASS_IRI, DSP_TEST_VOCAB_PROJECT, DSP_TEST_NON_ADMIN_TOKEN, DSP_TOKEN, and optionally
# DSP_TEST_ORDER_BY_IRI). DSP_LIVE_STRICT=1 makes a missing required variable
# panic instead of the test quietly skipping. `live_update_check` is excluded:
# it calls out to crates.io to check for a newer dsp-cli release, not a DSP
# stack, so it does not belong in a run gated on stack reachability.

# Run the dsp-cli live tests (layer 5, dsp-cli/ADR-0009).
[group('dsp-cli')]
dsp-cli-test-live:
    DSP_LIVE_STRICT=1 cargo nextest run -p dsp-cli --features live --run-ignored only -E 'binary(/^live_/) & !binary(live_update_check)'

# Review pending insta snapshots for dsp-cli.
[group('dsp-cli')]
dsp-cli-snap-review:
    cargo insta review -p dsp-cli

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
