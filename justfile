mod dsp-dpe

DOCKER_REPO := "daschswiss/dsp-repository"
CARGO_VERSION := `cargo metadata --manifest-path dsp-dpe/Cargo.toml --format-version=1 --no-deps | jq --raw-output '.packages[] | select(.name == "leptos-server") | .version'`
COMMIT_HASH := `git log --pretty=format:'%h' -n 1`
GIT_TAG := `git describe --tags --exact-match 2>/dev/null || true`
IMAGE_TAG := if GIT_TAG == "" { CARGO_VERSION + "-" + COMMIT_HASH } else { CARGO_VERSION }
DOCKER_IMAGE := DOCKER_REPO + ":" + IMAGE_TAG

# List all recipes
default:
    just --list --unsorted

# Install all requirements
install-requirements:
    #!/usr/bin/env sh
    rustup show
    brew install cargo-binstall
    cargo binstall -y cargo-watch@8.5.3
    cargo binstall -y mdbook@0.4.52
    cargo binstall -y mdbook-alerts@0.8.0
    cargo binstall -y leptosfmt@0.1.33
    cargo binstall -y cargo-leptos@0.3.4
    cd dsp-dpe/leptos-dpe && pnpm install

# Build linux/amd64 Docker image locally
docker-build-amd64:
    docker buildx build --platform linux/amd64 -f dsp-dpe/leptos-dpe/Dockerfile -t {{ DOCKER_IMAGE }}-amd64 --load .

# Push previously build linux/amd64 image to Docker hub
docker-push-amd64:
    docker push {{ DOCKER_IMAGE }}-amd64

# Build linux/arm64 Docker image locally
docker-build-arm64:
    docker buildx build --platform linux/arm64 -f dsp-dpe/leptos-dpe/Dockerfile -t {{ DOCKER_IMAGE }}-arm64 --load .

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
