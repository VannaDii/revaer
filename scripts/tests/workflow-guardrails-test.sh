#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
fixture_root="${repo_root}/scripts/tests/fixtures/workflow-guardrails"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/revaer-workflow-guardrails.XXXXXX")"
trap 'rm -rf "${test_root}"' EXIT

run_case() {
  local expectation="$1"
  local fixture="$2"
  local sonar_variant="${3:-valid}"
  local workflow_name="${4:-test.yml}"
  local case_root="${test_root}/$(basename "${fixture}" .yml)-${sonar_variant}"
  mkdir -p "${case_root}/workflows" "${case_root}/actions" "${case_root}/just"
  cp "${fixture_root}/${fixture}" "${case_root}/workflows/${workflow_name}"
  cp "${repo_root}/sonar-project.properties" "${case_root}/sonar-project.properties"
  cp "${repo_root}/justfile" "${case_root}/justfile"
  cp "${repo_root}/just/quality.just" "${case_root}/just/quality.just"

  case "${sonar_variant}" in
    valid) ;;
    leading-whitespace)
      printf '\n sonar.coverage.exclusions=**/*\n' >> "${case_root}/sonar-project.properties"
      ;;
    escaped-key)
      printf '\nsonar\\.coverage\\.exclusions=**/*\n' >> "${case_root}/sonar-project.properties"
      ;;
    continuation)
      printf '\nsonar.coverage.exclusions=\\\n  **/*\n' >> "${case_root}/sonar-project.properties"
      ;;
    duplicate-logical-key)
      printf '\nsonar\\u002ecoverage.exclusions=\n' >> "${case_root}/sonar-project.properties"
      ;;
    nonempty-filter)
      ruby -pi -e 'sub("sonar.coverage.exclusions=", "sonar.coverage.exclusions=**/*")' \
        "${case_root}/sonar-project.properties"
      ;;
    disabled-analyzer)
      ruby -pi -e 'sub("sonar.yaml.activate=true", "sonar.yaml.activate=false")' \
        "${case_root}/sonar-project.properties"
      ;;
    enabled-skip)
      ruby -pi -e 'sub("sonar.scm.disabled=false", "sonar.scm.disabled=true")' \
        "${case_root}/sonar-project.properties"
      ;;
    unknown-property)
      printf '\nsonar.unreviewed.option=true\n' >> "${case_root}/sonar-project.properties"
      ;;
    missing-property)
      ruby -ni -e 'print unless $_.start_with?("sonar.projectKey=")' \
        "${case_root}/sonar-project.properties"
      ;;
    invalid-unicode-value)
      ruby -pi -e 'sub(/^sonar.projectKey=.*$/, "sonar.projectKey=\\\\uZZZZ")' \
        "${case_root}/sonar-project.properties"
      ;;
    *)
      printf 'Unknown Sonar fixture variant: %s\n' "${sonar_variant}" >&2
      exit 1
      ;;
  esac

  command=(
    ruby "${repo_root}/scripts/workflow-structure-guardrails.rb"
    --workflows "${case_root}/workflows"
    --actions "${case_root}/actions"
    --sonar "${case_root}/sonar-project.properties"
    --justfile "${case_root}/justfile"
  )
  if "${command[@]}" >"${case_root}/stdout" 2>"${case_root}/stderr"; then
    if [[ "${expectation}" != pass ]]; then
      printf 'Expected guardrail failure for %s (%s)\n' "${fixture}" "${sonar_variant}" >&2
      exit 1
    fi
  elif [[ "${expectation}" = pass ]]; then
    printf 'Expected guardrail success for %s (%s)\n' "${fixture}" "${sonar_variant}" >&2
    cat "${case_root}/stderr" >&2
    exit 1
  fi
}

