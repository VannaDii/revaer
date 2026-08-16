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

if [[ -n "${CARGO_HOME:-}" ]]; then
  cargo_home="${CARGO_HOME}"
elif [[ -n "${HOME:-}" ]]; then
  cargo_home="${HOME}/.cargo"
else
  echo "CARGO_HOME or HOME is required to locate Cargo-installed tools" >&2
  exit 64
fi

for install_arg in "$@"; do
  case "${install_arg}" in
    --root|--root=*)
      echo "ensure-exact-cargo-tool.sh owns the Cargo install root" >&2
      exit 64
      ;;
  esac
done

cargo_bin_dir="${cargo_home}/bin"
tool_path="${cargo_bin_dir}/${binary}"
installed_version=""
read_installed_version() {
  local version_output

  if [[ "${binary}" == cargo-* ]]; then
    if ! version_output="$(
      "${tool_path}" "${binary#cargo-}" --version 2>/dev/null
    )"; then
      return 0
    fi
  elif ! version_output="$("${tool_path}" --version 2>/dev/null)"; then
    return 0
  fi
  installed_version="$(
    printf '%s\n' "${version_output}" | \
      awk 'NR == 1 { version = $2; sub(/^v/, "", version); print version }'
  )"
}

read_installed_version
if [[ "${installed_version}" == "${required_version}" ]]; then
  printf '%s %s is installed at %s\n' \
    "${binary}" "${required_version}" "${tool_path}"
  exit 0
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -n "${installed_version}" ]]; then
  printf 'replacing %s %s at %s with required version %s\n' \
    "${binary}" "${installed_version}" "${tool_path}" "${required_version}"
else
  printf 'installing %s %s at %s\n' \
    "${binary}" "${required_version}" "${tool_path}"
fi
if bash "${script_dir}/cargo-install-retry.sh" \
  "${crate}" --locked --force --version "${required_version}" \
  --root "${cargo_home}" "$@"; then
  install_status="0"
else
  install_status="$?"
fi

hash -r
installed_version=""
read_installed_version
if [[ "${install_status}" -ne 0 ]]; then
  printf 'exact Cargo installer exited with status %s for %s; observed %s at %s\n' \
    "${install_status}" "${binary}" "${installed_version:-missing}" \
    "${tool_path}" >&2
  exit "${install_status}"
fi
if [[ "${installed_version}" != "${required_version}" ]]; then
  printf '%s version check failed at %s: expected %s, found %s\n' \
    "${binary}" "${tool_path}" "${required_version}" \
    "${installed_version:-missing}" >&2
  exit 1
fi
printf '%s %s is installed at %s\n' \
  "${binary}" "${required_version}" "${tool_path}"
