#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture_root="${repo_root}/scripts/tests/fixtures/trivy"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-trivy-sarif.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT
validator="${repo_root}/scripts/verify-trivy-sarif.sh"

fail_test() {
  local message="$1"
  printf 'Trivy SARIF policy regression test failed: %s\n' "${message}" >&2
  exit 1
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >"${test_root}/${label}.stdout" 2>"${test_root}/${label}.stderr"; then
    fail_test "${label} unexpectedly passed"
  fi
}

write_sarif() {
  local path="$1"
  local version="$2"
  local tool_name="$3"
  local results="$4"
  jq -n \
    --arg version "${version}" \
    --arg tool_name "${tool_name}" \
    --argjson results "${results}" \
    '{
      version: $version,
      runs: [{tool: {driver: {name: $tool_name}}, results: $results}]
    }' > "${path}"
}

"${validator}" "${fixture_root}/clean.sarif"
expect_failure vulnerable-image "${validator}" "${fixture_root}/vulnerable-image.sarif"
expect_failure missing-argument "${validator}"
expect_failure extra-argument "${validator}" \
  "${fixture_root}/clean.sarif" "${fixture_root}/clean.sarif"
expect_failure missing-report "${validator}" "${test_root}/missing.sarif"
touch "${test_root}/empty.sarif"
expect_failure empty-report "${validator}" "${test_root}/empty.sarif"
printf '{not-json}\n' > "${test_root}/malformed.sarif"
expect_failure malformed-report "${validator}" "${test_root}/malformed.sarif"

write_sarif "${test_root}/wrong-version.sarif" 2.0.0 Trivy '[]'
expect_failure wrong-version "${validator}" "${test_root}/wrong-version.sarif"

jq -n '{version: "2.1.0", runs: []}' > "${test_root}/empty-runs.sarif"
expect_failure empty-runs "${validator}" "${test_root}/empty-runs.sarif"

write_sarif "${test_root}/wrong-tool.sarif" 2.1.0 AnotherScanner '[]'
expect_failure wrong-tool "${validator}" "${test_root}/wrong-tool.sarif"

jq -n '{
  version: "2.1.0",
  runs: [{tool: {driver: {name: "Trivy"}}}]
}' > "${test_root}/missing-results.sarif"
expect_failure missing-results "${validator}" "${test_root}/missing-results.sarif"

write_sarif "${test_root}/fallback-finding.sarif" 2.1.0 Trivy '[{}]'
expect_failure fallback-finding "${validator}" "${test_root}/fallback-finding.sarif"

write_sarif "${test_root}/multiple-findings.sarif" 2.1.0 Trivy \
  '[
    {"ruleId": "CVE-1", "message": {"text": "high severity"}},
    {"ruleId": "CVE-2", "message": {"text": "critical severity"}}
  ]'
expect_failure multiple-findings "${validator}" "${test_root}/multiple-findings.sarif"

printf 'Trivy SARIF policy regression tests passed\n'
