#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-script-coverage.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT
mkdir -p "${test_root}/kcov/report" "${test_root}/ruby"

cat > "${test_root}/kcov/report/sonarqube.xml" <<XML
<coverage version="1">
  <file path="${repo_root}/scripts/workflow-guardrails.sh">
    <lineToCover lineNumber="2" covered="true"/>
    <lineToCover lineNumber="3" covered="false"/>
  </file>
</coverage>
XML

cat > "${test_root}/ruby/ruby-fixture.json" <<JSON
{"files":[{"path":"scripts/workflow_guardrails/input_loader.rb","lines":[null,1,0],"branches":[{"line":2,"key":"fixture","hits":1}]}]}
JSON

(
  cd "${repo_root}"
  ruby scripts/generate-generic-coverage.rb \
    "${test_root}/kcov" "${test_root}/ruby" "${test_root}/coverage.xml"
)
grep -q 'scripts/workflow-guardrails.sh' "${test_root}/coverage.xml"
grep -q 'scripts/workflow_guardrails/input_loader.rb' "${test_root}/coverage.xml"
grep -Eq 'covered=.true.' "${test_root}/coverage.xml"
grep -Eq 'covered=.false.' "${test_root}/coverage.xml"
grep -Eq 'branchesToCover=.1.' "${test_root}/coverage.xml"

printf '%s\n' "Script coverage conversion tests passed"
