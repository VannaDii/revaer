#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -eq 0 ]]; then
  echo "usage: cargo-install-retry.sh <crate> [cargo install args...]" >&2
  exit 64
fi

max_attempts="${REVAER_CARGO_INSTALL_ATTEMPTS:-3}"
if ! [[ "${max_attempts}" =~ ^[1-9][0-9]*$ ]]; then
  echo "REVAER_CARGO_INSTALL_ATTEMPTS must be a positive integer" >&2
  exit 64
fi

attempt=1
while true; do
  install_log="$(mktemp "${TMPDIR:-/tmp}/revaer-cargo-install.XXXXXX")"
  set +e
  if [[ "${attempt}" -eq 1 ]]; then
    cargo install "$@" 2>&1 | tee "${install_log}"
  else
    CARGO_HTTP_MULTIPLEXING=false CARGO_NET_RETRY=10 \
      cargo install "$@" 2>&1 | tee "${install_log}"
  fi
  status="${PIPESTATUS[0]}"
  set -e
  if [[ "${status}" -eq 0 ]] && grep -nEi '(^|[[:space:]])warning:' "${install_log}" >&2; then
    echo "cargo install completed with warning output" >&2
    status=65
  fi
  rm -f "${install_log}"
  if [[ "${status}" -eq 0 ]]; then
    exit 0
  fi
  if [[ "${attempt}" -ge "${max_attempts}" ]]; then
    exit "${status}"
  fi

  sleep_seconds="$((attempt * 5))"
  printf 'cargo install failed with status %s; retrying in %ss\n' \
    "${status}" "${sleep_seconds}" >&2
  sleep "${sleep_seconds}"
  attempt="$((attempt + 1))"
done
