DOCKER_REPO := "daschswiss/dsp-repository"
CARGO_VERSION := `cargo metadata --format-version=1 --no-deps | jq --raw-output '.packages[].version'`
COMMIT_HASH := `git log --pretty=format:'%h' -n 1`
GIT_TAG := `git describe --tags --exact-match 2>/dev/null || true`
IMAGE_TAG := if GIT_TAG == "" { CARGO_VERSION + "-" + COMMIT_HASH } else { CARGO_VERSION }
DOCKER_IMAGE := DOCKER_REPO + ":" + IMAGE_TAG

# Tailwind CSS version (use "latest" or specific version like "v4.1.17")
TAILWIND_VERSION := "latest"

# List all recipes
default:
    just --list --unsorted

# Install all requirements
install-requirements:
    cargo install cargo-watch
    cargo install mdbook
    cargo install mdbook-alerts

# Run all fmt and clippy checks
check:
    just --check --fmt --unstable
    cargo +nightly fmt --check
    cargo clippy -- -D warnings

# Format all rust code
fmt:
    cargo +nightly fmt

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

docs-install-requirements:
    cargo install mdbook

docs-build:
    mdbook build docs

docs-serve:
    mdbook serve docs

docs-clean:
    mdbook clean docs

docs-test:
    mdbook test docs

run-watch-playground:
    cargo watch -s 'cargo run --bin playground-server'

# Run playground server in background (for MCP testing)
run-playground-background:
    cargo run --bin playground-server > /dev/null 2>&1 &
    @echo "Playground server started in background at http://localhost:3400"
    @echo "To stop: just stop-playground"

# Stop background playground server
stop-playground:
    @pkill -f playground-server || echo "No playground server running"

# Check if playground server is running
check-playground:
    @curl -s -o /dev/null -w "%{http_code}" http://localhost:3400 && echo " - Playground server is running at http://localhost:3400" || echo "Playground server is not running"

# Install Tailwind CSS standalone CLI binary
install-tailwind-cli:
    #!/usr/bin/env bash
    set -euo pipefail

    # Detect platform
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    # Map architecture names
    case "$ARCH" in
        x86_64|amd64) ARCH="x64" ;;
        aarch64|arm64) ARCH="arm64" ;;
        *) echo "Error: Unsupported architecture: $ARCH"; exit 1 ;;
    esac

    # Map OS names and build binary name
    case "$OS" in
        darwin) BINARY_NAME="tailwindcss-macos-${ARCH}" ;;
        linux)  BINARY_NAME="tailwindcss-linux-${ARCH}" ;;
        mingw*|msys*|cygwin*) BINARY_NAME="tailwindcss-windows-${ARCH}.exe" ;;
        *) echo "Error: Unsupported OS: $OS"; exit 1 ;;
    esac

    echo "Detected platform: $OS-$ARCH"
    echo "Downloading Tailwind CSS CLI (version: {{ TAILWIND_VERSION }})..."

    # Create tools directory if it doesn't exist
    mkdir -p tools

    # Download binary
    DOWNLOAD_URL="https://github.com/tailwindlabs/tailwindcss/releases/{{ TAILWIND_VERSION }}/download/${BINARY_NAME}"
    if ! curl -sL "$DOWNLOAD_URL" -o tools/tailwindcss; then
        echo "Error: Failed to download Tailwind CSS CLI"
        echo "Please download manually from: https://github.com/tailwindlabs/tailwindcss/releases/{{ TAILWIND_VERSION }}"
        exit 1
    fi

    # Make executable
    chmod +x tools/tailwindcss

    # Verify installation
    echo "Verifying installation..."
    if ! ./tools/tailwindcss --version; then
        echo "Error: Failed to verify Tailwind CSS CLI installation"
        exit 1
    fi

    echo "✓ Tailwind CSS CLI installed successfully"

# Build Tailwind CSS (development - unminified)
tailwind-dev:
    @if [ ! -f tools/tailwindcss ]; then \
        echo "Error: Tailwind CSS CLI not found. Run 'just install-tailwind-cli' first"; \
        exit 1; \
    fi
    ./tools/tailwindcss -i modules/design_system/input.css -o modules/design_system/tailwind.css

# Build Tailwind CSS (production - minified)
tailwind-build:
    @if [ ! -f tools/tailwindcss ]; then \
        echo "Error: Tailwind CSS CLI not found. Run 'just install-tailwind-cli' first"; \
        exit 1; \
    fi
    ./tools/tailwindcss -i modules/design_system/input.css -o modules/design_system/tailwind.css --minify

# Watch Tailwind CSS for changes (development)
tailwind-watch:
    @if [ ! -f tools/tailwindcss ]; then \
        echo "Error: Tailwind CSS CLI not found. Run 'just install-tailwind-cli' first"; \
        exit 1; \
    fi
    ./tools/tailwindcss -i modules/design_system/input.css -o modules/design_system/tailwind.css --watch

mod playground 'modules/design_system/playground'
