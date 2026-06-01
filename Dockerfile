# syntax=docker/dockerfile:1.7

## Build stage ---------------------------------------------------------------
FROM rust:alpine3.23 AS builder
WORKDIR /workspace

ARG RUST_TARGET
ARG BUILD_DATE
ARG VERSION=0.1.0
ARG REVISION=main

RUN apk add --no-cache \
        boost-dev \
        build-base \
        clang \
        libtorrent-rasterbar-dev \
        musl-dev \
        openssl-dev \
        pkgconf

COPY . .

# Install the toolchain components listed in rust-toolchain.toml for the host arch.
RUN rustup toolchain install stable --profile minimal \
        --component rustfmt --component clippy --component llvm-tools-preview \
    && rustup default stable

# Link dynamically against musl for third-party libs (libtorrent/openssl) on Alpine.
ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN set -eux; \
  rustup target add "${RUST_TARGET}"; \
  # Force a fixed target dir so we can normalize the output path
  export CARGO_TARGET_DIR=/workspace/target; \
  cargo build --release --locked --package revaer-app --target "${RUST_TARGET}"; \
  # Normalize to /workspace/target/release/revaer-app regardless of target triple
  mkdir -p /workspace/target/release; \
  cp "/workspace/target/${RUST_TARGET}/release/revaer-app" \
     "/workspace/target/release/revaer-app"; \
  ls -l /workspace/target/release

## Runtime stage -------------------------------------------------------------
FROM alpine:3.23 AS runtime

ARG BUILD_DATE
ARG VERSION=0.1.0
ARG REVISION=main

RUN addgroup -S revaer && adduser -S revaer -G revaer \
    && apk add --no-cache \
        bento4 \
        ca-certificates \
        curl \
        exiftool \
        ffmpeg \
        font-dejavu \
        fontconfig \
        gnutls \
        libass \
        libdav1d \
        libstdc++ \
        libtheora \
        libtorrent-rasterbar \
        libvorbis \
        mediainfo \
        mkvtoolnix \
        openssl \
        opus \
        x264-libs \
        x265-libs \
    && mkdir -p /app/compliance /data /config \
    && chown root:root /app /app/compliance \
    && chmod 0755 /app /app/compliance \
    && chown -R revaer:revaer /data /config

WORKDIR /app

# Always copy from normalized path
COPY --from=builder --chown=root:root --chmod=0555 /workspace/target/release/revaer-app /usr/local/bin/revaer-app
COPY --from=builder --chown=root:root /workspace/docs /app/docs
COPY --from=builder --chown=root:root /workspace/config /app/config
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/SOURCE-OFFER.txt /app/compliance/SOURCE-OFFER.txt
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/THIRD-PARTY-NOTICES.md /app/compliance/THIRD-PARTY-NOTICES.md
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/media-runtime-inventory.spdx.json /app/compliance/media-runtime-inventory.spdx.json
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/exiftool-exception.md /app/compliance/exiftool-exception.md

RUN install -d -o revaer -g revaer -m 0755 /app/docs/api

VOLUME ["/data", "/config"]
ENV RUST_LOG=info
ENV LD_LIBRARY_PATH=/usr/local/lib

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -fsS http://127.0.0.1:7070/health/full || exit 1

# Apply metadata labels to final image
LABEL org.opencontainers.image.title="Revaer"
LABEL org.opencontainers.image.description="BitTorrent automation and media management platform"
LABEL org.opencontainers.image.vendor="VannaDii"
LABEL org.opencontainers.image.url="https://revaer.com"
LABEL org.opencontainers.image.source="https://github.com/VannaDii/Revaer"
LABEL org.opencontainers.image.documentation="https://revaer.com/docs"
LABEL org.opencontainers.image.licenses="MIT AND GPL-3.0-or-later AND LGPL-2.1-or-later"
LABEL org.opencontainers.image.version="${VERSION}"
LABEL org.opencontainers.image.revision="${REVISION}"
LABEL org.opencontainers.image.created="${BUILD_DATE}"
LABEL org.opencontainers.image.authors="VannaDii"
LABEL org.opencontainers.image.base.name="alpine:3.23"
LABEL revaer.rust.version="1.91.0"
LABEL revaer.rust.edition="2024"
LABEL revaer.alpine.version="3.23"
LABEL revaer.homepage="https://revaer.com"
LABEL revaer.support="https://revaer.com"
LABEL revaer.api.version="v1"
LABEL revaer.media.license_mode="redistributable-gplv3-runtime"
LABEL revaer.media.source_offer="/app/compliance/SOURCE-OFFER.txt"
LABEL revaer.media.third_party_notices="/app/compliance/THIRD-PARTY-NOTICES.md"
LABEL revaer.media.sbom="/app/compliance/media-runtime-inventory.spdx.json"
LABEL revaer.media.inventory="/app/compliance/media-runtime-inventory.spdx.json"
LABEL revaer.media.exiftool_exception="/app/compliance/exiftool-exception.md"

USER revaer
ENTRYPOINT ["/usr/local/bin/revaer-app"]
