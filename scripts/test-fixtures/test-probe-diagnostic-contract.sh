#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"
source scripts/test-fixtures/lib.sh
source scripts/test-fixtures/probe-diagnostic-contract.sh

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-f1-test.XXXXXX")"
trap 'rm -rf "${temporary_root}"' EXIT HUP INT TERM
case_root="${temporary_root}/repo"
mkdir -p "${case_root}/scripts/test-fixtures" "${case_root}/test-fixtures/probe" \
  "${case_root}/test-fixtures/matroska" "${temporary_root}/bin"
cp scripts/test-fixtures/{probe-fixtures.sh,probe-diagnostic-contract.sh,lib.sh} "${case_root}/scripts/test-fixtures/"
fixture_id=mkv-theora-vorbis-live-style
fixture_path="test-fixtures/matroska/${fixture_id}.mkv"
snapshot_path="test-fixtures/probe/${fixture_id}.json"
cp test-fixtures/lock.json "${temporary_root}/lock.json"
jq --arg id "${fixture_id}" '.fixtures |= map(select(.id == $id))' \
  test-fixtures/manifest.json >"${temporary_root}/manifest.json"
cp "${snapshot_path}" "${case_root}/${snapshot_path}"

# The isolated script-plumbing cases use a synthetic, correctly sized payload
# and an explicit fake source hash. They are not real media/conversion evidence.
dd if=/dev/zero of="${case_root}/${fixture_path}" bs=21313902 count=1 2>"${temporary_root}/dd.log"
export F1_TEST_SOURCE_HASH=real
export F1_TEST_REAL_HASH
export F1_TEST_HASH_STYLE
if command -v sha256sum >/dev/null 2>&1; then
  F1_TEST_REAL_HASH="$(command -v sha256sum)"
  F1_TEST_HASH_STYLE=gnu
else
  F1_TEST_REAL_HASH="$(command -v shasum)"
  F1_TEST_HASH_STYLE=perl
fi
export F1_TEST_REAL_INSTALL
F1_TEST_REAL_INSTALL="$(command -v install)"
cat >"${temporary_root}/bin/sha256sum" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${F1_TEST_SOURCE_HASH}" == "fake" && "$1" == "test-fixtures/matroska/mkv-theora-vorbis-live-style.mkv" ]]; then
  printf '43df750a2a01a37949791b717051b41522081a266b71d113be4b713063843699  %s\n' "$1"
elif [[ "${F1_TEST_HASH_STYLE}" == "gnu" ]]; then
  "${F1_TEST_REAL_HASH}" "$1"
else
  "${F1_TEST_REAL_HASH}" -a 256 "$1"
fi
SH
cat >"${temporary_root}/bin/install" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
"${F1_TEST_REAL_INSTALL}" "$@"
destination="${!#}"
if [[ -n "${F1_TEST_LOST_EVIDENCE:-}" && "${destination}" == *"/${F1_TEST_LOST_EVIDENCE}" ]]; then
  if [[ "${F1_TEST_LOSS_MODE:-}" == "missing" ]]; then
    rm "${destination}"
  else
    printf 'corrupted evidence\n' >"${destination}"
  fi
fi
SH
cat >"${temporary_root}/bin/ffprobe" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "-version" ]]; then
  cat "${F1_TEST_VERSION}"
  printf '%s' "${F1_TEST_VERSION_STDERR:-}" >&2
  exit "${F1_TEST_VERSION_STATUS:-0}"
fi
cat "${F1_TEST_DIAGNOSTIC}" >&2
cat "${F1_TEST_JSON}"
exit "${F1_TEST_PROBE_STATUS:-0}"
SH
chmod 0700 "${temporary_root}/bin/"*
export PATH="${temporary_root}/bin:${PATH}"
export REVAER_FIXTURE_MANIFEST_PATH="${case_root}/test-fixtures/manifest.json"
export REVAER_FIXTURE_LOCK_PATH="${case_root}/test-fixtures/lock.json"
export REVAER_FIXTURE_PROBE_DIR="${case_root}/test-fixtures/probe"
export REVAER_FIXTURE_FFPROBE_BIN="${temporary_root}/bin/ffprobe"
export REVAER_MEDIA_CONVERSION_REPORT="${temporary_root}/report.md"
export F1_TEST_DIAGNOSTIC="${temporary_root}/diagnostic.txt"
export F1_TEST_JSON="${temporary_root}/probe.json"
export F1_TEST_VERSION="${temporary_root}/version.txt"
readonly diagnostic_prefix="[matroska,webm @ "
readonly diagnostic_suffix="] Length 5 indicated by an EBML number's first byte 0x0a at pos 35 (0x23) exceeds max length 4."
case_count=0

