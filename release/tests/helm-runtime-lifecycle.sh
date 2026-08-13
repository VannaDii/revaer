#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
chart="${repo_root}/charts/revaer"
database_url="postgres://example.invalid/revaer"

rendered="$(helm template revaer "${chart}" --set "database.url=${database_url}")"

require_rendered() {
  local expected="$1"
  if ! grep -Fq -- "${expected}" <<<"${rendered}"; then
    printf 'rendered chart is missing required lifecycle value: %s\n' "${expected}" >&2
    exit 1
  fi
}

require_rendered 'terminationGracePeriodSeconds: 45'
require_rendered 'name: REVAER_MEDIA_WORKSPACE_ROOT'
require_rendered 'value: "/data/media-workspaces"'
require_rendered 'http://127.0.0.1:7070/health/live'
require_rendered 'http://127.0.0.1:7070/health/ready'

if ! grep -Fq 'ENV REVAER_MEDIA_WORKSPACE_ROOT=/data/media-workspaces' "${repo_root}/Dockerfile"; then
  echo 'production image is missing the managed media workspace default' >&2
  exit 1
fi
if ! grep -Fq 'http://127.0.0.1:7070/health/live' "${repo_root}/Dockerfile"; then
  echo 'production image health check is not process-liveness-only' >&2
  exit 1
fi

if helm lint "${chart}" --set "database.url=${database_url}" \
  --set terminationGracePeriodSeconds=30 >/dev/null 2>&1; then
  echo 'chart accepted a termination grace shorter than worker shutdown' >&2
  exit 1
fi

if helm lint "${chart}" --set "database.url=${database_url}" \
  --set mediaWorkspace.path=relative/path >/dev/null 2>&1; then
  echo 'chart accepted a relative media workspace path' >&2
  exit 1
fi

echo 'Helm media runtime lifecycle contract passed'
