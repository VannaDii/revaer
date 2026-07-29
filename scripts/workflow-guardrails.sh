#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${repo_root}"

files=()
while IFS= read -r file; do
  files+=("${file}")
done < <(find .github/workflows .github/actions -type f \( -name '*.yml' -o -name '*.yaml' \) | sort)

if [[ "${#files[@]}" -eq 0 ]]; then
  exit 0
fi

failures=0

report_failure() {
  local message="$1"

  printf 'Workflow guardrail failed: %s\n' "${message}" >&2
}

report_matches() {
  local title="$1"
  local matches="$2"

  if [[ -n "${matches}" ]]; then
    report_failure "${title}"
    printf '%s\n' "${matches}" >&2
    printf '\n' >&2
    failures=1
  fi
}

report_missing() {
  local title="$1"
  local pattern="$2"
  local file="$3"

  if ! grep -Eq "${pattern}" "${file}"; then
    report_failure "${title}"
    printf '%s: missing pattern %s\n\n' "${file}" "${pattern}" >&2
    failures=1
  fi
}

report_property_empty() {
  local title="$1"
  local key="$2"
  local file="$3"
  local actual

  actual="$(awk -F= -v key="${key}" '$1 == key { print; found = 1 } END { if (!found) exit 1 }' "${file}" || true)"
  if [[ "${actual}" != "${key}=" ]]; then
    report_failure "${title}"
    printf '%s: expected %s to be explicitly empty\n' "${file}" "${key}" >&2
    printf '%s: actual %s\n\n' "${file}" "${actual}" >&2
    failures=1
  fi
}

report_property_equals() {
  local title="$1"
  local key="$2"
  local expected="$3"
  local file="$4"
  local actual

  actual="$(awk -F= -v key="${key}" '$1 == key { print; found = 1 } END { if (!found) exit 1 }' "${file}" || true)"
  if [[ "${actual}" != "${key}=${expected}" ]]; then
    report_failure "${title}"
    printf '%s: expected %s=%s\n' "${file}" "${key}" "${expected}" >&2
    printf '%s: actual %s\n\n' "${file}" "${actual}" >&2
    failures=1
  fi
}

