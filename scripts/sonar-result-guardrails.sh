#!/usr/bin/env bash
set -euo pipefail

project_key="${SONAR_PROJECT_KEY:?SONAR_PROJECT_KEY is required}"
auth_token="${SONAR_AUTH_TOKEN:?SONAR_AUTH_TOKEN is required}"
api_base="${SONAR_API_BASE_URL:-https://sonarcloud.io/api}"
pull_request="${SONAR_PULL_REQUEST:-}"
retry_attempts="${SONAR_API_RETRY_ATTEMPTS:-5}"
retry_delay_seconds="${SONAR_API_RETRY_DELAY_SECONDS:-3}"
result_retry_attempts="${SONAR_RESULT_RETRY_ATTEMPTS:-10}"
result_retry_delay_seconds="${SONAR_RESULT_RETRY_DELAY_SECONDS:-3}"
result_evidence_dir="${SONAR_RESULT_EVIDENCE_DIR:-artifacts/sonar/api}"
report_task_path="${SONAR_REPORT_TASK_PATH:-.scannerwork/report-task.txt}"
result_evidence_max_bytes=4096
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

if ! [[ "${result_retry_attempts}" =~ ^[1-9][0-9]*$ ]]; then
  echo "SONAR_RESULT_RETRY_ATTEMPTS must be a positive integer" >&2
  exit 2
fi

if ! [[ "${result_retry_delay_seconds}" =~ ^[0-9]+$ ]]; then
  echo "SONAR_RESULT_RETRY_DELAY_SECONDS must be a non-negative integer" >&2
  exit 2
fi

if [[ ! -s "${report_task_path}" ]]; then
  echo "Authoritative Sonar report-task.txt is missing" >&2
  exit 2
fi
task_id_count="$(grep -c '^ceTaskId=' "${report_task_path}" || true)"
if [[ "${task_id_count}" -ne 1 ]]; then
  echo "Authoritative Sonar report-task.txt must contain exactly one ceTaskId" >&2
  exit 2
fi
task_id="$(sed -n 's/^ceTaskId=//p' "${report_task_path}")"
if ! [[ "${task_id}" =~ ^[A-Za-z0-9_-]+$ ]]; then
  echo "Authoritative Sonar report-task.txt contains an invalid ceTaskId" >&2
  exit 2
fi

sonar_get() {
  local endpoint="$1"
  local output_path="$2"
  local include_pull_request="$3"
  local output_temp="${output_path}.tmp"
  local http_status
  local curl_status
  local attempt
  shift 3

  if [[ "${include_pull_request}" = "true" && -n "${pull_request}" ]]; then
    set -- --data-urlencode "pullRequest=${pull_request}" "$@"
  fi

  for ((attempt = 1; attempt <= retry_attempts; attempt += 1)); do
    rm -f "${output_temp}"
    set +e
    http_status="$(
      curl --silent --show-error \
        --user "${auth_token}:" \
        --get "${api_base}/${endpoint}" \
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

    if [[ "${http_status}" == "429" || "${http_status}" =~ ^5[0-9][0-9]$ || \
      "${http_status}" == "000" ]] && \
      ((attempt < retry_attempts)); then
      printf 'Sonar API %s returned HTTP %s; retrying attempt %s/%s\n' \
        "${endpoint}" "${http_status}" "${attempt}" "${retry_attempts}" >&2
      sleep "${retry_delay_seconds}"
      continue
    fi

    if [[ -s "${output_temp}" ]]; then
      head -c "${result_evidence_max_bytes}" "${output_temp}" >&2
      printf '\n' >&2
    fi
    printf 'Sonar API %s failed with curl status %s and HTTP status %s after %s attempt(s)\n' \
      "${endpoint}" "${curl_status}" "${http_status}" "${attempt}" >&2
    return 22
  done
}

issue_args=(
  --data-urlencode "componentKeys=${project_key}"
  --data-urlencode "resolved=false"
  --data-urlencode "ps=1"
)
hotspot_args=(
  --data-urlencode "projectKey=${project_key}"
  --data-urlencode "ps=1"
)
if [[ -n "${pull_request}" ]]; then
  issue_args+=(--data-urlencode "inNewCodePeriod=true")
  hotspot_args+=(--data-urlencode "inNewCodePeriod=true")
fi

