#!/usr/bin/env bash
set -euo pipefail

candidate_is_supported() {
  candidate="$1"
  [[ -x "${candidate}" ]] || return 1

  version="$("${candidate}" -c 'printf "%s %s" "${BASH_VERSINFO[0]}" "${BASH_VERSINFO[1]}"' 2>/dev/null)" || return 1
  major="${version%% *}"
  minor="${version#* }"
  [[ "${major}" =~ ^[0-9]+$ && "${minor}" =~ ^[0-9]+$ ]] || return 1
  if ((major < 4 || (major == 4 && minor < 2))); then
    return 1
  fi

  "${candidate}" -c \
    'exec 9>/dev/null && BASH_XTRACEFD=9 && [[ "${BASH_XTRACEFD}" == "9" ]]' \
    >/dev/null 2>&1
}

absolute_candidate() {
  candidate="$1"
  candidate_directory="$(cd "$(dirname "${candidate}")" && pwd -P)"
  printf '%s/%s\n' "${candidate_directory}" "$(basename "${candidate}")"
}

if [[ -n "${REVAER_KCOV_BASH:-}" ]]; then
  if candidate_is_supported "${REVAER_KCOV_BASH}"; then
    absolute_candidate "${REVAER_KCOV_BASH}"
    exit 0
  fi
  printf 'REVAER_KCOV_BASH must identify executable Bash 4.2 or newer with working BASH_XTRACEFD support: %s\n' \
    "${REVAER_KCOV_BASH}" >&2
  exit 1
fi

path_bash="$(command -v bash 2>/dev/null || true)"
for candidate in \
  "${path_bash}" \
  /opt/homebrew/bin/bash \
  /usr/local/bin/bash \
  /usr/bin/bash \
  /bin/bash; do
  if candidate_is_supported "${candidate}"; then
    absolute_candidate "${candidate}"
    exit 0
  fi
done

printf '%s\n' \
  'Script coverage requires Bash 4.2 or newer with working BASH_XTRACEFD support.' >&2
exit 1