run_image_case() {
  local expectation="$1"
  local mutation="$2"
  local case_root="${test_root}/build-images-${mutation}"
  mkdir -p "${case_root}/workflows" "${case_root}/actions" "${case_root}/just"
  cp "${repo_root}/.github/workflows/build-images.yml" "${case_root}/workflows/build-images.yml"
  cp "${repo_root}/sonar-project.properties" "${case_root}/sonar-project.properties"
  cp "${repo_root}/justfile" "${case_root}/justfile"
  cp "${repo_root}/just/quality.just" "${case_root}/just/quality.just"
  case "${mutation}" in
    valid) ;;
    mutable-tag)
      ruby -pi -e 'gsub("steps.build_image.outputs.image_reference", "steps.build_image.outputs.image_tag")' \
        "${case_root}/workflows/build-images.yml"
      ;;
    conditional-sarif)
      ruby -pi -e "gsub(\"always() && hashFiles('trivy-results.sarif') != ''\", \"success()\")" \
        "${case_root}/workflows/build-images.yml"
      ;;
    missing-build-steps)
      printf 'jobs:\n  build: {}\n' > "${case_root}/workflows/build-images.yml"
      ;;
    reordered-steps)
      ruby -pi -e 'gsub("Build & Push Image", "Scan digest-qualified image").gsub("Inventory digest-qualified image", "Build & Push Image").sub("Scan digest-qualified image", "Inventory digest-qualified image")' \
        "${case_root}/workflows/build-images.yml"
      ;;
    missing-build-digest)
      ruby -pi -e 'sub("--metadata-file", "--metadata-output")' \
        "${case_root}/workflows/build-images.yml"
      ;;
    weak-scan-severity)
      ruby -pi -e 'sub("severity: HIGH,CRITICAL", "severity: LOW")' \
        "${case_root}/workflows/build-images.yml"
      ;;
    *)
      printf 'Unknown image workflow mutation: %s\n' "${mutation}" >&2
      exit 1
      ;;
  esac

  command=(
    ruby "${repo_root}/scripts/workflow-structure-guardrails.rb"
    --workflows "${case_root}/workflows"
    --actions "${case_root}/actions"
    --sonar "${case_root}/sonar-project.properties"
    --justfile "${case_root}/justfile"
  )
  if "${command[@]}" >"${case_root}/stdout" 2>"${case_root}/stderr"; then
    if [[ "${expectation}" != pass ]]; then
      printf 'Expected image workflow guardrail failure for %s\n' "${mutation}" >&2
      exit 1
    fi
  elif [[ "${expectation}" = pass ]]; then
    printf 'Expected image workflow guardrail success for %s\n' "${mutation}" >&2
    cat "${case_root}/stderr" >&2
    exit 1
  fi
}

run_case pass valid.yml
run_case pass decoys.yml
run_case pass postgres-credentials.yml
run_case fail postgres-credential-drift.yml
run_case fail postgres-missing-environment.yml
run_case fail postgres-incomplete-service.yml
run_case fail postgres-admin-credential-drift.yml
run_case fail unpinned-action.yml
run_case fail input-in-run.yml
run_case fail unknown-just.yml
run_case fail invalid-permissions.yml
run_case fail invalid-condition.yml
run_case fail invalid-step.yml
run_case fail duplicate-key.yml
run_case fail root-sequence.yml
run_case fail empty-jobs.yml
run_case fail invalid-job.yml
run_case fail direct-cargo.yml
run_case fail alias.yml
run_case fail invalid-yaml.yml
run_case fail mapping-key.yml
run_case fail reusable-steps.yml
run_case fail empty-steps.yml
run_case fail invalid-step-node.yml
run_case fail empty-run.yml
run_case fail empty-permissions.yml
run_case fail valid.yml leading-whitespace
run_case fail valid.yml escaped-key
run_case fail valid.yml continuation
run_case fail valid.yml duplicate-logical-key
run_case fail valid.yml nonempty-filter
run_case fail valid.yml disabled-analyzer
run_case fail valid.yml enabled-skip
run_case fail valid.yml unknown-property
run_case fail valid.yml missing-property
run_case fail valid.yml invalid-unicode-value
run_image_case pass valid
run_image_case fail mutable-tag
run_image_case fail conditional-sarif
run_image_case fail missing-build-steps
run_image_case fail reordered-steps
run_image_case fail missing-build-digest
run_image_case fail weak-scan-severity
run_case fail pr-sonar-missing-scope.yml valid pr.yml

if ruby "${repo_root}/scripts/workflow-structure-guardrails.rb" \
  >"${test_root}/missing-options.stdout" 2>"${test_root}/missing-options.stderr"; then
  printf 'Expected missing guardrail options to fail\n' >&2
  exit 1
fi

printf 'Workflow guardrail regression tests passed\n'
