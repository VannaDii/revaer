#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

if [[ -n "${REVAER_KCOV_BASH_HELPER:-}" ]]; then
  ruby scripts/prepare-kcov-bash-helper.rb "${REVAER_KCOV_BASH_HELPER}"
fi

bash scripts/policy-guardrails.sh
bash scripts/workflow-guardrails.sh
for test_script in scripts/tests/*-test.sh; do
  bash "${test_script}"
done
