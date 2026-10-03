#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -eq 0 ]]; then
  echo "usage: cargo-install-retry.sh <crate> [cargo install args...]" >&2
  exit 64
fi

max_attempts="${REVAER_CARGO_INSTALL_ATTEMPTS:-3}"
case "${max_attempts}" in
  ''|*[!0-9]*)
    echo "REVAER_CARGO_INSTALL_ATTEMPTS must be a positive integer" >&2
    exit 64
    ;;
  *)
    ;;
esac
if [[ "${max_attempts}" -lt 1 ]]; then
  echo "REVAER_CARGO_INSTALL_ATTEMPTS must be at least 1" >&2
  exit 64
fi

attempt=1
while true; do
  if [[ "${attempt}" -eq 1 ]]; then
    cargo install "$@" && exit 0
  else
    CARGO_HTTP_MULTIPLEXING=false CARGO_NET_RETRY=10 cargo install "$@" && exit 0
  fi

  status="$?"
  if [[ "${attempt}" -ge "${max_attempts}" ]]; then
    exit "${status}"
  fi

  sleep_seconds=$((attempt * 5))
  echo "cargo install failed with status ${status}; retrying in ${sleep_seconds}s" >&2
  sleep "${sleep_seconds}"
  attempt=$((attempt + 1))
done
