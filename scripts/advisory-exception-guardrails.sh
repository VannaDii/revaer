#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

secignore_path="${REVAER_SECIGNORE_PATH:-.secignore}"
deny_path="${REVAER_DENY_PATH:-deny.toml}"

if [[ ! -f "${secignore_path}" ]]; then
  printf 'Advisory guardrail failed: missing %s\n' "${secignore_path}" >&2
  exit 1
fi
if [[ -s "${secignore_path}" ]]; then
  printf 'Advisory guardrail failed: %s must remain empty\n' "${secignore_path}" >&2
  exit 1
fi
if [[ ! -f "${deny_path}" ]]; then
  printf 'Advisory guardrail failed: missing %s\n' "${deny_path}" >&2
  exit 1
fi

advisory_ignore="$(
  awk '
    /^\[[^]]+\][[:space:]]*$/ {
      in_advisories = ($0 == "[advisories]")
      next
    }
    in_advisories && /^[[:space:]]*ignore[[:space:]]*=/ {
      value = $0
      sub(/^[^=]*=[[:space:]]*/, "", value)
      gsub(/[[:space:]]/, "", value)
      print value
    }
  ' "${deny_path}"
)"
if [[ "${advisory_ignore}" != "[]" ]]; then
  printf 'Advisory guardrail failed: %s [advisories].ignore must be exactly []\n' "${deny_path}" >&2
  exit 1
fi

printf 'advisory-exception-guardrails: advisory ignore lists are empty\n'
