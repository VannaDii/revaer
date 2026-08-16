#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
helper="${repo_root}/scripts/ensure-exact-cargo-tool.sh"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-exact-cargo-tool.XXXXXX")"
trap 'rm -rf "${temporary_root}"' EXIT

write_cargo_subcommand_tool() {
  local binary_path="$1"

  cat > "${binary_path}" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "fake" || "${2:-}" != "--version" ]]; then
  printf 'unexpected cargo-fake invocation: %s\n' "$*" >&2
  exit 64
fi
printf 'cargo-fake %s\n' "$(<"${FAKE_VERSION_FILE}")"
SCRIPT
  chmod +x "${binary_path}"
}

write_direct_tool() {
  local binary_path="$1"
  local version_prefix="${2:-}"

  cat > "${binary_path}" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" != "--version" ]]; then
  printf 'unexpected direct-tool invocation: %s\\n' "\$*" >&2
  exit 64
fi
printf 'direct-tool ${version_prefix}%s\\n' "\$(<"\${FAKE_VERSION_FILE}")"
SCRIPT
  chmod +x "${binary_path}"
}

write_shadow_tool() {
  local binary_path="$1"

  cat > "${binary_path}" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
printf 'shadowed-tool 9.9.9\n'
SCRIPT
  chmod +x "${binary_path}"
}

run_case() {
  local case_name="$1"
  local initial_version="$2"
  local installed_version="$3"
  local expected_installs="$4"
  local expected_status="$5"
  local case_root="${temporary_root}/${case_name}"
  local cargo_home="${case_root}/cargo-home"
  local cargo_bin_dir="${cargo_home}/bin"
  local shadow_dir="${case_root}/shadow"
  local version_file="${case_root}/version"
  local install_log="${case_root}/install.log"
  local output_log="${case_root}/output.log"
  local tool_path="${cargo_bin_dir}/cargo-fake"
  local actual_installs
  local actual_status

  mkdir -p "${cargo_bin_dir}" "${shadow_dir}"
  : > "${install_log}"
  write_shadow_tool "${shadow_dir}/cargo-fake"
  if [[ "${initial_version}" != "missing" ]]; then
    printf '%s\n' "${initial_version}" > "${version_file}"
    write_cargo_subcommand_tool "${tool_path}"
  fi

  cat > "${shadow_dir}/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "install" ]]; then
  printf 'unexpected Cargo invocation: %s\n' "$*" >&2
  exit 64
fi
printf '%s\n' "$*" >> "${FAKE_INSTALL_LOG}"
printf '%s\n' "${FAKE_INSTALL_VERSION}" > "${FAKE_VERSION_FILE}"
cat > "${FAKE_CARGO_BIN_DIR}/cargo-fake" <<'TOOL'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "fake" || "${2:-}" != "--version" ]]; then
  printf 'unexpected cargo-fake invocation: %s\n' "$*" >&2
  exit 64
