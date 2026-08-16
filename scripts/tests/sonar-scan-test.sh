#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-sonar-scan.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT

run_case() {
  local mode="$1"
  local expected="$2"
  local case_root="${test_root}/${mode}"
  mkdir -p "${case_root}/bin" "${case_root}/work/.scannerwork/scanner-report"
  cat > "${case_root}/bin/sonar-scanner" <<'MOCK'
#!/usr/bin/env bash
set -euo pipefail

case "${MOCK_SCANNER_MODE}" in
  clean)
    echo "INFO Analysis completed"
    ;;
  warn)
    echo "WARN Incomplete analysis input"
    ;;
  ansi-warn)
    printf '\033[33mWA\033[0mRN\033[0m Colored analysis warning\n'
    ;;
  embedded-warn)
    echo "INFO analyzer-WARN-property-registry"
    ;;
  fail)
    echo "ERROR Scanner failed"
    exit 17
    ;;
  empty)
    ;;
  duplicate-task|invalid-task)
    ;;
  *)
    echo "unknown mock mode" >&2
    exit 2
    ;;
esac
mkdir -p .scannerwork/scanner-report
case "${MOCK_SCANNER_MODE}" in
  duplicate-task)
    printf 'INFO Analysis completed\n'
    printf 'ceTaskId=fixture-one\nceTaskId=fixture-two\n' > .scannerwork/report-task.txt
    ;;
  invalid-task)
    printf 'INFO Analysis completed\n'
    printf 'ceTaskId=../../fixture\n' > .scannerwork/report-task.txt
    ;;
  *)
    printf 'ceTaskId=fixture\n' > .scannerwork/report-task.txt
    ;;
esac
MOCK
  chmod +x "${case_root}/bin/sonar-scanner"

  if (
    cd "${case_root}/work"
    PATH="${case_root}/bin:${PATH}" \
    MOCK_SCANNER_MODE="${mode}" \
    SONAR_TOKEN=fixture-token \
    SONAR_PROJECT_BASE_DIR="${case_root}/work" \
    SONAR_SCANNER_LOG="${case_root}/scanner.log" \
      bash "${repo_root}/scripts/sonar-scan.sh"
  ) >"${case_root}/stdout" 2>"${case_root}/stderr"; then
    if [[ "${expected}" != pass ]]; then
      printf 'Expected Sonar scan failure for %s\n' "${mode}" >&2
      exit 1
    fi
  elif [[ "${expected}" = pass ]]; then
    printf 'Expected Sonar scan success for %s\n' "${mode}" >&2
    cat "${case_root}/stderr" >&2
    exit 1
  fi

  if [[ "${mode}" != empty ]]; then
    test -s "${case_root}/scanner.log"
  fi
}

run_case clean pass
run_case warn fail
run_case ansi-warn fail
run_case embedded-warn fail
run_case duplicate-task fail
run_case invalid-task fail
run_case fail fail
run_case empty fail

printf '%s\n' "Sonar scanner output regression tests passed"
