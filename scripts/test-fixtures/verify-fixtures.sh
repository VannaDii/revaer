#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

source scripts/test-fixtures/lib.sh

lock_path="${REVAER_FIXTURE_LOCK_PATH:-test-fixtures/lock.json}"
manifest_path="${REVAER_FIXTURE_MANIFEST_PATH:-test-fixtures/manifest.json}"
report_path="${REVAER_MEDIA_CONVERSION_REPORT:-target/media-conversion-report.md}"
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
fixture_count="$(jq '.fixtures | length' "${manifest_path}")"
diagnostic_fixture_count="$(jq '[.fixtures[] | select(.allowProbeDiagnostics == true)] | length' "${manifest_path}")"
if [[ "${source_count}" -le 0 || "${fixture_count}" -le 0 || "${diagnostic_fixture_count}" -le 0 ]]; then
  printf 'verify-fixtures: report counts must be positive\n' >&2
  exit 1
fi

report_directory="$(dirname "${report_path}")"
mkdir -p "${report_directory}"
temporary_report="$(mktemp "${report_directory}/media-conversion-report.XXXXXX")"
cleanup_report() {
  rm -f "${temporary_report}"
}
trap cleanup_report EXIT HUP INT TERM
{
  printf '# Media Conversion Fixture Report\n\n'
  printf '## Summary\n'
  printf -- '- Outcome: passed\n'
  printf -- '- Locked source fixtures: %s\n' "${source_count}"
  printf -- '- Generated and reviewed fixtures: %s\n' "${fixture_count}"
  printf -- '- Explicitly bounded diagnostic fixtures: %s\n' "${diagnostic_fixture_count}"
  printf -- '- Verification: source integrity, FFmpeg-derived generation, and canonical ffprobe snapshots\n'
} >"${temporary_report}"
mv "${temporary_report}" "${report_path}"
trap - EXIT HUP INT TERM

printf 'verify-fixtures: verified locked source integrity and reviewed probe snapshots\n'
