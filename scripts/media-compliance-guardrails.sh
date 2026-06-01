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
  if [ ! -f "${path}" ]; then
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

if [ ! -f Dockerfile ]; then
  report_failure "Dockerfile is required for media runtime compliance"
else
  for package in \
    ffmpeg \
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
    revaer.media.exiftool_exception; do
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

if [ "${failures}" -ne 0 ]; then
  exit 1
fi
