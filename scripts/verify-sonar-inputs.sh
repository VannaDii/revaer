#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
coverage_root="${SONAR_COVERAGE_ROOT:-${repo_root}/coverage}"

require_nonempty() {
  local path="$1"
  if [[ ! -s "${path}" ]]; then
    printf 'Required Sonar input is missing or empty: %s\n' "${path}" >&2
    exit 1
  fi
}

rust_lcov="${coverage_root}/lcov.info"
javascript_lcov="${coverage_root}/js-lcov.info"
script_coverage="${coverage_root}/script-coverage.xml"
compile_commands="${coverage_root}/compile_commands.json"
native_report="${coverage_root}/llvm-cov.txt"
cxx_header="${coverage_root}/cxxbridge/include/rust/cxx.h"
bridge_header="${coverage_root}/cxxbridge/include/revaer-torrent-libt/src/ffi/bridge.rs.h"

for path in \
  "${rust_lcov}" \
  "${javascript_lcov}" \
  "${script_coverage}" \
  "${compile_commands}" \
  "${native_report}" \
  "${cxx_header}" \
  "${bridge_header}"; do
  require_nonempty "${path}"
done

grep -q '^SF:.*\.rs$' "${rust_lcov}"
grep -q '^DA:' "${rust_lcov}"
grep -Eq '^DA:[0-9]+,[1-9][0-9]*(,|$)' "${rust_lcov}"
grep -q '^SF:' "${javascript_lcov}"
grep -q '^DA:' "${javascript_lcov}"
if [[ "$(grep -c '^DA:' "${javascript_lcov}")" -lt 1000 ]]; then
  echo "JavaScript/TypeScript coverage has fewer than 1000 authored line records" >&2
  exit 1
fi
grep -Eq '^DA:[0-9]+,[1-9][0-9]*(,|$)' "${javascript_lcov}"
grep -Eq '^DA:[0-9]+,0(,|$)' "${javascript_lcov}"
grep -Eq "<file path=['\"]scripts/.*\.sh['\"]" "${script_coverage}"
grep -Eq "<file path=['\"]scripts/.*\.rb['\"]" "${script_coverage}"
grep -Eq "covered=['\"]true['\"]" "${script_coverage}"
grep -Eq "covered=['\"]false['\"]" "${script_coverage}"
grep -q '"file":' "${compile_commands}"
grep -q 'crates/revaer-torrent-libt/src/ffi/session.cpp' "${compile_commands}"
grep -q 'coverage/cxxbridge/include' "${compile_commands}"

native_required="${REVAER_REQUIRE_NATIVE_COVERAGE:-}"
if [[ -z "${native_required}" ]]; then
  if [[ "${CI:-}" == "true" && "$(uname -s)" == "Linux" ]]; then
    native_required=1
  else
    native_required=0
  fi
fi
if [[ "${native_required}" != "0" && "${native_required}" != "1" ]]; then
  echo "REVAER_REQUIRE_NATIVE_COVERAGE must be 0 or 1" >&2
  exit 1
fi

native_source_present=false
if grep -Eq '(^|/)crates/revaer-torrent-libt/src/ffi/session\.cpp:$' "${native_report}"; then
  native_source_present=true
fi
if [[ "${native_source_present}" == "true" ]]; then
  if ! awk '
    /(^|\/)crates\/revaer-torrent-libt\/src\/ffi\/session\.cpp:$/ { in_source = 1; found = 1; next }
    in_source && /^[^[:space:]].*:$/ { in_source = 0 }
    in_source && /\|[[:space:]]*[1-9][0-9]*\|/ { covered = 1 }
    END { exit !(found && covered) }
  ' "${native_report}"; then
    echo "Native llvm-cov report contains session.cpp but no covered line records" >&2
    exit 1
  fi
elif [[ "${native_required}" == "1" ]]; then
  echo "Native llvm-cov report does not contain authored session.cpp coverage" >&2
  exit 1
fi

printf '%s\n' "Sonar coverage and native analyzer inputs verified"
