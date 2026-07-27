#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

source scripts/test-fixtures/lib.sh

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-probe-verification-test.XXXXXX")"
cleanup() {
  rm -rf "${temporary_root}"
}
trap cleanup EXIT HUP INT TERM

fixture_path="${temporary_root}/sample.bin"
manifest_path="${temporary_root}/manifest.json"
probe_directory="${temporary_root}/probe"
fake_ffprobe="${temporary_root}/ffprobe"
mkdir -p "${probe_directory}"
printf 'not-media' >"${fixture_path}"

cat >"${manifest_path}" <<EOF
{
  "fixtures": [
    {"id": "sample", "path": "${fixture_path}"}
  ]
}
EOF
cat >"${fake_ffprobe}" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
cat <<'JSON'
{"streams":[{"index":0,"codec_name":"h264","codec_type":"video","disposition":{"default":1,"forced":0},"tags":{}}]}
JSON
EOF
chmod 0700 "${fake_ffprobe}"

export REVAER_FIXTURE_MANIFEST_PATH="${manifest_path}"
export REVAER_FIXTURE_PROBE_DIR="${probe_directory}"
export REVAER_FIXTURE_FFPROBE_BIN="${fake_ffprobe}"

scripts/test-fixtures/probe-fixtures.sh --update
before_sha="$(fixture_sha256 "${probe_directory}/sample.json")"
scripts/test-fixtures/probe-fixtures.sh --check
after_sha="$(fixture_sha256 "${probe_directory}/sample.json")"
if [[ "${before_sha}" != "${after_sha}" ]]; then
  printf 'test-probe-verification: read-only verification modified the reviewed snapshot\n' >&2
  exit 1
fi

printf '\n' >>"${probe_directory}/sample.json"
drifted_sha="$(fixture_sha256 "${probe_directory}/sample.json")"
if scripts/test-fixtures/probe-fixtures.sh --check >/dev/null 2>&1; then
  printf 'test-probe-verification: drifted snapshot unexpectedly passed\n' >&2
  exit 1
fi
after_drift_check_sha="$(fixture_sha256 "${probe_directory}/sample.json")"
if [[ "${drifted_sha}" != "${after_drift_check_sha}" ]]; then
  printf 'test-probe-verification: drift check modified the reviewed snapshot\n' >&2
  exit 1
fi

scripts/test-fixtures/probe-fixtures.sh --update
scripts/test-fixtures/probe-fixtures.sh --check
printf 'test-probe-verification: read-only check, drift failure, and explicit update passed\n'
