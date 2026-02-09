# Build Stage
FROM rust:bookworm AS builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y wget pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Install cargo-leptos
RUN cargo install cargo-leptos --locked

# Install Node.js and npm for Tailwind CSS
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get install -y nodejs

# Set the working directory
WORKDIR /work

# Copy package files first for better caching
COPY package.json package-lock.json* ./

# Install npm dependencies (including DaisyUI)
RUN npm install

# Copy the entire project
COPY . .

# Build the project in release mode with verbose output
RUN cargo leptos build --release -vv

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

# Copy the server binary from the builder stage (renamed to avoid conflict with server/data directory)
COPY --from=builder /work/target/release/server /app/leptos-dpe

# Copy the site directory (contains WASM, JS, CSS, and static assets)
COPY --from=builder /work/target/site /app/site

# Create the server directory structure for volume mounts
RUN mkdir -p /app/server/data

# Change ownership to the non-root user
RUN chown -R leptos:leptos /app
USER leptos

# Set environment variables for production
ENV RUST_LOG="info"
ENV LEPTOS_OUTPUT_NAME="leptos-dpe"
ENV LEPTOS_SITE_ROOT="site"
ENV LEPTOS_SITE_PKG_DIR="pkg"
ENV LEPTOS_SITE_ADDR="0.0.0.0:8080"
ENV LEPTOS_RELOAD_PORT="8081"
ENV LEPTOS_ENV="PROD"

# Expose the port
EXPOSE 8080

# Run the server
CMD ["./leptos-dpe"]
