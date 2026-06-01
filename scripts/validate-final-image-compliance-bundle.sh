#!/usr/bin/env bash
set -euo pipefail

bundle_path="${1:-}"

fail() {
  printf 'Final-image compliance bundle validation failed: %s\n' "$1" >&2
  exit 1
}

require_string() {
  local key="$1"
  local value
  value="$(jq -er ".${key} | select(type == \"string\" and length > 0)" "${bundle_path}")" \
    || fail "missing non-empty ${key}"
  printf '%s' "${value}"
}

require_artifact_hash() {
  local path_key="$1"
  local hash_key="$2"
  local artifact_path expected_hash actual_hash
  artifact_path="$(require_string "${path_key}")"
  expected_hash="$(require_string "${hash_key}")"
  case "${artifact_path}" in
    /*|*..*) fail "${path_key} must be a relative path inside the bundle directory" ;;
  esac
  if ! printf '%s' "${expected_hash}" | grep -Eq '^[0-9a-f]{64}$'; then
    fail "${hash_key} must be a lowercase SHA-256 digest"
  fi
  artifact_path="${bundle_dir}/${artifact_path}"
  [ -f "${artifact_path}" ] || fail "missing artifact ${artifact_path}"
  actual_hash="$(sha256sum "${artifact_path}" | awk '{print $1}')"
  [ "${actual_hash}" = "${expected_hash}" ] \
    || fail "${path_key} digest mismatch: expected ${expected_hash}, got ${actual_hash}"
}

[ -n "${bundle_path}" ] || fail "bundle manifest path is required"
[ -f "${bundle_path}" ] || fail "bundle manifest does not exist: ${bundle_path}"
command -v jq >/dev/null 2>&1 || fail "jq is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"

jq -e 'type == "object"' "${bundle_path}" >/dev/null \
  || fail "bundle manifest must be a JSON object"

schema_version="$(require_string schema_version)"
[ "${schema_version}" = "revaer.final-image-compliance.v1" ] \
  || fail "schema_version must be revaer.final-image-compliance.v1"
require_string revision >/dev/null
require_string source_offer_url >/dev/null

if ! jq -e '
  (.image_digest | type == "string" and length > 0)
  or (.build_provenance_ref | type == "string" and length > 0)
' "${bundle_path}" >/dev/null; then
  fail "image_digest or build_provenance_ref is required"
fi

bundle_dir="$(cd "$(dirname "${bundle_path}")" && pwd -P)"
require_artifact_hash spdx_path spdx_sha256
require_artifact_hash third_party_notices_path third_party_notices_sha256
require_artifact_hash source_compliance_path source_compliance_sha256
require_artifact_hash package_inventory_path package_inventory_sha256

jq -e '
  .release_gate == "passed"
  and (.generated_at | type == "string" and length > 0)
' "${bundle_path}" >/dev/null || fail "release_gate must be passed and generated_at is required"
