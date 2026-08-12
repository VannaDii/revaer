#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
helper="${repo_root}/scripts/local-postgres-url.sh"

password='p@ss/word?#'
actual="$(
  REVAER_LOCAL_DB_USER='local user' \
  REVAER_LOCAL_DB_PASSWORD="${password}" \
  REVAER_LOCAL_DB_HOST='127.0.0.1' \
  REVAER_LOCAL_DB_PORT='55432' \
    bash "${helper}" 'media_test'
)"

if [[ "${actual}" != postgres://local%20user:*@127.0.0.1:55432/media_test ]] ||
  [[ "${actual}" != *%40*%2F*%3F*%23* ]] ||
  [[ "${actual}" == *"${password}"* ]]; then
  echo "local-postgres-url: reserved credential characters were not encoded" >&2
  exit 1
fi

echo "local-postgres-url: credential components encoded"
