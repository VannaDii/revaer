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
  if ! printf '%s' "${expected_hash}" | grep -Eq '^[0-9a-f]{64}$'; then
    fail "${hash_key} must be a lowercase SHA-256 digest"
  fi
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
    type == "object"
    and (.spdxVersion | type == "string" and length > 0)
    and (.packages | type == "array" and length > 0)
  ' "${artifact_path}" >/dev/null \
    || fail "${label} must be a non-empty SPDX JSON inventory"

  jq -e '
    all(.packages[]; (
      (.name | type == "string" and length > 0)
      and (.versionInfo | type == "string" and length > 0 and . != "NOASSERTION")
      and (.downloadLocation | type == "string" and length > 0 and . != "NOASSERTION")
      and (.licenseConcluded | type == "string" and length > 0 and . != "NOASSERTION")
      and (.licenseDeclared | type == "string" and length > 0 and . != "NOASSERTION")
      and any(.checksums[]?; (
        (.algorithm | ascii_downcase) == "sha256"
        and (.checksumValue | test("^[0-9a-f]{64}$"))
      ))
    ))
  ' "${artifact_path}" >/dev/null \
    || fail "${label} packages must include version, license, checksum, and source evidence"

  if jq -e '
    any(.packages[]; (
      ((.name // "") | ascii_downcase | contains("aggregate"))
      and (
        (.versionInfo // "") == "NOASSERTION"
        or (.downloadLocation // "") == "NOASSERTION"
        or (.licenseConcluded // "") == "NOASSERTION"
        or (.licenseDeclared // "") == "NOASSERTION"
      )
    ))
  ' "${artifact_path}" >/dev/null; then
    fail "${label} must not contain aggregate NOASSERTION packages"
  fi
}

validate_source_compliance() {
  local artifact_path="$1"
  local binding="$2"

  jq -e '
    type == "object"
    and (((.entries // .packages) | type == "array") and ((.entries // .packages) | length > 0))
  ' "${artifact_path}" >/dev/null \
    || fail "source compliance artifact must contain entries or packages"

  jq -e '
    all((.entries // .packages)[]; (
      (
        (.source_url // .sourceUrl // .downloadLocation // "")
        | type == "string" and length > 0 and . != "NOASSERTION"
      )
      and (
        ((.source_sha256 // .sourceSha256 // .sha256 // "") | test("^[0-9a-f]{64}$"))
        or any(.checksums[]?; (
          (.algorithm | ascii_downcase) == "sha256"
          and (.checksumValue | test("^[0-9a-f]{64}$"))
        ))
      )
    ))
  ' "${artifact_path}" >/dev/null \
    || fail "source compliance entries must include source URL and SHA-256 checksum evidence"

  jq -e --arg binding "${binding}" '
    .. | strings | select(. == $binding)
  ' "${artifact_path}" >/dev/null \
    || fail "source compliance artifact must bind to manifest image digest or provenance"
}

main() {
  [[ "$#" -eq 1 ]] || fail "bundle manifest path is required"
  bundle_path="$1"
  [[ -f "${bundle_path}" ]] || fail "bundle manifest does not exist: ${bundle_path}"
  command -v jq >/dev/null 2>&1 || fail "jq is required"
  command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"

  jq -e 'type == "object"' "${bundle_path}" >/dev/null \
    || fail "bundle manifest must be a JSON object"

  local schema_version
  schema_version="$(require_string schema_version)"
  [[ "${schema_version}" = "revaer.final-image-compliance.v1" ]] \
    || fail "schema_version must be revaer.final-image-compliance.v1"
  require_string revision >/dev/null
  require_string source_offer_url >/dev/null

  if ! jq -e '
    (.image_digest | type == "string" and length > 0)
    or (.build_provenance_ref | type == "string" and length > 0)
  ' "${bundle_path}" >/dev/null; then
    fail "image_digest or build_provenance_ref is required"
  fi

  local spdx_artifact
  local source_compliance_artifact
  local package_inventory_artifact
  bundle_dir="$(cd "$(dirname "${bundle_path}")" && pwd -P)"
  spdx_artifact="$(require_artifact_hash spdx_path spdx_sha256)"
  require_artifact_hash third_party_notices_path third_party_notices_sha256 >/dev/null
  source_compliance_artifact="$(require_artifact_hash source_compliance_path source_compliance_sha256)"
  package_inventory_artifact="$(require_artifact_hash package_inventory_path package_inventory_sha256)"

  validate_spdx_inventory "${spdx_artifact}" "spdx_path"
  validate_spdx_inventory "${package_inventory_artifact}" "package_inventory_path"

  local image_digest
  local build_provenance_ref
  local source_binding
  image_digest="$(jq -er '.image_digest // empty' "${bundle_path}" || true)"
  build_provenance_ref="$(jq -er '.build_provenance_ref // empty' "${bundle_path}" || true)"
  source_binding="${image_digest:-${build_provenance_ref}}"
  [[ -n "${source_binding}" ]] || fail "image_digest or build_provenance_ref is required"
  validate_source_compliance "${source_compliance_artifact}" "${source_binding}"

  jq -e '
    .release_gate == "passed"
    and (.generated_at | type == "string" and length > 0)
  ' "${bundle_path}" >/dev/null || fail "release_gate must be passed and generated_at is required"
}

main "$@"
