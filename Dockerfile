# syntax=docker/dockerfile:1.7@sha256:a57df69d0ea827fb7266491f2813635de6f17269be881f696fbfdf2d83dda33e

ARG RUST_BUILDER_IMAGE=docker.io/library/rust@sha256:5dc2af9dd547c33f64d5fc1d299ab93b51f39eaa16c426c476b990ce6caf5b3e
ARG ALPINE_RUNTIME_IMAGE=docker.io/library/alpine@sha256:fd791d74b68913cbb027c6546007b3f0d3bc45125f797758156952bc2d6daf40
ARG UV_IMAGE=ghcr.io/astral-sh/uv:0.12.13@sha256:b485bd65cc2cf1c9a93b3554012c9c3778cf7b1b5fd3d3096ce9e1226c97e1e6
ARG RUST_VERSION=1.96.0
ARG ALPINE_VERSION=3.23.5

FROM ${UV_IMAGE} AS uv

# uv owns the interpreter, environment and locked core package. Normal developer
# environments include all groups; image construction only needs the core CLI.
FROM ${RUST_BUILDER_IMAGE} AS tooling
WORKDIR /workspace
ENV UV_PYTHON_INSTALL_DIR=/opt/revaer-python
ENV UV_PROJECT_ENVIRONMENT=/opt/revaer-venv
ENV UV_LINK_MODE=copy
COPY --from=uv /uv /usr/local/bin/uv
COPY pyproject.toml uv.lock .python-version .uv-version Cargo.toml rust-toolchain.toml ./
COPY tools/src ./tools/src
COPY tools/versions.toml ./tools/versions.toml
COPY .github/build-inputs.env ./.github/build-inputs.env
RUN ["uv", "sync", "--locked", "--no-default-groups", "--managed-python"]

FROM tooling AS builder
ARG RUST_BUILDER_IMAGE
ARG UV_IMAGE
ARG RUST_VERSION
ARG RUST_TARGET
ARG TARGETARCH
ARG BUILD_DATE
ARG VERSION=0.1.0
ARG REVISION=main
COPY . .
RUN ["uv", "run", "--locked", "--no-default-groups", "--", "rv", "container-build"]

FROM ${ALPINE_RUNTIME_IMAGE} AS runtime
ARG ALPINE_RUNTIME_IMAGE
ARG ALPINE_VERSION
ARG RUST_VERSION
ARG BUILD_DATE
ARG VERSION=0.1.0
ARG REVISION=main

# Always copy from normalized path
COPY --from=builder --chown=root:root --chmod=0555 /workspace/target/release/revaer-app /usr/local/bin/revaer-app
COPY --from=builder --chown=root:root /workspace/docs /app/docs
COPY --from=builder --chown=root:root /workspace/config /app/config
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/SOURCE-OFFER.txt /app/compliance/SOURCE-OFFER.txt
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/THIRD-PARTY-NOTICES.md /app/compliance/THIRD-PARTY-NOTICES.md
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/media-runtime-inventory.spdx.json /app/compliance/media-runtime-inventory.spdx.json
COPY --from=builder --chown=root:root --chmod=0444 /workspace/release/media-compliance/exiftool-exception.md /app/compliance/exiftool-exception.md
COPY --from=builder --chown=root:root --chmod=0444 /workspace/.github/build-inputs.env /app/compliance/build-inputs.env

# BuildKit mounts the uv-created environment only for this instruction. The
# final application image retains its existing native runtime package inventory.
WORKDIR /workspace
RUN --mount=type=bind,from=tooling,source=/workspace,target=/workspace \
    --mount=type=bind,from=tooling,source=/opt/revaer-python,target=/opt/revaer-python \
    --mount=type=bind,from=tooling,source=/opt/revaer-venv,target=/opt/revaer-venv \
    ["/opt/revaer-venv/bin/rv", "container-runtime"]
WORKDIR /app

VOLUME ["/data", "/config"]
ENV RUST_LOG=info
ENV LD_LIBRARY_PATH=/usr/local/lib
HEALTHCHECK --interval=30s --timeout=5s --retries=3 \
    CMD ["curl", "-fsS", "http://127.0.0.1:7070/health/full"]

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
