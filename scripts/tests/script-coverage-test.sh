#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture_root="$(mktemp -d)"
trap 'rm -rf "${fixture_root}"' EXIT

run_generator() {
  (
    cd "${repo_root}"
    ruby scripts/generate-generic-coverage.rb "$@"
  )
}

expect_failure() {
  local label="$1"
  shift
  if "$@" >"${fixture_root}/${label}.stdout" 2>"${fixture_root}/${label}.stderr"; then
    printf 'Expected coverage generator failure for %s\n' "${label}" >&2
    exit 1
  fi
}

kcov_directory="${fixture_root}/kcov/report"
ruby_directory="${fixture_root}/ruby"
mkdir -p "${kcov_directory}" "${ruby_directory}"

cat > "${kcov_directory}/sonarqube.xml" <<EOF
<coverage version="1">
  <file path="${repo_root}/scripts/policy-guardrails.sh">
    <lineToCover lineNumber="2" covered="true"/>
    <lineToCover lineNumber="3" covered="false"/>
  </file>
</coverage>
EOF

cat > "${ruby_directory}/ruby-fixture.json" <<'EOF'
{"files":[{"path":"scripts/workflow-structure-guardrails.rb","lines":[null,1,0],"branches":[{"line":2,"key":"fixture-covered","hits":1},{"line":2,"key":"fixture-uncovered","hits":0}]}]}
EOF

output_path="${fixture_root}/generic.xml"
(
  cd "${repo_root}"
  REVAER_RUBY_COVERAGE_DIR="${ruby_directory}" \
  RUBYOPT="-r${repo_root}/scripts/ruby-coverage-bootstrap.rb" \
    ruby scripts/generate-generic-coverage.rb \
      "${fixture_root}/kcov" \
      "${ruby_directory}" \
      "${output_path}"
)

ruby_report_count="$(find "${ruby_directory}" -name 'ruby-*.json' -type f | wc -l | tr -d ' ')"
if [[ "${ruby_report_count}" -lt 2 ]]; then
  echo "Ruby coverage bootstrap did not emit an execution report" >&2
  exit 1
fi

(
  cd "${repo_root}"
  ruby scripts/generate-generic-coverage.rb \
    "${fixture_root}/kcov" \
    "${ruby_directory}" \
    "${output_path}"
)

grep -q "path='scripts/policy-guardrails.sh'" "${output_path}"
grep -q "path='scripts/workflow-structure-guardrails.rb'" "${output_path}"
grep -q "path='scripts/generate-generic-coverage.rb'" "${output_path}"
grep -q "branchesToCover='2'" "${output_path}"
grep -q "coveredBranches='1'" "${output_path}"
grep -q "covered='true'" "${output_path}"
grep -q "covered='false'" "${output_path}"

duplicate_directory="${fixture_root}/kcov/duplicate"
mkdir -p "${duplicate_directory}"
cp "${kcov_directory}/sonarqube.xml" "${duplicate_directory}/sonarqube.xml"
if (
  cd "${repo_root}"
  ruby scripts/generate-generic-coverage.rb \
    "${fixture_root}/kcov" \
    "${ruby_directory}" \
    "${fixture_root}/duplicate.xml"
) 2>/dev/null; then
  echo "Multiple independent kcov reports were accepted" >&2
  exit 1
fi

expect_failure generator-usage run_generator

missing_ruby_directory="${fixture_root}/ruby-missing"
mkdir -p "${missing_ruby_directory}"
expect_failure generator-missing-ruby run_generator \
  "${fixture_root}/kcov" "${missing_ruby_directory}" "${fixture_root}/missing-ruby.xml"

unsupported_kcov_directory="${fixture_root}/kcov-unsupported/report"
mkdir -p "${unsupported_kcov_directory}"
printf '<coverage version="1"><file path="%s/docs/not-a-script.rb"><lineToCover lineNumber="1" covered="true"/></file></coverage>\n' \
  "${repo_root}" > "${unsupported_kcov_directory}/sonarqube.xml"
expect_failure generator-unsupported-path run_generator \
  "${fixture_root}/kcov-unsupported" "${ruby_directory}" "${fixture_root}/unsupported.xml"

escaping_kcov_directory="${fixture_root}/kcov-escaping/report"
mkdir -p "${escaping_kcov_directory}"
printf '<coverage version="1"><file path="%s/outside.sh"><lineToCover lineNumber="1" covered="true"/></file></coverage>\n' \
  "${fixture_root}" > "${escaping_kcov_directory}/sonarqube.xml"
expect_failure generator-escaping-path run_generator \
  "${fixture_root}/kcov-escaping" "${ruby_directory}" "${fixture_root}/escaping.xml"

empty_kcov_directory="${fixture_root}/kcov-empty/report"
empty_ruby_directory="${fixture_root}/ruby-empty"
mkdir -p "${empty_kcov_directory}" "${empty_ruby_directory}"
printf '<coverage version="1"/>\n' > "${empty_kcov_directory}/sonarqube.xml"
printf '{"files":[]}\n' > "${empty_ruby_directory}/ruby-empty.json"
expect_failure generator-empty-coverage run_generator \
  "${fixture_root}/kcov-empty" "${empty_ruby_directory}" "${fixture_root}/empty.xml"

printf '%s\n' "script coverage tests passed"
