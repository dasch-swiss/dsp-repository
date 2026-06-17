DOCKER_REPO := "daschswiss/dsp-repository"
CARGO_VERSION := `cargo metadata --format-version=1 --no-deps | jq --raw-output '.packages[].version'`
COMMIT_HASH := `git log --pretty=format:'%h' -n 1`
GIT_TAG := `git describe --tags --exact-match 2>/dev/null || true`
IMAGE_TAG := if GIT_TAG == "" { CARGO_VERSION + "-" + COMMIT_HASH } else { CARGO_VERSION }
DOCKER_IMAGE := DOCKER_REPO + ":" + IMAGE_TAG

# Pinned Tailwind v4 standalone CLI (DEV-6642) — bundles the official plugins (incl. typography), so the CSS build needs no Node/npm.

TAILWIND_VERSION := "4.1.18"

# Pre-migration HTML oracle (DEV-6642): base-commit server in a separate worktree for structural curl/diff against the migrated server. Override with ORACLE_DIR / ORACLE_PORT.

ORACLE_DIR := "../dpe-oracle"
ORACLE_PORT := "4100"

# List all recipes
default:
    just --list --unsorted

# Install all requirements
install-requirements: install-e2e-requirements
    #!/usr/bin/env sh
    rustup show
    brew install cargo-binstall
    cargo binstall -y cargo-watch@8.5.3
    cargo binstall -y mdbook@0.4.52
    cargo binstall -y mdbook-alerts@0.8.0
    cargo binstall -y mdbook-mermaid@0.16.2
    cargo binstall -y leptosfmt@0.1.33
    cargo binstall -y cargo-leptos@0.3.4
    cargo binstall -y bacon@3.23.0
    cargo binstall -y maudfmt@0.1.8
    cd modules/dpe && pnpm install

# Install Playwright browsers for E2E tests
install-e2e-requirements:
    cd modules/mosaic/playground-e2e-tests && npx playwright install
    cd modules/dpe/web-e2e-tests && npx playwright install

# Run all fmt and clippy checks
check:
    just --check --fmt --unstable
    cargo +nightly fmt --check --all
    leptosfmt --check . -x target -x .direnv
    cargo clippy -- -D warnings
    # cargo-leptos compiles dpe-web for wasm32 (lib-package). Catch wasm-only
    # build breaks here so they don't only surface in publish/cloud-run jobs.
    cargo check --target wasm32-unknown-unknown -p dpe-web --no-default-features

# Format all rust code (cargo fmt for non-leptos crates, leptosfmt for leptos crates)
fmt:
    cargo +nightly fmt --all
    leptosfmt .

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

# Start the mosaic playground and watch mosaic tiles
[group('mosaic')]
watch-mosaic-playground:
    #!/usr/bin/env sh
    cd modules/mosaic/playground
    cargo leptos watch

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

# Verify daisyui is installed in modules/dpe/node_modules — Tailwind needs it at build time
[private]
_check-dpe-node-modules:
    @test -d modules/dpe/node_modules/daisyui || { echo >&2 "error: modules/dpe/node_modules/daisyui not found — run 'just install-requirements' or 'pnpm -C modules/dpe install'"; exit 1; }

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

# Build the unified DPE stylesheet → public/assets/app.css (dev, unhashed). main.css is the single entry; cargo-leptos also reads it until the Phase 2 cutover. (DEV-6642)
[group('dpe')]
css:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i modules/dpe/style/main.css -o modules/dpe/public/assets/app.css --minify

# Build the release stylesheet with a content-hashed filename (app.<hash>.css); the server discovers it by scanning the asset dir at startup (wired in Phase 2). No build.rs / tracked-source edit, so `git diff --exit-code` stays clean. (DEV-6642)
[group('dpe')]
css-release:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    out=modules/dpe/public/assets
    "$bin" -i modules/dpe/style/main.css -o "$out/app.css" --minify
    if command -v sha256sum >/dev/null 2>&1; then h=$(sha256sum "$out/app.css" | cut -c1-8); else h=$(shasum -a 256 "$out/app.css" | cut -c1-8); fi
    rm -f "$out"/app.[0-9a-f]*.css
    mv "$out/app.css" "$out/app.$h.css"
    echo "built $out/app.$h.css"

# Dev loop (DEV-6642): Tailwind --watch + bacon (kill_then_restart). NOTE: fully functional only from Phase 2; until then `just watch-dpe` (cargo-leptos) is the working dev loop.
[group('dpe')]
dev:
    #!/usr/bin/env bash
    set -euo pipefail
    bin="$(just -q _tailwind-bin)"
    "$bin" -i modules/dpe/style/main.css -o modules/dpe/public/assets/app.css --watch &
    tw=$!
    trap 'kill $tw 2>/dev/null || true' EXIT
    bacon serve

