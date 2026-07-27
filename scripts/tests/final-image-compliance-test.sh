#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-image-compliance.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT
cd "${repo_root}"

generator="${repo_root}/scripts/generate-final-image-compliance-bundle.sh"
validator="${repo_root}/scripts/validate-final-image-compliance-bundle.sh"
digest_a="sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
digest_b="sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
uppercase_digest="sha256:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
image_a="ghcr.io/vannadii/revaer@${digest_a}"
image_b="ghcr.io/vannadii/revaer@${digest_b}"
inventory="${repo_root}/release/media-compliance/media-runtime-inventory.spdx.json"

fail_test() {
  local message="$1"
  printf 'Final-image compliance regression test failed: %s\n' "${message}" >&2
  exit 1
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >"${test_root}/${label}.stdout" 2>"${test_root}/${label}.stderr"; then
    fail_test "${label} unexpectedly passed"
  fi
}

copy_bundle() {
  local label="$1"
  local case_dir="${test_root}/${label}"
  cp -R "${test_root}/bundle" "${case_dir}"
  printf '%s' "${case_dir}"
}

mutate_manifest() {
  local case_dir="$1"
  local filter="$2"
  local manifest="${case_dir}/final-image-compliance-bundle.json"
  jq "${filter}" "${manifest}" > "${manifest}.tmp"
  mv "${manifest}.tmp" "${manifest}"
}

rehash_artifact() {
  local case_dir="$1"
  local path_key="$2"
  local hash_key="$3"
  local manifest="${case_dir}/final-image-compliance-bundle.json"
  local relative_path digest
  relative_path="$(jq -er ".${path_key}" "${manifest}")"
  digest="$(sha256sum "${case_dir}/${relative_path}" | awk '{print $1}')"
  jq --arg digest "${digest}" ".${hash_key} = \$digest" "${manifest}" > "${manifest}.tmp"
  mv "${manifest}.tmp" "${manifest}"
}

GITHUB_SHA="${digest_a#sha256:}" "${generator}" \
  "${image_a}" \
  "${inventory}" \
  "${test_root}/bundle"
"${validator}" \
  "${test_root}/bundle/final-image-compliance-bundle.json" \
  "${image_a}"

printf '{}\n' > "${test_root}/invalid-inventory.json"
expect_failure generator-usage "${generator}"
expect_failure generator-tagged-reference "${generator}" \
  ghcr.io/vannadii/revaer:latest "${inventory}" "${test_root}/tagged"
expect_failure generator-uppercase-digest "${generator}" \
  "ghcr.io/vannadii/revaer@${uppercase_digest}" "${inventory}" "${test_root}/uppercase"
expect_failure generator-missing-inventory "${generator}" \
  "${image_a}" "${test_root}/missing-inventory.json" "${test_root}/missing"
expect_failure generator-invalid-inventory "${generator}" \
  "${image_a}" "${test_root}/invalid-inventory.json" "${test_root}/invalid"

manifest="${test_root}/bundle/final-image-compliance-bundle.json"
expect_failure validator-usage "${validator}" "${manifest}"
expect_failure validator-tagged-reference "${validator}" "${manifest}" ghcr.io/vannadii/revaer:latest
expect_failure validator-uppercase-digest "${validator}" "${manifest}" \
  "ghcr.io/vannadii/revaer@${uppercase_digest}"
expect_failure validator-missing-manifest "${validator}" \
  "${test_root}/missing-bundle.json" "${image_a}"
expect_failure validator-wrong-image "${validator}" "${manifest}" "${image_b}"

case_dir="$(copy_bundle missing-revision)"
mutate_manifest "${case_dir}" '.revision = ""'
expect_failure validator-missing-string "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle schema)"
mutate_manifest "${case_dir}" '.schema_version = "unsupported"'
expect_failure validator-schema "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle predicate)"
mutate_manifest "${case_dir}" '.predicate_type = "unsupported"'
expect_failure validator-predicate "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle image-digest)"
mutate_manifest "${case_dir}" ".image_digest = \"${digest_b}\""
expect_failure validator-image-digest "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle package-reference)"
mutate_manifest "${case_dir}" ".package_inventory_image_reference = \"${image_b}\""
expect_failure validator-package-reference "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle absolute-path)"
mutate_manifest "${case_dir}" '.spdx_path = "/tmp/outside.json"'
expect_failure validator-absolute-path "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle traversal-path)"
mutate_manifest "${case_dir}" '.spdx_path = "../outside.json"'
expect_failure validator-traversal-path "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle malformed-hash)"
mutate_manifest "${case_dir}" '.spdx_sha256 = "not-a-digest"'
expect_failure validator-malformed-hash "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle missing-artifact)"
rm "${case_dir}/THIRD-PARTY-NOTICES.md"
expect_failure validator-missing-artifact "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle digest-mismatch)"
printf 'tampered\n' >> "${case_dir}/THIRD-PARTY-NOTICES.md"
expect_failure validator-digest-mismatch "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle declared-spdx)"
jq '.packages = []' "${case_dir}/declared-runtime-inventory.spdx.json" \
  > "${case_dir}/declared-runtime-inventory.spdx.json.tmp"
mv "${case_dir}/declared-runtime-inventory.spdx.json.tmp" \
  "${case_dir}/declared-runtime-inventory.spdx.json"
rehash_artifact "${case_dir}" spdx_path spdx_sha256
expect_failure validator-declared-spdx "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle package-spdx)"
jq '.packages[0].licenseDeclared = "NOASSERTION"' \
  "${case_dir}/image-package-inventory.spdx.json" \
  > "${case_dir}/image-package-inventory.spdx.json.tmp"
mv "${case_dir}/image-package-inventory.spdx.json.tmp" \
  "${case_dir}/image-package-inventory.spdx.json"
rehash_artifact "${case_dir}" package_inventory_path package_inventory_sha256
expect_failure validator-package-spdx "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle package-comment)"
jq '.comment = "another image"' "${case_dir}/image-package-inventory.spdx.json" \
  > "${case_dir}/image-package-inventory.spdx.json.tmp"
mv "${case_dir}/image-package-inventory.spdx.json.tmp" \
  "${case_dir}/image-package-inventory.spdx.json"
rehash_artifact "${case_dir}" package_inventory_path package_inventory_sha256
expect_failure validator-package-comment "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle source-compliance)"
jq '.entries = []' "${case_dir}/source-compliance.json" \
  > "${case_dir}/source-compliance.json.tmp"
mv "${case_dir}/source-compliance.json.tmp" "${case_dir}/source-compliance.json"
rehash_artifact "${case_dir}" source_compliance_path source_compliance_sha256
expect_failure validator-source-compliance "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle release-gate)"
mutate_manifest "${case_dir}" '.release_gate = "failed"'
expect_failure validator-release-gate "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

case_dir="$(copy_bundle generated-at)"
mutate_manifest "${case_dir}" '.generated_at = ""'
expect_failure validator-generated-at "${validator}" \
  "${case_dir}/final-image-compliance-bundle.json" "${image_a}"

printf 'Final-image compliance regression tests passed\n'
