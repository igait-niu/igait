# Multi-stage build for igait-backend
FROM rust:1.93-slim AS builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo manifests for dependency caching
COPY igait-lib/Cargo.toml ./igait-lib/Cargo.toml
COPY igait-backend/Cargo.toml ./igait-backend/Cargo.toml

# Create dummy source files to cache dependencies
RUN mkdir -p igait-lib/src && echo "pub fn dummy() {}" > igait-lib/src/lib.rs
RUN mkdir -p igait-backend/src && echo "fn main() {}" > igait-backend/src/main.rs

# Build dependencies only (this layer will be cached)
WORKDIR /app/igait-backend
RUN cargo build --release 2>/dev/null || true

# Now copy actual source files
WORKDIR /app
COPY igait-lib ./igait-lib
COPY igait-backend ./igait-backend

# Build the backend with actual source
WORKDIR /app/igait-backend
RUN cargo build --release -p igait-backend

# Runtime stage - use matching Debian version
FROM debian:trixie-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

# Copy the binary from workspace target directory
COPY --from=builder /app/target/release/igait-backend /usr/local/bin/igait-backend

# Set environment
ENV RUST_LOG=info
ENV PORT=3000

EXPOSE 3000

CMD ["igait-backend"]
