#!/usr/bin/env bash
set -euo pipefail

rustc_command="${RUSTC_COMMAND:-rustc}"
rustc_verbose="$(${rustc_command} -vV)"
rust_llvm_version="$(sed -n 's/^LLVM version: //p' <<<"${rustc_verbose}")"
rust_host="$(sed -n 's/^host: //p' <<<"${rustc_verbose}")"

if ! [[ "${rust_llvm_version}" =~ ^([0-9]+)\.[0-9]+\.[0-9]+ ]]; then
  echo "rustc did not report a parseable LLVM version" >&2
  exit 1
fi
if [[ -z "${rust_host}" ]]; then
  echo "rustc did not report a host triple" >&2
  exit 1
fi

rust_sysroot="$(${rustc_command} --print sysroot)"
rust_llvm_bin="${rust_sysroot}/lib/rustlib/${rust_host}/bin"

resolve_command() {
  local configured="$1"
  local versioned="$2"
  local fallback="$3"

  if [[ -n "${configured}" ]]; then
    printf '%s\n' "${configured}"
  elif command -v "${versioned}" >/dev/null 2>&1; then
    command -v "${versioned}"
  else
    command -v "${fallback}"
  fi
}

cc="$(resolve_command "${CC:-}" clang-19 clang)"
cxx="$(resolve_command "${CXX:-}" clang++-19 clang++)"
llvm_cov="${LLVM_COV:-${rust_llvm_bin}/llvm-cov}"
llvm_profdata="${LLVM_PROFDATA:-${rust_llvm_bin}/llvm-profdata}"

tool_version() {
  local tool="$1"
  local label="$2"
  local version_output

  if [[ ! -x "${tool}" ]] && ! command -v "${tool}" >/dev/null 2>&1; then
    printf '%s is unavailable: %s\n' "${label}" "${tool}" >&2
    return 1
  fi
  version_output="$(${tool} --version 2>&1)"
  if ! grep -Eq '(clang|LLVM) version [0-9]+' <<<"${version_output}"; then
    printf '%s did not report a parseable LLVM version: %s\n' "${label}" "${tool}" >&2
    return 1
  fi
  printf '%s\n' "$(head -n 1 <<<"${version_output}")"
}

for entry in "CC:${cc}" "CXX:${cxx}" "LLVM_COV:${llvm_cov}" "LLVM_PROFDATA:${llvm_profdata}"; do
  label="${entry%%:*}"
  tool="${entry#*:}"
  if ! version="$(tool_version "${tool}" "${label}")"; then
    exit 1
  fi
  printf '%s=%s\n' "${label}_VERSION" "${version}" >&2
done

printf 'CC=%q\n' "${cc}"
printf 'CXX=%q\n' "${cxx}"
printf 'LLVM_COV=%q\n' "${llvm_cov}"
printf 'LLVM_PROFDATA=%q\n' "${llvm_profdata}"
printf 'REVAER_RUST_LLVM_VERSION=%q\n' "${rust_llvm_version}"