fetch_published_result() {
  local analysis_id
  rm -f \
    "${temp_dir}/ce-task.json" \
    "${temp_dir}/measures.json" \
    "${temp_dir}/quality-gate.json" \
    "${temp_dir}/issues.json" \
    "${temp_dir}/hotspots.json" || return $?

  sonar_get ce/task "${temp_dir}/ce-task.json" false \
    --data-urlencode "id=${task_id}" || return $?
  if ! jq -e --arg task_id "${task_id}" --arg project_key "${project_key}" '
    .task.id == $task_id
    and .task.componentKey == $project_key
    and .task.status == "SUCCESS"
    and ((.task.analysisId // "") | length > 0)
  ' "${temp_dir}/ce-task.json" >/dev/null; then
    return 1
  fi
  analysis_id="$(jq -r '.task.analysisId' "${temp_dir}/ce-task.json")"
  sonar_get measures/component "${temp_dir}/measures.json" true \
    --data-urlencode "component=${project_key}" \
    --data-urlencode "metricKeys=coverage,line_coverage,lines_to_cover,uncovered_lines" || return $?
  sonar_get qualitygates/project_status "${temp_dir}/quality-gate.json" false \
    --data-urlencode "analysisId=${analysis_id}" || return $?
  sonar_get issues/search "${temp_dir}/issues.json" true "${issue_args[@]}" || return $?
  sonar_get hotspots/search "${temp_dir}/hotspots.json" true "${hotspot_args[@]}"
}

published_result_is_strict() {
  jq -e --arg task_id "${task_id}" --arg project_key "${project_key}" '
    .task.id == $task_id
    and .task.componentKey == $project_key
    and .task.status == "SUCCESS"
    and ((.task.analysisId // "") | length > 0)
  ' "${temp_dir}/ce-task.json" >/dev/null \
    && jq -e '
    .component.measures as $measures
    | (($measures | map(select(.metric == "coverage" and (.value | tonumber) > 0)) | length) == 1)
      and (($measures | map(select(.metric == "line_coverage" and (.value | tonumber) > 0)) | length) == 1)
      and (($measures | map(select(.metric == "lines_to_cover" and (.value | tonumber) > 0)) | length) == 1)
      and (($measures | map(select(.metric == "uncovered_lines" and (.value | tonumber) >= 0)) | length) == 1)
  ' "${temp_dir}/measures.json" >/dev/null \
    && jq -e '
      .projectStatus.status == "OK"
      and .projectStatus.ignoredConditions == false
    ' "${temp_dir}/quality-gate.json" >/dev/null \
    && jq -e '.total == 0' "${temp_dir}/issues.json" >/dev/null \
    && jq -e '.paging.total == 0' "${temp_dir}/hotspots.json" >/dev/null
}

print_published_result_evidence() {
  local evidence
  local evidence_excerpt
  local evidence_path
  local evidence_size

  for evidence in ce-task measures quality-gate issues hotspots; do
    evidence_path="${temp_dir}/${evidence}.json"
    if [[ ! -f "${evidence_path}" ]]; then
      printf '%s: <response unavailable>\n' "${evidence}" >&2
      continue
    fi

    evidence_size="$(wc -c < "${evidence_path}")"
    evidence_size="${evidence_size//[[:space:]]/}"
    evidence_excerpt="$(head -c "${result_evidence_max_bytes}" "${evidence_path}")"
    printf '%s: %s\n' "${evidence}" "${evidence_excerpt}" >&2
    if ((evidence_size > result_evidence_max_bytes)); then
      printf '<truncated: %s of %s bytes shown>\n' \
        "${result_evidence_max_bytes}" "${evidence_size}" >&2
    fi
  done
}

persist_published_result_evidence() {
  local evidence
  mkdir -p "${result_evidence_dir}"
  for evidence in ce-task measures quality-gate issues hotspots; do
    if [[ -f "${temp_dir}/${evidence}.json" ]]; then
      cp "${temp_dir}/${evidence}.json" "${result_evidence_dir}/${evidence}.json"
    fi
  done
}

result_attempt=1
while true; do
  if fetch_published_result && published_result_is_strict; then
    break
  fi
  if ((result_attempt >= result_retry_attempts)); then
    echo "Sonar published result did not satisfy strict criteria after ${result_attempt} attempt(s)" >&2
    persist_published_result_evidence
    print_published_result_evidence
    exit 1
  fi

  printf 'Sonar published result is not ready or not strict; retrying attempt %s/%s\n' \
    "${result_attempt}" "${result_retry_attempts}" >&2
  sleep "${result_retry_delay_seconds}"
  result_attempt="$((result_attempt + 1))"
done

coverage="$(jq -r '.component.measures[] | select(.metric == "coverage") | .value' "${temp_dir}/measures.json")"
lines_to_cover="$(jq -r '.component.measures[] | select(.metric == "lines_to_cover") | .value' "${temp_dir}/measures.json")"
persist_published_result_evidence
if [[ -n "${pull_request}" ]]; then
  printf 'Sonar PR result verified: task=%s coverage=%s%% lines_to_cover=%s unresolved_new_issues=0 current_new_hotspots=0\n' \
    "${task_id}" "${coverage}" "${lines_to_cover}"
else
  printf 'Sonar main result verified: task=%s coverage=%s%% lines_to_cover=%s unresolved_issues=0 current_hotspots=0\n' \
    "${task_id}" "${coverage}" "${lines_to_cover}"
fi
