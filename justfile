DOCKER_REPO := "daschswiss/dsp-repository"
CARGO_VERSION := `cargo metadata --format-version=1 --no-deps | jq --raw-output '.packages[].version'`
COMMIT_HASH := `git log --pretty=format:'%h' -n 1`
GIT_TAG := `git describe --tags --exact-match 2>/dev/null || true`
IMAGE_TAG := if GIT_TAG == "" { CARGO_VERSION + "-" + COMMIT_HASH } else { CARGO_VERSION }
DOCKER_IMAGE := DOCKER_REPO + ":" + IMAGE_TAG

# Pinned Tailwind v4 standalone CLI (DEV-6642) — bundles the official plugins (incl. typography), so the CSS build needs no Node/npm.

TAILWIND_VERSION := "4.1.18"

# List all recipes
default:
    just --list --unsorted

# Install all requirements
install-requirements: install-e2e-requirements
    #!/usr/bin/env sh
    rustup show
    brew install cargo-binstall
    # cocogitto (cog) powers the conventional-commit check in `just check-commits`
    cargo binstall -y cocogitto
    cargo binstall -y cargo-watch@8.5.3
    cargo binstall -y mdbook@0.4.52
    cargo binstall -y mdbook-alerts@0.8.0
    cargo binstall -y mdbook-mermaid@0.16.2
    cargo binstall -y bacon@3.23.0
    cargo binstall -y maudfmt@0.1.8
    cargo binstall -y cargo-machete@0.9.2

# Install Playwright browsers for E2E tests
install-e2e-requirements: _check-node
    cd modules/mosaic/playground-e2e-tests && npx playwright install
    cd modules/dpe/web-e2e-tests && npx playwright install

# Verify Node is on PATH. just runs recipes in sh, which does NOT see shell-function version managers (e.g. lazy nvm) — only real binaries on PATH. (DEV-6642)
[private]
_check-node:
    @command -v node >/dev/null 2>&1 || { echo >&2 "error: 'node' not on PATH. just runs recipes in sh, which can't see nvm's lazy shell functions — expose your default node bin on PATH for all shells (eager-load it in your shell rc, or use brew/volta/asdf). See docs/src/fundamentals/onboarding.md."; exit 1; }

# Run all fmt and clippy checks
check:
    #!/usr/bin/env bash
    set -euo pipefail
    just --check --fmt --unstable
    # maudfmt 0.1.8 has no --check mode, so verify it is a no-op on the `html!` macros by
    # formatting a throwaway copy of each tracked .rs file and diffing (non-mutating). maudfmt
    # leaves files without `html!` byte-identical, so iterating all .rs is safe.
    tmp="$(mktemp)"
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

# Run all tests
test:
    cargo test --tests
    # The dev-only live-reload code is feature-gated, so its tests need the feature enabled.
    cargo test -p dpe-server -p mosaic-playground --features dpe-server/dev,mosaic-playground/dev --tests
    # Commit-hygiene Layer 0 checker (dependency-free; exercises cog too if installed)
    bash .github/scripts/check-commit-hygiene.test.sh
    # Commit-advisory helpers (deterministic parts only; needs jq)
    bash .github/scripts/commit-advisory.test.sh

# Check commit hygiene of the current branch against `base` (Layer 0 gate, run locally)
check-commits base="origin/main":
    #!/usr/bin/env bash
    set -euo pipefail
    BASE_REF="{{ base }}" bash .github/scripts/check-commit-hygiene.sh

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

# Resolve the pinned Tailwind v4 standalone CLI (download + cache under target/, gitignored); echoes its path. Bundles plugins incl. typography → no Node/npm needed. (DEV-6642)
[private]
_tailwind-bin:
    #!/usr/bin/env bash
    set -euo pipefail
    ver="{{ TAILWIND_VERSION }}"
    case "$(uname -s)" in Darwin) os=macos ;; Linux) os=linux ;; *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;; esac
    case "$(uname -m)" in arm64|aarch64) arch=arm64 ;; x86_64) arch=x64 ;; *) echo "unsupported arch: $(uname -m)" >&2; exit 1 ;; esac
    bin="target/tailwind/tailwindcss-$ver-$os-$arch"
    if [ ! -x "$bin" ]; then
        mkdir -p target/tailwind
        url="https://github.com/tailwindlabs/tailwindcss/releases/download/v$ver/tailwindcss-$os-$arch"
        echo "fetching Tailwind standalone CLI: $url" >&2
        curl -fsSL -o "$bin" "$url"
        chmod +x "$bin"
    fi
    echo "$bin"

# Build the unified DPE stylesheet → public/assets/app.css (dev, unhashed). main.css is the single Tailwind entry. (DEV-6642)
[group('dpe')]
css:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i modules/dpe/style/main.css -o modules/dpe/public/assets/app.css --minify

# Build the release stylesheet with a content-hashed filename (app.<hash>.css); the server discovers it by scanning the asset dir at startup. No build.rs / tracked-source edit, so `git diff --exit-code` stays clean. (DEV-6642)
[group('dpe')]
css-release:
    #!/usr/bin/env bash
    set -euo pipefail
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

# Lint E2E test TypeScript with Biome
lint-e2e: _check-node
    cd modules/dpe/web-e2e-tests && npx @biomejs/biome check .
    cd modules/mosaic/playground-e2e-tests && npx @biomejs/biome check .
