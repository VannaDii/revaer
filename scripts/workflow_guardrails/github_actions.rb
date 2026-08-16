# frozen_string_literal: true

require "workflow_guardrails/input_loader"

module WorkflowGuardrails
  class GithubActions
    EXTERNAL_ACTION = %r{\A[^./][^/]*/[^@]+@[0-9a-f]{40}\z}
    DIGEST_PINNED_DOCKER_ACTION = %r{\Adocker://[^\s@]+@sha256:[0-9a-f]{64}\z}
    SONAR_ACTION_PREFIX = "SonarSource/sonarqube-scan-action@"
    GATE_EXECUTOR_ACTION_PREFIXES = %w[
      aquasecurity/trivy-action@
      docker/build-push-action@
    ].freeze
    MAX_JOB_TIMEOUT = 180
    SETUP_TIMEOUT = "20"

    def initialize(inputs, diagnostics)
      @inputs = inputs
      @diagnostics = diagnostics
    end

    def validate
      validate_documents
      validate_sonar_job("pr.yml", @inputs.document(".github/workflows/pr.yml"), pull_request: true)
      validate_sonar_job("sonar.yml", @inputs.document(".github/workflows/sonar.yml"), pull_request: false)
      validate_setup_action
      validate_node_contract
      validate_timeouts
    end

    private

    def validate_documents
      @inputs.documents.each do |path, document|
        if path.start_with?(@inputs.workflow_root)
          validate_workflow(path, document)
        else
          validate_action(path, document)
        end
      end
    end

    def validate_workflow(path, document)
      jobs = document["jobs"]
      unless jobs.is_a?(Hash) && !jobs.empty?
        error("#{path}: jobs must be a non-empty mapping")
        return
      end
      jobs.each do |job_name, job|
        unless job.is_a?(Hash)
          error("#{path}: job #{job_name} must be a mapping")
          next
        end
        if concealed_failure?(job["continue-on-error"])
          error("#{path}: job #{job_name} must not conceal failure with continue-on-error")
        end
        if job.key?("uses")
          validate_uses(path, "job #{job_name}", job["uses"])
        else
          validate_steps(path, "job #{job_name}", job["steps"])
        end
      end
    end

    def validate_action(path, document)
      runs = document["runs"]
      return unless runs.is_a?(Hash) && runs["using"] == "composite"

      validate_steps(path, "composite action", runs["steps"])
    end

    def validate_steps(path, owner, steps)
      unless steps.is_a?(Array) && !steps.empty?
        error("#{path}: #{owner} steps must be a non-empty sequence")
        return
      end
      named_steps = {}
      steps.each_with_index do |step, index|
        unless step.is_a?(Hash)
          error("#{path}: #{owner} step #{index + 1} must be a mapping")
          next
        end
        if concealed_failure?(step["continue-on-error"])
          error("#{path}: #{owner} step #{index + 1} must not conceal failure with continue-on-error")
        end
        name = step["name"]
        if name.is_a?(String) && !name.empty?
          if named_steps.key?(name)
            error("#{path}: #{owner} has duplicate nonempty step name #{name.inspect}")
          else
            named_steps[name] = index + 1
          end
        end
        command_keys = %w[uses run].select { |key| step.key?(key) }
        unless command_keys.length == 1
          error("#{path}: #{owner} step #{index + 1} must define exactly one of uses or run")
        end
        validate_uses(path, "#{owner} step #{index + 1}", step["uses"]) if step.key?("uses")
        validate_run(path, "#{owner} step #{index + 1}", step["run"]) if step.key?("run")
      end
    end

    def validate_uses(path, owner, uses)
      docker_action = uses.is_a?(String) && uses.start_with?("docker://")
      valid = uses.is_a?(String) && (
        uses.start_with?("./") || uses.match?(EXTERNAL_ACTION) || uses.match?(DIGEST_PINNED_DOCKER_ACTION)
      )
      error("#{path}: #{owner} external action must use a full lowercase 40-character SHA") unless valid
      if docker_action && !uses.match?(DIGEST_PINNED_DOCKER_ACTION)
        error("#{path}: #{owner} docker action must use an exact sha256 digest")
      end
      if uses.to_s.start_with?(SONAR_ACTION_PREFIX)
        error("#{path}: #{owner} must not use the official Sonar action as a scanner executor")
      end
      if GATE_EXECUTOR_ACTION_PREFIXES.any? { |prefix| uses.to_s.start_with?(prefix) }
        error("#{path}: #{owner} must run release and security gates through just")
      end
    end

    def validate_run(path, owner, run)
      unless run.is_a?(String) && !run.empty?
        error("#{path}: #{owner} run must be a non-empty scalar")
        return
      end
      if run.match?(/\$\{\{\s*inputs\./)
        error("#{path}: #{owner} must not interpolate inputs directly in run")
      end
      run.scan(/(?:\A|[\n;&|()]\s*)just\s+([a-zA-Z0-9_-]+)/).flatten.each do |recipe|
        error("#{path}: #{owner} calls unknown just recipe #{recipe.inspect}") unless @inputs.recipes.include?(recipe)
      end
      if run.match?(/(?:\A|[\n;&|()]\s*)cargo\s+(?:build|check|clippy|fmt|test)\b/)
        error("#{path}: #{owner} must run Rust quality gates through just")
      end
      direct_gate_patterns.each do |pattern, label|
        if run.match?(pattern)
          error("#{path}: #{owner} must run #{label} through just")
        end
      end
      if run.match?(/-Dsonar\./)
        error("#{path}: #{owner} overrides Sonar properties outside sonar-project.properties")
      end
    end

    def direct_gate_patterns
      {
        /(?:\A|[\n;&|()]\s*)docker\s+buildx\s+build\b/ => "container builds",
        /(?:\A|[\n;&|()]\s*)docker\s+buildx\s+imagetools\s+create\b/ => "manifest publication",
        /(?:\A|[\n;&|()]\s*)docker\s+(?:build|push|manifest)\b/ => "container publication",
        /(?:\A|[\n;&|()]\s*)trivy\s+image\b/ => "vulnerability scans",
        /(?:\A|[\n;&|()]\s*)cosign\s+sign\b/ => "artifact signing",
        /(?:\A|[\n;&|()]\s*)helm\s+(?:package|push)\b/ => "Helm release operations"
      }
    end

    def validate_sonar_job(label, workflow, pull_request:)
      unless workflow
        error("#{label}: workflow could not be loaded")
        return
      end
      job_key = pull_request ? "coverage" : "sonar"
      job = workflow.fetch("jobs").fetch(job_key)
      steps = job.fetch("steps")
      checkout = steps.find { |step| step.is_a?(Hash) && step["uses"].to_s.start_with?("actions/checkout@") }
      expected_ref = pull_request ? "${{ github.event.pull_request.head.sha }}" : "${{ github.sha }}"
      unless checkout&.dig("with", "fetch-depth") == "0" && checkout&.dig("with", "ref") == expected_ref
        error("#{label}: Sonar checkout must use full history and the exact head SHA")
      end
      setup = steps.find { |step| step.is_a?(Hash) && step["uses"] == "./.github/actions/setup-revaer" }
      unless setup&.dig("with", "apt-profile") == "coverage" &&
          setup&.dig("with", "node-version") == "${{ env.NODE_VERSION }}" &&
          setup&.dig("with", "sonar-scanner") == "true"
        error("#{label}: Sonar setup must enable coverage, pinned Node, and analyzer caching")
      end

      named = named_steps(label, job)
      validate_scanner_invocations(label, job)
      required_commands.each do |name, command|
        error("#{label}: #{name} must run #{command}") unless named.dig(name, "run") == command
      end
      scan_step = named["SonarQube scan"]
      unless scan_step&.fetch("env", nil) == {
        "SONAR_TOKEN" => "${{ env.SONAR_AUTH_TOKEN }}"
      } && !scan_step.key?("if")
        error("#{label}: authoritative scanner credentials or unconditional execution drifted")
      end
      unless job.dig("env", "REVAER_REQUIRE_NATIVE_COVERAGE") == "1"
        error("#{label}: hosted Linux must require authored native coverage")
      end
      validate_scm_environment(label, named["Prepare exact Sonar SCM context"], pull_request)
      validate_result_environment(label, named["Verify Sonar published result"], pull_request)
      validate_evidence_paths(label, named)
    rescue KeyError, NoMethodError => e
      error("#{label}: incomplete Sonar workflow structure: #{e.message}")
    end

    def required_commands
      {
        "Coverage" => "just cov",
        "Run Playwright with JavaScript coverage" => "just ui-e2e",
        "Release JavaScript coverage" => "just js-release-coverage",
        "Merge JavaScript coverage inputs" => "just js-coverage-merge",
        "Build native compile database" => "just sonar-compile-db",
        "Verify Sonar analysis inputs" => "just sonar-verify-inputs",
        "Remove untracked dependency installs before analysis" => "just sonar-prepare-sources",
        "Prepare exact Sonar SCM context" => "just sonar-prepare-scm",
        "SonarQube scan" => "just sonar-scan",
        "Package Sonar analysis evidence" => "just sonar-package-report",
        "Verify Sonar published result" => "just sonar-verify-result"
      }
    end

    def validate_scanner_invocations(label, job)
      steps = Array(job["steps"]).select { |step| step.is_a?(Hash) }
      action_scans = steps.select do |step|
        step["uses"].to_s.start_with?(SONAR_ACTION_PREFIX)
      end
      recipe_scans = steps.select { |step| step["run"] == "just sonar-scan" }
      error("#{label}: the official Sonar action must not execute the scanner") unless action_scans.empty?
      unless recipe_scans.length == 1
        error("#{label}: exactly one scanner invocation through just sonar-scan is required")
      end
      steps.each do |step|
        next unless step["run"].to_s.match?(/(?:\A|[\s;&|])sonar-scanner(?:\s|\z)/)

        error("#{label}: workflows must not invoke sonar-scanner directly")
      end
    end

    def validate_scm_environment(label, step, pull_request)
      expected = if pull_request
                   {
                     "SONAR_BASE_SHA" => "${{ github.event.pull_request.base.sha }}",
                     "SONAR_BASE_REF" => "${{ github.event.pull_request.base.ref }}",
                     "SONAR_HEAD_SHA" => "${{ github.event.pull_request.head.sha }}"
                   }
                 else
                   {
                     "SONAR_BASE_SHA" => "${{ github.event.before }}",
                     "SONAR_BASE_REF" => "main",
                     "SONAR_HEAD_SHA" => "${{ github.sha }}"
                   }
                 end
      error("#{label}: exact Sonar SCM event inputs drifted") unless step&.fetch("env", nil) == expected
    end

    def validate_result_environment(label, step, pull_request)
      environment = step&.fetch("env", nil) || {}
      unless environment["SONAR_PROJECT_KEY"] == "VannaDii_Revaer"
        error("#{label}: Sonar result verification project key drifted")
      end
      if pull_request
        unless environment["SONAR_PULL_REQUEST"] == "${{ github.event.pull_request.number }}"
          error("#{label}: PR result verification must set SONAR_PULL_REQUEST")
        end
      elsif environment.key?("SONAR_PULL_REQUEST")
        error("#{label}: main result verification must query the complete project backlog")
      end
    end

    def validate_evidence_paths(label, named)
      coverage_paths = named.dig("Upload coverage artifact", "with", "path").to_s
      %w[
        coverage/lcov.info coverage/llvm-cov.txt coverage/js-lcov.info
        coverage/script-coverage.xml coverage/compile_commands.json
        coverage/cxxbridge/include/rust/cxx.h
        coverage/cxxbridge/include/revaer-torrent-libt/src/ffi/bridge.rs.h
      ].each do |path|
        error("#{label}: coverage artifact omits #{path}") unless coverage_paths.lines.map(&:strip).include?(path)
      end
      evidence_paths = named.dig("Upload Sonar analysis evidence", "with", "path").to_s
      %w[
        artifacts/sonar/scanner.log artifacts/sonar/scm-evidence.txt .scannerwork/report-task.txt
        .scannerwork/scanner-report.tar.xz artifacts/sonar/api/*.json coverage/llvm-cov.txt
      ].each do |path|
        error("#{label}: Sonar evidence artifact omits #{path}") unless evidence_paths.lines.map(&:strip).include?(path)
      end
    end

    def validate_setup_action
      setup_path = @inputs.path(".github/actions/setup-revaer/action.yml")
      setup_text = File.read(setup_path, encoding: "UTF-8")
      installer_text = File.read(@inputs.path("scripts/install-sonar-scanner.sh"), encoding: "UTF-8")
      quality_text = File.read(@inputs.path("just/quality.just"), encoding: "UTF-8")
      scanner_text = File.read(@inputs.path("scripts/sonar-scan.sh"), encoding: "UTF-8")
      setup_contract = setup_text + installer_text
      %w[
        clang-19 coverage-toolchain-env.sh kcov sonar-analyzers install-sonar-scanner.sh
        config/sonarsource-public-key.asc
        8.1.0.6389 679F1EE92B19609DE816FDE81DB198F93525EC1A
        bb8f709f9cb73352f8d1260a3b3c506c0f41146754bc630762c126d795499d0b
        5e1c9328f4e261838de778c9e586ee608cca45ff7f0538108642219214628ba5
        8afc8bbff9008434e53b31cb681333ff643b999f84ca537db573d0fae8883cdc
        20d12be4081896b337cd873d98ebd3d554be666086a45e31dd84a12ef51c3688
        a39874f938ce13f7a65f253120d1ec946b349ffe
        dac01569171979477b500924be264d2a1bc649dae6010536228cbb319344d516
      ].each do |needle|
        error("setup-revaer coverage profile is missing #{needle}") unless setup_contract.include?(needle)
      end
      unless quality_text.scan(/^sonar-scan:\n    bash scripts\/sonar-scan\.sh$/).length == 1
        error("just sonar-scan must have exactly one scanner-wrapper owner")
      end
      unless scanner_text.scan(/^"\$\{scanner_command\}" "\$@" 2>&1 \| tee "\$\{scanner_log\}"$/).length == 1
        error("scanner wrapper must contain exactly one scanner process invocation")
      end
    rescue Errno::ENOENT => e
      error(e.message)
    end

    def validate_timeouts
      @inputs.documents.each do |path, workflow|
        next unless path.start_with?(@inputs.workflow_root)

        workflow.fetch("jobs", {}).each do |job_name, job|
          next unless job.is_a?(Hash)
          next if job.key?("uses")

          label = "#{File.basename(path)} job #{job_name}"
          validate_job_timeout(label, job)
          validate_setup_step_timeouts(label, job)
        end
      end
    end

    def validate_node_contract
      nvmrc = File.read(@inputs.path(".nvmrc"), encoding: "UTF-8").strip
      wrapper = File.read(@inputs.path("scripts/with-node.sh"), encoding: "UTF-8")
      error(".nvmrc must pin exact Node 24.19.0") unless nvmrc == "24.19.0"
      %w[required_node_version active_node_version REVAER_NODE_VERSION].each do |needle|
        error("NVM-aware Node wrapper is missing #{needle}") unless wrapper.include?(needle)
      end
      %w[pr.yml sonar.yml ci.yml].each do |name|
        workflow = @inputs.document(".github/workflows/#{name}")
        error("#{name}: NODE_VERSION must equal 24.19.0") unless workflow&.dig("env", "NODE_VERSION") == "24.19.0"
      end
    rescue Errno::ENOENT => e
      error(e.message)
    end

    def validate_job_timeout(label, job)
      value = job["timeout-minutes"]
      unless value.to_s.match?(/\A[1-9][0-9]*\z/) && value.to_i <= MAX_JOB_TIMEOUT
        error("#{label} must have a positive timeout-minutes no greater than #{MAX_JOB_TIMEOUT}")
      end
    end

    def validate_setup_step_timeouts(label, job)
      Array(job["steps"]).each do |step|
        next unless step.is_a?(Hash) && step["uses"] == "./.github/actions/setup-revaer"

        unless step["timeout-minutes"] == SETUP_TIMEOUT
          error("#{label} setup-revaer step must have timeout-minutes #{SETUP_TIMEOUT}")
        end
      end
    end

    def named_steps(label, job)
      named = {}
      Array(job["steps"]).each do |step|
        next unless step.is_a?(Hash)

        name = step["name"]
        next unless name.is_a?(String) && !name.empty?

        named[name] ||= step
      end
      named
    end

    def concealed_failure?(value)
      !value.nil? && value != false && value != "false"
    end

    def error(message)
      @diagnostics.add(message)
    end
  end
end
