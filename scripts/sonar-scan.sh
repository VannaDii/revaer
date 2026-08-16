#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
project_base_dir="${SONAR_PROJECT_BASE_DIR:-${repo_root}}"
cd "${project_base_dir}"

scanner_command="${SONAR_SCANNER_COMMAND:-sonar-scanner}"
scanner_log="${SONAR_SCANNER_LOG:-artifacts/sonar/scanner.log}"
normalized_log="$(mktemp "${TMPDIR:-/tmp}/revaer-sonar-log.XXXXXX")"
trap 'rm -f "${normalized_log}"' EXIT
mkdir -p "$(dirname "${scanner_log}")"
: > "${scanner_log}"

if [[ -z "${SONAR_TOKEN:-}" ]]; then
  echo "SONAR_TOKEN is required for the authoritative scanner invocation" >&2
  exit 1
fi

if ! command -v "${scanner_command}" >/dev/null 2>&1; then
  printf 'Sonar scanner is unavailable: %s\n' "${scanner_command}" >&2
  exit 1
fi

set +e
"${scanner_command}" "$@" 2>&1 | tee "${scanner_log}"
scanner_status="${PIPESTATUS[0]}"
set -e

if [[ "${scanner_status}" -ne 0 ]]; then
  printf 'Sonar scanner failed with status %s\n' "${scanner_status}" >&2
  exit "${scanner_status}"
fi

if [[ ! -s "${scanner_log}" ]]; then
  echo "Sonar scanner produced no retained output" >&2
  exit 1
fi

LC_ALL=C sed -E $'s/\033\\[[0-?]*[ -\\/]*[@-~]//g' "${scanner_log}" > "${normalized_log}"
if grep -nF 'WARN' "${normalized_log}" >&2; then
  echo "Sonar scanner emitted WARN output" >&2
  exit 1
fi

test -s .scannerwork/report-task.txt
test -d .scannerwork/scanner-report
task_id_count="$(grep -c '^ceTaskId=' .scannerwork/report-task.txt || true)"
if [[ "${task_id_count}" -ne 1 ]]; then
  echo "Sonar report-task.txt must contain exactly one ceTaskId" >&2
  exit 1
fi
task_id="$(sed -n 's/^ceTaskId=//p' .scannerwork/report-task.txt)"
if ! [[ "${task_id}" =~ ^[A-Za-z0-9_-]+$ ]]; then
  echo "Sonar report-task.txt contains an invalid ceTaskId" >&2
  exit 1
fi
printf 'Authoritative Sonar task: %s\n' "${task_id}"
