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

printf 'postgres://%s:%s@%s:%s/%s' \
  "${database_user}" \
  "${database_password}" \
  "${database_host}" \
  "${database_port}" \
  "${database_name}"
