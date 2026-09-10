#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

source scripts/test-fixtures/lib.sh
source scripts/test-fixtures/probe-diagnostic-contract.sh

mode="${1:-}"
if [[ "${mode}" != "--check" && "${mode}" != "--update" ]]; then
  printf 'usage: %s --check|--update\n' "$0" >&2
  exit 2
fi

manifest_path="${REVAER_FIXTURE_MANIFEST_PATH:-test-fixtures/manifest.json}"
probe_directory="${REVAER_FIXTURE_PROBE_DIR:-test-fixtures/probe}"
ffprobe_bin="${REVAER_FIXTURE_FFPROBE_BIN:-ffprobe}"
lock_path="${REVAER_FIXTURE_LOCK_PATH:-test-fixtures/lock.json}"
report_path="${REVAER_MEDIA_CONVERSION_REPORT:-target/media-conversion-report.md}"

for tool in jq diff mktemp install wc tr cmp awk; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf 'probe-fixtures: required tool missing: %s\n' "${tool}" >&2
    exit 1
  fi
done
if ! command -v "${ffprobe_bin}" >/dev/null 2>&1; then
  printf 'probe-fixtures: required tool missing: %s\n' "${ffprobe_bin}" >&2
  exit 1
fi

if ! fixture_probe_policy_validate "${manifest_path}"; then
  printf 'probe-fixtures: manifest is invalid or has duplicate fixture IDs: %s\n' "${manifest_path}" >&2
  exit 1
fi

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-fixture-probes.XXXXXX")"
cleanup() {
  rm -rf "${temporary_root}"
}
trap cleanup EXIT HUP INT TERM