uses_matches="$(
  awk '
    {
      if ($0 !~ /^[-[:space:]]*uses:[[:space:]]*[^[:space:]#]+/) {
        next;
      }

      ref = $0;
      sub(/^[-[:space:]]*uses:[[:space:]]*/, "", ref);
      sub(/[[:space:]]*#.*/, "", ref);

      comment = "";
      if (index($0, "#") > 0) {
        comment = substr($0, index($0, "#"));
      }

      if (ref ~ /^\.\// || ref ~ /^docker:\/\//) {
        next;
      }

      if (ref !~ /@/) {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
        next;
      }

      version = ref;
      sub(/^.*@/, "", version);

      if (length(version) != 40 || version !~ /^[0-9a-f]+$/) {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }
    }
  ' "${files[@]}"
)"
report_matches "external GitHub actions must pin full commit SHAs instead of mutable refs or release tags" "${uses_matches}"

run_matches="$(
  awk '
    function indent_of(line,    idx, ch) {
      for (idx = 1; idx <= length(line); idx++) {
        ch = substr(line, idx, 1);
        if (ch != " ") {
          return idx - 1;
        }
      }
      return length(line);
    }

    {
      if (in_run) {
        if ($0 !~ /^[[:space:]]*$/ && indent_of($0) <= run_indent) {
          in_run = 0;
        } else {
          if ($0 ~ /\$\{\{[[:space:]]*inputs\./) {
            printf "%s:%d:%s\n", FILENAME, FNR, $0;
          }
          next;
        }
      }

      if ($0 !~ /^[[:space:]]*run:[[:space:]]*/) {
        next;
      }

      run_indent = indent_of($0);
      run_value = $0;
      sub(/^[[:space:]]*run:[[:space:]]*/, "", run_value);

      if (run_value ~ /\$\{\{[[:space:]]*inputs\./) {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (run_value == "" || run_value ~ /^[|>][-+]?$/) {
        in_run = 1;
      }
    }
  ' "${files[@]}"
)"
report_matches 'workflow run blocks must not interpolate ${{ inputs.* }} directly' "${run_matches}"

pr_release_skip_matches="$(
  awk '
    $0 ~ /if: github\.ref == '\''refs\/heads\/main'\'' \|\| startsWith\(github\.ref, '\''refs\/tags\/'\''\)/ {
      printf "%s:%d:%s\n", FILENAME, FNR, $0;
    }
  ' .github/workflows/pr.yml
)"
report_matches "PR Build Release must run on pull requests instead of using main/tag-only guards" "${pr_release_skip_matches}"

supply_chain_job_matches="$(
  awk '
    /^  (audit|deny|udeps):/ {
      printf "%s:%d:%s\n", FILENAME, FNR, $0;
    }
  ' .github/workflows/pr.yml
)"
report_matches "PR supply-chain checks must run in one combined job instead of standalone audit, deny, or udeps jobs" "${supply_chain_job_matches}"

report_missing "PR workflow must define the combined supply-chain job" '^  supply-chain:' .github/workflows/pr.yml
report_missing "PR supply-chain job must cache installed Cargo tool binaries" 'cargo-supply-chain-tools' .github/workflows/pr.yml
report_missing "PR supply-chain job must cache the Cargo advisory database" '~/.cargo/advisory-db' .github/workflows/pr.yml
report_missing "PR supply-chain job must install cargo-audit as a prebuilt tool" 'cargo-audit@0\.22\.0' .github/workflows/pr.yml
report_missing "PR supply-chain job must install cargo-deny as a prebuilt tool" 'cargo-deny@0\.18\.9' .github/workflows/pr.yml
report_missing "PR supply-chain job must install cargo-udeps as a prebuilt tool" 'cargo-udeps@0\.1\.57' .github/workflows/pr.yml
report_missing "PR supply-chain job must run audit through just" 'just audit' .github/workflows/pr.yml
report_missing "PR supply-chain job must run deny through just" 'just deny' .github/workflows/pr.yml
report_missing "PR supply-chain job must run udeps through just" 'just udeps' .github/workflows/pr.yml

managed_db_call_count="$(
  grep -Ec 'REVAER_DB_MANAGED="\$\{db_managed\}".*just db-start' justfile || true
)"
if [[ "${managed_db_call_count}" -ne 2 ]]; then
  printf 'Workflow guardrail failed: validate and coverage must pass explicit managed-database provenance to db-start\n' >&2
  printf 'justfile: expected 2 managed db-start calls, found %s\n\n' "${managed_db_call_count}" >&2
  failures=1
fi
report_missing "validate must propagate managed-database provenance to nested gates" 'REVAER_DB_MANAGED="\$\{db_managed\}" REVAER_TEST_DATABASE_URL="\$\{test_database_url\}" DATABASE_URL="\$\{database_url\}"' justfile
report_missing "db-start must validate explicit managed-database provenance" 'REVAER_DB_MANAGED must be auto, 0, 1, true, or false' justfile
report_missing "Playwright local database bootstrap must pass managed-database provenance" "REVAER_DB_MANAGED: '1'" tests/global-setup.ts

if [[ -s .secignore ]]; then
  printf 'Workflow guardrail failed: advisory ignores require explicit operator consent and must remain absent by default\n' >&2
  printf '.secignore must be empty\n\n' >&2
  failures=1
fi
report_missing "cargo-deny advisory ignores must remain empty" '^ignore = \[\]$' deny.toml
audit_ignore_matches="$(rg -n -- '\.secignore|ignore_args|cargo audit.*--ignore' justfile || true)"
report_matches "the canonical cargo audit recipe must not provide an advisory-ignore mechanism" "${audit_ignore_matches}"
report_missing "canonical audit must reject every npm advisory severity in test tooling" 'npm --prefix tests audit --audit-level=low' justfile
report_missing "canonical audit must reject every npm advisory severity in release tooling" 'npm --prefix release audit --audit-level=low' justfile

sonar_relaxation_matches="$(
  awk '
    /^[[:space:]]*sonar\..*(enabled|activate)[[:space:]]*=[[:space:]]*false[[:space:]]*$/ ||
    /^[[:space:]]*sonar\.(skip|scanner\.skip|scm\.disabled)[[:space:]]*=[[:space:]]*true[[:space:]]*$/ ||
    /^[[:space:]]*sonar\.qualitygate\.wait[[:space:]]*=[[:space:]]*false[[:space:]]*$/ {
      printf "%s:%d:%s\n", FILENAME, FNR, $0;
    }
  ' sonar-project.properties
)"
report_matches "Sonar criteria must not be relaxed in sonar-project.properties" "${sonar_relaxation_matches}"

allowed_sonar_properties=(
  sonar.projectKey
  sonar.organization
  sonar.sourceEncoding
  sonar.scanner.excludeHiddenFiles
  sonar.text.activate
  sonar.text.inclusions.activate
  sonar.text.inclusions
  sonar.html.file.suffixes
  sonar.tsql.file.suffixes
  sonar.plsql.file.suffixes
  sonar.plsql.defaultSchema
  sonar.featureflag.cloud-security-enable-generic-yaml-and-json-analyzer
  sonar.yaml.activate
  sonar.json.activate
  sonar.lang.patterns.kubernetes
  sonar.lang.patterns.yaml
  sonar.sources
  sonar.scm.exclusions.disabled
  sonar.scm.disabled
  sonar.scm.provider
  sonar.scm.forceReloadAll
  sonar.sensor.cache.project.enable
  sonar.scanner.keepReport
  sonar.exclusions
  sonar.inclusions
  sonar.tests
  sonar.test.exclusions
  sonar.test.inclusions
  sonar.coverage.exclusions
  sonar.cpd.exclusions
  sonar.issue.ignore.multicriteria
  sonar.issue.ignore.allfile
  sonar.issue.ignore.block
  sonar.issue.enforce.multicriteria
  sonar.filesize.limit
  sonar.javascript.exclusions
  sonar.javascript.maxFileSize
  sonar.javascript.detectBundles
  sonar.sca.enabled
  sonar.sca.exclusions
  sonar.sca.allowManifestFailures
  sonar.sca.goNoResolve
  sonar.sca.mavenNoResolve
  sonar.sca.gradleNoResolve
  sonar.sca.pythonNoResolve
  sonar.sca.npmNoResolve
  sonar.sca.nugetNoResolve
  sonar.sca.cfamily
  sonar.sca.sbomImportPaths
  sonar.rust.clippy.enabled
  sonar.rust.lcov.reportPaths
  sonar.javascript.lcov.reportPaths
  sonar.cfamily.compile-commands
  sonar.newCode.referenceBranch
  sonar.qualitygate.wait
  sonar.qualitygate.timeout
)
actual_sonar_properties="$(awk -F= '/^sonar\./ { print $1 }' sonar-project.properties | sort)"
expected_sonar_properties="$(printf '%s\n' "${allowed_sonar_properties[@]}" | sort)"
unknown_sonar_properties="$(comm -13 <(printf '%s\n' "${expected_sonar_properties}") <(printf '%s\n' "${actual_sonar_properties}"))"
missing_sonar_properties="$(comm -23 <(printf '%s\n' "${expected_sonar_properties}") <(printf '%s\n' "${actual_sonar_properties}"))"
duplicate_sonar_properties="$(printf '%s\n' "${actual_sonar_properties}" | uniq -d)"
report_matches "unrecognized Sonar properties are forbidden until their strictness is reviewed and the allowlist is intentionally updated" "${unknown_sonar_properties}"
report_matches "required strict Sonar properties must not be removed" "${missing_sonar_properties}"
report_matches "duplicate Sonar properties are forbidden because later values can silently override strict settings" "${duplicate_sonar_properties}"

sonar_workflow_overrides="$(rg -n -i 'sonar\.[[:alnum:]_.-]+[[:space:]]*=' .github/workflows .github/actions || true)"
report_matches "workflow-level Sonar property overrides are forbidden; sonar-project.properties is the only criteria source" "${sonar_workflow_overrides}"

sonar_scope_relaxation_matches="$(
  awk -F= '
    /^[[:space:]]*(#|$)/ {
      next;
    }

    function trim(value) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value);
      return value;
    }

    {
      key = trim($1);
      value = trim(substr($0, index($0, "=") + 1));

      if (key == "sonar.scm.exclusions.disabled") {
        next;
      }

      if (key == "sonar.text.inclusions" && value == "**/*") {
        next;
      }

      if (key ~ /^sonar\..*(exclusions|inclusions)$/ && value != "") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key == "sonar.secrets.excluded.file.suffixes" || key == "sonar.text.excluded.file.suffixes") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key == "sonar.plsql.file.suffixes" && value != ".plsql") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key ~ /^sonar\.issue\.(ignore|enforce)\./ && value != "") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key ~ /^sonar\.sca\..*NoResolve$/ && value == "true") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key == "sonar.sca.allowManifestFailures" && value != "false") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key == "sonar.javascript.detectBundles" && value != "false") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key == "sonar.scm.forceReloadAll" && value != "true") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key == "sonar.sensor.cache.project.enable" && value != "true") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }

      if (key == "sonar.scanner.keepReport" && value != "true") {
        printf "%s:%d:%s\n", FILENAME, FNR, $0;
      }
    }
  ' sonar-project.properties
)"
report_matches "Sonar scope filters, binary suffix exclusions, SQL analyzer drift, issue filters, bundle skips, and degraded SCA options are forbidden unless explicitly strict" "${sonar_scope_relaxation_matches}"

