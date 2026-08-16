#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

bash scripts/policy-guardrails.sh
bash scripts/workflow-guardrails.sh
for test_script in scripts/tests/*-test.sh; do
  bash "${test_script}"
done
