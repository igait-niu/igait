FROM rust:1.93-alpine AS chef

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
RUN cargo install cargo-chef

WORKDIR /app

FROM chef AS planner

COPY igait-lib ./igait-lib
COPY igait-stages/cycle-detection ./igait-stages/cycle-detection

WORKDIR /app/igait-stages/cycle-detection
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY igait-lib ./igait-lib

COPY --from=planner /app/igait-stages/cycle-detection/recipe.json ./igait-stages/cycle-detection/recipe.json
WORKDIR /app/igait-stages/cycle-detection
RUN cargo chef cook --release --recipe-path recipe.json

WORKDIR /app
COPY igait-lib ./igait-lib
COPY igait-stages/cycle-detection ./igait-stages/cycle-detection

WORKDIR /app/igait-stages/cycle-detection
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    python3-pip \
    && pip3 install --no-cache-dir --break-system-packages \
        numpy \
        pandas \
        scipy \
        matplotlib \
    && rm -rf /var/lib/apt/lists/*

# Copy the Python gait cycle detection script
COPY igait-stages/cycle-detection/igait-gait-cycle-detection/gait_analysis_mediapipe.py /app/gait_analysis_mediapipe.py

COPY --from=builder /app/igait-stages/cycle-detection/target/release/cycle-detection /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]
