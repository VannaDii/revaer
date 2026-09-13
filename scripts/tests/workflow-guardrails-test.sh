#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-workflow-guardrails.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT
baseline="${test_root}/baseline"

make_fixture() {
  local root="$1"
  mkdir -p "${root}/config" "${root}/scripts" "${root}/.sonar-test-scope"
  cp -R "${repo_root}/.github" "${root}/.github"
  cp -R "${repo_root}/just" "${root}/just"
  cp "${repo_root}/justfile" "${root}/justfile"
  cp "${repo_root}/.nvmrc" "${root}/.nvmrc"
  cp "${repo_root}/sonar-project.properties" "${root}/sonar-project.properties"
  cp "${repo_root}/config/required-pr-checks.txt" "${root}/config/required-pr-checks.txt"
  cp "${repo_root}/config/sonarsource-public-key.asc" \
    "${root}/config/sonarsource-public-key.asc"
  cp "${repo_root}/scripts/install-sonar-scanner.sh" \
    "${root}/scripts/install-sonar-scanner.sh"
  cp "${repo_root}/scripts/image-release.sh" "${root}/scripts/image-release.sh"
  cp "${repo_root}/scripts/sonar-scan.sh" "${root}/scripts/sonar-scan.sh"
  cp "${repo_root}/scripts/with-node.sh" "${root}/scripts/with-node.sh"
  if [[ -f "${repo_root}/release/media-compliance/media-runtime-inventory.spdx.json" ]]; then
    mkdir -p "${root}/release/media-compliance"
    cp "${repo_root}/release/media-compliance/media-runtime-inventory.spdx.json" \
      "${root}/release/media-compliance/media-runtime-inventory.spdx.json"
  fi
  : > "${root}/.sonar-test-scope/.gitkeep"

  local source_entry
  IFS=',' read -r -a source_entries <<< "$(
    sed -n 's/^sonar.sources=//p' "${root}/sonar-project.properties"
  )"
  for source_entry in "${source_entries[@]}"; do
    if [[ -e "${root}/${source_entry}" ]]; then
      continue
    fi
    if [[ -d "${repo_root}/${source_entry}" ]]; then
      mkdir -p "${root}/${source_entry}"
      : > "${root}/${source_entry}/.fixture"
    else
      mkdir -p "$(dirname "${root}/${source_entry}")"
      : > "${root}/${source_entry}"
    fi
  done

  git -C "${root}" init -q
  git -C "${root}" add -A
}

run_guardrail() {
  local root="$1"
  ruby -I "${repo_root}/scripts" - "${root}" <<'RUBY'
# frozen_string_literal: true

require "workflow_guardrails/diagnostics"
require "workflow_guardrails/github_actions"
require "workflow_guardrails/input_loader"
require "workflow_guardrails/required_checks"
require "workflow_guardrails/sonar_properties"

diagnostics = WorkflowGuardrails::Diagnostics.new
inputs = WorkflowGuardrails::InputLoader.new(ARGV.fetch(0), diagnostics).load
WorkflowGuardrails::GithubActions.new(inputs, diagnostics).validate
WorkflowGuardrails::SonarProperties.new(inputs, diagnostics).validate
WorkflowGuardrails::RequiredChecks.new(inputs, diagnostics).validate
exit diagnostics.finish
RUBY
}

new_case() {
  local name="$1"
  local root="${test_root}/${name}"
  cp -R "${baseline}" "${root}"
  printf '%s\n' "${root}"
}

replace_once() {
  local path="$1"
  local before="$2"
  local after="$3"
  ruby - "${path}" "${before}" "${after}" <<'RUBY'
path, before, after = ARGV
text = File.read(path, encoding: "UTF-8")
abort("fixture replacement target missing: #{before.inspect}") unless text.sub!(before, after)
File.write(path, text, mode: "w", encoding: "UTF-8")
RUBY
}

expect_failure() {
  local label="$1"
  local root="$2"
  local output_name
  output_name="$(basename "${root}")"
  if run_guardrail "${root}" >"${test_root}/${output_name}.stdout" \
    2>"${test_root}/${output_name}.stderr"; then
    printf 'Workflow guardrail accepted %s\n' "${label}" >&2
    exit 1
  fi
}

