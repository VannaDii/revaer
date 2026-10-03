#!/usr/bin/env bash
set -euo pipefail

project_key="${SONAR_PROJECT_KEY:?SONAR_PROJECT_KEY is required}"
auth_token="${SONAR_AUTH_TOKEN:?SONAR_AUTH_TOKEN is required}"
api_base="${SONAR_API_BASE_URL:-https://sonarcloud.io/api}"
pull_request="${SONAR_PULL_REQUEST:-}"
temp_dir="$(mktemp -d)"
trap 'rm -rf "${temp_dir}"' EXIT

sonar_get() {
  local endpoint="$1"
  local output_path="$2"
  shift 2

  if [[ -n "${pull_request}" ]]; then
    set -- --data-urlencode "pullRequest=${pull_request}" "$@"
  fi

  curl --fail --silent --show-error \
    --user "${auth_token}:" \
    --get "${api_base}/${endpoint}" \
    "$@" \
    --output "${output_path}"
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
