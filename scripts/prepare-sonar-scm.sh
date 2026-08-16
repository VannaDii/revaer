#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
project_base_dir="${SONAR_PROJECT_BASE_DIR:-${repo_root}}"
cd "${project_base_dir}"

base_sha="${SONAR_BASE_SHA:?SONAR_BASE_SHA is required}"
base_ref="${SONAR_BASE_REF:?SONAR_BASE_REF is required}"
head_sha="${SONAR_HEAD_SHA:?SONAR_HEAD_SHA is required}"
evidence_path="${SONAR_SCM_EVIDENCE_PATH:-artifacts/sonar/scm-evidence.txt}"

validate_sha() {
  local label="$1"
  local value="$2"

  if ! [[ "${value}" =~ ^[0-9a-f]{40}$ ]]; then
    printf '%s must be an exact lowercase 40-character Git SHA: %s\n' \
      "${label}" "${value}" >&2
    exit 1
  fi
}

validate_sha SONAR_BASE_SHA "${base_sha}"
validate_sha SONAR_HEAD_SHA "${head_sha}"
if ! [[ "${base_ref}" =~ ^[A-Za-z0-9._/-]+$ ]] || \
  [[ "${base_ref}" == /* || "${base_ref}" == */ || "${base_ref}" == *..* ]]; then
  printf 'SONAR_BASE_REF is not a valid branch name: %s\n' "${base_ref}" >&2
  exit 1
fi

current_head="$(git rev-parse HEAD^{commit})"
if [[ "${current_head}" != "${head_sha}" ]]; then
  printf 'Checked-out HEAD %s does not match SONAR_HEAD_SHA %s\n' \
    "${current_head}" "${head_sha}" >&2
  exit 1
fi

git remote get-url origin >/dev/null
if [[ "$(git rev-parse --is-shallow-repository)" == "true" ]]; then
  git fetch --no-tags --prune --unshallow origin
else
  git fetch --no-tags --prune origin
fi
git fetch --no-tags origin \
  "+refs/heads/${base_ref}:refs/remotes/origin/${base_ref}"
git fetch --no-tags origin "${base_sha}"

git cat-file -e "${base_sha}^{commit}"
git cat-file -e "${head_sha}^{commit}"
if [[ "$(git rev-parse --is-shallow-repository)" != "false" ]]; then
  echo "Sonar checkout remains shallow after history preparation" >&2
  exit 1
fi

merge_base="$(git merge-base "${base_sha}" "${head_sha}")"
if [[ -z "${merge_base}" ]]; then
  echo "Sonar base and head do not share reachable Git history" >&2
  exit 1
fi
if [[ "${merge_base}" != "${base_sha}" ]]; then
  printf 'Exact Sonar base %s is not an ancestor of head %s; restack the branch onto its event base\n' \
    "${base_sha}" "${head_sha}" >&2
  exit 1
fi

mkdir -p "$(dirname "${evidence_path}")"
{
  printf 'base_ref=%s\n' "${base_ref}"
  printf 'base_sha=%s\n' "${base_sha}"
  printf 'head_sha=%s\n' "${head_sha}"
  printf 'merge_base=%s\n' "${merge_base}"
  printf 'shallow=false\n'
  printf 'reachable_commit_count=%s\n' "$(git rev-list --count --all)"
} > "${evidence_path}"
test -s "${evidence_path}"