make_fixture "${baseline}"
run_guardrail "${baseline}" >/dev/null

case_root="$(new_case deceptive-text)"
printf '%s\n' '# uses: owner/action@main' >> "${case_root}/.github/workflows/pr.yml"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '  IMAGE_NAME: revaer' \
  $'  IMAGE_NAME: revaer\n  DECEPTIVE_ACTION_TEXT: owner/action@main'
run_guardrail "${case_root}" >/dev/null

case_root="$(new_case duplicate-yaml-key)"
printf '\njobs:\n' >> "${case_root}/.github/workflows/pr.yml"
expect_failure "a duplicate YAML key" "${case_root}"

case_root="$(new_case property-continuation)"
printf '\nsonar.fixture=value\\\n' >> "${case_root}/sonar-project.properties"
expect_failure "a Java property continuation" "${case_root}"

case_root="$(new_case duplicate-property)"
printf '\nsonar.sources=invalid\n' >> "${case_root}/sonar-project.properties"
expect_failure "a duplicate Sonar property" "${case_root}"

case_root="$(new_case unknown-property)"
printf '\nsonar.unreviewed=true\n' >> "${case_root}/sonar-project.properties"
expect_failure "an unreviewed Sonar property" "${case_root}"

case_root="$(new_case javascript-size-limit)"
replace_once "${case_root}/sonar-project.properties" \
  'sonar.javascript.maxFileSize=100000' 'sonar.javascript.maxFileSize=10000'
expect_failure "a reduced JavaScript analysis size limit" "${case_root}"

case_root="$(new_case missing-context)"
replace_once "${case_root}/config/required-pr-checks.txt" $'Build Release\n' ''
expect_failure "a removed required context" "${case_root}"

case_root="$(new_case duplicate-step-name)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Format\n        run: just fmt' \
  $'      - name: Format\n        uses: actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd\n\n      - name: Format\n        run: just fmt'
expect_failure "a duplicate nonempty workflow step name" "${case_root}"

case_root="$(new_case duplicate-composite-step-name)"
replace_once "${case_root}/.github/actions/setup-revaer/action.yml" \
  '    - name: Install Rust toolchain' '    - name: Cache Rust artifacts'
expect_failure "a duplicate nonempty composite action step name" "${case_root}"

case_root="$(new_case missing-scanner-invocation)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: SonarQube scan\n        env:\n          SONAR_TOKEN: ${{ env.SONAR_AUTH_TOKEN }}\n        run: just sonar-scan\n\n' ''
expect_failure "a missing authoritative scanner invocation" "${case_root}"

case_root="$(new_case duplicate-scanner-invocation)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Package Sonar analysis evidence\n' \
  $'      - name: Duplicate Sonar scanner\n        env:\n          SONAR_TOKEN: ${{ env.SONAR_AUTH_TOKEN }}\n        run: just sonar-scan\n\n      - name: Package Sonar analysis evidence\n'
expect_failure "duplicate authoritative scanner invocations" "${case_root}"

case_root="$(new_case official-action-executor)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'        run: just sonar-scan\n\n      - name: Package Sonar analysis evidence' \
  $'        uses: SonarSource/sonarqube-scan-action@22918119ff8e1ca75a623e15c8296b6ea4fbe28f\n        with:\n          scannerVersion: 8.1.0.6389\n          skipSignatureVerification: "false"\n\n      - name: Package Sonar analysis evidence'
expect_failure "the official Sonar action as executor" "${case_root}"

case_root="$(new_case direct-scanner-executor)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '        run: just sonar-scan' '        run: sonar-scanner'
expect_failure "a direct workflow scanner invocation" "${case_root}"

case_root="$(new_case concealed-failure)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Format\n        run: just fmt' \
  $'      - name: Format\n        continue-on-error: true\n        run: just fmt'
expect_failure "a required step with continue-on-error" "${case_root}"

case_root="$(new_case concealed-failure-expression)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Format\n        run: just fmt' \
  $'      - name: Format\n        continue-on-error: ${{ false }}\n        run: just fmt'
