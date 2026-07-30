#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="${repo_root}/scripts/ensure-exact-cargo-tool.sh"
temporary_root="$(mktemp -d)"
trap 'rm -rf "${temporary_root}"' EXIT

write_fake_tool() {
  local binary_path="$1"
  local version_prefix="${2:-}"

  cat >"${binary_path}" <<SCRIPT
#!/usr/bin/env bash
set -euo pipefail
printf 'direct-tool ${version_prefix}%s\n' "\$(<"\${FAKE_VERSION_FILE}")"
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
  : >"${install_log}"
  if [[ "${initial_version}" != "missing" ]]; then
    printf '%s\n' "${initial_version}" >"${version_file}"
    write_fake_tool "${binary_dir}/cargo-fake"
  fi

  cat >"${binary_dir}/cargo" <<'SCRIPT'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$1" == "fake" && "${2:-}" == "--version" ]]; then
  printf 'cargo-fake %s\n' "$(<"${FAKE_VERSION_FILE}")"
  exit 0
fi
printf '%s\n' "$*" >>"${FAKE_INSTALL_LOG}"
printf '%s\n' "${FAKE_REQUIRED_VERSION}" >"${FAKE_VERSION_FILE}"
cat >"${FAKE_BIN_DIR}/cargo-fake" <<'TOOL'
#!/usr/bin/env bash
set -euo pipefail
exit 0
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

  actual_installs="$(wc -l <"${install_log}" | tr -d ' ')"
  if [[ "${actual_installs}" != "${expected_installs}" ]]; then
    echo "${case_name}: expected ${expected_installs} installs, found ${actual_installs}" >&2
    exit 1
  fi
  if [[ "${expected_installs}" == "1" ]] && \
    ! grep -Fxq 'install fake-crate --locked --force --version 2.0.0 --features fixture' "${install_log}"; then
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
printf '2.0.0\n' >"${direct_root}/version"
write_fake_tool "${direct_root}/bin/direct-tool"
PATH="${direct_root}/bin:${PATH}" \
  FAKE_VERSION_FILE="${direct_root}/version" \
  bash "${helper}" direct-tool direct-crate 2.0.0

write_fake_tool "${direct_root}/bin/direct-tool" v
PATH="${direct_root}/bin:${PATH}" \
  FAKE_VERSION_FILE="${direct_root}/version" \
  bash "${helper}" direct-tool direct-crate 2.0.0

grep -Fq 'required_udeps_version="0.1.57"' "${repo_root}/justfile"
grep -Fq 'required_udeps_toolchain="nightly-2026-06-13"' "${repo_root}/justfile"
grep -Fq 'cargo +"${udeps_toolchain}" udeps --workspace --all-targets' "${repo_root}/justfile"
grep -Fq 'REVAER_UDEPS_VERSION: "0.1.57"' "${repo_root}/.github/workflows/pr.yml"
grep -Fq 'REVAER_UDEPS_TOOLCHAIN: "nightly-2026-06-13"' "${repo_root}/.github/workflows/pr.yml"
grep -Fq 'set shell := ["bash", "-c"]' "${repo_root}/justfile"
grep -Fq 'required_mdbook_version="0.5.0"' "${repo_root}/justfile"
grep -Fq 'required_mdbook_mermaid_version="0.17.0"' "${repo_root}/justfile"
grep -Fq 'required_lychee_version="0.24.2"' "${repo_root}/justfile"

assert_exact_recipe_tool() {
  local recipe="$1"
  local binary="$2"
  local crate="$3"
  local version_variable="$4"
  local recipe_body

  recipe_body="$(just --unstable --dump --dump-format json | \
    jq -r --arg recipe "${recipe}" \
      '[.recipes[$recipe].body[][]] | join("\n")')"
  if ! grep -Fq \
    "${binary} ${crate} \"\$${version_variable}\"" <<<"${recipe_body}"; then
    echo "${recipe}: ${binary} is not installed through the exact-version helper" >&2
    exit 1
  fi
  if grep -Eq '(^|[[:space:]])cargo[[:space:]]+install([[:space:]]|$)' <<<"${recipe_body}"; then
    echo "${recipe}: direct cargo install bypasses exact version enforcement" >&2
    exit 1
  fi
  if grep -Fq 'cargo-install-retry.sh' <<<"${recipe_body}"; then
    echo "${recipe}: retry installer bypasses exact version verification" >&2
    exit 1
  fi
}

assert_exact_recipe_tool docs-install mdbook mdbook required_mdbook_version
assert_exact_recipe_tool docs-install mdbook-mermaid mdbook-mermaid required_mdbook_mermaid_version
assert_exact_recipe_tool docs-link-check lychee lychee required_lychee_version

if grep -Fq 'lychee --verbose --no-progress docs || true' "${repo_root}/justfile"; then
  echo "docs-link-check silently suppresses Lychee failures" >&2
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

echo "exact Cargo tool version tests passed"
