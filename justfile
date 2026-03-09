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
    cd modules/leptos-dpe && pnpm install

# Install Playwright browsers for E2E tests
install-e2e-requirements:
    cd modules/mosaic/demo/end2end && npx playwright install
    cd modules/leptos-dpe/end2end && npx playwright install

# Run all fmt and clippy checks
check:
    just --check --fmt --unstable
    cargo +nightly fmt -p mosaic-tiles -p demo_macro --check
    leptosfmt --check modules/leptos-dpe
    leptosfmt --check modules/mosaic/demo
    cargo clippy -- -D warnings

# Format all rust code (cargo fmt for non-leptos crates, leptosfmt for leptos crates)
fmt:
    cargo +nightly fmt -p mosaic-tiles -p demo_macro
    leptosfmt modules/leptos-dpe
    leptosfmt modules/mosaic/demo

# Fix justfile formatting. Warning: will change existing file. Please first use check.
fix:
    just --fmt --unstable

# Run all rust builds
build:
    cargo build --all-targets

# Run server
run:
    cargo run --bin server --release

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

# Start the mosaic demo and watch mosaic tiles
[group('mosaic')]
watch-mosaic-demo:
    #!/usr/bin/env sh
    cd modules/mosaic/demo
    cargo leptos watch -- --watch ../tiles

# Build Docker image for mosaic demo
[group('mosaic')]
build-docker-mosaic-demo:
    docker build -f modules/mosaic/demo/Dockerfile -t mosaic-demo .

# Run mosaic demo Docker container on port 8080
[group('mosaic')]
run-docker-mosaic-demo:
    docker run --rm -p 8080:8080 mosaic-demo

###################
# Leptos DPE targets
###################

# Start the leptos-dpe with hot reload
[group('leptos-dpe')]
watch-leptos-dpe:
    cargo leptos watch --project=leptos-dpe -- watch ../mosaic/tiles

# Build Docker image for leptos-dpe
[group('leptos-dpe')]
build-docker-dpe:
    docker build -f modules/leptos-dpe/Dockerfile -t leptos-dpe .

# Run leptos-dpe Docker container on port 8080
[group('leptos-dpe')]
run-docker-dpe:
    docker run --rm -p 8080:8080 leptos-dpe

# Run accessibility E2E tests for the Mosaic demo (requires running server on port 3000)
[group('mosaic')]
test-a11y-mosaic-demo:
    cd modules/mosaic/demo/end2end && npx playwright test tests/accessibility.spec.ts

# Run accessibility E2E tests for the DPE (requires running server on port 4000)
[group('leptos-dpe')]
test-a11y-leptos-dpe:
    cd modules/leptos-dpe/end2end && npx playwright test tests/accessibility.spec.ts