sonar_test_scope='.sonar-test-scope'
expected_sonar_sources="$(git ls-files | awk -F/ -v test_scope="${sonar_test_scope}" '$1 != test_scope {print $1}' | sort -u | paste -sd, -)"
report_property_equals "Sonar scanner must analyze every authored tracked top-level repository entry as main code without scanning Git internals" 'sonar.sources' "${expected_sonar_sources}" sonar-project.properties
report_property_equals "Sonar automatic test-path detection must be disabled by the empty strict-scope sentinel" 'sonar.tests' "${sonar_test_scope}" sonar-project.properties
unexpected_sonar_test_scope_files="$(git ls-files "${sonar_test_scope}" | awk -v marker="${sonar_test_scope}/.gitkeep" '$0 != marker { print }')"
report_matches "Sonar test scope must not contain authored files" "${unexpected_sonar_test_scope_files}"
if [[ -s "${sonar_test_scope}/.gitkeep" ]]; then
  report_failure "Sonar test-scope marker must remain empty"
fi
report_missing "Sonar scanner must disable SCM ignore exclusions" '^sonar\.scm\.exclusions\.disabled=true$' sonar-project.properties
report_property_equals "Sonar scanner SCM analysis must stay enabled" 'sonar.scm.disabled' 'false' sonar-project.properties
report_property_equals "Sonar scanner SCM provider must stay Git" 'sonar.scm.provider' 'git' sonar-project.properties
report_property_equals "Sonar scanner must reload blame for every indexed file" 'sonar.scm.forceReloadAll' 'true' sonar-project.properties
report_property_equals "Sonar scanner must retain project sensor-cache support for cross-file symbolic execution" 'sonar.sensor.cache.project.enable' 'true' sonar-project.properties
report_property_equals "Sonar scanner must retain its submitted report for verification" 'sonar.scanner.keepReport' 'true' sonar-project.properties
report_property_equals "Sonar hidden tracked files must remain eligible for text and secrets analysis" 'sonar.scanner.excludeHiddenFiles' 'false' sonar-project.properties
report_property_equals "Sonar text and secrets analysis must stay enabled" 'sonar.text.activate' 'true' sonar-project.properties
report_property_equals "Sonar additional tracked-file text scope must stay enabled" 'sonar.text.inclusions.activate' 'true' sonar-project.properties
report_property_equals "Sonar additional tracked-file text scope must cover the repository" 'sonar.text.inclusions' '**/*' sonar-project.properties
report_property_equals "Sonar HTML analyzer must stay limited to tracked HTML source suffixes" 'sonar.html.file.suffixes' '.html' sonar-project.properties
report_matches "Sonar Kubernetes suffix override must remain absent so built-in content detection can run" "$(rg -n '^sonar\.kubernetes\.file\.suffixes=' sonar-project.properties || true)"
expected_sonar_kubernetes_patterns="$(git ls-files '*.yaml' '*.yml' | awk '/^charts\/revaer\/templates\/.*\.ya?ml$/ { print }' | paste -sd, -)"
expected_sonar_yaml_patterns="$(git ls-files '*.yaml' '*.yml' | awk '!/^charts\/revaer\/templates\/.*\.ya?ml$/ { print }' | paste -sd, -)"
report_property_equals "Sonar Kubernetes language ownership must match every tracked Helm manifest exactly" 'sonar.lang.patterns.kubernetes' "${expected_sonar_kubernetes_patterns}" sonar-project.properties
report_property_equals "Sonar YAML language ownership must match every other tracked YAML file exactly" 'sonar.lang.patterns.yaml' "${expected_sonar_yaml_patterns}" sonar-project.properties
report_property_equals "Sonar T-SQL suffixes must stay isolated from PostgreSQL migrations" 'sonar.tsql.file.suffixes' '.tsql' sonar-project.properties
report_property_equals "Sonar PL/SQL suffixes must not claim PostgreSQL migrations" 'sonar.plsql.file.suffixes' '.plsql' sonar-project.properties
report_property_equals "Sonar generic file size limit must stay high enough for tracked first-party files" 'sonar.filesize.limit' '100' sonar-project.properties
report_property_equals "Sonar JavaScript file size limit must stay high enough for tracked JavaScript assets" 'sonar.javascript.maxFileSize' '10000' sonar-project.properties
report_property_equals "Sonar JavaScript bundle detection must not hide committed assets" 'sonar.javascript.detectBundles' 'false' sonar-project.properties
report_property_equals "Sonar generic YAML and JSON IaC analyzer must stay enabled" 'sonar.featureflag.cloud-security-enable-generic-yaml-and-json-analyzer' 'true' sonar-project.properties
report_property_equals "Sonar YAML analyzer must stay explicitly active" 'sonar.yaml.activate' 'true' sonar-project.properties
report_property_equals "Sonar JSON analyzer must stay explicitly active" 'sonar.json.activate' 'true' sonar-project.properties
report_property_equals "Sonar SCA must stay enabled" 'sonar.sca.enabled' 'true' sonar-project.properties
report_property_equals "Sonar SCA manifest failures must fail analysis" 'sonar.sca.allowManifestFailures' 'false' sonar-project.properties
report_property_equals "Sonar SCA Go resolution must stay enabled" 'sonar.sca.goNoResolve' 'false' sonar-project.properties
report_property_equals "Sonar SCA Maven resolution must stay enabled" 'sonar.sca.mavenNoResolve' 'false' sonar-project.properties
report_property_equals "Sonar SCA Gradle resolution must stay enabled" 'sonar.sca.gradleNoResolve' 'false' sonar-project.properties
report_property_equals "Sonar SCA Python resolution must stay enabled" 'sonar.sca.pythonNoResolve' 'false' sonar-project.properties
report_property_equals "Sonar SCA npm resolution must stay enabled" 'sonar.sca.npmNoResolve' 'false' sonar-project.properties
report_property_equals "Sonar SCA NuGet resolution must stay enabled" 'sonar.sca.nugetNoResolve' 'false' sonar-project.properties
report_property_equals "Sonar SCA CFamily dependency analysis must stay enabled" 'sonar.sca.cfamily' 'true' sonar-project.properties
report_property_equals "Sonar SCA must import the committed media runtime SPDX inventory" 'sonar.sca.sbomImportPaths' 'release/media-compliance/media-runtime-inventory.spdx.json' sonar-project.properties
report_property_equals "Sonar Rust Clippy integration must stay enabled" 'sonar.rust.clippy.enabled' 'true' sonar-project.properties
report_missing "Sonar scanner must import Rust LCOV coverage" '^sonar\.rust\.lcov\.reportPaths=coverage/lcov\.info$' sonar-project.properties
report_missing "Sonar scanner must import JavaScript and TypeScript LCOV coverage" '^sonar\.javascript\.lcov\.reportPaths=coverage/js-lcov\.info$' sonar-project.properties
report_missing "Coverage execution must retain full workspace and feature scope" 'cargo llvm-cov --workspace --all-features --no-report' justfile
report_missing "Coverage reports must retain LCOV source and line records" 'cargo llvm-cov report --lcov --output-path coverage/lcov\.info' justfile
report_missing "JavaScript and TypeScript coverage reports must retain LCOV source and line records" 'just js-coverage-merge' .github/workflows/pr.yml .github/workflows/sonar.yml
report_missing "Sonar scanner must import the native compile database" '^sonar\.cfamily\.compile-commands=coverage/compile_commands\.json$' sonar-project.properties
report_missing "Sonar scanner must declare the default schema for PL/SQL analysis" '^sonar\.plsql\.defaultSchema=public$' sonar-project.properties
report_missing "Sonar scanner must wait for the quality gate" '^sonar\.qualitygate\.wait=true$' sonar-project.properties
report_missing "PR workflow must define a separate Sonar scan job" '^  sonar:' .github/workflows/pr.yml
report_missing "Main Sonar workflow must define a separate Sonar scan job" '^  sonar:' .github/workflows/sonar.yml
report_missing "PR Sonar scan must download only prepared coverage inputs" 'Download Sonar coverage inputs' .github/workflows/pr.yml
report_missing "Main Sonar scan must download only prepared coverage inputs" 'Download Sonar coverage inputs' .github/workflows/sonar.yml
report_missing "PR Sonar scan must verify the exact reviewed base SHA" 'SONAR_BASE_SHA: \$\{\{ github\.event\.pull_request\.base\.sha \}\}' .github/workflows/pr.yml
report_missing "PR Sonar scan must compare the fetched base ref to the reviewed base SHA" 'actual_base_sha="\$\(git rev-parse "refs/remotes/origin/\$\{SONAR_BASE_REF\}"\)"' .github/workflows/pr.yml
report_missing "PR Sonar inputs must include the staged native CXX runtime bridge header" 'coverage/cxxbridge/include/rust/cxx\.h' .github/workflows/pr.yml
report_missing "Main Sonar inputs must include the staged native CXX runtime bridge header" 'coverage/cxxbridge/include/rust/cxx\.h' .github/workflows/sonar.yml
report_missing "PR Sonar inputs must include the staged native crate bridge header" 'coverage/cxxbridge/include/revaer-torrent-libt/src/ffi/bridge\.rs\.h' .github/workflows/pr.yml
report_missing "Main Sonar inputs must include the staged native crate bridge header" 'coverage/cxxbridge/include/revaer-torrent-libt/src/ffi/bridge\.rs\.h' .github/workflows/sonar.yml
report_missing "PR Sonar inputs must require Rust LCOV line records before scanning" "grep -q '\^DA:' coverage/lcov\.info" .github/workflows/pr.yml
report_missing "Main Sonar inputs must require Rust LCOV line records before scanning" "grep -q '\^DA:' coverage/lcov\.info" .github/workflows/sonar.yml
report_missing "PR Sonar inputs must require JavaScript and TypeScript LCOV line records before scanning" "grep -q '\^DA:' coverage/js-lcov\.info" .github/workflows/pr.yml
report_missing "Main Sonar inputs must require JavaScript and TypeScript LCOV line records before scanning" "grep -q '\^DA:' coverage/js-lcov\.info" .github/workflows/sonar.yml
report_missing "PR Sonar input checks must require the staged native CXX include path" 'coverage/cxxbridge/include' .github/workflows/pr.yml
report_missing "Main Sonar input checks must require the staged native CXX include path" 'coverage/cxxbridge/include' .github/workflows/sonar.yml
report_missing "PR Sonar scan must install the complete pinned Rust component set" 'components: rustfmt,clippy,llvm-tools-preview' .github/workflows/pr.yml
report_missing "Main Sonar scan must install the complete pinned Rust component set" 'components: rustfmt,clippy,llvm-tools-preview' .github/workflows/sonar.yml
report_missing "PR Sonar scan must install native libtorrent headers used by the compile database" 'libtorrent-rasterbar-dev' .github/workflows/pr.yml
report_missing "Main Sonar scan must install native libtorrent headers used by the compile database" 'libtorrent-rasterbar-dev' .github/workflows/sonar.yml
report_missing "PR Sonar scan must verify the published result through the Justfile" 'run: just sonar-verify-result' .github/workflows/pr.yml
report_missing "Main Sonar scan must verify the published result through the Justfile" 'run: just sonar-verify-result' .github/workflows/sonar.yml
report_missing "PR Sonar scan must retain the complete submitted scanner report" '\.scannerwork/scanner-report\.tar\.xz' .github/workflows/pr.yml
report_missing "Main Sonar scan must retain the complete submitted scanner report" '\.scannerwork/scanner-report\.tar\.xz' .github/workflows/sonar.yml
report_missing "Setup action must expose a Cargo cache opt-out" 'cargo-cache:' .github/actions/setup-revaer/action.yml