# Stand up the pre-migration HTML oracle: a worktree at the migration base commit with a native dpe-server build (no Node/Tailwind; structural comparison only). (DEV-6642)
[group('dpe')]
oracle-setup:
    #!/usr/bin/env bash
    set -euo pipefail
    base="$(git merge-base HEAD origin/main)"
    dir="{{ ORACLE_DIR }}"
    if [ -e "$dir/.git" ]; then
        echo "oracle worktree already present at $dir" >&2
    else
        echo "creating oracle worktree at $dir (base ${base:0:12})" >&2
        git worktree add --detach "$dir" "$base"
    fi
    echo "building base dpe-server (native, no Tailwind/Node)…" >&2
    cargo build --release -p dpe-server --manifest-path "$dir/Cargo.toml"
    echo "ready: $dir/target/release/dpe-server — serve it with 'just oracle-serve'" >&2

# Run the oracle (base-commit) dpe-server on ORACLE_PORT for curl/diff comparison. (DEV-6642)
[group('dpe')]
oracle-serve:
    #!/usr/bin/env bash
    set -euo pipefail
    dir="{{ ORACLE_DIR }}"
    test -x "$dir/target/release/dpe-server" || { echo "oracle binary missing — run 'just oracle-setup' first" >&2; exit 1; }
    echo "serving pre-migration oracle on http://127.0.0.1:{{ ORACLE_PORT }}" >&2
    cd "$dir"
    LEPTOS_OUTPUT_NAME=dpe LEPTOS_SITE_ROOT=modules/dpe/target/site LEPTOS_SITE_PKG_DIR=pkg LEPTOS_SITE_ADDR=127.0.0.1:{{ ORACLE_PORT }} LEPTOS_ENV=PROD RUST_LOG=error ./target/release/dpe-server serve

# Semantic HTML diff of one route: oracle (old) vs the migrated server on :4000. Usage: just oracle-diff /dpe/projects/0803 (DEV-6642)
[group('dpe')]
oracle-diff route:
    #!/usr/bin/env bash
    set -euo pipefail
    old="$(mktemp)"
    new="$(mktemp)"
    trap 'rm -f "$old" "$new"' EXIT
    curl -fsS "http://127.0.0.1:{{ ORACLE_PORT }}{{ route }}" | python3 scripts/oracle_normalize.py > "$old"
    curl -fsS "http://127.0.0.1:4000{{ route }}" | python3 scripts/oracle_normalize.py > "$new"
    if diff -u --label "oracle {{ route }}" "$old" --label "migrated {{ route }}" "$new"; then
        echo "✓ no semantic differences on {{ route }}" >&2
    fi

# Remove the oracle worktree. (DEV-6642)
[group('dpe')]
oracle-down:
    #!/usr/bin/env bash
    set -euo pipefail
    git worktree remove --force "{{ ORACLE_DIR }}" 2>/dev/null || rm -rf "{{ ORACLE_DIR }}"
    echo "removed oracle worktree {{ ORACLE_DIR }}" >&2

# Start the DPE with hot reload
[group('dpe')]
watch-dpe: _check-dpe-node-modules
    cargo leptos watch --project=dpe -- serve

# Start the Grafana LGTM (Loki, Grafana, Tempo, Mimir) all-in-one container for local observability
[group('dpe')]
lgtm-up:
    docker run --rm -p 3000:3000 -p 4317:4317 -p 4318:4318 -p 4040:4040 grafana/otel-lgtm

# Start the DPE with hot reload, exporting traces/metrics/logs to a local LGTM stack (run `just lgtm-up` in another terminal first)
[group('dpe')]
watch-dpe-otel: _check-dpe-node-modules
    OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317 \
    OTEL_SERVICE_NAME=dpe \
    OTEL_RESOURCE_ATTRIBUTES="service.namespace=dpe,service.version={{ CARGO_VERSION }},deployment.environment=dev" \
    PYROSCOPE_ENDPOINT=http://localhost:4040 \
    cargo leptos watch --project=dpe -- serve

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
test-a11y-dpe:
    cd modules/dpe/web-e2e-tests && npx playwright test tests/accessibility.spec.ts --project=chromium

# Lint E2E test TypeScript with Biome
lint-e2e:
    cd modules/dpe/web-e2e-tests && npx @biomejs/biome check .
    cd modules/mosaic/playground-e2e-tests && npx @biomejs/biome check .
