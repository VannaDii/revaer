#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
guardrail="${repo_root}/scripts/generated-api-schema-guardrail.sh"
fixture_root="$(mktemp -d)"
trap 'rm -rf "${fixture_root}"' EXIT

fixture_repo="${fixture_root}/repo"
schema_path="tests/support/api/schema.ts"

git init -q -b main "${fixture_repo}"
mkdir -p "${fixture_repo}/tests/support/api"
printf '/%s\n' "${schema_path}" > "${fixture_repo}/.gitignore"

REVAER_GUARDRAIL_REPO_ROOT="${fixture_repo}" bash "${guardrail}"

printf 'generated fixture\n' > "${fixture_repo}/${schema_path}"
git -C "${fixture_repo}" add -f "${schema_path}"
if REVAER_GUARDRAIL_REPO_ROOT="${fixture_repo}" bash "${guardrail}" >/dev/null 2>&1; then
  printf 'Expected the generated API schema guardrail to reject a tracked schema.\n' >&2
  exit 1
fi

git -C "${fixture_repo}" rm --cached -q -- "${schema_path}"
printf '' > "${fixture_repo}/.gitignore"
if REVAER_GUARDRAIL_REPO_ROOT="${fixture_repo}" bash "${guardrail}" >/dev/null 2>&1; then
  printf 'Expected the generated API schema guardrail to reject a non-ignored schema.\n' >&2
  exit 1
fi

printf 'Generated API schema guardrail tests passed.\n'