expect_failure "a continue-on-error expression" "${case_root}"

case_root="$(new_case explicit-false)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Format\n        run: just fmt' \
  $'      - name: Format\n        continue-on-error: false\n        run: just fmt'
run_guardrail "${case_root}" >/dev/null

case_root="$(new_case quoted-false)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Format\n        run: just fmt' \
  $'      - name: Format\n        continue-on-error: "false"\n        run: just fmt'
run_guardrail "${case_root}" >/dev/null

ruby -I "${repo_root}/scripts" <<'RUBY'
require "workflow_guardrails/diagnostics"
require "workflow_guardrails/github_actions"

guard = WorkflowGuardrails::GithubActions.allocate
raise "boolean false was concealed" if guard.send(:concealed_failure?, false)
raise "string false was concealed" if guard.send(:concealed_failure?, "false")
raise "boolean true was accepted" unless guard.send(:concealed_failure?, true)
RUBY

case_root="$(new_case narrowed-pr-trigger)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'  pull_request:\n' \
  $'  pull_request:\n    branches: [main]\n'
expect_failure "a branch-narrowed pull request trigger" "${case_root}"

case_root="$(new_case missing-database-rebaseline-proof)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Database rebaseline contract\n        run: just db-rebaseline-candidate\n\n' ''
expect_failure "a missing database rebaseline proof" "${case_root}"

case_root="$(new_case reordered-database-rebaseline-proof)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Database rebaseline contract\n        run: just db-rebaseline-candidate\n\n      - name: Run migrations\n        env:\n          DATABASE_URL: ${{ env.REVAER_TEST_DATABASE_URL }}\n        run: just db-migrate' \
  $'      - name: Run migrations\n        env:\n          DATABASE_URL: ${{ env.REVAER_TEST_DATABASE_URL }}\n        run: just db-migrate\n\n      - name: Database rebaseline contract\n        run: just db-rebaseline-candidate'
expect_failure "a database rebaseline proof after migrations" "${case_root}"

case_root="$(new_case ui-shard-upload-warn)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'          name: ui-e2e-coverage-shard-${{ matrix.shard }}\n          path: |\n            tests/test-results/api-coverage-*.json\n            tests/test-results/ui-coverage-*.json\n          if-no-files-found: error' \
  $'          name: ui-e2e-coverage-shard-${{ matrix.shard }}\n          path: |\n            tests/test-results/api-coverage-*.json\n            tests/test-results/ui-coverage-*.json\n          if-no-files-found: warn'
expect_failure "a non-failing UI shard coverage upload" "${case_root}"

case_root="$(new_case api-e2e-upload-warn)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'          name: api-e2e-coverage\n          path: tests/test-results/api-coverage-*.json\n          if-no-files-found: error' \
  $'          name: api-e2e-coverage\n          path: tests/test-results/api-coverage-*.json\n          if-no-files-found: warn'
expect_failure "a non-failing API E2E coverage upload" "${case_root}"

case_root="$(new_case aggregate-missing-api-e2e-dependency)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '    needs: [api-e2e, ui-e2e, feature-matrix, native-it, coverage]' \
  '    needs: [ui-e2e, feature-matrix, native-it, coverage]'
expect_failure "an aggregate missing the API E2E dependency" "${case_root}"

case_root="$(new_case aggregate-missing-api-e2e-download)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'      - name: Download API E2E coverage\n        uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1\n        with:\n          name: api-e2e-coverage' \
  $'      - name: Download API E2E coverage\n        uses: actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1\n        with:\n          name: ui-e2e-coverage-shard-1'
expect_failure "an aggregate missing the exact API E2E artifact" "${case_root}"

case_root="$(new_case missing-ui-shard-download)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '          name: ui-e2e-coverage-shard-2' '          name: ui-e2e-coverage-shard-1'
expect_failure "an aggregate missing an exact UI shard artifact" "${case_root}"

case_root="$(new_case missing-ui-shard-verification)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '        run: just ui-e2e-shard-coverage' '        run: just ui-e2e-coverage'
expect_failure "an aggregate that does not prove all UI shard records" "${case_root}"

