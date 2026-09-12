#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ruby "${repo_root}/scripts/tests/test-database-env-test.rb" "${repo_root}"
