FROM rust:1.93-alpine AS chef

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
RUN cargo install cargo-chef

WORKDIR /app

FROM chef AS planner

COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage6-prediction ./igait-stages/igait-stage6-prediction

WORKDIR /app/igait-stages/igait-stage6-prediction
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY igait-lib ./igait-lib

COPY --from=planner /app/igait-stages/igait-stage6-prediction/recipe.json ./igait-stages/igait-stage6-prediction/recipe.json
WORKDIR /app/igait-stages/igait-stage6-prediction
RUN cargo chef cook --release --recipe-path recipe.json

WORKDIR /app
COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage6-prediction ./igait-stages/igait-stage6-prediction

WORKDIR /app/igait-stages/igait-stage6-prediction
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates python3 python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Copy the Python prediction scripts
COPY igait-stages/igait-stage6-prediction/iGAIT_MODEL_IO /app/iGAIT_MODEL_IO

# Install Python dependencies (if any)
RUN pip3 install --no-cache-dir --break-system-packages \
    -r /app/iGAIT_MODEL_IO/requirements.txt || true

COPY --from=builder /app/igait-stages/igait-stage6-prediction/target/release/igait-stage6-prediction /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]
