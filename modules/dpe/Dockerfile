# Build Stage
FROM rust:1.92.0-bookworm AS builder

# Install cargo-binstall (pinned version with integrity check)
RUN BINSTALL_VERSION="1.17.9" && \
    wget -q "https://github.com/cargo-bins/cargo-binstall/releases/download/v${BINSTALL_VERSION}/cargo-binstall-x86_64-unknown-linux-musl.tgz" && \
    echo "bdea3fd820b118576316bf69c8240f454857287717809e6ecef010faace901ff  cargo-binstall-x86_64-unknown-linux-musl.tgz" | sha256sum -c - && \
    tar -xzf cargo-binstall-x86_64-unknown-linux-musl.tgz -C /usr/local/cargo/bin && \
    rm cargo-binstall-x86_64-unknown-linux-musl.tgz

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

# Build site assets (CSS, JS) with cargo-leptos (uses default host target + WASM)
WORKDIR /app
RUN cargo leptos build --project dpe --release -vv

# Rebuild the server binary as a static musl binary (cargo caches most of the compilation)
# Re-verify musl target is present (cargo-leptos may change the active toolchain)
RUN rustup target add x86_64-unknown-linux-musl 2>/dev/null; \
    cargo build -p dpe-server --release --target x86_64-unknown-linux-musl

# Runtime Stage — distroless/static (no glibc, no shell — fully static musl binary)
FROM gcr.io/distroless/static-debian12:nonroot AS runtime

WORKDIR /app

# Copy the statically-linked server binary
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/dpe-server /app/dpe-server

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
ENV LEPTOS_ENV="PROD"
ENV DPE_DATA_DIR="/app/server/data"

# Expose the port
EXPOSE 8080

# Run the server (distroless :nonroot runs as uid 65534 by default)
ENTRYPOINT ["./dpe-server"]
CMD ["serve"]

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD ["./dpe-server", "healthcheck"]
