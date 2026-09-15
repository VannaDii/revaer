# frozen_string_literal: true

require_relative "native_setting_phases"

module RevaerDatabaseRebaseline
  module NativeSettingProof
    NATIVE_SETTING_PRODUCERS = %w[native_setting_proof.rb native_setting_phases.rb native_sessions.rb
      native_processes.rb native_trace.rb native_tooling.rb native_trust_rank_proof.rb native_policy_proof.rb].freeze

    private

    def verify_ingestion_setting_paths!
      super
      verify_ingestion_native_settings!
    end

    def verify_ingestion_native_settings!
      @contract.validate_output_path!
      parent = File.join(@contract.output_path, "ingestion-native-settings")
      raise Failure, "native settings parent must not be a symlink" if File.symlink?(parent)

      FileUtils.mkdir_p(parent, mode: 0o700)
      @native_setting_evidence = Dir.mktmpdir("run-", parent)
      source_before = native_setting_source_hashes
      source_commit = @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip
      NATIVE_SETTING_PRODUCERS.each do |name|
        native_write(name, File.binread(File.join(@contract.root, "scripts/database_rebaseline", name)), directory: @native_setting_evidence)
      end
      first = @checks.length
      contexts = []
      completed = false
      begin
        @setting_path_validated_evidence ||= {}
        validation_sites!
        native_prepare_tooling!(parent)
        native_write("tooling-receipt.json", File.binread(@native_tooling.receipt_path), directory: @native_setting_evidence)
        @native_setting_sessions = NativeSessions.new(runner: @runner, processes: NativeProcesses.new(directory: @native_setting_evidence),
          directory: @native_setting_evidence, container: @container, tooling: @native_tooling, probe: "# Read-only helper and callback scope.\n")
        native_before = dependency_native_identity
        raise Failure, "native settings requires pinned arm64 PostgreSQL" unless native_before.fetch("architecture") == "arm64" && sql("SHOW server_version_num", role: "postgres") == "160014"

        native_policy_record(@native_setting_evidence, "routine-inventory.json", @ingestion_inventory)
        headers = @runner.run!(["docker", "exec", @container, "tar", "-cf", "-", "-C", "/usr/local/include/postgresql/server", *NativeTrustRankProof::NATIVE_HEADERS])
        native_write("target-abi-headers.tar", headers, directory: @native_setting_evidence)
        IngestionSettingPaths::SETTING_PATH_NAMES.product(%w[cold helpers-first]).each do |name, mode|
          variants = %w[reference final].to_h do |variant|
            comparable = native_setting_pair(name, mode, variant)
            contexts << { name:, mode:, variant:, plain_equals_observed: true }
            [variant, comparable]
          end
          check("native settings #{name}-#{mode} exact parity outside approved D4", variants.fetch("reference") == variants.fetch("final"))
        end
        check("native settings all twelve paired contexts", contexts.length == 12)
        check("native settings source bytes unchanged", source_before == native_setting_source_hashes)
        check("native settings target bytes unchanged", native_before == dependency_native_identity)
        native_policy_record(@native_setting_evidence, "identity.json", { source_commit:, source_before:, source_after: native_setting_source_hashes,
          native_before:, native_after: dependency_native_identity, target_abi_headers_sha256: Digest::SHA256.hexdigest(headers) })
        completed = true
      ensure
        @native_setting_context = nil
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |row| row.fetch(:passed) }, d3_complete: false,
          source_commit:, source_sha256: source_before, contexts:, checks:,
          cleanup_required: "cleanup.json must independently confirm removal of owned resources and clients",
          scope: "Existing success/rollback, year-validation and regex-error cases; complete application oracles retained. Not all callsites or callback-count closure." }
        native_policy_record(@native_setting_evidence, "proof-result.json", report)
      end
      raise Failure, "native settings proof failed; evidence retained" unless @checks.drop(first).all? { |row| row.fetch(:passed) }
    end

    def native_setting_source_hashes
      paths = NATIVE_SETTING_PRODUCERS.map { |name| "scripts/database_rebaseline/#{name}" }
      paths += %w[phases capture].map { |name| "scripts/tests/database-native-setting-#{name}-test.rb" }
      setting_path_hashes.merge(paths.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] })
    end

    def native_setting_pair(name, mode, variant)
      source, role = variant == "reference" ? ["reference_proof", "postgres"] : [@database, @runtime]
      reduced = nil
      arms = %w[plain observed].to_h do |arm|
        previous = [@setting_path_evidence, @correction_evidence, @native_setting_context]
        directory = File.join(@native_setting_evidence, "#{name}-#{mode}-#{variant}-#{arm}")
        Dir.mkdir(directory, 0o700)
        @setting_path_evidence = @correction_evidence = directory
        @native_setting_context = { name: "#{name}-#{mode}-#{variant}-#{arm}", directory:, database: "ingestion_setting_#{variant}",
          role:, variant:, observed: arm == "observed", calls: 0 }
        test_case = setting_path_case(name, mode)
        evidence = setting_path_isolated(test_case, variant, source, role, observed: false)
        raise Failure, "native settings capture absent or duplicated" unless @native_setting_context.fetch(:calls) == 1

        comparable = setting_path_comparable(evidence)
        native_policy_record(directory, "application.json", evidence)
        native_policy_record(directory, "comparable.json", comparable)
        reduced = setting_path_comparable(evidence, count: name == "success-rollback" ? 2 : nil) if arm == "plain"
        if arm == "observed"
          phases = NativeSettingPhases.new.validate!(events: @native_setting_context.fetch(:trace).fetch(:events), frames: evidence.fetch("frames"), name:, mode:, variant:)
          native_policy_record(directory, "readback.json", { name:, mode:, variant:, phases:, d3_complete: false })
        end
        [arm, comparable]
      ensure
        @setting_path_evidence, @correction_evidence, @native_setting_context = previous
      end
      equal = arms.fetch("plain") == arms.fetch("observed")
      check("native settings #{name}-#{mode}-#{variant} complete plain/observed application pair", equal)
      raise Failure, "native settings observation changed application behavior" unless equal

      reduced
    end

    def native_setting_result(query, role:, database:)
      context = @native_setting_context
      raise Failure, "native settings transport target changed" unless context.values_at(:database, :role) == [database, role]
      raise Failure, "native settings operation captured twice" unless context.fetch(:calls).zero?

      context[:calls] += 1
      capture = @native_setting_sessions.execute(command: command(role, database), query:, name: context.fetch(:name), observed: context.fetch(:observed))
      if capture.debugger
        trace = NativeTrace.new(query: ->(statement) { sql(statement, role: "postgres", database:) }, directory: context.fetch(:directory),
          observations: :settings).collect!(capture.debugger)
        native_bind_catalog_sources!(trace.fetch(:catalog), context.fetch(:variant), context.fetch(:directory))
        context[:trace] = trace
      end
      capture.application
    end
  end
end