case_root="$(new_case image-missing-media-dependency)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '    needs: [ui-e2e, feature-matrix, native-it, media-conversion, coverage, supply-chain, load-matrix]' \
  '    needs: [ui-e2e, feature-matrix, native-it, coverage, supply-chain, load-matrix]'
expect_failure "PR images without the media conversion dependency" "${case_root}"

case_root="$(new_case image-missing-supply-chain-dependency)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '    needs: [ui-e2e, feature-matrix, native-it, media-conversion, coverage, supply-chain, load-matrix]' \
  '    needs: [ui-e2e, feature-matrix, native-it, media-conversion, coverage, load-matrix]'
expect_failure "PR images without the supply-chain dependency" "${case_root}"

case_root="$(new_case floating-udeps-toolchain)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '      REVAER_UDEPS_TOOLCHAIN: "nightly-2026-06-13"' \
  '      REVAER_UDEPS_TOOLCHAIN: nightly'
expect_failure "a floating cargo-udeps toolchain" "${case_root}"

case_root="$(new_case missing-udeps-evidence)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  $'          name: cargo-udeps-toolchain-evidence\n          path: target/udeps-toolchain-evidence.txt\n          if-no-files-found: error' \
  $'          name: cargo-udeps-toolchain-evidence\n          path: target/udeps-toolchain-evidence.txt\n          if-no-files-found: warn'
expect_failure "non-failing cargo-udeps evidence retention" "${case_root}"

case_root="$(new_case image-dev-helm-disabled)"
replace_once "${case_root}/.github/workflows/pr.yml" \
  '      publish_dev_helm: true' '      publish_dev_helm: false'
expect_failure "same-repository images without dev Helm publication" "${case_root}"

case_root="$(new_case image-matrix-fail-fast)"
replace_once "${case_root}/.github/workflows/build-images.yml" \
  '      fail-fast: false' '      fail-fast: true'
expect_failure "an image matrix that cancels its second analysis" "${case_root}"

case_root="$(new_case image-matrix-cardinality)"
ruby -rjson - "${case_root}/.github/matrices/build-images.json" <<'RUBY'
path = ARGV.fetch(0)
document = JSON.parse(File.read(path, encoding: "UTF-8"))
document.fetch("include").pop
File.write(path, JSON.pretty_generate(document) + "\n", encoding: "UTF-8")
RUBY
expect_failure "an image matrix with fewer than two audited analyses" "${case_root}"

for gate_command in \
  'docker buildx build .' \
  'docker buildx imagetools create fixture' \
  'trivy image fixture' \
  'cosign sign fixture'; do
  case_name="direct-gate-$(printf '%s' "${gate_command}" | tr ' ' '-')"
  case_root="$(new_case "${case_name}")"
  replace_once "${case_root}/.github/workflows/pr.yml" \
    '        run: just fmt' "        run: ${gate_command}"
  expect_failure "direct workflow gate command ${gate_command}" "${case_root}"
done

case_root="$(new_case direct-trivy-action)"
replace_once "${case_root}/.github/workflows/build-images.yml" \
  '        uses: aquasecurity/setup-trivy@81e514348e19b6112ce2a7e3ecbafe19c1e1f567 # v0.3.1' \
  '        uses: aquasecurity/trivy-action@57a97c7e7821a5776cebc9bb87c984fa69cba8f1 # v0.35.0'
expect_failure "a Trivy action used as the scan executor" "${case_root}"

case_root="$(new_case unpinned-docker-action)"
replace_once "${case_root}/.github/workflows/docs.yml" \
  'actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd' \
  'docker://alpine:3.23'
expect_failure "an unpinned docker action" "${case_root}"

case_root="$(new_case digest-pinned-docker-action)"
replace_once "${case_root}/.github/workflows/docs.yml" \
  'actions/checkout@de0fac2e4500dabe0009e67214ff5f5447ce83dd' \
  'docker://alpine@sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'
run_guardrail "${case_root}" >/dev/null

case_root="$(new_case missing-timeout)"
replace_once "${case_root}/.github/workflows/docs.yml" \
  $'    timeout-minutes: 60\n' ''
