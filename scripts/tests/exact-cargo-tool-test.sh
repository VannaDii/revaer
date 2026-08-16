#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
helper="${repo_root}/scripts/ensure-exact-cargo-tool.sh"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-exact-cargo-tool.XXXXXX")"
trap 'rm -rf "${temporary_root}"' EXIT

write_fake_tool() {
  local binary_path="$1"

  cat > "${binary_path}" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
printf 'direct-tool %s\n' "$(<"${FAKE_VERSION_FILE}")"
SCRIPT
  chmod +x "${binary_path}"
}

run_case() {
  local case_name="$1"
  local initial_version="$2"
  local expected_installs="$3"
  local case_root="${temporary_root}/${case_name}"
  local binary_dir="${case_root}/bin"
  local version_file="${case_root}/version"
  local install_log="${case_root}/install.log"

  mkdir -p "${binary_dir}"
  : > "${install_log}"
  if [[ "${initial_version}" != "missing" ]]; then
    printf '%s\n' "${initial_version}" > "${version_file}"
    write_fake_tool "${binary_dir}/cargo-fake"
  fi

  cat > "${binary_dir}/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "fake" && "${2:-}" == "--version" ]]; then
  printf 'cargo-fake %s\n' "$(<"${FAKE_VERSION_FILE}")"
  exit 0
fi
printf '%s\n' "$*" >> "${FAKE_INSTALL_LOG}"
printf '%s\n' "${FAKE_REQUIRED_VERSION}" > "${FAKE_VERSION_FILE}"
cat > "${FAKE_BIN_DIR}/cargo-fake" <<'TOOL'
#!/usr/bin/env bash
set -euo pipefail
printf 'cargo-fake %s\n' "$(<"${FAKE_VERSION_FILE}")"
TOOL
chmod +x "${FAKE_BIN_DIR}/cargo-fake"
SCRIPT
  chmod +x "${binary_dir}/cargo"

  PATH="${binary_dir}:${PATH}" \
  FAKE_BIN_DIR="${binary_dir}" \
  FAKE_INSTALL_LOG="${install_log}" \
  FAKE_REQUIRED_VERSION="2.0.0" \
  FAKE_VERSION_FILE="${version_file}" \
  REVAER_CARGO_INSTALL_ATTEMPTS=1 \
    bash "${helper}" cargo-fake fake-crate 2.0.0 --features fixture

  actual_installs="$(wc -l < "${install_log}" | tr -d ' ')"
  if [[ "${actual_installs}" != "${expected_installs}" ]]; then
    echo "${case_name}: expected ${expected_installs} installs, found ${actual_installs}" >&2
    exit 1
  fi
  if [[ "${expected_installs}" == "1" ]] &&
    ! grep -Fxq 'install fake-crate --locked --force --version 2.0.0 --features fixture' \
      "${install_log}"; then
    echo "${case_name}: install arguments were not exact and locked" >&2
    exit 1
  fi
}

run_case missing missing 1
run_case older 1.9.9 1
run_case exact 2.0.0 0
run_case newer 2.0.1 1

direct_root="${temporary_root}/direct"
mkdir -p "${direct_root}/bin"
printf '2.0.0\n' > "${direct_root}/version"
write_fake_tool "${direct_root}/bin/direct-tool"
PATH="${direct_root}/bin:${PATH}" \
FAKE_VERSION_FILE="${direct_root}/version" \
  bash "${helper}" direct-tool direct-crate 2.0.0

quality="${repo_root}/just/quality.just"
database="${repo_root}/just/database.just"
ui="${repo_root}/just/ui.just"
grep -Fq 'required_udeps_version="0.1.57"' "${quality}"
grep -Fq 'required_udeps_toolchain="nightly-2026-06-13"' "${quality}"
grep -Fq 'cargo +"${udeps_toolchain}" udeps --workspace --all-targets' "${quality}"
grep -Fq 'required_audit_version="0.22.0"' "${quality}"
grep -Fq 'required_deny_version="0.18.9"' "${quality}"
grep -Fq 'cargo-llvm-cov cargo-llvm-cov 0.8.7' "${quality}"
grep -Fq 'required_sqlx_version="0.8.6"' "${database}"
grep -Fq 'required_trunk_version="0.21.14"' "${ui}"
grep -Fq 'set shell := ["bash", "-c"]' "${repo_root}/justfile"
grep -Fq 'REVAER_UDEPS_VERSION: "0.1.57"' "${repo_root}/.github/workflows/pr.yml"
grep -Fq 'REVAER_UDEPS_TOOLCHAIN: "nightly-2026-06-13"' "${repo_root}/.github/workflows/pr.yml"

warning_root="${temporary_root}/warning"
mkdir -p "${warning_root}/bin"
cat > "${warning_root}/bin/cargo" <<'SCRIPT'
#!/usr/bin/env bash
printf 'warning: package fixture 1.0.0 in Cargo.lock is yanked\n' >&2
exit 0
SCRIPT
chmod +x "${warning_root}/bin/cargo"
if PATH="${warning_root}/bin:${PATH}" REVAER_CARGO_INSTALL_ATTEMPTS=1 \
  bash "${repo_root}/scripts/cargo-install-retry.sh" fixture >/dev/null 2>&1; then
  echo "exact Cargo installer accepted warning output" >&2
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
