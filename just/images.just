docker-build:
    platforms="${PLATFORMS:-linux/amd64,linux/arm64}"; \
    version="${VERSION:-dev.$(date -u +%y%m%d).$(git rev-parse --short HEAD)}"; \
    tags="--tag revaer:latest --tag revaer:${version}"; \
    builder="${BUILDX_BUILDER:-revaer-builder}"; \
    if ! docker buildx inspect "$builder" >/dev/null 2>&1; then \
        docker buildx create --name "$builder" --driver docker-container --use; \
    else \
        docker buildx use "$builder"; \
    fi; \
    if printf "%s" "$platforms" | grep -q ','; then \
        mkdir -p artifacts; \
        docker buildx build --builder "$builder" --platform "$platforms" $tags \
            --output=type=oci,dest=artifacts/revaer-${version}.oci \
            .; \
    else \
        docker buildx build --builder "$builder" --platform "$platforms" $tags \
            --load \
            .; \
    fi

docker-scan:
    if ! command -v trivy >/dev/null 2>&1; then \
        echo "trivy not installed; install it to run this scan" >&2; \
        exit 1; \
    fi
    trivy image --exit-code 1 --severity HIGH,CRITICAL revaer:ci

image-build-push:
    bash scripts/image-release.sh build-push

image-build-verify:
    bash scripts/image-release.sh build-verify

image-inventory:
    bash scripts/image-release.sh inventory

image-scan:
    bash scripts/image-release.sh scan

image-sign-attest:
    bash scripts/image-release.sh sign-attest

image-attestation-verify:
    bash scripts/image-release.sh verify-attestation

image-manifest-create:
    bash scripts/image-release.sh create-manifest

image-manifest-verify:
    bash scripts/image-release.sh verify-manifest

image-manifest-sign:
    bash scripts/image-release.sh sign-manifest
