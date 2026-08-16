#!/usr/bin/env bash
set -euo pipefail

operation="${1:-}"
if [[ -z "${operation}" || "$#" -ne 1 ]]; then
  echo "usage: image-release.sh <build-push|build-verify|inventory|scan|sign-attest|verify-attestation|create-manifest|verify-manifest|sign-manifest>" >&2
  exit 64
fi

require_env() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    printf 'Required image release environment variable is empty: %s\n' "${name}" >&2
    exit 64
  fi
}

validate_image_name_and_version() {
  require_env IMAGE_NAME
  require_env VERSION_TAG
  if [[ ! "${IMAGE_NAME}" =~ ^[a-z0-9]+([._/-][a-z0-9]+)*$ ]]; then
    echo "IMAGE_NAME is not a valid lowercase image name" >&2
    exit 64
  fi
  if [[ ! "${VERSION_TAG}" =~ ^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$ ]]; then
    echo "VERSION_TAG is not a valid OCI tag" >&2
    exit 64
  fi
}

validate_release_inputs() {
  validate_image_name_and_version
  require_env REPOSITORY_OWNER
  if [[ ! "${REPOSITORY_OWNER}" =~ ^[A-Za-z0-9][A-Za-z0-9-]*$ ]]; then
    echo "REPOSITORY_OWNER is not a valid registry namespace" >&2
    exit 64
  fi
  if [[ -n "${ALIAS_TAG:-}" && ! "${ALIAS_TAG}" =~ ^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$ ]]; then
    echo "ALIAS_TAG is not a valid OCI tag" >&2
    exit 64
  fi
  if [[ "${INCLUDE_SHA_TAG:-false}" != "true" && "${INCLUDE_SHA_TAG:-false}" != "false" ]]; then
    echo "INCLUDE_SHA_TAG must be true or false" >&2
    exit 64
  fi
}

validate_platform() {
  require_env PLATFORM
  case "${PLATFORM}" in
    linux/amd64 | linux/arm64) ;;
    *)
      echo "PLATFORM must be linux/amd64 or linux/arm64" >&2
      exit 64
      ;;
  esac
}

validate_build_matrix() {
  validate_platform
  require_env ARCH_TAG
  require_env RUST_TARGET
  case "${PLATFORM}:${ARCH_TAG}:${RUST_TARGET}" in
    linux/amd64:amd64:x86_64-unknown-linux-musl | \
      linux/arm64:arm64:aarch64-unknown-linux-musl) ;;
    *)
      echo "Image platform, architecture tag, and Rust target do not match the audited matrix" >&2
      exit 64
      ;;
  esac
}

require_digest_reference() {
  require_env IMAGE_REFERENCE
  if [[ ! "${IMAGE_REFERENCE}" =~ ^ghcr\.io/[a-z0-9][a-z0-9._/-]*@sha256:[0-9a-f]{64}$ ]]; then
    echo "IMAGE_REFERENCE must be a digest-qualified GHCR reference" >&2
    exit 64
  fi
}

validate_digest() {
  local digest="$1"
  local source="$2"
  if [[ ! "${digest}" =~ ^sha256:[0-9a-f]{64}$ ]]; then
    printf '%s returned an invalid image digest: %s\n' "${source}" "${digest}" >&2
    exit 1
  fi
}

image_base() {
  local owner
  owner="$(printf '%s' "${REPOSITORY_OWNER}" | tr '[:upper:]' '[:lower:]')"
  printf 'ghcr.io/%s/%s\n' "${owner}" "${IMAGE_NAME}"
}

short_sha() {
  git rev-parse --short HEAD
}

manifest_tags() {
  local base="$1"
  local sha="$2"
  MANIFEST_TAGS=("${base}:${VERSION_TAG}")
  if [[ -n "${ALIAS_TAG:-}" ]]; then
    MANIFEST_TAGS+=("${base}:${ALIAS_TAG}")
  fi
  if [[ "${INCLUDE_SHA_TAG:-false}" == "true" ]]; then
    MANIFEST_TAGS+=("${base}:${sha}")
  fi
}