expect_failure "an unbounded checked-in workflow job" "${case_root}"

case_root="$(new_case floating-node-version)"
printf 'lts/*\n' > "${case_root}/.nvmrc"
expect_failure "a floating NVM Node version" "${case_root}"

case_root="$(new_case extra-just-module)"
: > "${case_root}/just/unapproved.just"
expect_failure "an eighth Just module" "${case_root}"

case_root="$(new_case duplicate-recipe-owner)"
printf '\nfmt:\n    true\n' >> "${case_root}/just/docs.just"
expect_failure "a duplicate Just recipe owner" "${case_root}"

case_root="$(new_case stale-asset-source)"
replace_once "${case_root}/sonar-project.properties" 'revaer-logo.svg' 'revaer-logo.png'
expect_failure "a stale PNG Sonar source after asset integration" "${case_root}"

case_root="$(new_case integrated-asset-source)"
run_guardrail "${case_root}" >/dev/null

conversion_recipe="$(just --justfile "${baseline}/justfile" --show test-media-conversion)"

case_root="$(new_case coverage-default-test-hiding)"
replace_once "${case_root}/just/quality.just" \
  'cargo llvm-cov report --no-default-ignore-filename-regex' 'cargo llvm-cov report'
expect_failure "coverage exports hiding authored tests by default" "${case_root}"

case_root="$(new_case coverage-softened-package-gate)"
replace_once "${case_root}/just/quality.just" '--fail-under-lines 90' '--fail-under-lines 0'
expect_failure "a softened Rust package coverage threshold" "${case_root}"

case_root="$(new_case coverage-tests-dilute-package-gate)"
replace_once "${case_root}/just/quality.just" \
  'cargo llvm-cov report --package' 'cargo llvm-cov report --no-default-ignore-filename-regex --package'
expect_failure "test-file counts changing the established package coverage gate" "${case_root}"

case_root="$(new_case coverage-execution-failure-masked)"
replace_once "${case_root}/just/quality.just" \
  $'set -euo pipefail; \\\n        toolchain_env=' $': \\\n        toolchain_env='
expect_failure "coverage execution without fail-closed shell behavior" "${case_root}"

case_root="$(new_case coverage-exports-detached)"
replace_once "${case_root}/just/quality.just" '    just cov-report' '    true'
expect_failure "coverage exports detached from canonical instrumentation" "${case_root}"

case_root="$(new_case coverage-export-error-masked)"
replace_once "${case_root}/just/quality.just" \
  '--lcov --output-path coverage/lcov.info' '--lcov --output-path coverage/lcov.info || true'
expect_failure "a suppressed coverage export failure" "${case_root}"

case_root="$(new_case coverage-destructive-reset)"
replace_once "${case_root}/just/quality.just" 'rm -rf coverage/html' 'rm -rf coverage'
expect_failure "Rust reports deleting independent coverage inputs" "${case_root}"

case_root="$(new_case coverage-stale-html)"
replace_once "${case_root}/just/quality.just" 'rm -rf coverage/html' 'true'
expect_failure "Rust reports retaining stale HTML" "${case_root}"

case_root="$(new_case coverage-stale-text)"
replace_once "${case_root}/just/quality.just" 'rm -f coverage/lcov.info coverage/llvm-cov.txt' 'true'
expect_failure "Rust reports retaining stale LCOV or native text" "${case_root}"

case_root="$(new_case coverage-symlink-root-accepted)"
replace_once "${case_root}/just/quality.just" 'if test -L coverage; then' 'if false; then'
expect_failure "Rust reports accepting a symlinked coverage root" "${case_root}"

for mode in pass lcov html text; do
  case_root="$(new_case "coverage-export-${mode}")"
  mkdir -p "${case_root}/bin"
  cat > "${case_root}/bin/cargo" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$*" >> "${REVAER_COVERAGE_CALLS}"
if [[ -n "${REVAER_COVERAGE_FAILURE}" && " $* " == *" ${REVAER_COVERAGE_FAILURE} "* ]]; then
  exit 17
