# Multi-stage build for igait-backend using cargo-chef
FROM rust:1.93-slim AS chef

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-chef

FROM chef AS planner

COPY igait-lib ./igait-lib
COPY igait-backend ./igait-backend

WORKDIR /app/igait-backend
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY igait-lib/Cargo.toml ./igait-lib/Cargo.toml
RUN mkdir -p igait-lib/src && echo "pub fn dummy() {}" > igait-lib/src/lib.rs

COPY --from=planner /app/igait-backend/recipe.json ./igait-backend/recipe.json
WORKDIR /app/igait-backend
RUN cargo chef cook --release --recipe-path recipe.json

WORKDIR /app
COPY igait-lib ./igait-lib
COPY igait-backend ./igait-backend

WORKDIR /app/igait-backend
RUN cargo build --release -p igait-backend

# Runtime stage
FROM debian:trixie-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3t64 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/igait-backend/target/release/igait-backend /usr/local/bin/igait-backend

ENV RUST_LOG=info
ENV PORT=3000

EXPOSE 3000

CMD ["igait-backend"]
