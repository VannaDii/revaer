#!/usr/bin/env bash
set -euo pipefail

project_key="${SONAR_PROJECT_KEY:?SONAR_PROJECT_KEY is required}"
auth_token="${SONAR_AUTH_TOKEN:?SONAR_AUTH_TOKEN is required}"
api_base="${SONAR_API_BASE_URL:-https://sonarcloud.io/api}"
pull_request="${SONAR_PULL_REQUEST:-}"
retry_attempts="${SONAR_API_RETRY_ATTEMPTS:-5}"
retry_delay_seconds="${SONAR_API_RETRY_DELAY_SECONDS:-3}"
temp_dir="$(mktemp -d)"
trap 'rm -rf "${temp_dir}"' EXIT

if ! [[ "${retry_attempts}" =~ ^[1-9][0-9]*$ ]]; then
  echo "SONAR_API_RETRY_ATTEMPTS must be a positive integer" >&2
  exit 2
fi

if ! [[ "${retry_delay_seconds}" =~ ^[0-9]+$ ]]; then
  echo "SONAR_API_RETRY_DELAY_SECONDS must be a non-negative integer" >&2
  exit 2
fi

scope_args=()
if [[ -n "${pull_request}" ]]; then
  scope_args+=(--data-urlencode "pullRequest=${pull_request}")
fi

sonar_get() {
  local endpoint="$1"
  local output_path="$2"
  local output_temp="${output_path}.tmp"
  local http_status
  local curl_status
  local attempt
  shift 2

  for ((attempt = 1; attempt <= retry_attempts; attempt += 1)); do
    rm -f "${output_temp}"
    set +e
    http_status="$(
      curl --silent --show-error \
        --user "${auth_token}:" \
        --get "${api_base}/${endpoint}" \
        "${scope_args[@]}" \
        "$@" \
        --output "${output_temp}" \
        --write-out "%{http_code}"
    )"
    curl_status=$?
    set -e

    if [[ "${curl_status}" -eq 0 && "${http_status}" =~ ^2[0-9][0-9]$ ]]; then
      mv "${output_temp}" "${output_path}"
      return 0
    fi

    if [[ "${http_status}" =~ ^5[0-9][0-9]$ || "${http_status}" == "000" ]]; then
      if ((attempt < retry_attempts)); then
        printf 'Sonar API %s returned HTTP %s; retrying attempt %s/%s\n' \
          "${endpoint}" "${http_status}" "${attempt}" "${retry_attempts}" >&2
        sleep "${retry_delay_seconds}"
        continue
      fi
    fi

    if [[ -s "${output_temp}" ]]; then
      cat "${output_temp}" >&2
      printf '\n' >&2
    fi
    printf 'Sonar API %s failed with curl status %s and HTTP status %s after %s attempt(s)\n' \
      "${endpoint}" "${curl_status}" "${http_status}" "${attempt}" >&2
    return 22
  done
}

sonar_get measures/component "${temp_dir}/measures.json" \
  --data-urlencode "component=${project_key}" \
  --data-urlencode "metricKeys=coverage,line_coverage,lines_to_cover,uncovered_lines"

jq -e '
  .component.measures as $measures
  | (($measures | map(select(.metric == "coverage" and (.value | tonumber) > 0)) | length) == 1)
    and (($measures | map(select(.metric == "line_coverage" and (.value | tonumber) > 0)) | length) == 1)
    and (($measures | map(select(.metric == "lines_to_cover" and (.value | tonumber) > 0)) | length) == 1)
    and (($measures | map(select(.metric == "uncovered_lines" and (.value | tonumber) >= 0)) | length) == 1)
' "${temp_dir}/measures.json" >/dev/null

sonar_get qualitygates/project_status "${temp_dir}/quality-gate.json" \
  --data-urlencode "projectKey=${project_key}"
jq -e '
  .projectStatus.status == "OK"
  and .projectStatus.ignoredConditions == false
' "${temp_dir}/quality-gate.json" >/dev/null

issue_args=(
  --data-urlencode "componentKeys=${project_key}"
  --data-urlencode "resolved=false"
  --data-urlencode "inNewCodePeriod=true"
  --data-urlencode "ps=1"
)
sonar_get issues/search "${temp_dir}/issues.json" "${issue_args[@]}"
jq -e '.total == 0' "${temp_dir}/issues.json" >/dev/null

hotspot_args=(
  --data-urlencode "projectKey=${project_key}"
  --data-urlencode "status=TO_REVIEW"
  --data-urlencode "inNewCodePeriod=true"
  --data-urlencode "ps=1"
)
sonar_get hotspots/search "${temp_dir}/hotspots.json" "${hotspot_args[@]}"
jq -e '.paging.total == 0' "${temp_dir}/hotspots.json" >/dev/null

coverage="$(jq -r '.component.measures[] | select(.metric == "coverage") | .value' "${temp_dir}/measures.json")"
lines_to_cover="$(jq -r '.component.measures[] | select(.metric == "lines_to_cover") | .value' "${temp_dir}/measures.json")"
printf 'Sonar result verified: coverage=%s%% lines_to_cover=%s unresolved_new_issues=0 unreviewed_new_hotspots=0\n' \
  "${coverage}" "${lines_to_cover}"
