#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 1 ]]; then
  printf 'usage: %s <database-name>\n' "$0" >&2
  exit 1
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
uv_arguments=(run --locked --directory "${repo_root}")
if [[ -f "${repo_root}/.env" ]]; then
  uv_arguments+=(--env-file "${repo_root}/.env")
fi

# Return the URI only to callers that explicitly request it; ordinary rv logs redact it.
exec uv "${uv_arguments[@]}" -- python -c '
import os
import sys
from revaer_tooling.database.connection import local_database_url, require_database_url
from revaer_tooling.errors import ToolingError

try:
    sys.stdout.write(require_database_url(local_database_url(os.environ, sys.argv[1])))
except ToolingError as error:
    print(str(error), file=sys.stderr)
    sys.exit(error.exit_code)
' "$1"
