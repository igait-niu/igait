FROM rust:1.93-alpine AS chef

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
RUN cargo install cargo-chef

WORKDIR /app

FROM chef AS planner

COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage2-validity-check/Cargo.toml ./igait-stages/igait-stage2-validity-check/Cargo.toml
COPY igait-stages/igait-stage2-validity-check/src ./igait-stages/igait-stage2-validity-check/src

WORKDIR /app/igait-stages/igait-stage2-validity-check
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY igait-lib/Cargo.toml ./igait-lib/Cargo.toml
RUN mkdir -p igait-lib/src && echo "pub fn dummy() {}" > igait-lib/src/lib.rs

COPY --from=planner /app/igait-stages/igait-stage2-validity-check/recipe.json ./igait-stages/igait-stage2-validity-check/recipe.json
WORKDIR /app/igait-stages/igait-stage2-validity-check
RUN cargo chef cook --release --recipe-path recipe.json

WORKDIR /app
COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage2-validity-check/Cargo.toml ./igait-stages/igait-stage2-validity-check/Cargo.toml
COPY igait-stages/igait-stage2-validity-check/src ./igait-stages/igait-stage2-validity-check/src

WORKDIR /app/igait-stages/igait-stage2-validity-check
RUN cargo build --release

# ── Runtime stage ────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    python3 \
    python3-pip \
    python3-venv \
    python3-dev \
    python3-opencv \
    git \
    wget \
    libgl1-mesa-glx \
    libglib2.0-0 \
    libgomp1 \
    && rm -rf /var/lib/apt/lists/*

# Install CPU-only PyTorch (pinned for pytorchvideo compatibility —
# functional_tensor was removed in torchvision 0.18)
RUN pip3 install --no-cache-dir --break-system-packages \
    "torch>=2.0,<2.3" "torchvision>=0.15,<0.18" --index-url https://download.pytorch.org/whl/cpu

# Shim: pytorchvideo imports torchvision.transforms.functional_tensor
# but torchvision 0.17 renamed it to _functional_tensor
RUN python3 -c "\
import torchvision.transforms, pathlib, os; \
d = pathlib.Path(torchvision.transforms.__file__).parent; \
src = d / '_functional_tensor.py'; \
dst = d / 'functional_tensor.py'; \
dst.exists() or dst.symlink_to(src); \
print(f'Shimmed {dst} -> {src}')"

RUN pip3 install --no-cache-dir --break-system-packages \
    pytorchvideo \
    scipy \
    "numpy<2" \
    "opencv-python>=4.1.1" \
    "pillow>=10.3.0" \
    "PyYAML>=5.3.1" \
    "tqdm>=4.66.3" \
    "matplotlib>=3.3" \
    seaborn \
    pandas \
    ultralytics \
    "gitpython>=3.1.30" \
    "setuptools>=70.0.0"

# Copy the detection pipeline
COPY igait-stages/igait-stage2-validity-check/igait-human-gait-detection /app/igait-human-gait-detection

# Copy DeepSort pretrained weights from the repository
COPY igait-stages/igait-stage2-validity-check/igait-human-gait-detection/deepsort_parameters/ckpt.t7 \
     /app/igait-human-gait-detection/deep_sort/deep_sort/deep/checkpoint/ckpt.t7

# Set torch cache dir so weights are always found
ENV TORCH_HOME=/root/.cache/torch

# Pre-cache YOLOv5 model (downloads repo + weights into torch hub cache)
RUN cd /app/igait-human-gait-detection && \
    python3 -c "\
import torch; \
model = torch.hub.load('ultralytics/yolov5', 'yolov5l6', device='cpu'); \
print('YOLOv5 model cached successfully')"

# Symlink cached YOLOv5 weights into the detection script's working directory
# (YOLOv5 also looks for weights in CWD)
RUN ln -sf /root/.cache/torch/hub/checkpoints/yolov5l6.pt /app/igait-human-gait-detection/yolov5l6.pt

# Pre-cache SlowFast model weights
RUN python3 -c "\
from pytorchvideo.models.hub import slowfast_r50_detection; \
model = slowfast_r50_detection(True); \
print('SlowFast model cached successfully')"

# Copy the compiled Rust worker binary
COPY --from=builder /app/igait-stages/igait-stage2-validity-check/target/release/igait-stage2-validity-check /usr/local/bin/worker

ENV RUST_LOG=info
CMD ["worker"]
