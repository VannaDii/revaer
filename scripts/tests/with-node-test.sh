#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-with-node.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT

nvm_dir="${test_root}/nvm"
nvm_bin="${test_root}/nvm-bin"
fallback_bin="${test_root}/fallback-bin"
nvm_log="${test_root}/nvm.log"
mkdir -p "${nvm_dir}" "${nvm_bin}" "${fallback_bin}"

cat > "${nvm_dir}/nvm.sh" <<'NVM'
nvm() {
    printf '%s\n' "$*" >> "${MOCK_NVM_LOG}"
    if [[ "${MOCK_NVM_FAIL:-0}" == "1" ]]; then
        return 1
    fi
    export PATH="${MOCK_NVM_BIN}:${PATH}"
}
NVM

cat > "${nvm_bin}/node" <<'NODE'
#!/usr/bin/env bash
printf 'nvm-node\n'
NODE
chmod +x "${nvm_bin}/node"

cat > "${fallback_bin}/node" <<'NODE'
#!/usr/bin/env bash
printf 'path-node\n'
NODE
chmod +x "${fallback_bin}/node"

explicit_output="$(
    NVM_DIR="${nvm_dir}" \
    MOCK_NVM_BIN="${nvm_bin}" \
    MOCK_NVM_LOG="${nvm_log}" \
    REVAER_NODE_VERSION="24.14.1" \
        bash "${repo_root}/scripts/with-node.sh" node --version
)"
[[ "${explicit_output}" == "nvm-node" ]]
grep -Fxq 'use --silent 24.14.1' "${nvm_log}"

: > "${nvm_log}"
implicit_output="$(
    NVM_DIR="${nvm_dir}" \
    MOCK_NVM_BIN="${nvm_bin}" \
    MOCK_NVM_LOG="${nvm_log}" \
        bash "${repo_root}/scripts/with-node.sh" node --version
)"
[[ "${implicit_output}" == "nvm-node" ]]
grep -Fxq 'use --silent lts/*' "${nvm_log}"

fallback_output="$(
    PATH="${fallback_bin}:${PATH}" \
    NVM_DIR="${nvm_dir}" \
    MOCK_NVM_BIN="${nvm_bin}" \
    MOCK_NVM_LOG="${nvm_log}" \
    MOCK_NVM_FAIL=1 \
    CI=true \
        bash "${repo_root}/scripts/with-node.sh" node --version
)"
[[ "${fallback_output}" == "path-node" ]]

path_output="$(
    PATH="${fallback_bin}:${PATH}" \
    NVM_DIR="${test_root}/missing-nvm" \
        bash "${repo_root}/scripts/with-node.sh" node --version
)"
[[ "${path_output}" == "path-node" ]]

set +e
PATH="${fallback_bin}:${PATH}" \
    NVM_DIR="${nvm_dir}" \
    MOCK_NVM_BIN="${nvm_bin}" \
    MOCK_NVM_LOG="${nvm_log}" \
    MOCK_NVM_FAIL=1 \
    CI=false \
        bash "${repo_root}/scripts/with-node.sh" node --version >/dev/null 2>&1
missing_lts_status="$?"
bash "${repo_root}/scripts/with-node.sh" >/dev/null 2>&1
empty_command_status="$?"
set -e

if [[ "${missing_lts_status}" -ne 3 ]]; then
    echo "with-node returned ${missing_lts_status} for a missing local NVM version" >&2
    exit 1
fi
if [[ "${empty_command_status}" -ne 2 ]]; then
    echo "with-node returned ${empty_command_status} for an empty command" >&2
    exit 1
fi
