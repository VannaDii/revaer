#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -eq 0 ]]; then
  echo "with-node requires a command to execute" >&2
  exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
required_node_version="$(tr -d '[:space:]' < "${repo_root}/.nvmrc")"
if [[ ! "${required_node_version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Repository .nvmrc must contain one exact semantic version" >&2
  exit 3
fi
requested_node_version="${REVAER_NODE_VERSION:-${required_node_version}}"
if [[ "${requested_node_version}" != "${required_node_version}" ]]; then
  printf 'REVAER_NODE_VERSION must equal %s\n' "${required_node_version}" >&2
  exit 3
fi

nvm_script="${NVM_DIR:-${HOME}/.nvm}/nvm.sh"
if [[ -s "${nvm_script}" ]]; then
  # shellcheck source=/dev/null
  . "${nvm_script}"
  if ! nvm use --silent "${required_node_version}" >/dev/null; then
    printf 'NVM is available, but Node %s is not installed.\n' "${required_node_version}" >&2
    exit 3
  fi
fi

if ! command -v node >/dev/null 2>&1; then
  echo "Node is required but was not found" >&2
  exit 3
fi
active_node_version="$(node --version 2>/dev/null || true)"
active_node_version="${active_node_version#v}"
if [[ "${active_node_version}" != "${required_node_version}" ]]; then
  printf 'Node %s is required, found %s\n' \
    "${required_node_version}" "${active_node_version:-unknown}" >&2
  exit 3
fi

exec "$@"