fi
printf 'cargo-fake %s\n' "$(<"${FAKE_VERSION_FILE}")"
TOOL
chmod +x "${FAKE_CARGO_BIN_DIR}/cargo-fake"
SCRIPT
  chmod +x "${shadow_dir}/cargo"

  if [[ "$(PATH="${shadow_dir}:${PATH}" command -v cargo-fake)" != \
    "${shadow_dir}/cargo-fake" ]]; then
    echo "${case_name}: test PATH does not resolve the shadow binary first" >&2
    exit 1
  fi

  if PATH="${shadow_dir}:${PATH}" \
    CARGO_HOME="${cargo_home}" \
    CARGO_INSTALL_ROOT="${case_root}/ambient-install-root" \
    FAKE_CARGO_BIN_DIR="${cargo_bin_dir}" \
    FAKE_INSTALL_LOG="${install_log}" \
    FAKE_INSTALL_VERSION="${installed_version}" \
    FAKE_VERSION_FILE="${version_file}" \
    REVAER_CARGO_INSTALL_ATTEMPTS=1 \
      bash "${helper}" cargo-fake fake-crate 2.0.0 \
        --features fixture > "${output_log}" 2>&1; then
    actual_status=0
  else
    actual_status="$?"
  fi

  if [[ "${actual_status}" != "${expected_status}" ]]; then
    echo "${case_name}: expected status ${expected_status}, found ${actual_status}" >&2
    cat "${output_log}" >&2
    exit 1
  fi

  actual_installs="$(wc -l < "${install_log}" | tr -d ' ')"
  if [[ "${actual_installs}" != "${expected_installs}" ]]; then
    echo "${case_name}: expected ${expected_installs} installs, found ${actual_installs}" >&2
    exit 1
  fi
  if [[ "${expected_installs}" == "1" ]] &&
    ! grep -Fxq "install fake-crate --locked --force --version 2.0.0 --root ${cargo_home} --features fixture" \
      "${install_log}"; then
    echo "${case_name}: install arguments were not exact and locked" >&2
    exit 1
  fi
  if [[ "${expected_status}" == "0" ]]; then
    if ! grep -Fxq "cargo-fake 2.0.0 is installed at ${tool_path}" "${output_log}"; then
      echo "${case_name}: success did not report the exact executable" >&2
      exit 1
    fi
  elif ! grep -Fxq \
    "cargo-fake version check failed at ${tool_path}: expected 2.0.0, found ${installed_version}" \
    "${output_log}"; then
    echo "${case_name}: post-install verification did not fail at the exact executable" >&2
    exit 1
  fi
}

run_case missing missing 2.0.0 1 0
run_case older 1.9.9 2.0.0 1 0
run_case exact 2.0.0 2.0.0 0 0
run_case newer 2.0.1 2.0.0 1 0
run_case post-install-mismatch missing 2.0.1 1 1

direct_root="${temporary_root}/direct"
direct_home="${direct_root}/home"
direct_cargo_bin="${direct_home}/.cargo/bin"
direct_shadow_dir="${direct_root}/shadow"
direct_install_log="${direct_root}/install.log"
direct_output_log="${direct_root}/output.log"
mkdir -p "${direct_cargo_bin}" "${direct_shadow_dir}"
: > "${direct_install_log}"
printf '2.0.0\n' > "${direct_root}/version"
write_direct_tool "${direct_cargo_bin}/direct-tool"
write_shadow_tool "${direct_shadow_dir}/direct-tool"
cat > "${direct_shadow_dir}/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${DIRECT_INSTALL_LOG}"
exit 97
SCRIPT
chmod +x "${direct_shadow_dir}/cargo"
env -u CARGO_HOME \
  HOME="${direct_home}" \
  PATH="${direct_shadow_dir}:${PATH}" \
  DIRECT_INSTALL_LOG="${direct_install_log}" \
  FAKE_VERSION_FILE="${direct_root}/version" \
  bash "${helper}" direct-tool direct-crate 2.0.0 > "${direct_output_log}"
grep -Fxq \
  "direct-tool 2.0.0 is installed at ${direct_cargo_bin}/direct-tool" \
  "${direct_output_log}"
if [[ -s "${direct_install_log}" ]]; then
  echo "direct exact match unexpectedly invoked Cargo install" >&2
  exit 1
fi

write_direct_tool "${direct_cargo_bin}/direct-tool" v
env -u CARGO_HOME \
  HOME="${direct_home}" \
  PATH="${direct_shadow_dir}:${PATH}" \
  DIRECT_INSTALL_LOG="${direct_install_log}" \
  FAKE_VERSION_FILE="${direct_root}/version" \
  bash "${helper}" direct-tool direct-crate 2.0.0 > "${direct_output_log}"
grep -Fxq \
  "direct-tool 2.0.0 is installed at ${direct_cargo_bin}/direct-tool" \
  "${direct_output_log}"

configuration_output="${temporary_root}/configuration-output.log"
if CARGO_HOME="${direct_home}/.cargo" \
  bash "${helper}" direct-tool direct-crate 2.0.0 \
    --root "${temporary_root}/other-root" > "${configuration_output}" 2>&1; then
  configuration_status=0