report_pr_cache_opt_out() {
  local title="$1"
  local job_name="$2"
  local cache_opt_out

  cache_opt_out="$(
    awk -v job_header="  ${job_name}:" '
      $0 == job_header {
        in_job = 1;
        next;
      }
      in_job && /^  [^[:space:]][^:]*:/ {
        in_job = 0;
      }
      in_job && /cargo-cache: "false"/ {
        found = 1;
      }
      END {
        if (found) {
          print "present";
        }
      }
    ' .github/workflows/pr.yml
  )"
  if [[ "${cache_opt_out}" != "present" ]]; then
    report_failure "${title}"
    printf '.github/workflows/pr.yml: %s job must pass cargo-cache: "false" to setup-revaer\n\n' "${job_name}" >&2
    failures=1
  fi
}

report_pr_cache_opt_out "Instruction drift must skip the shared Cargo cache restore" instruction-drift
report_pr_cache_opt_out "Formatting must skip the shared Cargo cache restore" fmt
report_pr_cache_opt_out "Lint must skip the shared Cargo cache restore" lint
report_pr_cache_opt_out "Run Checks must skip the shared Cargo cache restore" check
report_pr_cache_opt_out "Helm lint must skip the shared Cargo cache restore" helm-lint
report_pr_cache_opt_out "Media conversion fixtures must skip the shared Cargo cache restore" media-conversion
report_pr_cache_opt_out "Supply-chain checks must skip the shared Cargo cache restore" supply-chain

