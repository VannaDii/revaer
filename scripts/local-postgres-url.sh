#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 1 ]]; then
  printf 'usage: %s <database-name>\n' "$0" >&2
  exit 1
fi

database_name="$1"
database_user="${REVAER_LOCAL_DB_USER:-revaer}"
database_password="${REVAER_LOCAL_DB_PASSWORD:-${database_user}}"
database_host="${REVAER_LOCAL_DB_HOST:-localhost}"
database_port="${REVAER_LOCAL_DB_PORT:-5432}"

urlencode() {
  local value="$1" encoded="" character
  local index
  LC_ALL=C
  for ((index = 0; index < ${#value}; index += 1)); do
    character="${value:index:1}"
    case "${character}" in
      [a-zA-Z0-9.~_-]) encoded+="${character}" ;;
      *) printf -v encoded '%s%%%02X' "${encoded}" "'${character}" ;;
    esac
  done
  printf '%s' "${encoded}"
}

printf 'postgres://%s:%s@%s:%s/%s' \
  "$(urlencode "${database_user}")" \
  "$(urlencode "${database_password}")" \
  "${database_host}" \
  "${database_port}" \
  "${database_name}"
