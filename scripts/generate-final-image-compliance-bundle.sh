#!/usr/bin/env bash
set -euo pipefail

fail() {
  local message="$1"
  printf 'Final-image compliance bundle generation failed: %s\n' "${message}" >&2
  exit 1
}

[[ "$#" -eq 3 ]] || fail "usage: $0 IMAGE_REFERENCE PACKAGE_INVENTORY OUTPUT_DIRECTORY"

image_reference="$1"
package_inventory="$2"
output_dir="$3"
image_digest="${image_reference##*@}"

[[ "${image_reference}" == *@sha256:* ]] || fail "image reference must be digest-qualified"
[[ "${image_digest}" =~ ^sha256:[0-9a-f]{64}$ ]] || fail "image reference must contain a lowercase SHA-256 digest"
[[ -f "${package_inventory}" ]] || fail "package inventory does not exist: ${package_inventory}"
command -v jq >/dev/null 2>&1 || fail "jq is required"
command -v sha256sum >/dev/null 2>&1 || fail "sha256sum is required"

jq -e '
  .spdxVersion == "SPDX-2.3"
  and (.packages | type == "array" and length > 0)
' "${package_inventory}" >/dev/null || fail "package inventory must be non-empty SPDX 2.3 JSON"

mkdir -p "${output_dir}"
output_dir="$(cd "${output_dir}" && pwd -P)"
inventory_name="image-package-inventory.spdx.json"
source_name="source-compliance.json"
notices_name="THIRD-PARTY-NOTICES.md"
declared_inventory_name="declared-runtime-inventory.spdx.json"
build_inputs_name="build-inputs.env"

jq --arg image_reference "${image_reference}" '
  .comment = ("Inventory generated from " + $image_reference)
' "${package_inventory}" > "${output_dir}/${inventory_name}"
cp release/media-compliance/THIRD-PARTY-NOTICES.md "${output_dir}/${notices_name}"
cp release/media-compliance/media-runtime-inventory.spdx.json "${output_dir}/${declared_inventory_name}"
cp .github/build-inputs.env "${output_dir}/${build_inputs_name}"

jq --arg image_reference "${image_reference}" --arg image_digest "${image_digest}" '
  {
    schema_version: "revaer.source-compliance.v1",
    image_reference: $image_reference,
    image_digest: $image_digest,
    entries: [
      .packages[] | {
        name,
        version: .versionInfo,
        source_url: .downloadLocation,
        checksums
      }
    ]
  }
' release/media-compliance/media-runtime-inventory.spdx.json > "${output_dir}/${source_name}"

sha256_file() {
  local path="$1"
  sha256sum "${path}" | awk '{print $1}'
}

revision="${GITHUB_SHA:-$(git rev-parse HEAD)}"
generated_at="$(date -u +'%Y-%m-%dT%H:%M:%SZ')"
source_offer_url="https://github.com/VannaDii/revaer/tree/${revision}/release/media-compliance"

jq -n \
  --arg image_reference "${image_reference}" \
  --arg image_digest "${image_digest}" \
  --arg revision "${revision}" \
  --arg generated_at "${generated_at}" \
  --arg source_offer_url "${source_offer_url}" \
  --arg inventory_name "${inventory_name}" \
  --arg inventory_hash "$(sha256_file "${output_dir}/${inventory_name}")" \
  --arg source_name "${source_name}" \
  --arg source_hash "$(sha256_file "${output_dir}/${source_name}")" \
  --arg notices_name "${notices_name}" \
  --arg notices_hash "$(sha256_file "${output_dir}/${notices_name}")" \
  --arg declared_inventory_name "${declared_inventory_name}" \
  --arg declared_inventory_hash "$(sha256_file "${output_dir}/${declared_inventory_name}")" \
  --arg build_inputs_name "${build_inputs_name}" \
  --arg build_inputs_hash "$(sha256_file "${output_dir}/${build_inputs_name}")" \
  '{
    schema_version: "revaer.final-image-compliance.v2",
    predicate_type: "https://revaer.com/attestations/final-image-compliance/v2",
    image_reference: $image_reference,
    image_digest: $image_digest,
    package_inventory_image_reference: $image_reference,
    revision: $revision,
    generated_at: $generated_at,
    source_offer_url: $source_offer_url,
    release_gate: "passed",
    spdx_path: $declared_inventory_name,
    spdx_sha256: $declared_inventory_hash,
    build_inputs_path: $build_inputs_name,
    build_inputs_sha256: $build_inputs_hash,
    package_inventory_path: $inventory_name,
    package_inventory_sha256: $inventory_hash,
    source_compliance_path: $source_name,
    source_compliance_sha256: $source_hash,
    third_party_notices_path: $notices_name,
    third_party_notices_sha256: $notices_hash
  }' > "${output_dir}/final-image-compliance-bundle.json"

scripts/validate-final-image-compliance-bundle.sh \
  "${output_dir}/final-image-compliance-bundle.json" \
  "${image_reference}"
