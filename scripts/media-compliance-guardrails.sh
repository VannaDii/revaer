#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

failures=0

report_failure() {
  local message="$1"
  printf 'Media compliance guardrail failed: %s\n' "${message}" >&2
  failures=1
}

require_file() {
  local path="$1"
  if [[ ! -f "${path}" ]]; then
    report_failure "missing ${path}"
  fi
}

require_dockerfile_token() {
  local token="$1"
  if ! grep -Eq "(^|[[:space:]])${token}([[:space:]\\\\]|$)" Dockerfile; then
    report_failure "Dockerfile runtime image must install ${token}"
  fi
}

require_dockerfile_label() {
  local label="$1"
  if ! grep -Fq "LABEL ${label}=" Dockerfile; then
    report_failure "Dockerfile missing ${label} label"
  fi
}

require_workflow_token() {
  local path="$1"
  local token="$2"
  local message="$3"
  if ! grep -Fq "${token}" "${path}"; then
    report_failure "${message}"
  fi
}

if [[ ! -f Dockerfile ]]; then
  report_failure "Dockerfile is required for media runtime compliance"
else
  for package in \
    ffmpeg \
    ffplay \
    exiftool \
    mediainfo \
    mkvtoolnix \
    bento4 \
    libass \
    x264-libs \
    x265-libs \
    libdav1d \
    opus \
    libvorbis \
    libtheora \
    fontconfig \
    font-dejavu \
    gnutls; do
    require_dockerfile_token "${package}"
  done

  for label in \
    revaer.media.license_mode \
    revaer.media.source_offer \
    revaer.media.third_party_notices \
    revaer.media.sbom \
    revaer.media.inventory \
    revaer.media.exiftool_exception \
    revaer.media.source_compliance_bundle; do
    require_dockerfile_label "${label}"
  done
fi

nonfree_scan_paths=(Dockerfile)
while IFS= read -r path; do
  nonfree_scan_paths+=("${path}")
done < <(
  git ls-files release/scripts 'release/*.js' scripts \
    | grep -E '\.(js|sh)$' \
    | grep -v '^scripts/media-compliance-guardrails\.sh$' \
    || true
)

if git grep -n -- '--enable-nonfree' -- "${nonfree_scan_paths[@]}" >/tmp/revaer-media-nonfree.matches 2>/dev/null; then
  cat /tmp/revaer-media-nonfree.matches >&2
  report_failure "default media runtime must not use --enable-nonfree"
fi
rm -f /tmp/revaer-media-nonfree.matches

require_file release/media-compliance/SOURCE-OFFER.txt
require_file release/media-compliance/THIRD-PARTY-NOTICES.md
require_file release/media-compliance/media-runtime-inventory.spdx.json
require_file release/media-compliance/exiftool-exception.md
require_file release/media-compliance/final-image-compliance-bundle.json

require_workflow_token \
  .github/workflows/build-images.yml \
  "final_image_compliance_bundle:" \
  "image workflow must require an explicit final-image compliance bundle input"
require_workflow_token \
  .github/workflows/build-images.yml \
  "if: inputs.final_image_compliance_bundle != ''" \
  "image publication jobs must stay disabled until the final-image compliance bundle exists"
require_workflow_token \
  .github/workflows/build-images.yml \
  "Validate final-image compliance bundle" \
  "image workflow must validate the final-image compliance bundle before pushing images"
require_workflow_token \
  .github/workflows/build-images.yml \
  "scripts/validate-final-image-compliance-bundle.sh" \
  "image workflow must run structured final-image compliance bundle validation"
require_workflow_token \
  .github/workflows/build-images.yml \
  "release/media-compliance/final-image-compliance-bundle.json" \
  "image workflow must stage the validated final-image compliance bundle into the Docker build context"
require_workflow_token \
  .github/workflows/build-images.yml \
  "Build Image (verification only)" \
  "PR image checks must build without publishing when the final-image compliance bundle is absent"
require_workflow_token \
  .github/workflows/build-images.yml \
  "Verify multi-arch manifest inputs" \
  "PR image checks must validate manifest inputs without publishing when the final-image compliance bundle is absent"
require_workflow_token \
  .github/workflows/build-images.yml \
  "Verify packaged Helm chart without publishing" \
  "PR image checks must verify Helm packaging without publishing when the final-image compliance bundle is absent"

if grep -Fq "final_image_compliance_bundle:" .github/workflows/ci.yml; then
  report_failure "CI image publication must remain disabled until release automation generates a final-image compliance bundle"
fi

if command -v jq >/dev/null 2>&1 \
   && [[ -f release/media-compliance/media-runtime-inventory.spdx.json ]]; then
  for package in \
    bento4 \
    ca-certificates \
    curl \
    exiftool \
    ffmpeg \
    ffplay \
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
    x265-libs; do
    if ! jq -e --arg package "${package}" '
      .packages[]
      | select(.name == $package)
      | select((.versionInfo | type == "string" and length > 0)
        and (.licenseConcluded | type == "string" and length > 0 and . != "NOASSERTION")
        and (.licenseDeclared | type == "string" and length > 0 and . != "NOASSERTION")
        and any(.checksums[]?; .algorithm == "SHA256"
          and (.checksumValue | test("^[0-9a-f]{64}$"))))
    ' release/media-compliance/media-runtime-inventory.spdx.json >/dev/null; then
      report_failure "SPDX inventory must include version, license, and SHA-256 for ${package}"
    fi
  done

  if jq -e '
    .packages[]
    | select(.name == "alpine-media-runtime-packages"
      or .licenseConcluded == "NOASSERTION"
      or .licenseDeclared == "NOASSERTION")
  ' release/media-compliance/media-runtime-inventory.spdx.json >/dev/null; then
    report_failure "SPDX inventory must not collapse runtime packages or use NOASSERTION package licenses"
  fi
else
  report_failure "jq is required to validate media runtime SPDX inventory"
fi

if [[ "${failures}" -ne 0 ]]; then
  exit 1
fi
