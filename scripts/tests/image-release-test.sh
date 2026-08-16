#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-image-release.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT
mkdir -p "${test_root}/bin" "${test_root}/evidence"

cat > "${test_root}/bin/docker" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'docker %s\n' "$*" >> "${MOCK_COMMAND_LOG}"
if [[ "${1:-}" == "buildx" && "${2:-}" == "build" ]]; then
  while [[ "$#" -gt 0 ]]; do
    if [[ "$1" == "--metadata-file" ]]; then
      printf '{"containerimage.digest":"%s"}\n' \
        "${MOCK_METADATA_DIGEST:-sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}" > "$2"
      break
    fi
    shift
  done
elif [[ "${1:-}" == "buildx" && "${2:-}" == "imagetools" && "${3:-}" == "inspect" ]]; then
  printf '%s\n' "${MOCK_RESOLVED_DIGEST:-sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"
fi
MOCK
cat > "${test_root}/bin/jq" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "${MOCK_METADATA_DIGEST:-sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"
MOCK
cat > "${test_root}/bin/cosign" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'cosign %s\n' "$*" >> "${MOCK_COMMAND_LOG}"
if [[ "${1:-}" == "verify-attestation" ]]; then
  printf '{"verified":true}\n'
fi
MOCK
cat > "${test_root}/bin/trivy" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail
printf 'TRIVY_IMAGE_SRC=%s TRIVY_PLATFORM=%s trivy %s\n' \
  "${TRIVY_IMAGE_SRC:-}" "${TRIVY_PLATFORM:-}" "$*" >> "${MOCK_COMMAND_LOG}"
while [[ "$#" -gt 0 ]]; do
  if [[ "$1" == "--output" ]]; then
    printf '{"runs":[],"packages":[]}\n' > "$2"
    shift 2
  else
    shift
  fi
done
exit "${MOCK_TRIVY_STATUS:-0}"
MOCK
chmod +x \
  "${test_root}/bin/docker" \
  "${test_root}/bin/jq" \
  "${test_root}/bin/cosign" \
  "${test_root}/bin/trivy"

digest_a="sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
digest_b="sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
image_reference="ghcr.io/vannadii/revaer@${digest_a}"

common_environment=(
  "PATH=${test_root}/bin:${PATH}"
  "MOCK_COMMAND_LOG=${test_root}/commands.log"
  "REPOSITORY_OWNER=VannaDii"
  "IMAGE_NAME=revaer"
  "VERSION_TAG=pr-195-fixture"
  "ALIAS_TAG=pr-195"
  "INCLUDE_SHA_TAG=false"
)

# Publish-authorized build: push, attest, resolve the registry digest, and emit an immutable reference.
: > "${test_root}/commands.log"
: > "${test_root}/publish-output"
env "${common_environment[@]}" \
  MOCK_METADATA_DIGEST="${digest_a}" \
  MOCK_RESOLVED_DIGEST="${digest_a}" \
  RUNNER_TEMP="${test_root}" \
  PLATFORM=linux/amd64 \
  ARCH_TAG=amd64 \
  RUST_TARGET=x86_64-unknown-linux-musl \
  GITHUB_OUTPUT="${test_root}/publish-output" \
  bash "${repo_root}/scripts/image-release.sh" build-push
grep -Fq 'docker buildx build --platform linux/amd64' "${test_root}/commands.log"
grep -Fq -- '--push --attest type=provenance,mode=max --attest type=sbom --metadata-file' \
  "${test_root}/commands.log"
grep -Fq 'docker buildx imagetools inspect ghcr.io/vannadii/revaer:' \
  "${test_root}/commands.log"
grep -Fqx "image_digest=${digest_a}" "${test_root}/publish-output"
grep -Fqx "image_reference=${image_reference}" "${test_root}/publish-output"

if env "${common_environment[@]}" \
  MOCK_METADATA_DIGEST="${digest_a}" \
  MOCK_RESOLVED_DIGEST="${digest_b}" \
  RUNNER_TEMP="${test_root}" \
  PLATFORM=linux/amd64 \
  ARCH_TAG=amd64 \
  RUST_TARGET=x86_64-unknown-linux-musl \
  GITHUB_OUTPUT="${test_root}/mismatch-output" \
  bash "${repo_root}/scripts/image-release.sh" build-push >/dev/null 2>&1; then
  echo "Image publication accepted mismatched metadata and registry digests" >&2
  exit 1
fi

# Digest-qualified inventory and scan must use the remote image source.
env "${common_environment[@]}" \
  IMAGE_REFERENCE="${image_reference}" \
  IMAGE_SOURCE=remote \
  PLATFORM=linux/amd64 \
  TRIVY_OUTPUT_PATH="${test_root}/inventory.spdx.json" \
  bash "${repo_root}/scripts/image-release.sh" inventory
test -s "${test_root}/inventory.spdx.json"
grep -Fq "TRIVY_IMAGE_SRC=remote TRIVY_PLATFORM=linux/amd64 trivy image" \
  "${test_root}/commands.log"
grep -Fq -- "--format spdx-json --output ${test_root}/inventory.spdx.json --exit-code 0 ${image_reference}" \
  "${test_root}/commands.log"

