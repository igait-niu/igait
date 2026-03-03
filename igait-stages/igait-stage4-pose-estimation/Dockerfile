FROM rust:1.93-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig

WORKDIR /app

# Copy Cargo manifests for dependency caching
COPY igait-lib/Cargo.toml ./igait-lib/Cargo.toml
COPY igait-stages/igait-stage4-pose-estimation/Cargo.toml ./igait-stages/igait-stage4-pose-estimation/Cargo.toml

# Create dummy source files to cache dependencies
RUN mkdir -p igait-lib/src && echo "pub fn dummy() {}" > igait-lib/src/lib.rs
RUN mkdir -p igait-stages/igait-stage4-pose-estimation/src && echo "fn main() {}" > igait-stages/igait-stage4-pose-estimation/src/main.rs

# Build dependencies only (this layer will be cached)
WORKDIR /app/igait-stages/igait-stage4-pose-estimation
RUN cargo build --release 2>/dev/null || true

# Now copy actual source files
WORKDIR /app
COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage4-pose-estimation ./igait-stages/igait-stage4-pose-estimation

# Build with actual source
WORKDIR /app/igait-stages/igait-stage4-pose-estimation
# Clean the dummy build artifacts first
RUN cargo clean --release
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
