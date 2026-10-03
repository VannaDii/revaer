#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/revaer-advisory-guardrails.XXXXXX")"
cleanup() {
  rm -rf "${tmpdir}"
}
trap cleanup EXIT HUP INT TERM

secignore="${tmpdir}/secignore"
deny="${tmpdir}/deny.toml"

: >"${secignore}"
printf '[advisories]\nignore = []\n' >"${deny}"
REVAER_SECIGNORE_PATH="${secignore}" REVAER_DENY_PATH="${deny}" \
  bash scripts/advisory-exception-guardrails.sh >/dev/null

printf 'RUSTSEC-2099-0001\n' >"${secignore}"
if REVAER_SECIGNORE_PATH="${secignore}" REVAER_DENY_PATH="${deny}" \
  bash scripts/advisory-exception-guardrails.sh >/dev/null 2>&1; then
  printf 'advisory-exception-guardrails-test: nonempty secignore passed unexpectedly\n' >&2
  exit 1
fi

: >"${secignore}"
printf '[advisories]\nignore = ["RUSTSEC-2099-0001"]\n' >"${deny}"
if REVAER_SECIGNORE_PATH="${secignore}" REVAER_DENY_PATH="${deny}" \
  bash scripts/advisory-exception-guardrails.sh >/dev/null 2>&1; then
  printf 'advisory-exception-guardrails-test: nonempty deny ignore passed unexpectedly\n' >&2
  exit 1
fi

printf '[advisories]\n' >"${deny}"
if REVAER_SECIGNORE_PATH="${secignore}" REVAER_DENY_PATH="${deny}" \
  bash scripts/advisory-exception-guardrails.sh >/dev/null 2>&1; then
  printf 'advisory-exception-guardrails-test: missing deny ignore passed unexpectedly\n' >&2
  exit 1
fi

printf 'advisory-exception-guardrails-test: advisory exceptions fail closed\n'
