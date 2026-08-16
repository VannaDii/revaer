#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd -P)"
supported_bash="${BASH}"

resolved="$(REVAER_KCOV_BASH="${supported_bash}" \
  bash "${repo_root}/scripts/resolve-kcov-bash.sh")"
if [[ ! -x "${resolved}" ]]; then
  printf 'Kcov Bash resolver returned a non-executable path: %s\n' "${resolved}" >&2
  exit 1
fi

if REVAER_KCOV_BASH=/bin/sh \
  bash "${repo_root}/scripts/resolve-kcov-bash.sh" >/dev/null 2>&1; then
  printf '%s\n' 'Kcov Bash resolver accepted a non-Bash override' >&2
  exit 1
fi
if REVAER_KCOV_BASH=/missing/revaer-kcov-bash \
  bash "${repo_root}/scripts/resolve-kcov-bash.sh" >/dev/null 2>&1; then
  printf '%s\n' 'Kcov Bash resolver accepted a missing override' >&2
  exit 1
fi

temp_directory="$(mktemp -d)"
cleanup() {
  rm -rf "${temp_directory}"
}
trap cleanup EXIT

cat >"${temp_directory}/bash" <<'FAKE_BASH'
#!/bin/sh
printf '%s' '3 2'
FAKE_BASH
chmod +x "${temp_directory}/bash"

fallback="$(PATH="${temp_directory}:/usr/bin:/bin" \
  "${supported_bash}" "${repo_root}/scripts/resolve-kcov-bash.sh")"
if [[ "${fallback}" == "${temp_directory}/bash" ]] || [[ ! -x "${fallback}" ]]; then
  printf 'Kcov Bash resolver did not reject the unsupported PATH candidate: %s\n' \
    "${fallback}" >&2
  exit 1
fi

printf '%s\n' 'Kcov Bash resolver tests passed'
