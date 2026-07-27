#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf 'verify-fixtures: required tool missing: %s\n' "${tool}" >&2
    exit 1
  fi
}

for tool in ffmpeg ffprobe curl git base64; do
  require_tool "${tool}"
done

mkdir -p test-fixtures/probe

REVAER_MEDIA_FIXTURE_WRITE_PROBES=1 \
  cargo --config 'build.rustflags=["-Dwarnings"]' test \
    -p revaer-media-runtime \
    --test media_fixtures \
    verify_prepared_fixture_suite \
    --all-features \
    -- --ignored --nocapture

printf 'verify-fixtures: complete\n'
