# syntax=docker/dockerfile:1.7@sha256:a57df69d0ea827fb7266491f2813635de6f17269be881f696fbfdf2d83dda33e

ARG RUST_BUILDER_IMAGE=docker.io/library/rust@sha256:5dc2af9dd547c33f64d5fc1d299ab93b51f39eaa16c426c476b990ce6caf5b3e
ARG ALPINE_RUNTIME_IMAGE=docker.io/library/alpine@sha256:fd791d74b68913cbb027c6546007b3f0d3bc45125f797758156952bc2d6daf40
ARG RUST_VERSION=1.96.0
ARG ALPINE_VERSION=3.23.5

## Build stage ---------------------------------------------------------------
FROM ${RUST_BUILDER_IMAGE} AS builder
WORKDIR /workspace

ARG RUST_BUILDER_IMAGE
ARG RUST_VERSION
ARG RUST_TARGET
ARG TARGETARCH
ARG BUILD_DATE
ARG VERSION=0.1.0
ARG REVISION=main

COPY .github/build-inputs.env /tmp/revaer-build-inputs.env
RUN set -eux; \
    requested_builder_image="${RUST_BUILDER_IMAGE}"; \
    requested_rust_version="${RUST_VERSION}"; \
    . /tmp/revaer-build-inputs.env; \
    test "${requested_builder_image}" = "${RUST_BUILDER_IMAGE}"; \
    test "${requested_rust_version}" = "${RUST_VERSION}"; \
    test "$(rustc --version | awk '{print $2}')" = "${RUST_VERSION}"; \
    printf '%s\n' "${ALPINE_BUILDER_PACKAGES}" | xargs apk add --no-cache; \
    rm /tmp/revaer-build-inputs.env

COPY . .

# Verify and install only the components listed for the repository's exact toolchain.
RUN set -eux; \
    configured_toolchain="$(sed -n 's/^channel = "\([^"]*\)"$/\1/p' rust-toolchain.toml)"; \
    test "${configured_toolchain}" = "${RUST_VERSION}"; \
    rustup component add --toolchain "${configured_toolchain}" rustfmt clippy llvm-tools-preview

# Link dynamically against musl for third-party libs (libtorrent/openssl) on Alpine.
ENV RUSTFLAGS="-C target-feature=-crt-static"

RUN set -eux; \
  case "${TARGETARCH}" in \
    amd64) expected_rust_target="x86_64-unknown-linux-musl" ;; \
    arm64) expected_rust_target="aarch64-unknown-linux-musl" ;; \
    *) echo "unsupported target architecture: ${TARGETARCH}" >&2; exit 1 ;; \
  esac; \
  rust_target="${RUST_TARGET:-"${expected_rust_target}"}"; \
  test "${rust_target}" = "${expected_rust_target}"; \
  rustup target add "${rust_target}"; \
  # Force a fixed target dir so we can normalize the output path
  export CARGO_TARGET_DIR=/workspace/target; \
  cargo build --release --locked --package revaer-app --target "${rust_target}"; \
  # Normalize to /workspace/target/release/revaer-app regardless of target triple
  mkdir -p /workspace/target/release; \
  cp "/workspace/target/${rust_target}/release/revaer-app" \
     "/workspace/target/release/revaer-app"; \
  ls -l /workspace/target/release

## Runtime stage -------------------------------------------------------------
FROM ${ALPINE_RUNTIME_IMAGE} AS runtime

ARG ALPINE_RUNTIME_IMAGE
ARG ALPINE_VERSION
ARG RUST_VERSION
ARG BUILD_DATE
ARG VERSION=0.1.0
ARG REVISION=main

COPY .github/build-inputs.env /tmp/revaer-build-inputs.env
RUN set -eux; \
    requested_runtime_image="${ALPINE_RUNTIME_IMAGE}"; \
    requested_alpine_version="${ALPINE_VERSION}"; \
    . /tmp/revaer-build-inputs.env; \
    test "${requested_runtime_image}" = "${ALPINE_RUNTIME_IMAGE}"; \
    test "${requested_alpine_version}" = "${ALPINE_VERSION}"; \
    test "$(cut -d. -f1-3 /etc/alpine-release)" = "${ALPINE_VERSION}"; \
    addgroup -S revaer && adduser -S revaer -G revaer \
    && printf '%s\n' "${ALPINE_RUNTIME_PACKAGES}" | xargs apk add --no-cache \
    && mkdir -p /app/compliance /data /config \
    && chown root:root /app /app/compliance \
    && chmod 0755 /app /app/compliance \
    && chown -R revaer:revaer /data /config \
    && rm /tmp/revaer-build-inputs.env

WORKDIR /app

# Always copy from normalized path
COPY --from=builder --chown=root:root --chmod=0555 /workspace/target/release/revaer-app /usr/local/bin/revaer-app
COPY --from=builder --chown=root:root /workspace/docs /app/docs
COPY --from=builder --chown=root:root /workspace/config /app/config
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/SOURCE-OFFER.txt /app/compliance/SOURCE-OFFER.txt
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/THIRD-PARTY-NOTICES.md /app/compliance/THIRD-PARTY-NOTICES.md
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/media-runtime-inventory.spdx.json /app/compliance/media-runtime-inventory.spdx.json
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/exiftool-exception.md /app/compliance/exiftool-exception.md
COPY --from=builder --chown=root:root --chmod=0444 /workspace/.github/build-inputs.env /app/compliance/build-inputs.env

RUN install -d -o revaer -g revaer -m 0755 /app/docs/api

VOLUME ["/data", "/config"]
ENV RUST_LOG=info
ENV REVAER_MEDIA_WORKSPACE_ROOT=/data/media-workspaces
ENV LD_LIBRARY_PATH=/usr/local/lib

HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD curl -fsS http://127.0.0.1:7070/health/live || exit 1

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
LABEL org.opencontainers.image.base.name="${ALPINE_RUNTIME_IMAGE}"
LABEL revaer.rust.version="${RUST_VERSION}"
LABEL revaer.rust.edition="2024"
LABEL revaer.alpine.version="${ALPINE_VERSION}"
LABEL revaer.homepage="https://revaer.com"
LABEL revaer.support="https://revaer.com"
LABEL revaer.api.version="v1"
LABEL revaer.media.license_mode="redistributable-gplv3-runtime"
LABEL revaer.media.source_offer="/app/compliance/SOURCE-OFFER.txt"
LABEL revaer.media.third_party_notices="/app/compliance/THIRD-PARTY-NOTICES.md"
LABEL revaer.media.sbom="/app/compliance/media-runtime-inventory.spdx.json"
LABEL revaer.media.inventory="/app/compliance/media-runtime-inventory.spdx.json"
LABEL revaer.media.exiftool_exception="/app/compliance/exiftool-exception.md"
LABEL revaer.media.build_inputs="/app/compliance/build-inputs.env"
LABEL revaer.media.compliance_attestation="https://revaer.com/attestations/final-image-compliance/v2"

USER revaer
ENTRYPOINT ["/usr/local/bin/revaer-app"]
