# ─────────────────────────────────────────────────────────────────────────────
# Unified multi-stage Dockerfile for the iGait Rust workspace.
#
# One cargo-chef builder compiles every workspace member once, sharing the
# target/ directory and Cargo.lock. Each service image is a target below that
# just COPY --from=builder's its binary and installs its runtime deps.
#
# Build a specific service:
#   docker build -f Dockerfile --target backend -t igait-backend .
#   docker build -f Dockerfile --target pose-estimation -t igait-pose .
# ─────────────────────────────────────────────────────────────────────────────

FROM rust:1.93-slim-bookworm AS chef
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

# ── Planner: read every Cargo.toml, emit one workspace recipe.json ──────────
FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY igait-lib ./igait-lib
COPY igait-backend ./igait-backend
COPY igait-stages ./igait-stages
RUN cargo chef prepare --recipe-path recipe.json

# ── Builder: cook deps (cached layer), then build every workspace binary ────
FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY Cargo.toml Cargo.lock ./
COPY igait-lib ./igait-lib
COPY igait-backend ./igait-backend
COPY igait-stages ./igait-stages
RUN cargo build --release --workspace \
    --bin igait-backend \
    --bin media-conversion \
    --bin pose-estimation \
    --bin cycle-detection \
    --bin prediction \
    --bin finalize

# ─────────────────────────────────────────────────────────────────────────────
# Runtime stages — one per service. All use debian:bookworm-slim so the
# binaries built against the bookworm builder run unmodified.
# ─────────────────────────────────────────────────────────────────────────────

# ── backend ─────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS backend
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/igait-backend /usr/local/bin/igait-backend
ENV RUST_LOG=info
CMD ["igait-backend"]

# ── media-conversion (ffmpeg) ───────────────────────────────────────────────
FROM debian:bookworm-slim AS media-conversion
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 ffmpeg \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/media-conversion /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]

# ── pose-estimation (Python + mediapipe + torch) ────────────────────────────
FROM debian:bookworm-slim AS pose-estimation
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    python3 python3-pip \
    libgl1 libglib2.0-0 ffmpeg \
    && pip3 install --no-cache-dir --break-system-packages \
        mediapipe opencv-python-headless numpy torch \
    && rm -rf /var/lib/apt/lists/*
COPY igait-stages/pose-estimation/igait-mediapipe/3DPoseEstimation.py /app/3DPoseEstimation.py
COPY --from=builder /app/target/release/pose-estimation /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]

# ── cycle-detection (Python + scientific stack) ─────────────────────────────
FROM debian:bookworm-slim AS cycle-detection
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    python3 python3-pip \
    && pip3 install --no-cache-dir --break-system-packages \
        numpy pandas scipy matplotlib \
    && rm -rf /var/lib/apt/lists/*
COPY igait-stages/cycle-detection/igait-gait-cycle-detection/gait_analysis_mediapipe.py /app/gait_analysis_mediapipe.py
COPY --from=builder /app/target/release/cycle-detection /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]

# ── prediction (Python + model IO requirements) ─────────────────────────────
FROM debian:bookworm-slim AS prediction
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    python3 python3-pip \
    && rm -rf /var/lib/apt/lists/*
COPY igait-stages/prediction/iGAIT_MODEL_IO /app/iGAIT_MODEL_IO
RUN pip3 install --no-cache-dir --break-system-packages \
    -r /app/iGAIT_MODEL_IO/requirements.txt || true
COPY --from=builder /app/target/release/prediction /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]

# ── finalize (just needs TLS for SES) ───────────────────────────────────────
FROM debian:bookworm-slim AS finalize
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/finalize /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]
