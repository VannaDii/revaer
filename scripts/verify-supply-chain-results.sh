#!/usr/bin/env bash
set -euo pipefail

readonly required_results=(
  AUDIT_RESULT
  DENY_RESULT
  UDEPS_RESULT
)

failures=0
for variable_name in "${required_results[@]}"; do
  value="${!variable_name-}"
  if [[ "${value}" != "success" ]]; then
    printf 'supply-chain-results: %s must be success, received %s\n' \
      "${variable_name}" "${value:-<empty>}" >&2
    failures=1
  fi
done

if [[ "${failures}" -ne 0 ]]; then
  exit 1
fi

printf 'supply-chain-results: audit, deny, and unused-dependency checks passed\n'