fi
SH
  chmod +x "${case_root}/bin/cargo"
  failure="--${mode}"
  expected_calls=3
  case "${mode}" in
    pass) failure="" ;;
    lcov) expected_calls=1 ;;
    html) expected_calls=2 ;;
  esac
  status=0
  (
    cd "${case_root}"
    PATH="${case_root}/bin:${PATH}" REVAER_COVERAGE_FAILURE="${failure}" \
      REVAER_COVERAGE_CALLS="${case_root}/calls" \
      just --no-dotenv --justfile "${case_root}/justfile" cov-report
  ) > "${case_root}/export.log" 2>&1 || status=$?
  if [[ "$(wc -l < "${case_root}/calls" | tr -d ' ')" != "${expected_calls}" ]]; then
    echo "Coverage export ${mode} continued past a failed report or missed a report" >&2
    exit 1
  fi
  if [[ ( "${mode}" = pass && "${status}" != 0 ) || ( "${mode}" != pass && "${status}" = 0 ) ]]; then
    echo "Coverage export ${mode} did not preserve command failure" >&2
    exit 1
  fi
done

case_root="$(new_case media-fixture-only)"
replace_once "${case_root}/just/media.just" "$conversion_recipe" \
  'test-media-conversion: verify-test-fixtures'
expect_failure "fixture preparation without Rust conversion" "$case_root"

case_root="$(new_case media-command-decoy)"
replace_once "${case_root}/just/media.just" "$conversion_recipe" \
  $'test-media-conversion: verify-test-fixtures\n\nunused-conversion-decoy:\n    # cargo test --test media_fixtures -- --include-ignored'
expect_failure "a conversion command outside the real recipe" "$case_root"

case_root="$(new_case media-named-test-filter)"
replace_once "${case_root}/just/media.just" \
  '--test media_fixtures -- --include-ignored' \
  '--test media_fixtures missing_prepared_fixture_suite -- --include-ignored'
expect_failure "a named filter capable of selecting zero tests" "$case_root"

case_root="$(new_case media-ignored-suite-omitted)"
replace_once "${case_root}/just/media.just" '-- --include-ignored' ''
expect_failure "omitted ignored-test execution" "$case_root"

case_root="$(new_case media-filtered-ignored-only)"
replace_once "${case_root}/just/media.just" '--include-ignored' '--ignored'
expect_failure "excluding the regular conversion binary tests" "$case_root"

case_root="$(new_case media-feature-selection)"
replace_once "${case_root}/just/media.just" '--all-features --test media_fixtures' \
  '--test media_fixtures'
expect_failure "reduced media feature selection" "$case_root"

case_root="$(new_case media-warnings-allowed)"
replace_once "${case_root}/just/media.just" '-Dwarnings' '-Awarnings'
expect_failure "allowed conversion compiler warnings" "$case_root"

case_root="$(new_case media-swallowed-rust-failure)"
replace_once "${case_root}/just/media.just" '-- --include-ignored' \
  '-- --include-ignored || true'
expect_failure "a swallowed Rust conversion failure" "$case_root"

case_root="$(new_case media-errexit-disabled)"
replace_once "${case_root}/just/media.just" 'set -euo pipefail' 'set -uo pipefail'
expect_failure "disabled conversion failure propagation" "$case_root"

case_root="$(new_case media-preparation-report-reused)"
replace_once "${case_root}/just/media.just" \
  'REVAER_MEDIA_CONVERSION_REPORT="${report_path}.preparation"' \
  'REVAER_MEDIA_CONVERSION_REPORT="${report_path}"'
expect_failure "preparation written as the conversion report" "$case_root"

case_root="$(new_case media-stale-report)"
replace_once "${case_root}/just/media.just" 'rm -f -- "${report_path}"' \
  '# stale reports were not removed'
expect_failure "stale conversion evidence" "$case_root"

case_root="$(new_case media-report-proof-removed)"
replace_once "${case_root}/just/media.just" \
  'grep -Eq -- '"'"'^- Video transcodes: [1-9][0-9]*$'"'"' "${report_path}"' \
  'true'
expect_failure "missing video-transcode evidence" "$case_root"

