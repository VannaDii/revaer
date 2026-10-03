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
if [[ -n "${REVAER_FAKE_FFPROBE_DIAGNOSTIC_PATH:-}" ]]; then
  cat "${REVAER_FAKE_FFPROBE_DIAGNOSTIC_PATH}" >&2
fi
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

diagnostic_path="${temporary_root}/diagnostic.txt"
printf 'expected parser diagnostic\n' >"${diagnostic_path}"
export REVAER_FAKE_FFPROBE_DIAGNOSTIC_PATH="${diagnostic_path}"
if scripts/test-fixtures/probe-fixtures.sh --check >/dev/null 2>&1; then
  printf 'test-probe-verification: unapproved diagnostics unexpectedly passed\n' >&2
  exit 1
fi

cat >"${manifest_path}" <<EOF
{
  "fixtures": [
    {"id": "sample", "path": "${fixture_path}", "allowProbeDiagnostics": true}
  ]
}
EOF
diagnostic_output="$(scripts/test-fixtures/probe-fixtures.sh --check)"
if [[ "${diagnostic_output}" != *"accepted bounded diagnostics for sample"* ]]; then
  printf 'test-probe-verification: approved diagnostic classification was not reported\n' >&2
  exit 1
fi

oversized_diagnostic_path="${temporary_root}/oversized-diagnostic.txt"
awk 'BEGIN { for (byte_count = 0; byte_count < 4097; byte_count += 1) printf "x" }' >"${oversized_diagnostic_path}"
export REVAER_FAKE_FFPROBE_DIAGNOSTIC_PATH="${oversized_diagnostic_path}"
if scripts/test-fixtures/probe-fixtures.sh --check >/dev/null 2>&1; then
  printf 'test-probe-verification: oversized diagnostics unexpectedly passed\n' >&2
  exit 1
fi

cat >"${manifest_path}" <<EOF
{
  "fixtures": [
    {"id": "sample", "path": "${fixture_path}", "allowProbeDiagnostics": "yes"}
  ]
}
EOF
if scripts/test-fixtures/probe-fixtures.sh --check >/dev/null 2>&1; then
  printf 'test-probe-verification: non-boolean diagnostic policy unexpectedly passed\n' >&2
  exit 1
fi

cat >"${manifest_path}" <<EOF
{
  "fixtures": [
    {"id": "sample", "path": "${fixture_path}"}
  ]
}
EOF
unset REVAER_FAKE_FFPROBE_DIAGNOSTIC_PATH

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
printf 'test-probe-verification: read-only check, strict diagnostics, drift failure, and explicit update passed\n'
