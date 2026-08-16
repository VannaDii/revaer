#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

AUDIT_RESULT=success DENY_RESULT=success UDEPS_RESULT=success \
  bash "${repo_root}/scripts/verify-supply-chain-results.sh" >/dev/null

for rejected in failure cancelled skipped unknown ""; do
  if AUDIT_RESULT="${rejected}" DENY_RESULT=success UDEPS_RESULT=success \
    bash "${repo_root}/scripts/verify-supply-chain-results.sh" >/dev/null 2>&1; then
    printf 'Supply-chain result guard accepted rejected audit result: %s\n' \
      "${rejected:-<empty>}" >&2
    exit 1
  fi
done

printf '%s\n' "Supply-chain result regression tests passed"
