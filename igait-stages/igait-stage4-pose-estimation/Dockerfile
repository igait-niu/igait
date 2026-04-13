FROM rust:1.93-alpine AS chef

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
RUN cargo install cargo-chef

WORKDIR /app

FROM chef AS planner

COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage4-pose-estimation ./igait-stages/igait-stage4-pose-estimation

WORKDIR /app/igait-stages/igait-stage4-pose-estimation
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY igait-lib ./igait-lib

COPY --from=planner /app/igait-stages/igait-stage4-pose-estimation/recipe.json ./igait-stages/igait-stage4-pose-estimation/recipe.json
WORKDIR /app/igait-stages/igait-stage4-pose-estimation
RUN cargo chef cook --release --recipe-path recipe.json

WORKDIR /app
COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage4-pose-estimation ./igait-stages/igait-stage4-pose-estimation

WORKDIR /app/igait-stages/igait-stage4-pose-estimation
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    python3-pip \
    python3-opencv \
    libgl1-mesa-glx \
    libglib2.0-0 \
    ffmpeg \
    && pip3 install --no-cache-dir --break-system-packages \
        "mediapipe>=0.10.13,<0.10.14" \
        "opencv-python>=4.13" \
        "numpy>=2.4" \
        "torch>=2.10" \
    && rm -rf /var/lib/apt/lists/*

# Copy the Python pose estimation script
COPY igait-stages/igait-stage4-pose-estimation/igait-mediapipe/3DPoseEstimation.py /app/3DPoseEstimation.py

COPY --from=builder /app/igait-stages/igait-stage4-pose-estimation/target/release/igait-stage4-pose-estimation /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]
