#!/usr/bin/env bash
set -euo pipefail
root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
exec ruby "$root/scripts/tests/database-ingestion-runtime-rank-test.rb" "$@"
