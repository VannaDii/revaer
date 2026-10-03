#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

bash scripts/policy-guardrails.sh
bash scripts/workflow-guardrails.sh
bash scripts/tests/workflow-guardrails-test.sh
bash scripts/tests/trivy-sarif-policy-test.sh
bash scripts/tests/final-image-compliance-test.sh
bash scripts/tests/media-compliance-guardrails-test.sh
bash scripts/test-exact-cargo-tool.sh
bash scripts/generated-api-schema-guardrail.sh
bash scripts/test-generated-api-schema-guardrail.sh
bash scripts/tests/script-coverage-test.sh
bash scripts/tests/sonar-result-guardrails-test.sh
