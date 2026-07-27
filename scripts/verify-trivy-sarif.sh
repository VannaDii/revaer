#!/usr/bin/env bash
set -euo pipefail

fail() {
  local message="$1"
  printf 'Trivy SARIF verification failed: %s\n' "${message}" >&2
  exit 1
}

[[ "$#" -eq 1 ]] || fail "SARIF path is required"
sarif_path="$1"
[[ -s "${sarif_path}" ]] || fail "SARIF report is missing or empty: ${sarif_path}"
command -v jq >/dev/null 2>&1 || fail "jq is required"

jq -e '
  .version == "2.1.0"
  and (.runs | type == "array" and length > 0)
  and all(.runs[]; .tool.driver.name == "Trivy" and (.results | type == "array"))
' "${sarif_path}" >/dev/null || fail "report must be Trivy SARIF 2.1.0 with explicit results arrays"

finding_count="$(jq '[.runs[].results[]] | length' "${sarif_path}")"
if [[ "${finding_count}" -ne 0 ]]; then
  jq -r '.runs[].results[] | "\(.ruleId // "unknown"): \(.message.text // "HIGH/CRITICAL vulnerability")"' "${sarif_path}" >&2
  fail "${finding_count} HIGH or CRITICAL finding(s) block verification and publication"
fi
