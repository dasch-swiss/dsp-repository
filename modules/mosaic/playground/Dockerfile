# Mosaic playground — plain Axum + Maud server (Leptos → Maud migration, DEV-6642).
# No cargo-leptos, no WASM target, no Node: the stylesheet is built with the
# standalone Tailwind CLI and the binary is a normal `cargo build` artifact.

# Chef base image — includes cargo-chef and the Rust toolchain.
FROM lukemathwalker/cargo-chef:latest-rust-1.93.0-bookworm AS chef
WORKDIR /app

# Planner — compute the dependency recipe (changes only when Cargo.toml/Cargo.lock change).
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder — cook dependencies (cached), build the stylesheet, then build the binary.
FROM chef AS builder

# Download the pinned Tailwind v4 standalone CLI for the build platform (bundles
# the official plugins, so no Node/npm is needed). Keep the version in sync with
# TAILWIND_VERSION in the justfile. TARGETARCH is provided automatically by
# BuildKit (amd64 on the CI/Cloud Run x64 path, arm64 on Apple-Silicon).
ARG TARGETARCH
RUN case "$TARGETARCH" in \
      amd64) tw_arch=x64 ;; \
      arm64) tw_arch=arm64 ;; \
      *) echo "unsupported TARGETARCH: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    curl -fsSL "https://github.com/tailwindlabs/tailwindcss/releases/download/v4.1.18/tailwindcss-linux-$tw_arch" \
      -o /usr/local/bin/tailwindcss \
    && chmod +x /usr/local/bin/tailwindcss

# Cook dependencies (cached as long as Cargo.toml/Cargo.lock don't change).
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Copy the full source.
COPY . .

# Build the stylesheet → playground/public/assets/app.css (same as `just css-mosaic`).
RUN tailwindcss -i modules/mosaic/playground/style/main.css \
    -o modules/mosaic/playground/public/assets/app.css --minify

# Build the server binary.
RUN cargo build --release -p mosaic-playground

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
  && apt-get upgrade -y --no-install-recommends \
  && apt-get install -y --no-install-recommends ca-certificates \
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

# The server binary and the public dir (static assets + the built app.css).
COPY --from=builder /app/target/release/mosaic-playground /app/mosaic-playground
COPY --from=builder /app/modules/mosaic/playground/public /app/public

ENV RUST_LOG="info"
ENV MOSAIC_SITE_ADDR="0.0.0.0:8080"
ENV MOSAIC_PUBLIC_DIR="/app/public"
EXPOSE 8080

CMD ["/app/mosaic-playground"]
