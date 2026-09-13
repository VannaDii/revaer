#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ruby "${repo_root}/scripts/tests/coverage-report-test.rb" "${repo_root}"
