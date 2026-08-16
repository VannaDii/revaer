#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-ui-shard-coverage.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT

write_valid_shard() {
  local shard="$1"
  printf '["GET /fixture/%s"]\n' "${shard}" > \
    "${test_root}/api-coverage-api-none-shard-${shard}.json"
  printf '["/fixture/%s"]\n' "${shard}" > \
    "${test_root}/ui-coverage-ui-chromium-shard-${shard}.json"
}

for shard in 1 2 3; do
  write_valid_shard "${shard}"
done

E2E_COVERAGE_DIR="${test_root}" \
  ruby "${repo_root}/scripts/verify-ui-e2e-shard-coverage.rb" >/dev/null

rm "${test_root}/ui-coverage-ui-chromium-shard-2.json"
if E2E_COVERAGE_DIR="${test_root}" \
  ruby "${repo_root}/scripts/verify-ui-e2e-shard-coverage.rb" >/dev/null 2>&1; then
  echo "UI shard coverage verifier accepted a missing shard record" >&2
  exit 1
fi
write_valid_shard 2

printf '[]\n' > "${test_root}/api-coverage-api-none-shard-3.json"
if E2E_COVERAGE_DIR="${test_root}" \
  ruby "${repo_root}/scripts/verify-ui-e2e-shard-coverage.rb" >/dev/null 2>&1; then
  echo "UI shard coverage verifier accepted an empty shard record" >&2
  exit 1
fi

printf '%s\n' "UI E2E shard coverage tests passed"
