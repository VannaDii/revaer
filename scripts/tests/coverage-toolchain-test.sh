#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-coverage-toolchain.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT

make_tool() {
  local path="$1"
  local version="$2"
  cat > "${path}" <<EOF
#!/usr/bin/env bash
set -euo pipefail
if [[ "\${1:-}" = "-vV" ]]; then
  printf 'host: x86_64-unknown-linux-gnu\nLLVM version: ${version}\n'
elif [[ "\${1:-}" = "--print" && "\${2:-}" = "sysroot" ]]; then
  printf '%s\n' '${test_root}/sysroot'
else
  printf 'LLVM version ${version}\n'
fi
EOF
  chmod +x "${path}"
}

mkdir -p "${test_root}/bin" "${test_root}/sysroot/lib/rustlib/x86_64-unknown-linux-gnu/bin"
make_tool "${test_root}/bin/rustc" 21.1.2
make_tool "${test_root}/bin/clang" 19.1.7
make_tool "${test_root}/bin/clang++" 19.1.7
make_tool "${test_root}/bin/llvm-cov" 21.1.2
make_tool "${test_root}/bin/llvm-profdata" 21.1.2

env_output="$(
  RUSTC_COMMAND="${test_root}/bin/rustc" \
  CC="${test_root}/bin/clang" \
  CXX="${test_root}/bin/clang++" \
  LLVM_COV="${test_root}/bin/llvm-cov" \
  LLVM_PROFDATA="${test_root}/bin/llvm-profdata" \
    bash "${repo_root}/scripts/coverage-toolchain-env.sh"
)"
grep -q '^CC=' <<<"${env_output}"
grep -q '^REVAER_RUST_LLVM_VERSION=21.1.2$' <<<"${env_output}"

if ! RUSTC_COMMAND="${test_root}/bin/rustc" \
  CC="${test_root}/bin/clang" \
  CXX="${test_root}/bin/clang++" \
  LLVM_COV="${test_root}/bin/llvm-cov" \
  LLVM_PROFDATA="${test_root}/bin/llvm-profdata" \
    bash "${repo_root}/scripts/coverage-toolchain-env.sh" >/dev/null; then
  echo "Coverage toolchain rejected behaviorally compatible Clang 19 with Rust LLVM 21" >&2
  exit 1
fi

if RUSTC_COMMAND="${test_root}/bin/rustc" \
  CC="${test_root}/bin/missing-clang" \
  CXX="${test_root}/bin/clang++" \
  LLVM_COV="${test_root}/bin/llvm-cov" \
  LLVM_PROFDATA="${test_root}/bin/llvm-profdata" \
    bash "${repo_root}/scripts/coverage-toolchain-env.sh" >/dev/null 2>&1; then
  echo "Coverage toolchain accepted a missing compiler" >&2
  exit 1
fi

if RUSTC_COMMAND="${test_root}/bin/rustc" \
  CC="${test_root}/bin/clang" \
  CXX="${test_root}/bin/clang++" \
  LLVM_COV="${test_root}/bin/missing-llvm-cov" \
  LLVM_PROFDATA="${test_root}/bin/llvm-profdata" \
    bash "${repo_root}/scripts/coverage-toolchain-env.sh" >/dev/null 2>&1; then
  echo "Coverage toolchain accepted a missing llvm-cov" >&2
  exit 1
fi

printf '%s\n' "Coverage toolchain compatibility tests passed"
