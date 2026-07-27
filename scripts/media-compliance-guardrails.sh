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

require_runtime_package() {
  local package="$1"
  if [[ " ${ALPINE_RUNTIME_PACKAGES:-} " != *" ${package}="* ]]; then
    report_failure "build-input manifest must pin runtime package ${package}"
  fi
}

runtime_package_version() {
  local package="$1"
  local entry
  for entry in ${ALPINE_RUNTIME_PACKAGES:-}; do
    if [[ "${entry}" == "${package}="* ]]; then
      printf '%s' "${entry#*=}"
      return 0
    fi
  done
  return 1
}

require_dockerfile_label() {
  local label="$1"
  if ! grep -Fq "LABEL ${label}=" Dockerfile; then
    report_failure "Dockerfile missing ${label} label"
  fi
}

require_file .github/build-inputs.env
if [[ -f .github/build-inputs.env ]]; then
  set -a
  source .github/build-inputs.env
  set +a
fi

if [[ -f .github/actions/setup-revaer/action.yml ]]; then
  grep -Fq "npm install --global --ignore-scripts --no-audit --no-fund \"npm@${NPM_VERSION:-missing}\"" \
    .github/actions/setup-revaer/action.yml \
    || report_failure "setup action npm pin must match the exact build-input manifest version"
fi

if [[ ! -f Dockerfile ]]; then
  report_failure "Dockerfile is required for media runtime compliance"
else
  grep -Fqx "# syntax=${DOCKERFILE_FRONTEND_IMAGE:-missing}" Dockerfile \
    || report_failure "Dockerfile frontend must match the exact build-input manifest digest"
  grep -Fqx "ARG RUST_BUILDER_IMAGE=${RUST_BUILDER_IMAGE:-missing}" Dockerfile \
    || report_failure "Dockerfile builder image must match the exact build-input manifest digest"
  grep -Fqx "ARG ALPINE_RUNTIME_IMAGE=${ALPINE_RUNTIME_IMAGE:-missing}" Dockerfile \
    || report_failure "Dockerfile runtime image must match the exact build-input manifest digest"
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
    require_runtime_package "${package}"
  done

  for label in \
    revaer.media.license_mode \
    revaer.media.source_offer \
    revaer.media.third_party_notices \
    revaer.media.sbom \
    revaer.media.inventory \
    revaer.media.exiftool_exception \
    revaer.media.build_inputs \
    revaer.media.compliance_attestation; do
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

matches_file="$(mktemp "${TMPDIR:-/tmp}/revaer-media-nonfree.XXXXXX")"
chmod 0600 "${matches_file}"
cleanup_matches() {
  rm -f "${matches_file}"
}
trap cleanup_matches EXIT
if git grep -n -- '--enable-nonfree' -- "${nonfree_scan_paths[@]}" >"${matches_file}" 2>/dev/null; then
  cat "${matches_file}" >&2
  report_failure "default media runtime must not use --enable-nonfree"
fi
cleanup_matches
trap - EXIT

require_file release/media-compliance/SOURCE-OFFER.txt
require_file release/media-compliance/THIRD-PARTY-NOTICES.md
require_file release/media-compliance/media-runtime-inventory.spdx.json
require_file release/media-compliance/exiftool-exception.md

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
    expected_version="$(runtime_package_version "${package}")" || {
      report_failure "build-input manifest does not resolve ${package}"
      continue
    }
    if ! jq -e --arg package "${package}" --arg version "${expected_version}" '
      .packages[]
      | select(.name == $package)
      | select((.versionInfo == $version)
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
