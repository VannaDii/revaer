#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
temp_directory="$(mktemp -d)"
cleanup() {
  rm -rf "${temp_directory}"
}
trap cleanup EXIT

fake_bin="${temp_directory}/bin"
mkdir -p "${fake_bin}"
cat >"${fake_bin}/kcov" <<'FAKE_KCOV'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" == '--version' ]]; then
  printf '%s\n' "${REVAER_TEST_KCOV_VERSION:-kcov 43}"
  exit 0
fi
printf '%s\n' "${REVAER_TEST_KCOV_OUTPUT:-coverage complete}"
exit "${REVAER_TEST_KCOV_STATUS:-0}"
FAKE_KCOV
chmod +x "${fake_bin}/kcov"

run_coverage() {
  case_name="$1"
  shift
  PATH="${fake_bin}:${PATH}" \
  REVAER_KCOV_BASH="${BASH}" \
    "$@" bash "${repo_root}/scripts/run-kcov-script-coverage.sh" \
      "${temp_directory}/${case_name}/out" "${temp_directory}/${case_name}/kcov.log"
}

run_coverage success env >/dev/null
if [[ ! -s "${temp_directory}/success/kcov.log" ]]; then
  printf '%s\n' 'Kcov coverage runner did not retain its complete log' >&2
  exit 1
fi

if run_coverage diagnostic env \
  REVAER_TEST_KCOV_OUTPUT='kcov: error: parser rejected trace input' \
  >/dev/null 2>&1; then
  printf '%s\n' 'Kcov coverage runner accepted an error with exit status zero' >&2
  exit 1
fi
if run_coverage warning env \
  REVAER_TEST_KCOV_OUTPUT='kcov: warning: incomplete trace input' \
  >/dev/null 2>&1; then
  printf '%s\n' 'Kcov coverage runner accepted a warning with exit status zero' >&2
  exit 1
fi
if run_coverage descriptor env \
  REVAER_TEST_KCOV_OUTPUT='Failed to exchange stderr for pipe: Bad file descriptor' \
  >/dev/null 2>&1; then
  printf '%s\n' 'Kcov coverage runner accepted a trace-descriptor failure' >&2
  exit 1
fi
if run_coverage nonzero env REVAER_TEST_KCOV_STATUS=17 >/dev/null 2>&1; then
  printf '%s\n' 'Kcov coverage runner accepted a nonzero process status' >&2
  exit 1
fi
if run_coverage version env REVAER_TEST_KCOV_VERSION='kcov 42' >/dev/null 2>&1; then
  printf '%s\n' 'Kcov coverage runner accepted version drift' >&2
  exit 1
fi

printf '%s\n' 'Kcov coverage runner tests passed'
