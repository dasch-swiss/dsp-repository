# Build Stage
FROM rust:1.92.0-bookworm AS builder

# Install cargo-binstall for faster tool installation
RUN wget https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN tar -xvf cargo-binstall-x86_64-unknown-linux-musl.tgz
RUN cp cargo-binstall /usr/local/cargo/bin

# Install build dependencies
RUN apt-get update -y \
  && apt-get install -y --no-install-recommends clang

# Install cargo-leptos
RUN cargo binstall cargo-leptos -y

# Add the WASM target
RUN rustup target add wasm32-unknown-unknown

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

# Runtime Stage
FROM debian:bookworm-slim AS runtime

# Install runtime dependencies (OpenSSL and CA certificates)
RUN apt-get update -y && \
    apt-get install -y --no-install-recommends \
    openssl \
    ca-certificates && \
    apt-get autoremove -y && \
    apt-get clean -y && \
    rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -m -u 1001 leptos

# Set the working directory
WORKDIR /app

# Copy the server binary from the builder stage
COPY --from=builder /app/target/release/dpe /app/dpe

# Copy the site directory (contains WASM, JS, CSS, and static assets)
COPY --from=builder /app/modules/dpe/target/site /app/site

# Copy the data files from the builder stage
COPY --from=builder /app/modules/dpe/server/data /app/server/data

# Change ownership to the non-root user
RUN chown -R leptos:leptos /app
USER leptos

# Set environment variables for production
ENV RUST_LOG="info"
ENV LEPTOS_OUTPUT_NAME="dpe"
ENV LEPTOS_SITE_ROOT="site"
ENV LEPTOS_SITE_PKG_DIR="pkg"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_RELOAD_PORT="8081"
ENV LEPTOS_ENV="PROD"
ENV DATA_DIR="/app/server/data"

# Expose the port
EXPOSE 8080

# Run the server
CMD ["./dpe"]
