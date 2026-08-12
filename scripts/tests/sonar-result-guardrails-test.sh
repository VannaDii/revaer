#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-sonar-result.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT

mock_bin="${test_root}/bin"
mock_log="${test_root}/curl.log"
mkdir -p "${mock_bin}"
cat > "${mock_bin}/curl" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail

printf '%q ' "$@" >> "${MOCK_CURL_LOG}"
printf '\n' >> "${MOCK_CURL_LOG}"

output_path=""
endpoint=""
write_out=""
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --output)
      output_path="$2"
      shift 2
      ;;
    --write-out)
      write_out="$2"
      shift 2
      ;;
    http://*|https://*)
      endpoint="$1"
      shift
      ;;
    *)
      shift
      ;;
  esac
done

case "${endpoint}" in
  */measures/component) endpoint_key="measures" ;;
  */qualitygates/project_status) endpoint_key="quality-gate" ;;
  */issues/search) endpoint_key="issues" ;;
  */hotspots/search) endpoint_key="hotspots" ;;
  *) endpoint_key="unexpected" ;;
esac
state_file="${MOCK_CURL_STATE_DIR}/${endpoint_key}"
call_count=0
if [[ -f "${state_file}" ]]; then
  call_count="$(<"${state_file}")"
fi
call_count="$((call_count + 1))"
printf '%s\n' "${call_count}" > "${state_file}"

case "${endpoint}" in
  */measures/component)
    if [[ "${call_count}" -eq 1 && -n "${MOCK_MEASURES_FIRST_JSON:-}" ]]; then
      payload="${MOCK_MEASURES_FIRST_JSON}"
    else
      payload="${MOCK_MEASURES_JSON}"
    fi
    ;;
  */qualitygates/project_status) payload="${MOCK_QUALITY_GATE_JSON}" ;;
  */issues/search) payload="${MOCK_ISSUES_JSON}" ;;
  */hotspots/search) payload="${MOCK_HOTSPOTS_JSON}" ;;
  *)
    printf 'Unexpected Sonar endpoint: %s\n' "${endpoint}" >&2
    exit 22
    ;;
esac

