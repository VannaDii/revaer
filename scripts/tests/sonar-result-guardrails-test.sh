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
while [[ "$#" -gt 0 ]]; do
  case "$1" in
    --output)
      output_path="$2"
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
  */measures/component) payload="${MOCK_MEASURES_JSON}" ;;
  */qualitygates/project_status) payload="${MOCK_QUALITY_GATE_JSON}" ;;
  */issues/search) payload="${MOCK_ISSUES_JSON}" ;;
  */hotspots/search) payload="${MOCK_HOTSPOTS_JSON}" ;;
  *)
    printf 'Unexpected Sonar endpoint: %s\n' "${endpoint}" >&2
    exit 22
    ;;
esac

printf '%s\n' "${payload}" > "${output_path}"
MOCK
chmod +x "${mock_bin}/curl"

valid_measures='{"component":{"measures":[{"metric":"coverage","value":"84.2"},{"metric":"line_coverage","value":"84.2"},{"metric":"lines_to_cover","value":"100"},{"metric":"uncovered_lines","value":"16"}]}}'
valid_quality_gate='{"projectStatus":{"status":"OK","ignoredConditions":false}}'
valid_issues='{"total":0}'
valid_hotspots='{"paging":{"total":0}}'

run_guardrail() {
  local pull_request="$1"
  env \
    PATH="${mock_bin}:${PATH}" \
    MOCK_CURL_LOG="${mock_log}" \
    MOCK_MEASURES_JSON="${MOCK_MEASURES_JSON:-${valid_measures}}" \
    MOCK_QUALITY_GATE_JSON="${MOCK_QUALITY_GATE_JSON:-${valid_quality_gate}}" \
    MOCK_ISSUES_JSON="${MOCK_ISSUES_JSON:-${valid_issues}}" \
    MOCK_HOTSPOTS_JSON="${MOCK_HOTSPOTS_JSON:-${valid_hotspots}}" \
    SONAR_PROJECT_KEY="VannaDii_Revaer" \
    SONAR_AUTH_TOKEN="fixture-token" \
    SONAR_API_BASE_URL="https://sonar.invalid/api" \
    SONAR_PULL_REQUEST="${pull_request}" \
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

expect_failure "a missing project key" env \
  -u SONAR_PROJECT_KEY \
  SONAR_AUTH_TOKEN="fixture-token" \
  bash "${repo_root}/scripts/sonar-result-guardrails.sh"
expect_failure "a missing authentication token" env \
  -u SONAR_AUTH_TOKEN \
  SONAR_PROJECT_KEY="VannaDii_Revaer" \
  bash "${repo_root}/scripts/sonar-result-guardrails.sh"

printf '%s\n' "Sonar result guardrail regression tests passed"