case "${operation}" in
  build-push)
    validate_release_inputs
    validate_build_matrix
    require_env GITHUB_OUTPUT
    git_sha_short="$(short_sha)"
    base="$(image_base)"
    image_tag="${base}:${git_sha_short}-${ARCH_TAG}"
    build_date="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    temporary_root="${RUNNER_TEMP:-${TMPDIR:-/tmp}}"
    metadata_file="$(mktemp "${temporary_root%/}/revaer-build-metadata.XXXXXX")"
    trap 'rm -f "${metadata_file}"' EXIT
    docker buildx build \
      --platform "${PLATFORM}" \
      --tag "${image_tag}" \
      --push \
      --attest type=provenance,mode=max \
      --attest type=sbom \
      --metadata-file "${metadata_file}" \
      --build-arg "RUST_TARGET=${RUST_TARGET}" \
      --build-arg "BUILD_DATE=${build_date}" \
      --build-arg "VERSION=${VERSION_TAG}" \
      --build-arg "REVISION=${git_sha_short}" \
      .
    metadata_digest="$(jq -er '.["containerimage.digest"]' "${metadata_file}")"
    resolved_digest="$(docker buildx imagetools inspect "${image_tag}" --format '{{.Manifest.Digest}}')"
    validate_digest "${metadata_digest}" "Build metadata"
    validate_digest "${resolved_digest}" "Registry"
    if [[ "${metadata_digest}" != "${resolved_digest}" ]]; then
      echo "Build metadata digest ${metadata_digest} does not match registry digest ${resolved_digest}." >&2
      exit 1
    fi
    {
      printf 'image_tag=%s\n' "${image_tag}"
      printf 'image_digest=%s\n' "${resolved_digest}"
      printf 'image_reference=%s@%s\n' "${base}" "${resolved_digest}"
    } >> "${GITHUB_OUTPUT}"
    ;;
  build-verify)
    validate_image_name_and_version
    validate_build_matrix
    require_env GITHUB_OUTPUT
    git_sha_short="$(short_sha)"
    image_tag="${IMAGE_NAME}:verify-${ARCH_TAG}"
    build_date="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    docker buildx build \
      --platform "${PLATFORM}" \
      --tag "${image_tag}" \
      --load \
      --build-arg "RUST_TARGET=${RUST_TARGET}" \
      --build-arg "BUILD_DATE=${build_date}" \
      --build-arg "VERSION=${VERSION_TAG}" \
      --build-arg "REVISION=${git_sha_short}" \
      .
    printf 'image_tag=%s\n' "${image_tag}" >> "${GITHUB_OUTPUT}"
    ;;
  inventory)
    require_digest_reference
    validate_platform
    if [[ "${IMAGE_SOURCE:-}" != "remote" ]]; then
      echo "Digest-qualified inventory requires IMAGE_SOURCE=remote" >&2
      exit 64
    fi
    output_path="${TRIVY_OUTPUT_PATH:-image-package-inventory.spdx.json}"
    TRIVY_IMAGE_SRC=remote TRIVY_PLATFORM="${PLATFORM}" \
      trivy image \
        --format spdx-json \
        --output "${output_path}" \
        --exit-code 0 \
        "${IMAGE_REFERENCE}"
    if [[ ! -s "${output_path}" ]]; then
      echo "Trivy did not produce a nonempty image package inventory" >&2
      exit 1
    fi
    ;;
  scan)
    require_env IMAGE_REFERENCE
    validate_platform
    case "${IMAGE_SOURCE:-}" in
      remote) require_digest_reference ;;
      docker) ;;
      *)
        echo "IMAGE_SOURCE must be remote or docker" >&2
        exit 64
        ;;
    esac
    output_path="${TRIVY_OUTPUT_PATH:-trivy-results.sarif}"
    scan_status=0
    TRIVY_IMAGE_SRC="${IMAGE_SOURCE}" TRIVY_PLATFORM="${PLATFORM}" \
      trivy image \
        --format sarif \
        --output "${output_path}" \
        --severity HIGH,CRITICAL \
        --exit-code 0 \
        "${IMAGE_REFERENCE}" || scan_status=$?
    if [[ ! -s "${output_path}" ]]; then
      echo "Trivy did not produce a nonempty SARIF report" >&2
      exit 1
    fi
    exit "${scan_status}"
    ;;
  sign-attest)
    require_digest_reference
    require_env COMPLIANCE_PREDICATE
    if [[ ! -s "${COMPLIANCE_PREDICATE}" ]]; then
      echo "Compliance predicate is missing or empty: ${COMPLIANCE_PREDICATE}" >&2
      exit 64
    fi
    cosign sign --yes "${IMAGE_REFERENCE}"
    cosign attest --yes \
      --predicate "${COMPLIANCE_PREDICATE}" \
      --type https://revaer.com/attestations/final-image-compliance/v2 \
      "${IMAGE_REFERENCE}"
    ;;
  verify-attestation)
    require_digest_reference
    require_env GITHUB_REPOSITORY
    require_env ATTESTATION_OUTPUT
    if [[ ! "${GITHUB_REPOSITORY}" =~ ^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$ ]]; then
      echo "GITHUB_REPOSITORY is not a valid owner/repository identifier" >&2
      exit 64
    fi
    identity="^https://github.com/${GITHUB_REPOSITORY}/.github/workflows/(pr|ci)\\.yml@"
    cosign verify-attestation \
      --certificate-identity-regexp "${identity}" \
      --certificate-oidc-issuer https://token.actions.githubusercontent.com \
      --type https://revaer.com/attestations/final-image-compliance/v2 \
      "${IMAGE_REFERENCE}" > "${ATTESTATION_OUTPUT}"
    if [[ ! -s "${ATTESTATION_OUTPUT}" ]]; then
      echo "Cosign did not produce nonempty verified attestation evidence" >&2
      exit 1
    fi
    ;;
  create-manifest)
    validate_release_inputs
    git_sha_short="$(short_sha)"
    base="$(image_base)"
    manifest_tags "${base}" "${git_sha_short}"
    tag_arguments=()
    for tag in "${MANIFEST_TAGS[@]}"; do
      tag_arguments+=(--tag "${tag}")
    done
    docker buildx imagetools create \
      "${tag_arguments[@]}" \
      "${base}:${git_sha_short}-amd64" \
      "${base}:${git_sha_short}-arm64"
    ;;
  verify-manifest)
    validate_release_inputs
    git_sha_short="$(short_sha)"
    base="$(image_base)"
    manifest_tags "${base}" "${git_sha_short}"
    echo "Verified manifest inputs without publishing:"
    printf '  %s\n' "${MANIFEST_TAGS[@]}"
    printf '  %s\n' "${base}:${git_sha_short}-amd64" "${base}:${git_sha_short}-arm64"
    ;;
  sign-manifest)
    validate_release_inputs
    git_sha_short="$(short_sha)"
    base="$(image_base)"
    manifest_tags "${base}" "${git_sha_short}"
    for tag in "${MANIFEST_TAGS[@]}"; do
      cosign sign --yes "${tag}"
    done
    ;;
  *)
    printf 'Unsupported image release operation: %s\n' "${operation}" >&2
    exit 64
    ;;
esac
