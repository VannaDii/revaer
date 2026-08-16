#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

require_tool() {
  local tool="$1"
  if ! command -v "${tool}" >/dev/null 2>&1; then
    printf 'clean-test-fixtures: required tool missing: %s\n' "${tool}" >&2
    exit 1
  fi
}

for tool in ffmpeg ffprobe curl git base64; do
  require_tool "${tool}"
done

rm -rf test-fixtures/source test-fixtures/matroska test-fixtures/chromium test-fixtures/derived
mkdir -p test-fixtures/probe

printf 'clean-test-fixtures: removed ignored media fixture directories\n'
