#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-sonar-inputs.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT

make_fixture() {
  local root="$1"
  mkdir -p \
    "${root}/cxxbridge/include/rust" \
    "${root}/cxxbridge/include/revaer-torrent-libt/src/ffi"
  printf 'SF:crates/example/src/lib.rs\nDA:1,1\nend_of_record\n' > "${root}/lcov.info"
  {
    printf 'SF:tests/example.ts\n'
    for line in $(seq 1 1000); do
      if [[ "${line}" -eq 1000 ]]; then
        printf 'DA:%s,0\n' "${line}"
      else
        printf 'DA:%s,1\n' "${line}"
      fi
    done
    printf 'end_of_record\n'
  } > "${root}/js-lcov.info"
  printf '<coverage version="1"><file path="scripts/a.sh"><lineToCover lineNumber="1" covered="true"/></file><file path="scripts/a.rb"><lineToCover lineNumber="1" covered="false"/></file></coverage>\n' \
    > "${root}/script-coverage.xml"
  printf '[{"directory":"/repo","file":"/repo/crates/revaer-torrent-libt/src/ffi/session.cpp","command":"clang++ -I /repo/coverage/cxxbridge/include"}]\n' \
    > "${root}/compile_commands.json"
  printf '/repo/crates/revaer-torrent-libt/src/ffi/session.cpp:\n    1|      1|int covered();\n' \
    > "${root}/llvm-cov.txt"
  printf 'header\n' > "${root}/cxxbridge/include/rust/cxx.h"
  printf 'bridge\n' > "${root}/cxxbridge/include/revaer-torrent-libt/src/ffi/bridge.rs.h"
}

make_fixture "${test_root}/valid"
REVAER_REQUIRE_NATIVE_COVERAGE=1 SONAR_COVERAGE_ROOT="${test_root}/valid" \
  bash "${repo_root}/scripts/verify-sonar-inputs.sh" >/dev/null

cp -R "${test_root}/valid" "${test_root}/missing-native"
printf 'TOTAL 10 0 100.00%%\n' > "${test_root}/missing-native/llvm-cov.txt"
if REVAER_REQUIRE_NATIVE_COVERAGE=1 SONAR_COVERAGE_ROOT="${test_root}/missing-native" \
  bash "${repo_root}/scripts/verify-sonar-inputs.sh" >/dev/null 2>&1; then
  echo "Sonar input verification accepted a native report without session.cpp" >&2
  exit 1
fi

REVAER_REQUIRE_NATIVE_COVERAGE=0 SONAR_COVERAGE_ROOT="${test_root}/missing-native" \
  bash "${repo_root}/scripts/verify-sonar-inputs.sh" >/dev/null

cp -R "${test_root}/valid" "${test_root}/native-uncovered"
printf '/repo/crates/revaer-torrent-libt/src/ffi/session.cpp:\n    1|       |int uncovered();\n' \
  > "${test_root}/native-uncovered/llvm-cov.txt"
if REVAER_REQUIRE_NATIVE_COVERAGE=0 SONAR_COVERAGE_ROOT="${test_root}/native-uncovered" \
  bash "${repo_root}/scripts/verify-sonar-inputs.sh" >/dev/null 2>&1; then
  echo "Sonar input verification accepted session.cpp with no covered native lines" >&2
  exit 1
fi

cp -R "${test_root}/valid" "${test_root}/zero-rust"
printf 'SF:crates/example/src/lib.rs\nDA:1,0\nend_of_record\n' > "${test_root}/zero-rust/lcov.info"
if SONAR_COVERAGE_ROOT="${test_root}/zero-rust" \
  bash "${repo_root}/scripts/verify-sonar-inputs.sh" >/dev/null 2>&1; then
  echo "Sonar input verification accepted Rust coverage with no hit lines" >&2
  exit 1
fi

printf '%s\n' "Sonar input regression tests passed"