reset_case() {
  cp "${temporary_root}/manifest.json" "${REVAER_FIXTURE_MANIFEST_PATH}"
  cp "${temporary_root}/lock.json" "${REVAER_FIXTURE_LOCK_PATH}"
  cp "${repo_root}/${snapshot_path}" "${case_root}/${snapshot_path}"
  cp "${repo_root}/${snapshot_path}" "${F1_TEST_JSON}"
  cp scripts/test-fixtures/testdata/f1-host-version.txt "${F1_TEST_VERSION}"
  printf '%s0x1%s\n' "${diagnostic_prefix}" "${diagnostic_suffix}" >"${F1_TEST_DIAGNOSTIC}"
  unset F1_TEST_PROBE_STATUS F1_TEST_VERSION_STATUS F1_TEST_VERSION_STDERR F1_TEST_LOST_EVIDENCE F1_TEST_LOSS_MODE
  export REVAER_MEDIA_CONVERSION_REPORT="${temporary_root}/report.md"
}

reject_probe() {
  local label="$1" expected="$2" mode="${3:---check}"
  if bash "${case_root}/scripts/test-fixtures/probe-fixtures.sh" "${mode}" >"${temporary_root}/output" 2>&1; then
    printf 'test-probe-diagnostic-contract: unexpectedly accepted %s\n' "${label}" >&2
    exit 1
  fi
  if ! grep -Fq -- "${expected}" "${temporary_root}/output"; then
    cat "${temporary_root}/output" >&2
    printf 'test-probe-diagnostic-contract: wrong failure for %s\n' "${label}" >&2
    exit 1
  fi
  case_count="$((case_count + 1))"
}

accept_probe() {
  local output evidence
  output="$(bash "${case_root}/scripts/test-fixtures/probe-fixtures.sh" --check 2>"${temporary_root}/accepted.stderr")"
  [[ "${output}" == *"ADR578 F1 exact diagnostic emissions accepted: 1"* ]]
  [[ "${output}" == *"exact-locked-fixture-recovery-diagnostic; fixture=${fixture_id}; count=1;"* ]]
  [[ "${output}" == *"$(cat "${F1_TEST_VERSION}")"* ]]
  evidence="$(printf '%s\n' "${output}" | sed -n 's/.*; evidence=//p')"
  cmp "${F1_TEST_DIAGNOSTIC}" "${temporary_root}/accepted.stderr"
  cmp "${F1_TEST_DIAGNOSTIC}" "${evidence}/probe.stderr"
  cmp "${F1_TEST_VERSION}" "${evidence}/ffprobe-version.txt"
  jq -e --arg diagnostic "$(fixture_sha256 "${F1_TEST_DIAGNOSTIC}")" \
    --arg version "$(fixture_sha256 "${F1_TEST_VERSION}")" '
      .count == 1 and .contract == "adr578-f1" and
      .classification == "exact-locked-fixture-recovery-diagnostic" and
      .diagnostic_sha256 == $diagnostic and .version_report_sha256 == $version
    ' "${evidence}/classification.json" >/dev/null
  case_count="$((case_count + 1))"
}

reset_case
reject_probe 'synthetic source bytes with real hash' 'SHA-256 mismatch'
export F1_TEST_SOURCE_HASH=fake
for profile in host linux; do
  for pointer in 0x0 0xabcdef0123456789; do
    reset_case
    cp "scripts/test-fixtures/testdata/f1-${profile}-version.txt" "${F1_TEST_VERSION}"
    printf '%s%s%s\n' "${diagnostic_prefix}" "${pointer}" "${diagnostic_suffix}" >"${F1_TEST_DIAGNOSTIC}"
    accept_probe
  done