canonical_filter='{
  streams: [
    .streams[] |
    {
      codec_name: .codec_name,
      codec_type: .codec_type,
      disposition: {
        default: (.disposition.default // 0),
        forced: (.disposition.forced // 0)
      },
      index: .index,
      tags: {
        language: (.tags.language // null),
        title: (.tags.title // null)
      }
    }
  ]
}'

fixture_count="$(jq '.fixtures | length' "${manifest_path}")"
fixture_index=0
accepted_diagnostic_count=0
accepted_f1_count=0
maximum_diagnostic_bytes=4096
while ((fixture_index < fixture_count)); do
  fixture_id="$(jq -r --argjson index "${fixture_index}" '.fixtures[$index].id' "${manifest_path}")"
  fixture_path="$(jq -r --argjson index "${fixture_index}" '.fixtures[$index].path' "${manifest_path}")"
  allow_probe_diagnostics="$(jq -r --argjson index "${fixture_index}" '.fixtures[$index].allowProbeDiagnostics // false' "${manifest_path}")"
  diagnostic_contract="$(jq -r --argjson index "${fixture_index}" '.fixtures[$index].probeDiagnosticContract // ""' "${manifest_path}")"
  if [[ ! -s "${fixture_path}" ]]; then
    printf 'probe-fixtures: fixture is missing or empty: %s\n' "${fixture_path}" >&2
    exit 1
  fi

  snapshot_name="${fixture_id}.json"
  generated_snapshot="${temporary_root}/${snapshot_name}"
  reviewed_snapshot="${probe_directory}/${snapshot_name}"
  diagnostic_path="${temporary_root}/${fixture_id}.stderr"

  if [[ "${diagnostic_contract}" == "adr578-f1" ]]; then
    if ! fixture_f1_identity_validate "${manifest_path}" "${lock_path}" ||
       ! fixture_f1_snapshot_validate "${reviewed_snapshot}"; then
      printf 'probe-fixtures: ADR578 F1 identity or reviewed snapshot expired for %s\n' "${fixture_id}" >&2
      exit 1
    fi
    fixture_validate "${fixture_id}" "${fixture_path}" \
      43df750a2a01a37949791b717051b41522081a266b71d113be4b713063843699 21313902 21313902
  fi

  if ! "${ffprobe_bin}" \
      -v error \
      -show_streams \
      -of json \
      "${fixture_path}" 2>"${diagnostic_path}" |
      jq "${canonical_filter}" >"${generated_snapshot}"; then
    printf 'probe-fixtures: probe command failed for %s\n' "${fixture_path}" >&2
    if [[ -s "${diagnostic_path}" ]]; then
      cat "${diagnostic_path}" >&2
    fi
    exit 1
  fi

  diagnostic_bytes="$(wc -c <"${diagnostic_path}" | tr -d '[:space:]')"
  if ((diagnostic_bytes > maximum_diagnostic_bytes)); then
    printf 'probe-fixtures: diagnostics for %s exceed the %s-byte limit: %s bytes\n' \
      "${fixture_id}" "${maximum_diagnostic_bytes}" "${diagnostic_bytes}" >&2
    exit 1
  fi
  if ((diagnostic_bytes > 0)); then
    if [[ "${diagnostic_contract}" == "adr578-f1" ]]; then
      if ! fixture_f1_diagnostic_validate "${diagnostic_path}"; then
        printf 'probe-fixtures: ADR578 F1 diagnostic contract expired for %s\n' "${fixture_id}" >&2
        cat "${diagnostic_path}" >&2
        exit 1
      fi
    elif [[ "${allow_probe_diagnostics}" != "true" ]]; then
      printf 'probe-fixtures: unapproved diagnostics emitted for %s (%s bytes)\n' \
        "${fixture_id}" "${diagnostic_bytes}" >&2
      cat "${diagnostic_path}" >&2
      exit 1
    else
      accepted_diagnostic_count="$((accepted_diagnostic_count + 1))"
      printf 'probe-fixtures: accepted bounded diagnostics for %s (%s bytes)\n' \
        "${fixture_id}" "${diagnostic_bytes}"
    fi
  fi

  if [[ "${mode}" == "--check" || "${diagnostic_contract}" == "adr578-f1" ]]; then
    if [[ ! -f "${reviewed_snapshot}" ]]; then
      printf 'probe-fixtures: reviewed snapshot is missing: %s\n' "${reviewed_snapshot}" >&2
      exit 1
    fi
    if ! diff -u "${reviewed_snapshot}" "${generated_snapshot}"; then
      printf 'probe-fixtures: probe drift detected for %s\n' "${fixture_path}" >&2
      exit 1
    fi
  else
    mkdir -p "${probe_directory}"
    install -m 0644 "${generated_snapshot}" "${reviewed_snapshot}"
  fi

  if [[ "${diagnostic_contract}" == "adr578-f1" ]] && ((diagnostic_bytes > 0)); then
    version_path="${temporary_root}/ffprobe-version.txt"
    version_diagnostic_path="${temporary_root}/ffprobe-version.stderr"
    if ! "${ffprobe_bin}" -version >"${version_path}" 2>"${version_diagnostic_path}" ||
       [[ -s "${version_diagnostic_path}" ]] ||
       ! tool_profile="$(fixture_f1_tool_profile "${version_path}")"; then
      printf 'probe-fixtures: ADR578 F1 tool report is failed, diagnostic-bearing or unapproved\n' >&2
      cat "${diagnostic_path}" "${version_diagnostic_path}" >&2
      exit 1
    fi
    if ! fixture_f1_retain_evidence "${diagnostic_path}" "${version_path}" "${report_path}.probe-evidence" "${tool_profile}"; then
      printf 'probe-fixtures: ADR578 F1 raw evidence retention failed\n' >&2
      cat "${diagnostic_path}" >&2
      exit 1
    fi
    printf 'probe-fixtures: ADR578 F1 full tool report follows\n'
    cat "${version_path}"
    cat "${diagnostic_path}" >&2
    accepted_f1_count="$((accepted_f1_count + 1))"
  fi

  fixture_index="$((fixture_index + 1))"
done

if [[ "${mode}" == "--check" ]]; then
  printf 'probe-fixtures: %s reviewed snapshots match; %s bounded diagnostic emissions accepted; reviewed snapshots unchanged\n' \
    "${fixture_count}" "${accepted_diagnostic_count}"
else
  printf 'probe-fixtures: verified %s snapshots in update mode (F1 never replaced); %s bounded diagnostic emissions accepted\n' \
    "${fixture_count}" "${accepted_diagnostic_count}"
fi
printf 'probe-fixtures: ADR578 F1 exact diagnostic emissions accepted: %s\n' "${accepted_f1_count}"
