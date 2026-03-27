# Build Stage
FROM rust:1.92.0-bookworm AS builder

# Install cargo-binstall for faster tool installation
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN cp cargo-binstall /usr/local/cargo/bin

# Install build dependencies (clang for linking, musl-tools for static builds)
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends clang musl-tools

# Install cargo-leptos
RUN cargo binstall cargo-leptos -y

# Add the WASM target (needed by cargo-leptos for the lib crate stub)
RUN rustup target add wasm32-unknown-unknown

# Add musl target for static binary
RUN rustup target add x86_64-unknown-linux-musl

# Install Node.js and pnpm for Tailwind CSS / DaisyUI
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs && \
    npm install -g pnpm

# Set the working directory
RUN mkdir -p /app
WORKDIR /app

# Copy the entire workspace (needed for workspace dependencies)
COPY . .

# Install npm dependencies
WORKDIR /app/modules/dpe
RUN CI=true pnpm install --frozen-lockfile

# Build the project from the workspace root
WORKDIR /app
RUN cargo leptos build --project dpe --release -vv

# Runtime Stage — distroless (no shell, no package manager, minimal attack surface)
FROM gcr.io/distroless/static-debian12:nonroot AS runtime

WORKDIR /app

# Copy the server binary from the builder stage
COPY --from=builder /app/target/release/dpe-server /app/dpe-server

# Copy the site directory (contains JS, CSS, and static assets — no WASM)
COPY --from=builder /app/modules/dpe/target/site /app/site

# Copy the data files from the builder stage
COPY --from=builder /app/modules/dpe/server/data /app/server/data

# Copy CA certificates for outbound HTTPS (if ever needed)
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

# Set environment variables for production
ENV RUST_LOG="info"
ENV LEPTOS_OUTPUT_NAME="dpe"
ENV LEPTOS_SITE_ROOT="site"
ENV LEPTOS_SITE_PKG_DIR="pkg"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_RELOAD_PORT="8081"
ENV LEPTOS_ENV="PROD"
ENV DPE_DATA_DIR="/app/server/data"

# Expose the port
EXPOSE 8080

# Run the server (distroless :nonroot runs as uid 65534 by default)
CMD ["./dpe-server"]
