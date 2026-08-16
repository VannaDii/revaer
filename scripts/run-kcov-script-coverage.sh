#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "${repo_root}"

if [[ "${1:-}" != --resolved-bash=* ]]; then
  kcov_bash="$(bash "${repo_root}/scripts/resolve-kcov-bash.sh")"
  kcov_command="$(command -v kcov 2>/dev/null || true)"
  if [[ -z "${kcov_command}" ]]; then
    printf '%s\n' 'kcov is required for authored shell coverage' >&2
    exit 1
  fi
  kcov_directory="$(cd "$(dirname "${kcov_command}")" && pwd -P)"
  kcov_command="${kcov_directory}/$(basename "${kcov_command}")"
  kcov_bash_directory="$(dirname "${kcov_bash}")"
  PATH="${kcov_bash_directory}:${PATH}" \
    exec "${kcov_bash}" "$0" \
      "--resolved-bash=${kcov_bash}" "--resolved-kcov=${kcov_command}" "$@"
fi

kcov_bash="${1#--resolved-bash=}"
shift
if [[ "${1:-}" != --resolved-kcov=* ]]; then
  printf '%s\n' 'Resolved kcov command is required after interpreter selection' >&2
  exit 1
fi
kcov_command="${1#--resolved-kcov=}"
shift
output_directory="${1:?kcov output directory is required}"
log_path="${2:?kcov log path is required}"
if [[ "${output_directory}" != /* ]]; then
  output_directory="${repo_root}/${output_directory}"
fi
if [[ "${log_path}" != /* ]]; then
  log_path="${repo_root}/${log_path}"
fi

validated_bash="$(REVAER_KCOV_BASH="${kcov_bash}" \
  bash "${repo_root}/scripts/resolve-kcov-bash.sh")"
if [[ "${validated_bash}" != "${kcov_bash}" ]]; then
  printf 'Resolved kcov Bash changed during coverage startup: %s != %s\n' \
    "${validated_bash}" "${kcov_bash}" >&2
  exit 1
fi

if [[ ! -x "${kcov_command}" ]]; then
  printf 'Resolved kcov command is not executable: %s\n' "${kcov_command}" >&2
  exit 1
fi
kcov_version="$("${kcov_command}" --version 2>&1)"
if [[ "${kcov_version}" != 'kcov 43' ]]; then
  printf 'Expected kcov 43, found %s\n' "${kcov_version}" >&2
  exit 1
fi

open_file_limit="$(ulimit -n)"
if [[ "${open_file_limit}" == unlimited ]] || \
  [[ "${open_file_limit}" =~ ^[0-9]+$ && "${open_file_limit}" -gt 4096 ]]; then
  ulimit -n 4096
  open_file_limit=4096
fi
if [[ ! "${open_file_limit}" =~ ^[0-9]+$ ]] || \
  [[ "${open_file_limit}" -lt 16 ]]; then
  printf 'Kcov received an invalid open-file limit: %s\n' "${open_file_limit}" >&2
  exit 1
fi

mkdir -p "$(dirname "${log_path}")"
printf 'Kcov Bash: %s (%s); open-file limit: %s\n' \
  "${kcov_bash}" "${BASH_VERSION}" "${open_file_limit}" | tee "${log_path}"

set +e
REVAER_KCOV_BASH_HELPER="${output_directory}/bash-helper.sh" \
REVAER_RUBY_COVERAGE_DIR="${repo_root}/coverage/scripts/ruby" \
RUBYOPT="-r${repo_root}/scripts/ruby-coverage-bootstrap.rb" \
  "${kcov_command}" \
    --bash-parser="${kcov_bash}" \
    --include-path="${repo_root}/scripts" \
    "${output_directory}" "${repo_root}/scripts/tests/policy-suite.sh" \
    2>&1 | tee -a "${log_path}"
pipeline_status=("${PIPESTATUS[@]}")
set -e

if [[ "${pipeline_status[1]}" -ne 0 ]]; then
  printf 'Cannot retain complete kcov log at %s\n' "${log_path}" >&2
  exit "${pipeline_status[1]}"
fi
if grep -Eiq \
  '^[[:space:]]*kcov: (error|warning)(:|$)|^Failed to (exchange stderr|get the maximum number of open file descriptors|execute script)' \
  "${log_path}"; then
  printf '%s\n' 'Kcov emitted an error or warning; refusing incomplete script coverage.' >&2
  exit 1
fi
if [[ "${pipeline_status[0]}" -ne 0 ]]; then
  printf 'Kcov failed with exit status %s.\n' "${pipeline_status[0]}" >&2
  exit "${pipeline_status[0]}"
fi
