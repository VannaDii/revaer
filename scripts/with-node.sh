#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -eq 0 ]]; then
    echo "with-node requires a command to execute" >&2
    exit 2
fi

nvm_script="${NVM_DIR:-${HOME}/.nvm}/nvm.sh"
if [[ -s "${nvm_script}" ]]; then
    # shellcheck source=/dev/null
    . "${nvm_script}"
    if [[ -n "${REVAER_NODE_VERSION:-}" ]]; then
        nvm use --silent "${REVAER_NODE_VERSION}" >/dev/null
    elif ! nvm use --silent lts/* >/dev/null && [[ "${CI:-}" != "true" ]]; then
        echo "NVM is available, but no local lts/* Node version is installed." >&2
        echo "Install an LTS Node with NVM or set REVAER_NODE_VERSION." >&2
        exit 3
    fi
fi

exec "$@"