else
  configuration_status="$?"
fi
if [[ "${configuration_status}" != "64" ]] ||
  ! grep -Fxq \
    "ensure-exact-cargo-tool.sh owns the Cargo install root" \
    "${configuration_output}"; then
  echo "caller-provided Cargo install root was not rejected" >&2
  exit 1
fi

if env -u CARGO_HOME -u HOME \
  bash "${helper}" direct-tool direct-crate 2.0.0 \
    > "${configuration_output}" 2>&1; then
  configuration_status=0
else
  configuration_status="$?"
fi
if [[ "${configuration_status}" != "64" ]] ||
  ! grep -Fxq \
    "CARGO_HOME or HOME is required to locate Cargo-installed tools" \
    "${configuration_output}"; then
  echo "missing Cargo-home configuration was not rejected" >&2
  exit 1
fi

quality="${repo_root}/just/quality.just"
database="${repo_root}/just/database.just"
ui="${repo_root}/just/ui.just"
docs="${repo_root}/just/docs.just"
grep -Fq 'required_udeps_version="0.1.57"' "${quality}"
grep -Fq 'required_udeps_toolchain="nightly-2026-06-13"' "${quality}"
grep -Fq 'cargo +"${udeps_toolchain}" udeps --workspace --all-targets' "${quality}"
grep -Fq 'required_audit_version="0.22.0"' "${quality}"
grep -Fq 'required_deny_version="0.18.9"' "${quality}"
grep -Fq 'cargo-llvm-cov cargo-llvm-cov 0.8.7' "${quality}"
grep -Fq 'required_sqlx_version="0.8.6"' "${database}"
grep -Fq 'required_trunk_version="0.21.14"' "${ui}"
grep -Fq 'ensure-exact-cargo-tool.sh mdbook mdbook 0.5.0' "${docs}"
grep -Fq 'ensure-exact-cargo-tool.sh mdbook-mermaid mdbook-mermaid 0.17.0' "${docs}"
grep -Fq 'ensure-exact-cargo-tool.sh lychee lychee 0.24.2' "${docs}"
if grep -Fq 'lychee --verbose --no-progress docs || true' "${docs}"; then
  echo "docs-link-check silently suppresses Lychee failures" >&2
  exit 1
fi
grep -Fq 'set shell := ["bash", "-c"]' "${repo_root}/justfile"
grep -Fq 'REVAER_UDEPS_VERSION: "0.1.57"' "${repo_root}/.github/workflows/pr.yml"
grep -Fq 'REVAER_UDEPS_TOOLCHAIN: "nightly-2026-06-13"' "${repo_root}/.github/workflows/pr.yml"

warning_root="${temporary_root}/warning"
warning_cargo_home="${warning_root}/cargo-home"
warning_cargo_bin="${warning_cargo_home}/bin"
warning_path="${warning_root}/path"
warning_install_log="${warning_root}/install.log"
warning_output_log="${warning_root}/output.log"
warning_version_file="${warning_root}/version"
mkdir -p "${warning_cargo_bin}" "${warning_path}"
: > "${warning_install_log}"
cat > "${warning_path}/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${WARNING_INSTALL_LOG}"
printf '2.0.0\n' > "${WARNING_VERSION_FILE}"
cat > "${WARNING_CARGO_BIN}/cargo-fake" <<'TOOL'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${1:-}" != "fake" || "${2:-}" != "--version" ]]; then
  printf 'unexpected cargo-fake invocation: %s\n' "$*" >&2
  exit 64