case_root="$(new_case media-integrity-noop)"
replace_once "${case_root}/just/media.just" \
  'bash scripts/test-fixtures/verify-fixtures.sh' 'true'
expect_failure "a no-op integrity prerequisite" "$case_root"

recipe_root="$(new_case media-recipe-execution)"
mkdir -p "$recipe_root/scripts/test-fixtures" "$recipe_root/stub-bin"
cat >"$recipe_root/scripts/test-fixtures/verify-fixtures.sh" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
printf 'verify\n' >>"$REVAER_RECIPE_TRACE"
test ! -e "$REVAER_GATE_REPORT"
test "$REVAER_MEDIA_CONVERSION_REPORT" = "${REVAER_GATE_REPORT}.preparation"
if [[ "$REVAER_RECIPE_CASE" == integrity-failure ]]; then exit 23; fi
printf '%s\n' '- Outcome: passed' >"$REVAER_MEDIA_CONVERSION_REPORT"
STUB
cat >"$recipe_root/stub-bin/cargo" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
printf 'cargo\n' >>"$REVAER_RECIPE_TRACE"
case "$REVAER_RECIPE_CASE" in
  rust-failure) exit 41 ;;
  missing-tool) exit 127 ;;
  stale-success|zero-executed) exit 0 ;;
  preparation-only)
    cp "${REVAER_GATE_REPORT}.preparation" "$REVAER_MEDIA_CONVERSION_REPORT"
    exit 0
    ;;
esac
outcome=passed
video_count=1
audio_count=1
if [[ "$REVAER_RECIPE_CASE" == failed-outcome ]]; then outcome=failed; fi
if [[ "$REVAER_RECIPE_CASE" == zero-video ]]; then video_count=0; fi
if [[ "$REVAER_RECIPE_CASE" == zero-audio ]]; then audio_count=0; fi
printf '%s\n' \
  "- Outcome: $outcome" \
  '- Pipeline actions: 1' \
  "- Video transcodes: $video_count" \
  "- Audio transcodes: $audio_count" \
  '- Pipeline failures: 0' \
  '- Suite failures: 0' >"$REVAER_MEDIA_CONVERSION_REPORT"
STUB
chmod +x "$recipe_root/stub-bin/cargo"

for recipe_case in success integrity-failure rust-failure missing-tool stale-success \
  zero-executed preparation-only failed-outcome zero-video zero-audio; do
  report="$recipe_root/$recipe_case.md"
  trace="$recipe_root/$recipe_case.trace"
  if [[ "$recipe_case" == stale-success || "$recipe_case" == integrity-failure ]]; then
    printf '%s\n' '- Outcome: passed' '- Pipeline actions: 9' '- Video transcodes: 9' \
      '- Audio transcodes: 9' '- Pipeline failures: 0' '- Suite failures: 0' >"$report"
  fi
  if PATH="$recipe_root/stub-bin:$PATH" \
    REVAER_RECIPE_CASE="$recipe_case" REVAER_RECIPE_TRACE="$trace" \
    REVAER_GATE_REPORT="$report" REVAER_MEDIA_CONVERSION_REPORT="$report" \
    just --justfile "$recipe_root/justfile" test-media-conversion \
      >"$recipe_root/$recipe_case.log" 2>&1; then
    recipe_status=0
  else
    recipe_status=$?
  fi
  if [[ "$recipe_case" == success ]]; then
    if [[ "$recipe_status" -ne 0 ]]; then
      printf 'conversion recipe rejected fresh success evidence\n' >&2
      exit 1
    fi
  elif [[ "$recipe_status" -eq 0 ]]; then
    printf 'conversion recipe accepted failure case: %s\n' "$recipe_case" >&2
    exit 1
  fi
  expected_trace=$'verify\ncargo'
  if [[ "$recipe_case" == integrity-failure ]]; then expected_trace=verify; fi
  if [[ "$(tr -d '\r' <"$trace")" != "$expected_trace" ]]; then
    printf 'conversion recipe ordering changed: %s\n' "$recipe_case" >&2
    exit 1
  fi
done

printf '%s\n' "Workflow guardrail regression tests passed"
