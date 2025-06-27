FROM mcr.microsoft.com/playwright:v1.53.1-jammy

# Set working directory
WORKDIR /app

# Set locale for consistent font rendering
ENV LANG=en_US.UTF-8
ENV LC_ALL=en_US.UTF-8

# Install build dependencies and font packages
RUN apt-get update && apt-get install -y \
    build-essential \
    fonts-liberation \
    fonts-dejavu-core \
    fontconfig \
    locales \
    && locale-gen en_US.UTF-8 \
    && fc-cache -f -v \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy the entire project
COPY . .

# Install Node.js dependencies
WORKDIR /app/modules/design_system/playground
RUN npm ci

# Build Rust project
WORKDIR /app
RUN cargo build --bin playground-server --release

# Set working directory back to playground
WORKDIR /app/modules/design_system/playground

# Command to run tests
CMD ["npm", "run", "test:update-snapshots"]