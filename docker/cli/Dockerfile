# syntax=docker/dockerfile:1

FROM rust:1.100.0-slim-bookworm AS builder

ARG TARGETARCH

RUN apt-get update && apt-get install -y --no-install-recommends \
    musl-tools \
    && rm -rf /var/lib/apt/lists/*

RUN case "${TARGETARCH:-}" in \
        arm64) target="aarch64-unknown-linux-musl" ;; \
        amd64) target="x86_64-unknown-linux-musl" ;; \
        *) target="$(uname -m)-unknown-linux-musl" ;; \
    esac \
    && rustup target add "$target" \
    && printf '%s\n' "$target" > /tmp/musl-target

WORKDIR /src
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates
COPY recursos ./recursos

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    target="$(cat /tmp/musl-target)" \
    && RUSTUP_TOOLCHAIN=1.100.0 cargo build --release --locked --package sexta-feira-cli --target "$target" \
    && mkdir -p /out \
    && install -m 0755 "/src/target/${target}/release/sexta-feira" /out/sexta-feira

FROM scratch AS export
COPY --from=builder /out/sexta-feira /sexta-feira

FROM alpine:3.21 AS runtime

RUN apk add --no-cache ca-certificates \
    && addgroup -g 10000 sextafeira \
    && adduser -D -u 10000 -G sextafeira -h /dados sextafeira \
    && mkdir -p /dados /workspace \
    && chown 10000:10000 /dados

COPY --from=builder /out/sexta-feira /usr/local/bin/sexta-feira

USER 10000:10000
ENV HOME=/dados \
    HERMES_HOME=/dados/.hermes
WORKDIR /workspace

ENTRYPOINT ["sexta-feira"]
