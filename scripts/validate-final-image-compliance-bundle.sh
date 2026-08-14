#!/usr/bin/env bash
set -euo pipefail

bundle_path=""
bundle_dir=""

fail() {
  local message="$1"
  printf 'Final-image compliance bundle validation failed: %s\n' "${message}" >&2
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
    *) ;;
  esac
  [[ "${expected_hash}" =~ ^[0-9a-f]{64}$ ]] || fail "${hash_key} must be a lowercase SHA-256 digest"
  artifact_path="${bundle_dir}/${artifact_path}"
  [[ -f "${artifact_path}" ]] || fail "missing artifact ${artifact_path}"
  actual_hash="$(sha256sum "${artifact_path}" | awk '{print $1}')"
  [[ "${actual_hash}" = "${expected_hash}" ]] \
    || fail "${path_key} digest mismatch: expected ${expected_hash}, got ${actual_hash}"
  printf '%s' "${artifact_path}"
}

validate_spdx_inventory() {
  local artifact_path="$1"
  local label="$2"
  jq -e '
    .spdxVersion == "SPDX-2.3"
    and (.packages | type == "array" and length > 0)
    and all(.packages[]; (
      (.name | type == "string" and length > 0)
      and (.versionInfo | type == "string" and length > 0 and . != "NOASSERTION")
      and (.downloadLocation | type == "string" and length > 0 and . != "NOASSERTION")
      and (.licenseConcluded | type == "string" and length > 0 and . != "NOASSERTION")
      and (.licenseDeclared | type == "string" and length > 0 and . != "NOASSERTION")
      and any(.checksums[]?; ((.algorithm | ascii_downcase) == "sha256") and (.checksumValue | test("^[0-9a-f]{64}$")))
    ))
  ' "${artifact_path}" >/dev/null || fail "${label} must contain complete SPDX package evidence"
}

main() {
  [[ "$#" -eq 2 ]] || fail "usage: $0 BUNDLE_PATH EXPECTED_DIGEST_QUALIFIED_IMAGE_REFERENCE"
  bundle_path="$1"
  local expected_image_reference="$2"
  local expected_digest="${expected_image_reference##*@}"
  [[ "${expected_image_reference}" == *@sha256:* ]] || fail "expected image reference must be digest-qualified"
  [[ "${expected_digest}" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "expected image digest must be lowercase SHA-256"
  [[ -f "${bundle_path}" ]] || fail "bundle manifest does not exist: ${bundle_path}"
  command -v jq >/dev/null 2>&1 || fail "jq is required"
  command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"
  bundle_dir="$(cd "$(dirname "${bundle_path}")" && pwd -P)"

  [[ "$(require_string schema_version)" = "revaer.final-image-compliance.v2" ]] || fail "unsupported schema_version"
  [[ "$(require_string predicate_type)" = "https://revaer.com/attestations/final-image-compliance/v2" ]] || fail "unsupported predicate_type"
  [[ "$(require_string image_reference)" = "${expected_image_reference}" ]] || fail "bundle image_reference does not match the built image"
  [[ "$(require_string image_digest)" = "${expected_digest}" ]] || fail "bundle image_digest does not match the built image"
  [[ "$(require_string package_inventory_image_reference)" = "${expected_image_reference}" ]] || fail "package inventory is bound to another image"
  require_string revision >/dev/null
  require_string source_offer_url >/dev/null

  local declared_inventory package_inventory source_compliance
  declared_inventory="$(require_artifact_hash spdx_path spdx_sha256)"
  package_inventory="$(require_artifact_hash package_inventory_path package_inventory_sha256)"
  source_compliance="$(require_artifact_hash source_compliance_path source_compliance_sha256)"
  require_artifact_hash third_party_notices_path third_party_notices_sha256 >/dev/null
  require_artifact_hash build_inputs_path build_inputs_sha256 >/dev/null
  validate_spdx_inventory "${declared_inventory}" "spdx_path"
  validate_spdx_inventory "${package_inventory}" "package_inventory_path"

  jq -e --arg image_reference "${expected_image_reference}" '
    .comment == ("Inventory generated from " + $image_reference)
  ' "${package_inventory}" >/dev/null || fail "package inventory does not identify the exact built image"
  jq -e --arg image_reference "${expected_image_reference}" --arg image_digest "${expected_digest}" '
    .image_reference == $image_reference
    and .image_digest == $image_digest
    and (.entries | type == "array" and length > 0)
    and all(.entries[]; (
      (.source_url | type == "string" and length > 0 and . != "NOASSERTION")
      and any(.checksums[]?; ((.algorithm | ascii_downcase) == "sha256") and (.checksumValue | test("^[0-9a-f]{64}$")))
    ))
  ' "${source_compliance}" >/dev/null || fail "source compliance evidence is incomplete or bound to another image"
  jq -e '.release_gate == "passed" and (.generated_at | type == "string" and length > 0)' "${bundle_path}" >/dev/null \
    || fail "release_gate must be passed and generated_at is required"
}

main "$@"
