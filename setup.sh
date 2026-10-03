#!/usr/bin/env bash
# Bootstrap Python with uv; project provisioning belongs to `rv setup`.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")"

if ! command -v uv >/dev/null 2>&1; then
    # UV_INSTALL_DIR is an official installer option. Keep its location known
    # so this invocation can continue without requiring a new login shell.
    rv_install_dir="${UV_INSTALL_DIR:-${UV_UNMANAGED_INSTALL:-${HOME}/.local/bin}}"
    curl --fail --silent --show-error --location \
        "https://astral.sh/uv/$(cat .uv-version)/install.sh" \
        | env UV_INSTALL_DIR="${rv_install_dir}" sh
    export PATH="${rv_install_dir}:${PATH}"
fi

# Each worktree owns its project environment. An activated neighbouring
# checkout must not redirect setup; uv still owns creation and synchronization.
unset VIRTUAL_ENV UV_PROJECT_ENVIRONMENT UV_PROJECT UV_WORKING_DIR PYTHONHOME PYTHONPATH
uv sync --locked
exec uv run --locked -- rv setup "$@"
