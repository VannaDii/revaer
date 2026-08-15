#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

AUDIT_RESULT=success DENY_RESULT=success UDEPS_RESULT=success \
  bash scripts/verify-supply-chain-results.sh >/dev/null

for rejected in failure cancelled skipped unknown ""; do
  if AUDIT_RESULT="${rejected}" DENY_RESULT=success UDEPS_RESULT=success \
    bash scripts/verify-supply-chain-results.sh >/dev/null 2>&1; then
    printf 'supply-chain-results-test: accepted rejected audit result: %s\n' \
      "${rejected:-<empty>}" >&2
    exit 1
  fi
done

if DENY_RESULT=success UDEPS_RESULT=success \
  bash scripts/verify-supply-chain-results.sh >/dev/null 2>&1; then
  printf 'supply-chain-results-test: accepted missing audit result\n' >&2
  exit 1
fi

printf 'supply-chain-results-test: all non-success and missing results fail closed\n'
