DOCKER_REPO := "daschswiss/dsp-repository"
CARGO_VERSION := `cargo metadata --format-version=1 --no-deps | jq --raw-output '.packages[].version'`
COMMIT_HASH := `git log --pretty=format:'%h' -n 1`
GIT_TAG := `git describe --tags --exact-match 2>/dev/null || true`
IMAGE_TAG := if GIT_TAG == "" { CARGO_VERSION + "-" + COMMIT_HASH } else { CARGO_VERSION }
DOCKER_IMAGE := DOCKER_REPO + ":" + IMAGE_TAG

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
    cargo binstall -y leptosfmt@0.1.33
    cargo binstall -y cargo-leptos@0.3.4
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

[group('docs')]
docs-build:
    mdbook build docs

[group('docs')]
docs-serve:
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

# Start the DPE with hot reload
[group('dpe')]
watch-dpe:
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
