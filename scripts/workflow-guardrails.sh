#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

exec ruby scripts/workflow-structure-guardrails.rb \
  --workflows .github/workflows \
  --actions .github/actions \
  --sonar sonar-project.properties \
  --justfile justfile
