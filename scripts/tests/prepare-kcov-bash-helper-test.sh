#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
temp_dir="$(mktemp -d)"

cleanup() {
  rm -rf "${temp_dir}"
}
trap cleanup EXIT

helper_path="${temp_dir}/bash-helper.sh"
cat >"${helper_path}" <<'HELPER'
#!/bin/sh
PS4='kcov@${BASH_SOURCE}@${LINENO}@'
set -x
HELPER

ruby "${repo_root}/scripts/prepare-kcov-bash-helper.rb" "${helper_path}"
ruby "${repo_root}/scripts/prepare-kcov-bash-helper.rb" "${helper_path}"

if ! grep -Fqx 'PS4='"'"'kcov@${BASH_SOURCE:-kcov-inline}@${LINENO}@'"'"'' "${helper_path}"; then
  echo "kcov Bash helper did not receive the nounset-safe trace prompt" >&2
  exit 1
fi
if grep -Fqx 'PS4='"'"'kcov@${BASH_SOURCE}@${LINENO}@'"'"'' "${helper_path}"; then
  echo "kcov Bash helper retained the unsafe trace prompt" >&2
  exit 1
fi

printf '%s\n' "PS4='unexpected'" >"${helper_path}"
if ruby "${repo_root}/scripts/prepare-kcov-bash-helper.rb" "${helper_path}" >/dev/null 2>&1; then
  echo "kcov Bash helper preparation accepted an unknown prompt" >&2
  exit 1
fi

printf '%s\n' "Kcov Bash helper preparation tests passed"