done

for mutation in \
  '.fixtures[0].probeDiagnosticContract = "unknown"' \
  '.fixtures[0].probeDiagnosticContract = null' \
  '.fixtures[0].probeDiagnosticContract = true' \
  '.fixtures[0].allowProbeDiagnostics = true' \
  '.fixtures[0].allowProbeDiagnostics = false' \
  'del(.fixtures[0].probeDiagnosticContract) | .fixtures[0].allowProbeDiagnostics = true' \
  '.fixtures[0].id = "another-fixture"' \
  '.fixtures += .fixtures'; do
  reset_case
  jq "${mutation}" "${temporary_root}/manifest.json" >"${REVAER_FIXTURE_MANIFEST_PATH}"
  reject_probe "policy: ${mutation}" 'manifest is invalid'
done
for mutation in \
  '.fixtures[0].path = "scripts/test-fixtures/lib.sh"' \
  '.fixtures[0].source += "?changed"' \
  '.fixtures[0].shouldDownload = false' \
  '.fixtures[0].generated = true' \
  '.fixtures[0].shouldGenerate = true'; do
  reset_case
  jq "${mutation}" "${temporary_root}/manifest.json" >"${REVAER_FIXTURE_MANIFEST_PATH}"
  reject_probe "identity: ${mutation}" 'identity or reviewed snapshot expired'
done
for mutation in \
  '.upstreams.matroska.revision = "other"' \
  '.upstreams.matroska.repository = "https://example.invalid/other"' \
  '(.sources[] | select(.id == "mkv-theora-vorbis-live-style").sha256) = ("0" * 64)' \
  '(.sources[] | select(.id == "mkv-theora-vorbis-live-style").minimumBytes) -= 1' \
  '(.sources[] | select(.id == "mkv-theora-vorbis-live-style").maximumBytes) += 1' \
  '(.sources[] | select(.id == "mkv-theora-vorbis-live-style").encoding) = "base64"' \
  '(.sources[] | select(.id == "mkv-theora-vorbis-live-style").urls) += ["https://example.invalid/other"]' \
  '.sources |= map(select(.id != "mkv-theora-vorbis-live-style"))' \
  '.sources += [.sources[] | select(.id == "mkv-theora-vorbis-live-style")]'; do
  reset_case
  jq "${mutation}" "${temporary_root}/lock.json" >"${REVAER_FIXTURE_LOCK_PATH}"
  reject_probe "lock: ${mutation}" 'identity or reviewed snapshot expired'
done
for diagnostic in \
  "${diagnostic_prefix}0xA${diagnostic_suffix}\n" \
  "${diagnostic_prefix}0x12345678901234567${diagnostic_suffix}\n" \
  "${diagnostic_prefix}0x${diagnostic_suffix}\n" \
  "${diagnostic_prefix}0X1${diagnostic_suffix}\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix/pos 35/pos 36}\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix/Length 5/Length 4}\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix/0x0a/0x0b}\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix/0x23/0x24}\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix/max length 4/max length 5}\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix}" \
  "${diagnostic_prefix}0x1${diagnostic_suffix}\r\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix}\n\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix}\nextra\n" \
  "\n${diagnostic_prefix}0x1${diagnostic_suffix}\n" \
  "${diagnostic_prefix}0x1\0${diagnostic_suffix}\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix}\0\n" \
  "${diagnostic_prefix}0x1${diagnostic_suffix}\303\251\n"; do
  reset_case
  printf '%b' "${diagnostic}" >"${F1_TEST_DIAGNOSTIC}"
  reject_probe 'diagnostic bytes/framing' 'diagnostic contract expired'
