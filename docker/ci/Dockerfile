# syntax=docker/dockerfile:1

FROM rust:1.100.0-slim-bookworm

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    libssl-dev \
    musl-tools \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

RUN rustup component add rustfmt clippy \
    && rustup target add "$(uname -m)-unknown-linux-musl"

RUN arch="$(uname -m)" \
    && curl -fsSL "https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-${arch}-unknown-linux-musl.tgz" \
    | tar -xz -C /usr/local/cargo/bin \
    && cargo-binstall -y \
        cargo-nextest \
        cargo-deny \
        cargo-machete \
        zizmor \
        just

WORKDIR /workspace
