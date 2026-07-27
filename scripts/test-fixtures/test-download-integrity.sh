#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

source scripts/test-fixtures/lib.sh

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-fixture-integrity-test.XXXXXX")"
cleanup() {
  rm -rf "${temporary_root}"
}
trap cleanup EXIT HUP INT TERM

fake_curl="${temporary_root}/curl"
cat >"${fake_curl}" <<'FAKE_CURL'
#!/usr/bin/env bash
set -euo pipefail

output=""
url=""
printf '%s\n' "$*" >>"${REVAER_FAKE_CURL_LOG}"
while (($# > 0)); do
  case "$1" in
    --output)
      output="$2"
      shift 2
      ;;
    --connect-timeout|--max-time|--retry-max-time|--max-filesize|--retry|--retry-delay|--proto|--proto-redir)
      shift 2
      ;;
    --fail|--location|--show-error|--silent)
      shift
      ;;
    *)
      url="$1"
      shift
      ;;
  esac
done

case "${url}" in
  https://fixture.invalid/good)
    printf 'fixture-data' >"${output}"
    ;;
  https://fixture.invalid/base64-good)
    printf 'Zml4dHVyZS1kYXRh' >"${output}"
    ;;
  https://fixture.invalid/corrupt)
    printf 'wrong-payload' >"${output}"
    ;;
  https://fixture.invalid/oversize)
    printf 'fixture-data-with-unbounded-suffix' >"${output}"
    ;;
  https://fixture.invalid/timeout)
    exit 28
    ;;
  https://fixture.invalid/fail)
    exit 22
    ;;
  *)
    exit 64
    ;;
esac
FAKE_CURL
chmod 0700 "${fake_curl}"

export REVAER_FIXTURE_CURL_BIN="${fake_curl}"
export REVAER_FAKE_CURL_LOG="${temporary_root}/curl.log"
export REVAER_FIXTURE_CONNECT_TIMEOUT_SECONDS=1
export REVAER_FIXTURE_DEADLINE_SECONDS=1

printf 'fixture-data' >"${temporary_root}/expected"
expected_sha256="$(fixture_sha256 "${temporary_root}/expected")"
expected_bytes="$(fixture_size "${temporary_root}/expected")"

printf 'corrupted-cache' >"${temporary_root}/cached"
fixture_acquire \
  cache-corruption \
  "${temporary_root}/cached" \
  raw \
  "${expected_sha256}" \
  "${expected_bytes}" \
  "${expected_bytes}" \
  "${temporary_root}" \
  https://fixture.invalid/good
fixture_validate cache-corruption "${temporary_root}/cached" "${expected_sha256}" "${expected_bytes}" "${expected_bytes}"

if fixture_acquire \
  oversize \
  "${temporary_root}/oversize" \
  raw \
  "${expected_sha256}" \
  1 \
  "${expected_bytes}" \
  "${temporary_root}" \
  https://fixture.invalid/oversize; then
  printf 'test-download-integrity: oversized fixture unexpectedly succeeded\n' >&2
  exit 1
fi
[[ ! -e "${temporary_root}/oversize" ]]

if fixture_acquire \
  mismatch \
  "${temporary_root}/mismatch" \
  raw \
  "${expected_sha256}" \
  1 \
  64 \
  "${temporary_root}" \
  https://fixture.invalid/corrupt; then
  printf 'test-download-integrity: hash mismatch unexpectedly succeeded\n' >&2
  exit 1
fi
[[ ! -e "${temporary_root}/mismatch" ]]

if fixture_acquire \
  timeout \
  "${temporary_root}/timeout" \
  raw \
  "${expected_sha256}" \
  1 \
  64 \
  "${temporary_root}" \
  https://fixture.invalid/timeout; then
  printf 'test-download-integrity: timed-out fixture unexpectedly succeeded\n' >&2
  exit 1
fi
[[ ! -e "${temporary_root}/timeout" ]]
grep -F -- '--connect-timeout 1 --max-time 1' "${REVAER_FAKE_CURL_LOG}" >/dev/null
grep -F -- '--retry-max-time 1' "${REVAER_FAKE_CURL_LOG}" >/dev/null

fixture_acquire \
  mirror-fallback \
  "${temporary_root}/fallback" \
  raw \
  "${expected_sha256}" \
  "${expected_bytes}" \
  "${expected_bytes}" \
  "${temporary_root}" \
  https://fixture.invalid/fail \
  https://fixture.invalid/good
fixture_validate mirror-fallback "${temporary_root}/fallback" "${expected_sha256}" "${expected_bytes}" "${expected_bytes}"

fixture_acquire \
  base64-decoding \
  "${temporary_root}/base64" \
  base64 \
  "${expected_sha256}" \
  "${expected_bytes}" \
  "${expected_bytes}" \
  "${temporary_root}" \
  https://fixture.invalid/base64-good
fixture_validate base64-decoding "${temporary_root}/base64" "${expected_sha256}" "${expected_bytes}" "${expected_bytes}"

printf 'test-download-integrity: corruption, oversize, mismatch, timeout, mirror fallback, and base64 decoding passed\n'
