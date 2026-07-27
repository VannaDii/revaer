#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

source scripts/test-fixtures/lib.sh

lock_path="${REVAER_FIXTURE_LOCK_PATH:-test-fixtures/lock.json}"
for tool in jq awk wc tr; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf 'verify-fixtures: required tool missing: %s\n' "${tool}" >&2
    exit 1
  fi
done
if ! command -v sha256sum >/dev/null 2>&1 && ! command -v shasum >/dev/null 2>&1; then
  printf 'verify-fixtures: required tool missing: sha256sum or shasum\n' >&2
  exit 1
fi

source_count="$(jq '.sources | length' "${lock_path}")"
source_index=0
while ((source_index < source_count)); do
  id="$(jq -r --argjson index "${source_index}" '.sources[$index].id' "${lock_path}")"
  path="$(jq -r --argjson index "${source_index}" '.sources[$index].path' "${lock_path}")"
  expected_sha256="$(jq -r --argjson index "${source_index}" '.sources[$index].sha256' "${lock_path}")"
  minimum_bytes="$(jq -r --argjson index "${source_index}" '.sources[$index].minimumBytes' "${lock_path}")"
  maximum_bytes="$(jq -r --argjson index "${source_index}" '.sources[$index].maximumBytes' "${lock_path}")"
  fixture_validate "${id}" "${path}" "${expected_sha256}" "${minimum_bytes}" "${maximum_bytes}"
  source_index="$((source_index + 1))"
done

scripts/test-fixtures/probe-fixtures.sh --check
printf 'verify-fixtures: verified locked source integrity and reviewed probe snapshots\n'
