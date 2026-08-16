#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

set +e
ruby -I "${repo_root}/scripts" - "${repo_root}" <<'RUBY'
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
status=$?
set -e

exit "${status}"
