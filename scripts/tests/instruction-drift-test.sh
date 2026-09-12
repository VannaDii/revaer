#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

exec ruby --disable-gems "${repo_root}/scripts/tests/instruction-drift-test.rb" "${repo_root}"
