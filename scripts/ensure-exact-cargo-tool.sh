#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -lt 3 ]]; then
  echo "usage: ensure-exact-cargo-tool.sh <binary> <crate> <version> [cargo install args...]" >&2
  exit 64
fi

binary="$1"
crate="$2"
required_version="$3"
shift 3

installed_version=""
read_installed_version() {
  local version_output

  if ! command -v "${binary}" >/dev/null 2>&1; then
    return 0
  fi
  if [[ "${binary}" == cargo-* ]]; then
    if ! version_output="$(cargo "${binary#cargo-}" --version 2>/dev/null)"; then
      return 0
    fi
  else
    if ! version_output="$("${binary}" --version 2>/dev/null)"; then
      return 0
    fi
  fi
  installed_version="$(printf '%s\n' "${version_output}" | awk 'NR == 1 { print $2 }')"
}

read_installed_version
if [[ "${installed_version}" == "${required_version}" ]]; then
  printf '%s %s is installed\n' "${binary}" "${required_version}"
  exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -n "${installed_version}" ]]; then
  printf 'replacing %s %s with required version %s\n' \
    "${binary}" "${installed_version}" "${required_version}"
else
  printf 'installing %s %s\n' "${binary}" "${required_version}"
fi
bash "${script_dir}/cargo-install-retry.sh" \
  "${crate}" --locked --force --version "${required_version}" "$@"

hash -r
installed_version=""
read_installed_version
if [[ "${installed_version}" != "${required_version}" ]]; then
  echo "${binary} version check failed: expected ${required_version}, found ${installed_version:-missing}" >&2
  exit 1
fi
