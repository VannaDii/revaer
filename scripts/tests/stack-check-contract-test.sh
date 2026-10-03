#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${repo_root}"

temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-stack-check-contract.XXXXXX")"
cleanup() {
  rm -rf "${temporary_root}"
}
trap cleanup EXIT HUP INT TERM

assert_rejected() {
  local fixture="$1"
  local label="$2"
  if REVAER_PR_WORKFLOW_PATH="${fixture}" ruby --disable-gems scripts/stack-check-contract.rb >/dev/null 2>&1; then
    printf 'stack-check-contract-test: mutation was accepted: %s\n' "${label}" >&2
    exit 1
  fi
}

ruby --disable-gems scripts/stack-check-contract.rb >/dev/null

fixture="${temporary_root}/supply-if.yml"
cp .github/workflows/pr.yml "${fixture}"
ruby --disable-gems -pi -e 'sub(/^    if: always\(\)$/, "    if: success()")' "${fixture}"
assert_rejected "${fixture}" "supply aggregate condition"

fixture="${temporary_root}/supply-needs.yml"
cp .github/workflows/pr.yml "${fixture}"
ruby --disable-gems -pi -e 'sub(/needs: \[audit, deny, udeps\]/, "needs: [audit, deny]")' "${fixture}"
assert_rejected "${fixture}" "supply aggregate dependencies"

fixture="${temporary_root}/media-cleanup.yml"
cp .github/workflows/pr.yml "${fixture}"
ruby --disable-gems -e '
  path = ARGV.fetch(0)
  source = File.read(path)
  expected = "      - name: Clean media fixtures\n        if: always()"
  replacement = "      - name: Clean media fixtures\n        if: success()"
  abort "cleanup fixture did not match" unless source.sub!(expected, replacement)
  File.write(path, source)
' "${fixture}"
assert_rejected "${fixture}" "media cleanup condition"

fixture="${temporary_root}/release-condition.yml"
cp .github/workflows/pr.yml "${fixture}"
ruby --disable-gems -e '
  path = ARGV.fetch(0)
  source = File.read(path)
  expected = "  build-release:\n    name: Build Release"
  replacement = "  build-release:\n    name: Build Release\n    if: github.ref == \"refs/heads/main\""
  abort "release fixture did not match" unless source.sub!(expected, replacement)
  File.write(path, source)
' "${fixture}"
assert_rejected "${fixture}" "release pull-request condition"

printf 'stack-check-contract-test: structural mutations fail closed\n'
