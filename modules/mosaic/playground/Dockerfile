# Mosaic playground — plain Axum + Maud server. No Node: the stylesheet is built
# with the standalone Tailwind CLI and the binary is a normal `cargo build` artifact.

# Chef base image — includes cargo-chef and the Rust toolchain.
FROM lukemathwalker/cargo-chef:latest-rust-1.97.1-bookworm AS chef
WORKDIR /app

# Planner — compute the dependency recipe (changes only when Cargo.toml/Cargo.lock change).
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder — cook dependencies (cached), build the stylesheet, then build the binary.
FROM chef AS builder

# Cook dependencies first (the slow, well-cached layer; busts only on
# Cargo.toml/Cargo.lock changes) so a Tailwind version bump below doesn't
# invalidate it.
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Download the pinned Tailwind v4 standalone CLI for the build platform (bundles
# the official plugins, so no Node/npm is needed). TAILWIND_VERSION must match
# the justfile's TAILWIND_VERSION. TARGETARCH is provided automatically by
# BuildKit (amd64 on the CI/Cloud Run x64 path, arm64 on Apple-Silicon).
ARG TAILWIND_VERSION=4.1.18
ARG TARGETARCH
RUN case "$TARGETARCH" in \
      amd64) tw_arch=x64 ;; \
      arm64) tw_arch=arm64 ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/tailwindlabs/tailwindcss/releases/download/v${TAILWIND_VERSION}/tailwindcss-linux-$tw_arch" \
      -o /usr/local/bin/tailwindcss \
    && chmod +x /usr/local/bin/tailwindcss

# Copy the full source.
COPY . .

# Build the stylesheet → playground/public/assets/app.css (same as `just css-mosaic`).
RUN tailwindcss -i modules/mosaic/playground/style/main.css \
    -o modules/mosaic/playground/public/assets/app.css --minify

# Build the server binary.
RUN cargo build --release -p mosaic-playground

# Distroless runtime — glibc + C/C++ runtime only, no shell, no apt, no perl.
# The server is a plain Axum binary serving static files; it makes no outbound
# TLS calls, so it needs no extra OS packages. This keeps the image minimal and
# free of the OS-package CVEs that ship with debian-slim (e.g. perl's
# CVE-2026-12087, "Not Fixed" in bookworm).
FROM gcr.io/distroless/cc-debian12:nonroot AS runtime
WORKDIR /app

# The server binary and the public dir (static assets + the built app.css).
COPY --from=builder /app/target/release/mosaic-playground /app/mosaic-playground
COPY --from=builder /app/modules/mosaic/playground/public /app/public

ENV MOSAIC_SITE_ADDR="0.0.0.0:8080"
ENV MOSAIC_PUBLIC_DIR="/app/public"
EXPOSE 8080

ENTRYPOINT ["/app/mosaic-playground"]