report_pr_no_apt_profile() {
  local title="$1"
  local job_name="$2"
  local apt_profile

  apt_profile="$(
    awk -v job_header="  ${job_name}:" '
      $0 == job_header {
        in_job = 1;
        next;
      }
      in_job && /^  [^[:space:]][^:]*:/ {
        in_job = 0;
      }
      in_job && /apt-profile:/ {
        found = 1;
      }
      END {
        if (found) {
          print "present";
        }
      }
    ' .github/workflows/pr.yml
  )"
  if [[ "${apt_profile}" == "present" ]]; then
    report_failure "${title}"
    printf '.github/workflows/pr.yml: %s job must not set apt-profile for setup-revaer\n\n' "${job_name}" >&2
    failures=1
  fi
}

report_pr_no_apt_profile "Lint must not install the native apt profile" lint
report_pr_no_apt_profile "Run Checks must not install the native apt profile" check
report_pr_no_apt_profile "Helm lint must not install the native apt profile" helm-lint
report_pr_no_apt_profile "Supply-chain checks must not install the native apt profile" supply-chain

sonar_artifact_directory_matches="$(
  awk '
    /^[[:space:]]*-[[:space:]]*name:/ {
      in_upload_artifact = 0;
      in_sonar_artifact = 0;
    }
    /uses:[[:space:]]*actions\/upload-artifact@/ {
      in_upload_artifact = 1;
      next;
    }
    in_upload_artifact && /name:[[:space:]]*sonar-coverage[[:space:]]*$/ {
      in_sonar_artifact = 1;
      next;
    }
    in_upload_artifact && in_sonar_artifact && /^[[:space:]]*path:[[:space:]]*coverage[[:space:]]*$/ {
      printf "%s:%d:%s\n", FILENAME, FNR, $0;
    }
    in_upload_artifact && in_sonar_artifact && /^[[:space:]]*if-no-files-found:/ {
      in_upload_artifact = 0;
      in_sonar_artifact = 0;
    }
  ' .github/workflows/pr.yml .github/workflows/sonar.yml
)"
report_matches "Sonar coverage artifacts must not upload the entire coverage directory" "${sonar_artifact_directory_matches}"

