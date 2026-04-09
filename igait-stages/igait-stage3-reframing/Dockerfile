FROM rust:1.93-alpine AS chef

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static pkgconfig
RUN cargo install cargo-chef

WORKDIR /app

FROM chef AS planner

COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage3-reframing ./igait-stages/igait-stage3-reframing

WORKDIR /app/igait-stages/igait-stage3-reframing
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

COPY igait-lib/Cargo.toml ./igait-lib/Cargo.toml
RUN mkdir -p igait-lib/src && echo "pub fn dummy() {}" > igait-lib/src/lib.rs

COPY --from=planner /app/igait-stages/igait-stage3-reframing/recipe.json ./igait-stages/igait-stage3-reframing/recipe.json
WORKDIR /app/igait-stages/igait-stage3-reframing
RUN cargo chef cook --release --recipe-path recipe.json

WORKDIR /app
COPY igait-lib ./igait-lib
COPY igait-stages/igait-stage3-reframing ./igait-stages/igait-stage3-reframing

WORKDIR /app/igait-stages/igait-stage3-reframing
RUN cargo build --release

FROM alpine:latest
RUN apk add --no-cache ca-certificates libgcc
COPY --from=builder /app/igait-stages/igait-stage3-reframing/target/release/igait-stage3-reframing /usr/local/bin/worker
ENV RUST_LOG=info
CMD ["worker"]