if [[ -n "${write_out}" ]]; then
  if [[ "${endpoint_key}" == "hotspots" && -n "${MOCK_HOTSPOTS_HTTP_STATUS_SEQUENCE:-}" ]]; then
    status_sequence="${MOCK_HOTSPOTS_HTTP_STATUS_SEQUENCE}"
  else
    status_sequence="${MOCK_CURL_HTTP_STATUS_SEQUENCE:-${MOCK_CURL_HTTP_STATUS:-200}}"
  fi
  IFS=',' read -r -a statuses <<< "${status_sequence}"
  status_index="$((call_count - 1))"
  if ((status_index >= ${#statuses[@]})); then
    status_index="$((${#statuses[@]} - 1))"
  fi
  http_status="${statuses[status_index]}"
else
  http_status="200"
fi

if [[ "${MOCK_CURL_OMIT_NON_SUCCESS_OUTPUT:-false}" != "true" || \
  "${http_status}" =~ ^2[0-9][0-9]$ ]]; then
  printf '%s\n' "${payload}" > "${output_path}"
fi
if [[ -n "${write_out}" ]]; then
  printf '%s' "${http_status}"
fi
MOCK
chmod +x "${mock_bin}/curl"

valid_measures='{"component":{"measures":[{"metric":"coverage","value":"84.2"},{"metric":"line_coverage","value":"84.2"},{"metric":"lines_to_cover","value":"100"},{"metric":"uncovered_lines","value":"16"}]}}'
valid_quality_gate='{"projectStatus":{"status":"OK","ignoredConditions":false}}'
valid_issues='{"total":0}'
valid_hotspots='{"paging":{"total":0}}'

run_guardrail() {
  local pull_request="$1"
  local mock_state="${test_root}/state"
  rm -rf "${mock_state}"
  mkdir -p "${mock_state}"
  env \
    PATH="${mock_bin}:${PATH}" \
    MOCK_CURL_LOG="${mock_log}" \
    MOCK_MEASURES_FIRST_JSON="${MOCK_MEASURES_FIRST_JSON:-}" \
    MOCK_MEASURES_JSON="${MOCK_MEASURES_JSON:-${valid_measures}}" \
    MOCK_QUALITY_GATE_JSON="${MOCK_QUALITY_GATE_JSON:-${valid_quality_gate}}" \
    MOCK_ISSUES_JSON="${MOCK_ISSUES_JSON:-${valid_issues}}" \
    MOCK_HOTSPOTS_JSON="${MOCK_HOTSPOTS_JSON:-${valid_hotspots}}" \
    MOCK_CURL_HTTP_STATUS="${MOCK_CURL_HTTP_STATUS:-200}" \
    MOCK_CURL_HTTP_STATUS_SEQUENCE="${MOCK_CURL_HTTP_STATUS_SEQUENCE:-}" \
    MOCK_HOTSPOTS_HTTP_STATUS_SEQUENCE="${MOCK_HOTSPOTS_HTTP_STATUS_SEQUENCE:-}" \
    MOCK_CURL_OMIT_NON_SUCCESS_OUTPUT="${MOCK_CURL_OMIT_NON_SUCCESS_OUTPUT:-false}" \
    MOCK_CURL_STATE_DIR="${mock_state}" \
    SONAR_PROJECT_KEY="VannaDii_Revaer" \
    SONAR_AUTH_TOKEN="fixture-token" \
    SONAR_API_BASE_URL="https://sonar.invalid/api" \
    SONAR_PULL_REQUEST="${pull_request}" \
    SONAR_RESULT_RETRY_ATTEMPTS="${SONAR_RESULT_RETRY_ATTEMPTS:-1}" \
    SONAR_RESULT_RETRY_DELAY_SECONDS="${SONAR_RESULT_RETRY_DELAY_SECONDS:-0}" \
    bash "${repo_root}/scripts/sonar-result-guardrails.sh"
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >/dev/null 2>&1; then
    printf 'Sonar result guardrail accepted %s\n' "${label}" >&2
    exit 1
  fi
}

run_guardrail "" > "${test_root}/result.txt"
grep -q 'coverage=84.2% lines_to_cover=100' "${test_root}/result.txt"

: > "${mock_log}"
run_guardrail "73" >/dev/null
grep -q 'pullRequest=73' "${mock_log}"

MOCK_MEASURES_JSON='{"component":{"measures":[{"metric":"coverage","value":"0"},{"metric":"line_coverage","value":"0"},{"metric":"lines_to_cover","value":"100"},{"metric":"uncovered_lines","value":"100"}]}}' \
  expect_failure "zero coverage" run_guardrail "73"
MOCK_QUALITY_GATE_JSON='{"projectStatus":{"status":"OK","ignoredConditions":true}}' \
  expect_failure "ignored quality-gate conditions" run_guardrail "73"
MOCK_ISSUES_JSON='{"total":1}' \
  expect_failure "an unresolved new-code issue" run_guardrail "73"
MOCK_HOTSPOTS_JSON='{"paging":{"total":1}}' \
  expect_failure "an unreviewed new-code hotspot" run_guardrail "73"
MOCK_CURL_HTTP_STATUS="401" \
  expect_failure "a terminal Sonar API response" run_guardrail "73"

: > "${mock_log}"
MOCK_CURL_HTTP_STATUS_SEQUENCE="503,200" \
  SONAR_API_RETRY_ATTEMPTS="2" \
  SONAR_API_RETRY_DELAY_SECONDS="0" \
  run_guardrail "73" >/dev/null
[[ "$(grep -c 'measures/component' "${mock_log}")" -eq 2 ]]

: > "${mock_log}"
MOCK_CURL_HTTP_STATUS_SEQUENCE="429,200" \
  SONAR_API_RETRY_ATTEMPTS="2" \
  SONAR_API_RETRY_DELAY_SECONDS="0" \
  run_guardrail "73" >/dev/null
[[ "$(grep -c 'measures/component' "${mock_log}")" -eq 2 ]]

: > "${mock_log}"
MOCK_MEASURES_FIRST_JSON='{"component":{"measures":[]}}' \
  SONAR_RESULT_RETRY_ATTEMPTS="2" \
  run_guardrail "73" >/dev/null
[[ "$(grep -c 'measures/component' "${mock_log}")" -eq 2 ]]

: > "${mock_log}"
MOCK_HOTSPOTS_HTTP_STATUS_SEQUENCE="503,200" \
  MOCK_CURL_OMIT_NON_SUCCESS_OUTPUT="true" \
  SONAR_API_RETRY_ATTEMPTS="1" \
  SONAR_RESULT_RETRY_ATTEMPTS="2" \
  run_guardrail "73" > "${test_root}/fetch-retry.stdout" 2> "${test_root}/fetch-retry.stderr"
[[ "$(grep -c 'hotspots/search' "${mock_log}")" -eq 2 ]]
grep -q 'published result is not ready or not strict; retrying attempt 1/2' \
  "${test_root}/fetch-retry.stderr"

: > "${mock_log}"
if MOCK_MEASURES_FIRST_JSON='{"component":{"measures":[]}}' \
  MOCK_HOTSPOTS_HTTP_STATUS_SEQUENCE="200,503" \
  MOCK_CURL_OMIT_NON_SUCCESS_OUTPUT="true" \
  SONAR_API_RETRY_ATTEMPTS="1" \
  SONAR_RESULT_RETRY_ATTEMPTS="2" \
  run_guardrail "73" > "${test_root}/stale.stdout" 2> "${test_root}/stale.stderr"; then
  printf 'Sonar result guardrail accepted an exhausted partial fetch\n' >&2
  exit 1
fi
[[ "$(grep -c 'hotspots/search' "${mock_log}")" -eq 2 ]]
grep -q 'did not satisfy strict criteria after 2 attempt(s)' "${test_root}/stale.stderr"
grep -q '^measures: {"component":{"measures":' "${test_root}/stale.stderr"
grep -q '^quality-gate: {"projectStatus":' "${test_root}/stale.stderr"
grep -q '^issues: {"total":0}' "${test_root}/stale.stderr"
grep -q '^hotspots: <response unavailable>$' "${test_root}/stale.stderr"

large_response="$(printf '%05000d' 0)"
if MOCK_MEASURES_JSON="${large_response}" \
  SONAR_RESULT_RETRY_ATTEMPTS="1" \
  run_guardrail "73" > "${test_root}/bounded.stdout" 2> "${test_root}/bounded.stderr"; then
  printf 'Sonar result guardrail accepted malformed oversized evidence\n' >&2
  exit 1
fi
grep -q '<truncated: 4096 of 5001 bytes shown>' "${test_root}/bounded.stderr"
[[ "$(wc -c < "${test_root}/bounded.stderr")" -lt 5000 ]]

expect_failure "a missing project key" env \
  -u SONAR_PROJECT_KEY \
  SONAR_AUTH_TOKEN="fixture-token" \
  bash "${repo_root}/scripts/sonar-result-guardrails.sh"
expect_failure "a missing authentication token" env \
  -u SONAR_AUTH_TOKEN \
  SONAR_PROJECT_KEY="VannaDii_Revaer" \
  bash "${repo_root}/scripts/sonar-result-guardrails.sh"
expect_failure "an invalid retry attempt count" env \
  SONAR_PROJECT_KEY="VannaDii_Revaer" \
  SONAR_AUTH_TOKEN="fixture-token" \
  SONAR_API_RETRY_ATTEMPTS="0" \
  bash "${repo_root}/scripts/sonar-result-guardrails.sh"
expect_failure "an invalid retry delay" env \
  SONAR_PROJECT_KEY="VannaDii_Revaer" \
  SONAR_AUTH_TOKEN="fixture-token" \
  SONAR_API_RETRY_DELAY_SECONDS="invalid" \
  bash "${repo_root}/scripts/sonar-result-guardrails.sh"
expect_failure "an invalid result retry attempt count" env \
  SONAR_PROJECT_KEY="VannaDii_Revaer" \
  SONAR_AUTH_TOKEN="fixture-token" \
  SONAR_RESULT_RETRY_ATTEMPTS="0" \
  bash "${repo_root}/scripts/sonar-result-guardrails.sh"
expect_failure "an invalid result retry delay" env \
  SONAR_PROJECT_KEY="VannaDii_Revaer" \
  SONAR_AUTH_TOKEN="fixture-token" \
  SONAR_RESULT_RETRY_DELAY_SECONDS="invalid" \
  bash "${repo_root}/scripts/sonar-result-guardrails.sh"

printf '%s\n' "Sonar result guardrail regression tests passed"
