#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-sonar-scm.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT

git init --bare "${test_root}/origin.git" >/dev/null
git init -b main "${test_root}/seed" >/dev/null
git -C "${test_root}/seed" config user.name "Sonar Fixture"
git -C "${test_root}/seed" config user.email "sonar-fixture@example.invalid"
printf 'base\n' > "${test_root}/seed/evidence.txt"
git -C "${test_root}/seed" add evidence.txt
git -C "${test_root}/seed" commit -m "test: add base" >/dev/null
base_sha="$(git -C "${test_root}/seed" rev-parse HEAD)"
git -C "${test_root}/seed" remote add origin "${test_root}/origin.git"
git -C "${test_root}/seed" push -u origin main >/dev/null
git -C "${test_root}/seed" switch -c feature >/dev/null
printf 'head\n' >> "${test_root}/seed/evidence.txt"
git -C "${test_root}/seed" commit -am "test: add head" >/dev/null
head_sha="$(git -C "${test_root}/seed" rev-parse HEAD)"
git -C "${test_root}/seed" push -u origin feature >/dev/null
git -C "${test_root}/seed" switch main >/dev/null
printf 'stale base\n' >> "${test_root}/seed/evidence.txt"
git -C "${test_root}/seed" commit -am "test: advance base" >/dev/null
stale_base_sha="$(git -C "${test_root}/seed" rev-parse HEAD)"
git -C "${test_root}/seed" push origin main >/dev/null

git clone --quiet --branch feature --depth 1 \
  "file://${test_root}/origin.git" "${test_root}/work"
SONAR_BASE_SHA="${base_sha}" \
SONAR_BASE_REF=main \
SONAR_HEAD_SHA="${head_sha}" \
SONAR_PROJECT_BASE_DIR="${test_root}/work" \
SONAR_SCM_EVIDENCE_PATH="${test_root}/scm-evidence.txt" \
  bash "${repo_root}/scripts/prepare-sonar-scm.sh"

grep -q "base_sha=${base_sha}" "${test_root}/scm-evidence.txt"
grep -q "head_sha=${head_sha}" "${test_root}/scm-evidence.txt"
grep -q 'shallow=false' "${test_root}/scm-evidence.txt"

if SONAR_BASE_SHA="${base_sha}" \
  SONAR_BASE_REF=main \
  SONAR_HEAD_SHA="${base_sha}" \
  SONAR_PROJECT_BASE_DIR="${test_root}/work" \
  bash "${repo_root}/scripts/prepare-sonar-scm.sh" >/dev/null 2>&1; then
  echo "Sonar SCM preparation accepted a mismatched head SHA" >&2
  exit 1
fi

if SONAR_BASE_SHA="${stale_base_sha}" \
  SONAR_BASE_REF=main \
  SONAR_HEAD_SHA="${head_sha}" \
  SONAR_PROJECT_BASE_DIR="${test_root}/work" \
  bash "${repo_root}/scripts/prepare-sonar-scm.sh" \
    >"${test_root}/divergent.stdout" 2>"${test_root}/divergent.stderr"; then
  echo "Sonar SCM preparation accepted a head missing the exact event base" >&2
  exit 1
fi
grep -q 'restack the branch' "${test_root}/divergent.stderr"

printf '%s\n' "Sonar SCM preparation tests passed"