env "${common_environment[@]}" \
  IMAGE_REFERENCE="${image_reference}" \
  IMAGE_SOURCE=remote \
  PLATFORM=linux/amd64 \
  TRIVY_OUTPUT_PATH="${test_root}/publish.sarif" \
  bash "${repo_root}/scripts/image-release.sh" scan
test -s "${test_root}/publish.sarif"
grep -Fq -- '--severity HIGH,CRITICAL --exit-code 0' "${test_root}/commands.log"

if env "${common_environment[@]}" \
  IMAGE_REFERENCE=ghcr.io/vannadii/revaer:mutable \
  IMAGE_SOURCE=remote \
  PLATFORM=linux/amd64 \
  bash "${repo_root}/scripts/image-release.sh" scan >/dev/null 2>&1; then
  echo "Remote image scan accepted a mutable tag" >&2
  exit 1
fi

# The signed predicate and its verification remain bound to the immutable image reference.
printf '{"predicate":"fixture"}\n' > "${test_root}/evidence/final-image-compliance-bundle.json"
env "${common_environment[@]}" \
  IMAGE_REFERENCE="${image_reference}" \
  COMPLIANCE_PREDICATE="${test_root}/evidence/final-image-compliance-bundle.json" \
  bash "${repo_root}/scripts/image-release.sh" sign-attest
grep -Fq "cosign sign --yes ${image_reference}" "${test_root}/commands.log"
grep -Fq "cosign attest --yes --predicate ${test_root}/evidence/final-image-compliance-bundle.json" \
  "${test_root}/commands.log"

env "${common_environment[@]}" \
  IMAGE_REFERENCE="${image_reference}" \
  GITHUB_REPOSITORY=VannaDii/revaer \
  ATTESTATION_OUTPUT="${test_root}/evidence/verified-attestation.json" \
  bash "${repo_root}/scripts/image-release.sh" verify-attestation
test -s "${test_root}/evidence/verified-attestation.json"
grep -Fq "cosign verify-attestation" "${test_root}/commands.log"
grep -Fq -- "--type https://revaer.com/attestations/final-image-compliance/v2 ${image_reference}" \
  "${test_root}/commands.log"

env "${common_environment[@]}" \
  bash "${repo_root}/scripts/image-release.sh" create-manifest
grep -Fq 'docker buildx imagetools create' "${test_root}/commands.log"

env "${common_environment[@]}" \
  bash "${repo_root}/scripts/image-release.sh" sign-manifest
grep -Fq 'cosign sign --yes ghcr.io/vannadii/revaer:pr-195-fixture' \
  "${test_root}/commands.log"

# Verification-only PR execution builds and scans locally without registry mutation or signing.
: > "${test_root}/commands.log"
: > "${test_root}/verify-output"
env "${common_environment[@]}" \
  PLATFORM=linux/amd64 \
  ARCH_TAG=amd64 \
  RUST_TARGET=x86_64-unknown-linux-musl \
  GITHUB_OUTPUT="${test_root}/verify-output" \
  bash "${repo_root}/scripts/image-release.sh" build-verify
grep -Fq 'docker buildx build --platform linux/amd64 --tag revaer:verify-amd64 --load' \
  "${test_root}/commands.log"
grep -Fqx 'image_tag=revaer:verify-amd64' "${test_root}/verify-output"

env "${common_environment[@]}" \
  IMAGE_REFERENCE=revaer:verify-amd64 \
  IMAGE_SOURCE=docker \
  PLATFORM=linux/amd64 \
  TRIVY_OUTPUT_PATH="${test_root}/verify.sarif" \
  bash "${repo_root}/scripts/image-release.sh" scan
test -s "${test_root}/verify.sarif"
grep -Fq 'TRIVY_IMAGE_SRC=docker TRIVY_PLATFORM=linux/amd64 trivy image' \
  "${test_root}/commands.log"

env "${common_environment[@]}" \
  bash "${repo_root}/scripts/image-release.sh" verify-manifest > "${test_root}/manifest-verification.txt"
grep -Fq 'Verified manifest inputs without publishing:' "${test_root}/manifest-verification.txt"
if grep -Eq -- '--push|docker buildx imagetools create|cosign (sign|attest)' "${test_root}/commands.log"; then
  echo "Verification-only image path attempted a registry mutation or signature" >&2
  exit 1
fi

if env "${common_environment[@]}" \
  MOCK_TRIVY_STATUS=2 \
  IMAGE_REFERENCE=revaer:verify-amd64 \
  IMAGE_SOURCE=docker \
  PLATFORM=linux/amd64 \
  TRIVY_OUTPUT_PATH="${test_root}/trivy-error.sarif" \
  bash "${repo_root}/scripts/image-release.sh" scan >/dev/null 2>&1; then
  echo "Image scan concealed a Trivy execution failure" >&2
  exit 1
fi
test -s "${test_root}/trivy-error.sarif"

if env "${common_environment[@]}" \
  PLATFORM=linux/amd64 \
  ARCH_TAG=arm64 \
  RUST_TARGET=x86_64-unknown-linux-musl \
  GITHUB_OUTPUT="${test_root}/invalid-output" \
  bash "${repo_root}/scripts/image-release.sh" build-verify >/dev/null 2>&1; then
  echo "Image build accepted an inconsistent platform matrix" >&2
  exit 1
fi

printf '%s\n' "Image release recipe tests passed"
