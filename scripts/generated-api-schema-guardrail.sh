#!/usr/bin/env bash
set -euo pipefail

repo_root="${REVAER_GUARDRAIL_REPO_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
readonly schema_path="tests/support/api/schema.ts"

cd "${repo_root}"

failures=0

if git ls-files --error-unmatch -- "${schema_path}" >/dev/null 2>&1; then
  printf 'Generated API schema guardrail failed: %s must not be tracked.\n' "${schema_path}" >&2
  failures=1
fi

if ! git check-ignore --quiet -- "${schema_path}"; then
  printf 'Generated API schema guardrail failed: %s must remain ignored.\n' "${schema_path}" >&2
  failures=1
fi

if [ "${failures}" -ne 0 ]; then
  exit 1
fi
