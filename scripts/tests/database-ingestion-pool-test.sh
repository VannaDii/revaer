#!/usr/bin/env bash
set -euo pipefail
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ruby --disable-gems "${repo_root}/scripts/tests/database-ingestion-pool-test.rb"