fi
printf 'cargo-fake %s\n' "$(<"${FAKE_VERSION_FILE}")"
TOOL
chmod +x "${WARNING_CARGO_BIN}/cargo-fake"
printf 'warning: rustup could not update the shell PATH\n' >&2
exit 0
SCRIPT
chmod +x "${warning_path}/cargo"
if PATH="${warning_path}:${PATH}" \
  CARGO_HOME="${warning_cargo_home}" \
  FAKE_VERSION_FILE="${warning_version_file}" \
  REVAER_CARGO_INSTALL_ATTEMPTS=3 \
  WARNING_CARGO_BIN="${warning_cargo_bin}" \
  WARNING_INSTALL_LOG="${warning_install_log}" \
  WARNING_VERSION_FILE="${warning_version_file}" \
    bash "${helper}" cargo-fake fake-crate 2.0.0 \
      > "${warning_output_log}" 2>&1; then
  warning_status=0
else
  warning_status="$?"
fi
if [[ "${warning_status}" != "65" ]]; then
  echo "warning-marked install did not preserve status 65" >&2
  exit 1
fi
if [[ "$(wc -l < "${warning_install_log}" | tr -d ' ')" != "1" ]]; then
  echo "warning-marked successful install was retried" >&2
  exit 1
fi
if ! grep -Fxq \
  "exact Cargo installer exited with status 65 for cargo-fake; observed 2.0.0 at ${warning_cargo_bin}/cargo-fake" \
  "${warning_output_log}"; then
  echo "warning-marked install did not report its exact observed executable" >&2
  exit 1
fi
PATH="${warning_path}:${PATH}" \
CARGO_HOME="${warning_cargo_home}" \
FAKE_VERSION_FILE="${warning_version_file}" \
  bash "${helper}" cargo-fake fake-crate 2.0.0 \
    > "${warning_output_log}" 2>&1
if [[ "$(wc -l < "${warning_install_log}" | tr -d ' ')" != "1" ]] ||
  ! grep -Fxq \
    "cargo-fake 2.0.0 is installed at ${warning_cargo_bin}/cargo-fake" \
    "${warning_output_log}"; then
  echo "exact tool installed by warning-marked Cargo run was reinstalled" >&2
  exit 1
fi

failure_root="${temporary_root}/installer-failure"
failure_cargo_home="${failure_root}/cargo-home"
failure_path="${failure_root}/path"
failure_install_log="${failure_root}/install.log"
failure_output_log="${failure_root}/output.log"
mkdir -p "${failure_cargo_home}/bin" "${failure_path}"
: > "${failure_install_log}"
cat > "${failure_path}/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${FAILURE_INSTALL_LOG}"
exit 42
SCRIPT
chmod +x "${failure_path}/cargo"
cat > "${failure_path}/sleep" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
exit 0
SCRIPT
chmod +x "${failure_path}/sleep"
if PATH="${failure_path}:${PATH}" \
  CARGO_HOME="${failure_cargo_home}" \
  FAILURE_INSTALL_LOG="${failure_install_log}" \
  REVAER_CARGO_INSTALL_ATTEMPTS=3 \
    bash "${helper}" direct-tool direct-crate 2.0.0 \
      > "${failure_output_log}" 2>&1; then
  failure_status=0
else
  failure_status="$?"
fi
if [[ "$(wc -l < "${failure_install_log}" | tr -d ' ')" != "3" ]]; then
  echo "genuine Cargo failure did not retain bounded retries" >&2
  exit 1
fi
if [[ "${failure_status}" != "42" ]] ||
  ! grep -Fxq \
    "exact Cargo installer exited with status 42 for direct-tool; observed missing at ${failure_cargo_home}/bin/direct-tool" \
    "${failure_output_log}"; then
  echo "exact-tool helper did not preserve the Cargo installer status" >&2
  exit 1
fi

if (cd "${repo_root}" && REVAER_UDEPS_VERSION=0.1.58 just udeps >/dev/null 2>&1); then
  echo "just udeps accepted a mismatched cargo-udeps version" >&2
  exit 1
fi
if (cd "${repo_root}" && REVAER_UDEPS_TOOLCHAIN=nightly just udeps >/dev/null 2>&1); then
  echo "just udeps accepted a moving nightly toolchain" >&2
  exit 1
fi

printf '%s\n' "Exact Cargo tool version tests passed"
