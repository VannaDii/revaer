#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

source scripts/test-fixtures/lib.sh

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf 'download-test-fixtures: required tool missing: %s\n' "${tool}" >&2
    exit 1
  fi
}

for tool in jq base64 awk wc tr mktemp; do
  require_tool "${tool}"
done
if ! command -v sha256sum >/dev/null 2>&1; then
  require_tool shasum
fi
require_tool "${REVAER_FIXTURE_CURL_BIN:-curl}"

lock_path="${REVAER_FIXTURE_LOCK_PATH:-test-fixtures/lock.json}"
if ! jq -e '
  .schemaVersion == 1 and
  (.sources | type == "array" and length > 0) and
  ([.sources[].id] | length == (unique | length)) and
  ([.sources[].path] | length == (unique | length)) and
  all(.sources[];
    (.id | type == "string" and length > 0) and
    (.path | type == "string" and test("^test-fixtures/(source|chromium)/[A-Za-z0-9._-]+$")) and
    (.encoding == "raw" or .encoding == "base64") and
    (.sha256 | test("^[0-9a-f]{64}$")) and
    (.minimumBytes | type == "number" and . > 0 and . == floor) and
    (.maximumBytes | type == "number" and . == floor) and
    (.maximumBytes >= .minimumBytes) and
    (.urls | type == "array" and length > 0) and
    all(.urls[]; type == "string" and startswith("https://")))
' "${lock_path}" >/dev/null; then
  printf 'download-test-fixtures: invalid fixture lock: %s\n' "${lock_path}" >&2
  exit 1
fi

umask 077
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-fixture-download.XXXXXX")"
cleanup() {
  rm -rf "${temporary_root}"
}
trap cleanup EXIT HUP INT TERM

source_count="$(jq '.sources | length' "${lock_path}")"
source_index=0
while ((source_index < source_count)); do
  id="$(jq -r --argjson index "${source_index}" '.sources[$index].id' "${lock_path}")"
  destination="$(jq -r --argjson index "${source_index}" '.sources[$index].path' "${lock_path}")"
  encoding="$(jq -r --argjson index "${source_index}" '.sources[$index].encoding' "${lock_path}")"
  expected_sha256="$(jq -r --argjson index "${source_index}" '.sources[$index].sha256' "${lock_path}")"
  minimum_bytes="$(jq -r --argjson index "${source_index}" '.sources[$index].minimumBytes' "${lock_path}")"
  maximum_bytes="$(jq -r --argjson index "${source_index}" '.sources[$index].maximumBytes' "${lock_path}")"
  urls=()
  while IFS= read -r url; do
    urls+=("${url}")
  done < <(jq -r --argjson index "${source_index}" '.sources[$index].urls[]' "${lock_path}")

  fixture_acquire \
    "${id}" \
    "${destination}" \
    "${encoding}" \
    "${expected_sha256}" \
    "${minimum_bytes}" \
    "${maximum_bytes}" \
    "${temporary_root}" \
    "${urls[@]}"
  source_index="$((source_index + 1))"
done

printf 'download-test-fixtures: verified %s locked source fixtures\n' "${source_count}"