done
reset_case
awk 'BEGIN { for (i = 0; i < 4097; i++) printf "x" }' >"${F1_TEST_DIAGNOSTIC}"
reject_probe 'global stderr limit' 'exceed the 4096-byte limit'
reset_case
export F1_TEST_PROBE_STATUS=1
reject_probe 'nonzero probe status' 'probe command failed'
for json in '' 'invalid JSON' '{"streams":[]}' '{}'; do
  reset_case
  printf '%s' "${json}" >"${F1_TEST_JSON}"
  reject_probe 'empty, malformed or mismatched JSON' 'probe-fixtures:'
done
reset_case
printf '\n' >>"${case_root}/${snapshot_path}"
reject_probe 'reviewed snapshot drift' 'identity or reviewed snapshot expired'
reject_probe 'update must not replace F1 snapshot' 'identity or reviewed snapshot expired' --update
cmp <(printf '%s\n\n' "$(cat "${repo_root}/${snapshot_path}")") "${case_root}/${snapshot_path}"
reset_case
rm "${case_root}/${snapshot_path}"
reject_probe 'missing snapshot' 'identity or reviewed snapshot expired'
reset_case
mv "${case_root}/${fixture_path}" "${temporary_root}/source.bin"
reject_probe 'missing media' 'fixture is missing or empty'
printf 'too small' >"${case_root}/${fixture_path}"
reject_probe 'wrong media size' 'expected 21313902..21313902'
rm "${case_root}/${fixture_path}"
ln -s "${temporary_root}/source.bin" "${case_root}/${fixture_path}"
reject_probe 'symlink media' 'must not be a symbolic link'
rm "${case_root}/${fixture_path}"
mv "${temporary_root}/source.bin" "${case_root}/${fixture_path}"

for version in '' 'ffprobe version 9.0.2' 'ffprobe version 8.0.1'; do
  reset_case
  printf '%s\n' "${version}" >"${F1_TEST_VERSION}"
  reject_probe 'unknown version/build report' 'tool report is failed, diagnostic-bearing or unapproved'
done
reset_case
printf '\n' >>"${F1_TEST_VERSION}"
reject_probe 'full report byte drift' 'tool report is failed, diagnostic-bearing or unapproved'
reset_case
export F1_TEST_VERSION_STATUS=1
reject_probe 'failed version command' 'tool report is failed, diagnostic-bearing or unapproved'
reset_case
export F1_TEST_VERSION_STDERR=unexpected
reject_probe 'version stderr' 'tool report is failed, diagnostic-bearing or unapproved'
for lost in probe.stderr ffprobe-version.txt; do
  for loss_mode in missing corrupt; do
    reset_case
    export F1_TEST_LOST_EVIDENCE="${lost}" F1_TEST_LOSS_MODE="${loss_mode}"
    reject_probe 'lost or changed retained evidence' 'raw evidence retention failed'
  done
done
reset_case
printf 'not a directory' >"${temporary_root}/unwritable-parent"
export REVAER_MEDIA_CONVERSION_REPORT="${temporary_root}/unwritable-parent/report"
reject_probe 'unwritable evidence destination' 'raw evidence retention failed'

reset_case
printf '' >"${F1_TEST_DIAGNOSTIC}"
printf 'unknown version' >"${F1_TEST_VERSION}"
output="$(bash "${case_root}/scripts/test-fixtures/probe-fixtures.sh" --check)"
[[ "${output}" == *'ADR578 F1 exact diagnostic emissions accepted: 0'* ]]
case_count="$((case_count + 1))"
reset_case
jq 'del(.fixtures[0].probeDiagnosticContract)' "${temporary_root}/manifest.json" >"${REVAER_FIXTURE_MANIFEST_PATH}"
reject_probe 'no contract still rejects exact error' 'unapproved diagnostics emitted'
jq '.fixtures[0].id = "other-fixture"' "${REVAER_FIXTURE_MANIFEST_PATH}" >"${temporary_root}/other.json"
cp "${temporary_root}/other.json" "${REVAER_FIXTURE_MANIFEST_PATH}"
reject_probe 'other fixture still rejects exact error' 'unapproved diagnostics emitted'

printf 'test-probe-diagnostic-contract: %s adversarial/acceptance cases passed; synthetic source/hash and probe doubles, not real conversion evidence\n' "${case_count}"
