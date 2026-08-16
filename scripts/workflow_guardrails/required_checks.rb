# frozen_string_literal: true

require "set"
require "workflow_guardrails/github_actions"
require "workflow_guardrails/input_loader"

module WorkflowGuardrails
  class RequiredChecks
    AUDITED_CONTEXTS = [
      "Check Lint",
      "Check Coverage",
      "Check Formatting",
      "Run Checks",
      "Native Integration Tests",
      "Feature Matrix",
      "UI E2E Coverage",
      "Images / Manifest",
      "Check Instruction Drift",
      "Helm Lint",
      "Images / Helm Chart",
      "Images / Build arm64",
      "Images / Build amd64",
      "Load Matrix",
      "UI E2E (shard 1/3)",
      "UI E2E (shard 2/3)",
      "UI E2E (shard 3/3)",
      "SonarCloud Code Analysis",
      "Media Conversion Fixtures",
      "Supply Chain Checks",
      "Build Release"
    ].freeze
    IMAGE_PUBLISH_CONDITION = "inputs.final_image_compliance_bundle != ''"
    IMAGE_VERIFY_CONDITION = "inputs.final_image_compliance_bundle == ''"
    IMAGE_REACHABILITY_CONDITION =
      "inputs.final_image_compliance_bundle != '' || github.event_name == 'pull_request'"

    def initialize(inputs, diagnostics)
      @inputs = inputs
      @diagnostics = diagnostics
    end

    def validate
      validate_audited_snapshot
      validate_pr_trigger
      validate_pr_dependency_graph
      validate_required_pr_job_conditions
      validate_udeps_contract
      validate_supply_chain_aggregate
      validate_ui_coverage_aggregate
      validate_image_workflow_contract
      emitted = emitted_contexts
      missing = @inputs.required_contexts.reject { |context| emitted.include?(context) }
      missing.each { |context| error("required PR context is not emitted: #{context}") }
    end

    def validate_pr_trigger
      workflow = @inputs.document(".github/workflows/pr.yml")
      trigger = workflow&.dig("on", "pull_request")
      return if trigger == "" || trigger == {}

      error("PR workflow must run for every pull_request without branch, path, or activity narrowing")
    end

    def validate_pr_dependency_graph
      jobs = pr_jobs
      jobs.each do |job_name, job|
        next unless job.is_a?(Hash)

        normalize_needs(job["needs"]).each do |dependency|
          error("PR job #{job_name} depends on missing job #{dependency}") unless jobs.key?(dependency)
        end
      end

      visiting = Set.new
      visited = Set.new
      jobs.each_key { |job_name| visit_job(job_name, jobs, visiting, visited) }
    end

    def visit_job(job_name, jobs, visiting, visited)
      return if visited.include?(job_name)
      if visiting.include?(job_name)
        error("PR workflow dependency cycle includes #{job_name}")
        return
      end

      visiting.add(job_name)
      normalize_needs(jobs.dig(job_name, "needs")).each do |dependency|
        visit_job(dependency, jobs, visiting, visited) if jobs.key?(dependency)
      end
      visiting.delete(job_name)
      visited.add(job_name)
    end

    def validate_required_pr_job_conditions
      pr_jobs.each do |job_name, job|
        next unless job.is_a?(Hash)

        name = job["name"]
        next unless required_direct_pr_context?(name)
        next if name == "Supply Chain Checks" && job["if"] == "always()"
        next unless job.key?("if")

        error("required PR job #{job_name} must not be conditionally skipped")
      end
    end

    def required_direct_pr_context?(name)
      return false unless name.is_a?(String)
      return true if @inputs.required_contexts.include?(name)

      name == "UI E2E (shard ${{ matrix.shard }}/3)"
    end

    def validate_supply_chain_aggregate
      job = pr_jobs["supply-chain"]
      unless job.is_a?(Hash) && normalize_needs(job["needs"]).sort == %w[audit deny udeps]
        error("Supply Chain Checks must depend on audit, deny, and udeps")
        return
      end
      unless job["if"] == "always()"
        error("Supply Chain Checks must run with always() and fail closed over dependency results")
      end
      step = named_step(job, "Verify supply chain results")
      expected_environment = {
        "AUDIT_RESULT" => "${{ needs.audit.result }}",
        "DENY_RESULT" => "${{ needs.deny.result }}",
        "UDEPS_RESULT" => "${{ needs.udeps.result }}"
      }
      unless step&.dig("run") == "just verify-supply-chain-results" &&
          step["env"] == expected_environment
        error("Supply Chain Checks must verify every dependency result through just")
      end
    end

    def validate_udeps_contract
      job = pr_jobs["udeps"]
      expected_environment = {
        "REVAER_UDEPS_VERSION" => "0.1.57",
        "REVAER_UDEPS_TOOLCHAIN" => "nightly-2026-06-13"
      }
      unless job.is_a?(Hash) && job["env"] == expected_environment
        error("cargo-udeps version and nightly toolchain must remain exact")
        return
      end
      cache = named_step(job, "Restore exact cargo-udeps binary")
      expected_cache = {
        "path" => "~/.cargo/bin/cargo-udeps",
        "key" => "${{ runner.os }}-cargo-udeps-${{ env.REVAER_UDEPS_VERSION }}-${{ env.REVAER_UDEPS_TOOLCHAIN }}"
      }
      unless cache&.dig("with") == expected_cache
        error("cargo-udeps exact binary cache contract drifted")
      end
      evidence = named_step(job, "Upload cargo-udeps toolchain evidence")
      unless evidence&.dig("if") == "always()" && evidence&.dig("with") == {
        "name" => "cargo-udeps-toolchain-evidence",
        "path" => "target/udeps-toolchain-evidence.txt",
        "if-no-files-found" => "error"
      }
        error("cargo-udeps toolchain evidence must be retained fail closed")
      end
    end

    def validate_ui_coverage_aggregate
      shard_job = pr_jobs["ui-e2e"]
      upload = named_step(shard_job, "Upload E2E coverage")
      unless upload&.dig("with", "if-no-files-found") == "error"
        error("every UI shard coverage upload must fail when coverage files are missing")
      end

      aggregate = pr_jobs["ui-e2e-coverage"]
      expected_needs = %w[coverage feature-matrix native-it ui-e2e]
      unless normalize_needs(aggregate&.fetch("needs", nil)).sort == expected_needs.sort
        error("UI E2E Coverage must wait for the complete UI matrix and prerequisite gates")
      end
      %w[1 2 3].each do |shard|
        step = named_step(aggregate, "Download E2E coverage shard #{shard}")
        unless step&.dig("with") == {
          "name" => "ui-e2e-coverage-shard-#{shard}",
          "path" => "tests/test-results"
        }
          error("UI E2E Coverage must download exact shard artifact #{shard}")
        end
      end
      verification = named_step(aggregate, "Verify all UI E2E shard coverage inputs")
      unless verification&.dig("run") == "just ui-e2e-shard-coverage"
        error("UI E2E Coverage must prove nonempty records from all three shards through just")
      end
    end

    def validate_image_workflow_contract
      validate_image_matrix
      validate_image_caller
      validate_reusable_image_jobs
      validate_image_gate_recipes
    end

    def validate_image_matrix
      expected = [
        {
          "name" => "amd64",
          "runner" => "ubuntu-latest",
          "platform" => "linux/amd64",
          "rust_target" => "x86_64-unknown-linux-musl",
          "arch_tag" => "amd64",
          "needs_qemu" => false
        },
        {
          "name" => "arm64",
          "runner" => "ubuntu-24.04-arm",
          "platform" => "linux/arm64",
          "rust_target" => "aarch64-unknown-linux-musl",
          "arch_tag" => "arm64",
          "needs_qemu" => false
        }
      ]
      unless @inputs.image_matrix["include"] == expected
        error("image matrix must define exactly the audited amd64 and arm64 jobs")
      end
    end

    def validate_image_caller
      caller = pr_jobs["build-pr-images"]
      expected_needs = %w[
        ui-e2e feature-matrix native-it media-conversion coverage supply-chain load-matrix
      ]
      unless caller.is_a?(Hash) && caller["uses"] == "./.github/workflows/build-images.yml"
        error("PR image contexts must come from the checked-in reusable image workflow")
        return
      end
      unless normalize_needs(caller["needs"]) == expected_needs
        error("PR images must wait for UI, feature, native, media, coverage, supply-chain, and matrix gates")
      end
      unless caller["if"] == "github.event.pull_request.head.repo.fork == false"
        error("PR images must be reachable on every same-repository pull request and nowhere else")
      end
      expected_inputs = {
        "matrix" => "${{ needs.load-matrix.outputs.matrix }}",
        "image_name" => "revaer",
        "version_tag" => "pr-${{ github.event.pull_request.number }}-${{ needs.load-matrix.outputs.short_sha }}",
        "alias_tag" => "pr-${{ github.event.pull_request.number }}",
        "include_sha_tag" => "false",
        "publish_dev_helm" => "true",
        "pr_number" => "${{ github.event.pull_request.number }}",
        "checkout_ref" => "${{ github.event.pull_request.head.sha }}"
      }
      unless caller["with"] == expected_inputs
        error("same-repository PR image caller inputs or dev Helm publication drifted")
      end
    end

    def validate_reusable_image_jobs
      workflow = @inputs.document(".github/workflows/build-images.yml")
      jobs = workflow&.fetch("jobs", {}) || {}
      compliance_input = workflow&.dig("on", "workflow_call", "inputs", "final_image_compliance_bundle")
      unless compliance_input == {
        "description" => "Final-image compliance bundle path required before publishing images.",
        "required" => "false",
        "type" => "string",
        "default" => ""
      }
        error("image publication must retain the optional final-image compliance authorization input")
      end

      build = jobs["build"]
      unless build&.dig("if") == IMAGE_REACHABILITY_CONDITION &&
          build&.dig("timeout-minutes") == "120" &&
          build&.dig("strategy", "fail-fast") == "false" &&
          build&.dig("strategy", "matrix") == "${{ fromJSON(inputs.matrix) }}"
        error("both image architectures must run with bounded time and without fail-fast cancellation")
      end

      publish_build = named_step(build, "Build & Push Image")
      unless publish_build&.dig("if") == IMAGE_PUBLISH_CONDITION &&
          publish_build&.dig("run") == "just image-build-push"
        error("authorized image publication must build and push through just")
      end
      verify_build = named_step(build, "Build Image (verification only)")
      unless verify_build&.dig("if") == IMAGE_VERIFY_CONDITION &&
          verify_build&.dig("run") == "just image-build-verify"
        error("PR verification must build each image architecture locally through just")
      end

      validate_publish_compliance_steps(build)
      validate_image_scan_steps(build)

      trivy = named_step(build, "Install Trivy")
      unless trivy&.dig("uses") == "aquasecurity/setup-trivy@81e514348e19b6112ce2a7e3ecbafe19c1e1f567" &&
          trivy&.dig("with", "version") == "v0.69.3"
        error("image jobs must install the exact reviewed Trivy version")
      end

      manifest = jobs["create-manifest"]
      unless normalize_needs(manifest&.fetch("needs", nil)) == ["build"] &&
          manifest&.dig("if") == IMAGE_REACHABILITY_CONDITION &&
          manifest&.dig("timeout-minutes") == "30"
        error("manifest handling must wait for both bounded image analyses")
      end
      validate_conditional_just_step(
        manifest, "Create and push multi-arch manifest", IMAGE_PUBLISH_CONDITION,
        "just image-manifest-create"
      )
      validate_conditional_just_step(
        manifest, "Verify multi-arch manifest inputs", IMAGE_VERIFY_CONDITION,
        "just image-manifest-verify"
      )
      validate_conditional_just_step(
        manifest, "Sign multi-arch tags", IMAGE_PUBLISH_CONDITION,
        "just image-manifest-sign"
      )

      helm = jobs["publish-dev-helm"]
      unless normalize_needs(helm&.fetch("needs", nil)) == ["create-manifest"] &&
          helm&.dig("if") == "inputs.publish_dev_helm && (#{IMAGE_REACHABILITY_CONDITION})" &&
          helm&.dig("timeout-minutes") == "30"
        error("dev Helm handling must remain bounded and wait for successful manifest handling")
      end
      validate_helm_release_paths(helm)
      validate_no_direct_image_gates(jobs)
    end

    def validate_image_gate_recipes
      script = File.read(@inputs.path("scripts/image-release.sh"), encoding: "UTF-8")
      recipes = File.read(@inputs.path("just/images.just"), encoding: "UTF-8")
      expected_recipes = {
        "image-build-push" => "build-push",
        "image-build-verify" => "build-verify",
        "image-inventory" => "inventory",
        "image-scan" => "scan",
        "image-sign-attest" => "sign-attest",
        "image-attestation-verify" => "verify-attestation",
        "image-manifest-create" => "create-manifest",
        "image-manifest-verify" => "verify-manifest",
        "image-manifest-sign" => "sign-manifest"
      }
      expected_recipes.each do |recipe, operation|
        expected = "#{recipe}:\n    bash scripts/image-release.sh #{operation}"
        error("image release recipe #{recipe} must delegate to operation #{operation}") unless recipes.include?(expected)
      end
      [
        "docker buildx build", "--metadata-file", "docker buildx imagetools inspect",
        "image_reference=%s@%s", "--push", "--load", "trivy image",
        "--format spdx-json", "--format sarif", "--exit-code 0", "cosign sign",
        "cosign attest", "cosign verify-attestation", "docker buildx imagetools create"
      ].each do |needle|
        error("image release implementation is missing #{needle}") unless script.include?(needle)
      end
    rescue Errno::ENOENT => e
      error(e.message)
    end

    def validate_publish_compliance_steps(build)
      built_ref = "${{ steps.build_image.outputs.image_reference }}"
      login = named_step(build, "Log in to GHCR")
      error("GHCR login must require compliance authorization") unless login&.dig("if") == IMAGE_PUBLISH_CONDITION

      supplied = named_step(build, "Validate supplied compliance authorization against built digest")
      unless supplied&.dig("if") == IMAGE_PUBLISH_CONDITION &&
          supplied&.dig("env", "IMAGE_REFERENCE") == built_ref &&
          supplied&.dig("run").to_s.include?("just image-compliance-validate")
        error("supplied compliance authorization must be validated against the built digest")
      end

      {
        "Generate digest-bound compliance evidence" => "just image-compliance-generate",
        "Validate digest-bound compliance evidence" => "just image-compliance-validate",
        "Sign image and compliance evidence" => "just image-sign-attest",
        "Verify signed compliance evidence" => "just image-attestation-verify"
      }.each do |name, command|
        step = named_step(build, name)
        unless step&.dig("if") == IMAGE_PUBLISH_CONDITION &&
            step&.dig("env", "IMAGE_REFERENCE") == built_ref &&
            step&.dig("run").to_s.include?(command)
          error("#{name} must consume the built digest through #{command} only when authorized")
        end
      end
      upload = named_step(build, "Upload compliance evidence")
      unless upload&.dig("if") == IMAGE_PUBLISH_CONDITION &&
          upload&.dig("with", "if-no-files-found") == "error"
        error("authorized image compliance evidence must be retained fail closed")
      end
    end

    def validate_image_scan_steps(build)
      built_ref = "${{ steps.build_image.outputs.image_reference }}"
      inventory = named_step(build, "Inventory digest-qualified image")
      unless inventory&.dig("if") == IMAGE_PUBLISH_CONDITION &&
          inventory&.dig("run") == "just image-inventory" &&
          inventory&.dig("env", "IMAGE_REFERENCE") == built_ref &&
          inventory&.dig("env", "IMAGE_SOURCE") == "remote"
        error("authorized inventory must inspect the immutable pushed image through just")
      end
      published_scan = named_step(build, "Scan digest-qualified image")
      unless published_scan&.dig("if") == IMAGE_PUBLISH_CONDITION &&
          published_scan&.dig("run") == "just image-scan" &&
          published_scan&.dig("env", "IMAGE_REFERENCE") == built_ref &&
          published_scan&.dig("env", "IMAGE_SOURCE") == "remote"
        error("authorized scan must inspect the immutable pushed image through just")
      end
      verification_scan = named_step(build, "Scan verification image")
      unless verification_scan&.dig("if") == IMAGE_VERIFY_CONDITION &&
          verification_scan&.dig("run") == "just image-scan" &&
          verification_scan&.dig("env", "IMAGE_REFERENCE") == "${{ steps.verify_image.outputs.image_tag }}" &&
          verification_scan&.dig("env", "IMAGE_SOURCE") == "docker"
        error("PR verification scan must inspect the locally loaded architecture image through just")
      end

      verifier = named_step(build, "Verify HIGH and CRITICAL findings")
      upload = named_step(build, "Upload scan results")
      scan_positions = [
        step_index(build, "Inventory digest-qualified image"),
        step_index(build, "Scan digest-qualified image"),
        step_index(build, "Scan verification image"),
        step_index(build, "Verify HIGH and CRITICAL findings"),
        step_index(build, "Upload scan results")
      ]
      unless scan_positions.none?(&:nil?) && scan_positions == scan_positions.sort &&
          verifier&.dig("run") == "just trivy-sarif-verify trivy-results.sarif" &&
          upload&.dig("if") == "always() && hashFiles('trivy-results.sarif') != ''"
        error("each architecture must retain SARIF after digest-qualified or local fail-closed scanning")
      end
    end

    def validate_helm_release_paths(helm)
      validate_step_condition(helm, "Package Helm chart", IMAGE_PUBLISH_CONDITION)
      validate_step_condition(helm, "Package Helm chart (verification only)", IMAGE_VERIFY_CONDITION)
      validate_step_condition(helm, "Publish dev Helm chart", IMAGE_PUBLISH_CONDITION)
      validate_step_condition(helm, "Verify packaged Helm chart without publishing", IMAGE_VERIFY_CONDITION)
    end

    def validate_no_direct_image_gates(jobs)
      forbidden = /(?:docker\s+buildx\s+(?:build|imagetools\s+create)|trivy\s+image|cosign\s+(?:sign|attest))/
      jobs.each do |job_name, job|
        Array(job.is_a?(Hash) ? job["steps"] : nil).each do |step|
          run = step.is_a?(Hash) ? step["run"] : nil
          next unless run.is_a?(String) && run.match?(forbidden)

          error("image workflow job #{job_name} contains a direct build, scan, manifest, or signing gate")
        end
      end
    end

    def validate_conditional_just_step(job, name, condition, command)
      step = named_step(job, name)
      return if step&.dig("if") == condition && step&.dig("run") == command

      error("#{name} must run #{command} only when #{condition}")
    end

    def validate_step_condition(job, name, condition)
      step = named_step(job, name)
      error("#{name} must remain gated by #{condition}") unless step&.dig("if") == condition
    end

    def named_step(job, name)
      Array(job.is_a?(Hash) ? job["steps"] : nil).find do |step|
        step.is_a?(Hash) && step["name"] == name
      end
    end

    def step_index(job, name)
      Array(job.is_a?(Hash) ? job["steps"] : nil).index do |step|
        step.is_a?(Hash) && step["name"] == name
      end
    end

    def pr_jobs
      @inputs.document(".github/workflows/pr.yml")&.fetch("jobs", {}) || {}
    end

    def normalize_needs(needs)
      case needs
      when Array then needs
      when String then [needs]
      else []
      end
    end

    private

    def validate_audited_snapshot
      contexts = @inputs.required_contexts
      duplicates = contexts.group_by(&:itself).select { |_context, values| values.length > 1 }.keys
      duplicates.sort.each { |context| error("required PR context is duplicated: #{context}") }
      return if contexts == AUDITED_CONTEXTS

      error(
        "config/required-pr-checks.txt must exactly match the operator-supplied audited context snapshot"
      )
    end

    def emitted_contexts
      contexts = Set.new
      add_pr_job_contexts(contexts)
      add_image_contexts(contexts)
      contexts.add("SonarCloud Code Analysis") if sonar_analysis_emitted?
      contexts
    end

    def add_pr_job_contexts(contexts)
      workflow = @inputs.document(".github/workflows/pr.yml")
      return unless workflow

      workflow.fetch("jobs", {}).each_value do |job|
        next unless job.is_a?(Hash)
        next if job.key?("uses")

        name = job["name"]
        next unless name.is_a?(String) && !name.empty?

        if name == "UI E2E (shard ${{ matrix.shard }}/3)"
          shards = job.dig("strategy", "matrix", "shard")
          unless shards == %w[1 2 3]
            error("PR UI E2E matrix must emit shards 1, 2, and 3")
            next
          end
          shards.each { |shard| contexts.add("UI E2E (shard #{shard}/3)") }
        else
          contexts.add(name)
        end
      end
    end

    def add_image_contexts(contexts)
      workflow = @inputs.document(".github/workflows/build-images.yml")
      return unless workflow

      workflow.fetch("jobs", {}).each_value do |job|
        next unless job.is_a?(Hash)

        name = job["name"]
        next unless name.is_a?(String) && !name.empty?

        if name == "Build ${{ matrix.name }}"
          architectures = @inputs.image_matrix.fetch("include", []).filter_map do |entry|
            entry["name"] if entry.is_a?(Hash)
          end
          if architectures.empty?
            error("image matrix must define at least one named architecture")
            next
          end
          architectures.each { |architecture| contexts.add("Images / Build #{architecture}") }
        else
          contexts.add("Images / #{name}")
        end
      end
    end

    def sonar_analysis_emitted?
      workflow = @inputs.document(".github/workflows/pr.yml")
      return false unless workflow

      workflow.fetch("jobs", {}).values.any? do |job|
        Array(job.is_a?(Hash) ? job["steps"] : nil).any? do |step|
          step.is_a?(Hash) && step["run"] == "just sonar-scan"
        end
      end
    end

    def error(message)
      @diagnostics.add(message)
    end
  end
end
