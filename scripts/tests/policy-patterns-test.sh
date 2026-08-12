#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
guardrail="${repo_root}/scripts/policy-guardrails.sh"
pattern="$(sed -n 's/^readonly sqlx_migration_pattern="\(.*\)"$/\1/p' "${guardrail}")"

if [[ -z "${pattern}" ]]; then
  echo "policy-patterns: SQLx migration pattern is missing" >&2
  exit 1
fi

for forbidden in \
  'sqlx::mig''rate!()' \
  'sqlx mig''rate run' \
  '_sqlx_mig''rations' \
  'runCommand("sqlx", ["mig''rate", "run"])' \
  "runCommand('sqlx', ['mig""rate', 'run'])"; do
  if ! printf '%s\n' "${forbidden}" | grep -Eq "${pattern}"; then
    echo "policy-patterns: failed to reject ${forbidden}" >&2
    exit 1
  fi
done

if printf '%s\n' "migration policy documentation" | grep -Eq "${pattern}"; then
  echo "policy-patterns: descriptive migration text must not be rejected" >&2
  exit 1
fi

echo "policy-patterns: SQLx migration command variants rejected"
