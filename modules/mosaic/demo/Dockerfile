# Chef base image — includes cargo-chef and Rust toolchain
FROM lukemathwalker/cargo-chef:latest-rust-1.93.0-bookworm AS chef
WORKDIR /app

# Planner — compute dependency recipe (changes only when Cargo.toml/Cargo.lock change)
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder — cook dependencies (cached), then build source
FROM chef AS builder

# Install cargo-binstall
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN cp cargo-binstall /usr/local/cargo/bin

# Install required tools
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends clang

# Install cargo-leptos
RUN cargo binstall cargo-leptos -y

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

# Download Tailwind CSS binary so build.rs doesn't need network access during compilation
RUN curl -fsSL https://github.com/tailwindlabs/tailwindcss/releases/download/v4.1.17/tailwindcss-linux-x64 \
    -o /usr/local/bin/tailwindcss \
    && chmod +x /usr/local/bin/tailwindcss

# Cook dependencies (cached as long as Cargo.toml/Cargo.lock don't change)
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo chef cook --release --target wasm32-unknown-unknown -p mosaic-playground -p mosaic-tiles --recipe-path recipe.json

# Copy full source and build
COPY . .

# Build the app from the demo directory
WORKDIR /app/modules/mosaic/demo
ENV SINGLESTAGE_TAILWIND_PATH=/usr/local/bin/tailwindcss
RUN cargo leptos build --release -vv

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends openssl ca-certificates \
  && apt-get autoremove -y \
  && apt-get clean -y \
  && rm -rf /var/lib/apt/lists/*

# Copy the server binary to the /app directory
COPY --from=builder /app/target/release/mosaic-playground /app/

# /target/site contains our JS/WASM/CSS, etc.
COPY --from=builder /app/target/site /app/site

# Copy Cargo.toml if it's needed at runtime
COPY --from=builder /app/modules/mosaic/demo/Cargo.toml /app/

# Set any required env variables
ENV RUST_LOG="info"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_SITE_ROOT="site"
EXPOSE 8080

# Run the server
CMD ["/app/mosaic-playground"]
