#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-media-compliance-guardrails.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT
guardrail="${repo_root}/scripts/media-compliance-guardrails.sh"

fail_test() {
  local message="$1"
  printf 'Media compliance regression test failed: %s\n' "${message}" >&2
  exit 1
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >"${test_root}/${label}.stdout" 2>"${test_root}/${label}.stderr"; then
    fail_test "${label} unexpectedly passed"
  fi
}

make_repository() {
  local label="$1"
  local fixture_root="${test_root}/${label}"
  mkdir -p "${fixture_root}/scripts" "${fixture_root}/release" "${fixture_root}/.github"
  cp "${guardrail}" "${fixture_root}/scripts/media-compliance-guardrails.sh"
  cp "${repo_root}/Dockerfile" "${fixture_root}/Dockerfile"
  cp -R "${repo_root}/release/media-compliance" "${fixture_root}/release/media-compliance"
  cp "${repo_root}/.github/build-inputs.env" "${fixture_root}/.github/build-inputs.env"
  git -C "${fixture_root}" init -q
  git -C "${fixture_root}" add Dockerfile scripts release
  printf '%s' "${fixture_root}"
}

sentinel="${test_root}/sentinel"
predictable_path="${test_root}/revaer-media-nonfree.matches"
printf 'operator-owned sentinel\n' > "${sentinel}"
ln -s "${sentinel}" "${predictable_path}"

TMPDIR="${test_root}" bash "${guardrail}"

[[ -L "${predictable_path}" ]] || {
  printf 'Guardrail replaced or deleted the pre-placed symlink\n' >&2
  exit 1
}
[[ "$(cat "${sentinel}")" = "operator-owned sentinel" ]] || {
  printf 'Guardrail wrote through the pre-placed symlink\n' >&2
  exit 1
}

remaining="$(find "${test_root}" -maxdepth 1 -type f -name 'revaer-media-nonfree.*' -print)"
[[ -z "${remaining}" ]] || {
  printf 'Guardrail retained private match files:\n%s\n' "${remaining}" >&2
  exit 1
}

fixture_root="$(make_repository missing-dockerfile)"
rm "${fixture_root}/Dockerfile"
expect_failure missing-dockerfile env TMPDIR="${test_root}" \
  bash "${fixture_root}/scripts/media-compliance-guardrails.sh"

fixture_root="$(make_repository incomplete-dockerfile)"
printf 'FROM scratch\n' > "${fixture_root}/Dockerfile"
expect_failure incomplete-dockerfile env TMPDIR="${test_root}" \
  bash "${fixture_root}/scripts/media-compliance-guardrails.sh"

fixture_root="$(make_repository nonfree-option)"
printf '#!/usr/bin/env sh\nffmpeg %s\n' "--enable""-nonfree" \
  > "${fixture_root}/scripts/nonfree.sh"
git -C "${fixture_root}" add scripts/nonfree.sh
expect_failure nonfree-option env TMPDIR="${test_root}" \
  bash "${fixture_root}/scripts/media-compliance-guardrails.sh"

fixture_root="$(make_repository missing-evidence)"
rm "${fixture_root}/release/media-compliance/SOURCE-OFFER.txt" \
  "${fixture_root}/release/media-compliance/THIRD-PARTY-NOTICES.md" \
  "${fixture_root}/release/media-compliance/media-runtime-inventory.spdx.json" \
  "${fixture_root}/release/media-compliance/exiftool-exception.md"
expect_failure missing-evidence env TMPDIR="${test_root}" \
  bash "${fixture_root}/scripts/media-compliance-guardrails.sh"

fixture_root="$(make_repository invalid-inventory)"
jq '.packages = [{
  name: "alpine-media-runtime-packages",
  versionInfo: "1",
  licenseConcluded: "NOASSERTION",
  licenseDeclared: "NOASSERTION",
  checksums: []
}]' "${fixture_root}/release/media-compliance/media-runtime-inventory.spdx.json" \
  > "${fixture_root}/release/media-compliance/media-runtime-inventory.spdx.json.tmp"
mv "${fixture_root}/release/media-compliance/media-runtime-inventory.spdx.json.tmp" \
  "${fixture_root}/release/media-compliance/media-runtime-inventory.spdx.json"
expect_failure invalid-inventory env TMPDIR="${test_root}" \
  bash "${fixture_root}/scripts/media-compliance-guardrails.sh"

fixture_root="$(make_repository missing-jq)"
path_root="${test_root}/path-without-jq"
mkdir "${path_root}"
for command_name in cat chmod dirname git grep mktemp rm; do
  ln -s "$(command -v "${command_name}")" "${path_root}/${command_name}"
done
expect_failure missing-jq env PATH="${path_root}" TMPDIR="${test_root}" \
  /bin/bash "${fixture_root}/scripts/media-compliance-guardrails.sh"

printf 'Media compliance guardrail regression tests passed\n'