report_property_empty "Sonar source exclusions must be explicitly empty" 'sonar.exclusions' sonar-project.properties
report_property_empty "Sonar source inclusions must be explicitly empty" 'sonar.inclusions' sonar-project.properties
report_property_empty "Sonar test exclusions must be explicitly empty" 'sonar.test.exclusions' sonar-project.properties
report_property_empty "Sonar test inclusions must be explicitly empty" 'sonar.test.inclusions' sonar-project.properties
report_property_empty "Sonar coverage exclusions must be explicitly empty" 'sonar.coverage.exclusions' sonar-project.properties
report_property_empty "Sonar duplication exclusions must be explicitly empty" 'sonar.cpd.exclusions' sonar-project.properties
report_property_empty "Sonar issue-ignore criteria must be explicitly empty" 'sonar.issue.ignore.multicriteria' sonar-project.properties
report_property_empty "Sonar all-file issue ignores must be explicitly empty" 'sonar.issue.ignore.allfile' sonar-project.properties
report_property_empty "Sonar block issue ignores must be explicitly empty" 'sonar.issue.ignore.block' sonar-project.properties
report_property_empty "Sonar rule-scope restrictions must be explicitly empty" 'sonar.issue.enforce.multicriteria' sonar-project.properties
report_property_empty "Sonar JavaScript analyzer exclusions must be explicitly empty" 'sonar.javascript.exclusions' sonar-project.properties
report_property_empty "Sonar SCA exclusions must be explicitly empty" 'sonar.sca.exclusions' sonar-project.properties

if [[ "${failures}" -ne 0 ]]; then
  exit 1
fi
