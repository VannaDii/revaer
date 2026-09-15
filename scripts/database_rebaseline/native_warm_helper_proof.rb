# frozen_string_literal: true

require_relative "native_warm_helper_phases"

module RevaerDatabaseRebaseline
  module NativeWarmHelperProof
    NATIVE_WARM_SCENARIOS = { "magnet-uri" => "identity-magnet-no-query-new", "title-size" => "identity-title-size-new" }.freeze
    NATIVE_WARM_PRODUCERS = %w[native_warm_helper_proof.rb native_warm_helper_phases.rb native_sessions.rb
      native_processes.rb native_trace.rb native_tooling.rb native_trust_rank_proof.rb native_policy_proof.rb].freeze

    private

    def verify_ingestion_native_fk!
      super
      verify_ingestion_native_warm_helpers!
    end

    def verify_ingestion_native_warm_helpers!
      @contract.validate_output_path!
      parent = File.join(@contract.output_path, "ingestion-native-warm-helpers")
      raise Failure, "native warm parent must not be a symlink" if File.symlink?(parent)

      FileUtils.mkdir_p(parent, mode: 0o700)
      @native_warm_evidence = Dir.mktmpdir("run-", parent)
      source_before = native_warm_source_hashes
      source_commit = @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip
      NATIVE_WARM_PRODUCERS.each do |name|
        native_write(name, File.binread(File.join(@contract.root, "scripts/database_rebaseline", name)), directory: @native_warm_evidence)
      end
      first = @checks.length
      contexts = []
      completed = false
      begin
        native_prepare_tooling!(parent)
        native_write("tooling-receipt.json", File.binread(@native_tooling.receipt_path), directory: @native_warm_evidence)
        @native_warm_sessions = NativeSessions.new(runner: @runner, processes: NativeProcesses.new(directory: @native_warm_evidence),
          directory: @native_warm_evidence, container: @container, tooling: @native_tooling, probe: "# Read-only committed-warm helper scope.\n")
        native_before = dependency_native_identity
        raise Failure, "native warm requires pinned arm64 PostgreSQL" unless native_before.fetch("architecture") == "arm64" && sql("SHOW server_version_num", role: "postgres") == "160014"

        native_policy_record(@native_warm_evidence, "routine-inventory.json", @ingestion_inventory)
        headers = @runner.run!(["docker", "exec", @container, "tar", "-cf", "-", "-C", "/usr/local/include/postgresql/server", *NativeTrustRankProof::NATIVE_HEADERS])
        native_write("target-abi-headers.tar", headers, directory: @native_warm_evidence)
        NATIVE_WARM_SCENARIOS.each_key do |scenario|
          variants = %w[reference final].to_h do |variant|
            comparable = native_warm_pair(scenario, variant)
            contexts << { scenario:, variant:, plain_equals_observed: true }
            [variant, comparable]
          end
          check("native warm #{scenario} complete first-call frozen/final parity", variants.fetch("reference").first == variants.fetch("final").first)
        end
        check("native warm all four paired contexts", contexts.length == 4)
        check("native warm source bytes unchanged", source_before == native_warm_source_hashes)
        check("native warm target bytes unchanged", native_before == dependency_native_identity)
        native_policy_record(@native_warm_evidence, "identity.json", { source_commit:, source_before:, source_after: native_warm_source_hashes,
          native_before:, native_after: dependency_native_identity, target_abi_headers_sha256: Digest::SHA256.hexdigest(headers) })
        completed = true
      ensure
        @native_warm_context = nil
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |row| row.fetch(:passed) }, d3_complete: false,
          source_commit:, source_sha256: source_before, contexts:, checks:,
          cleanup_required: "cleanup.json must independently confirm removal of owned resources and clients",
          scope: "Magnet URI and title-size cold/committed-warm helper calls; exact frozen D4 failure is not final success. Not complete callsite or callback-count closure." }
        native_policy_record(@native_warm_evidence, "proof-result.json", report)
      end
      raise Failure, "native warm proof failed; evidence retained" unless @checks.drop(first).all? { |row| row.fetch(:passed) }
    end

    def native_warm_source_hashes
      paths = NATIVE_WARM_PRODUCERS.map { |name| "scripts/database_rebaseline/#{name}" }
      paths += %w[scripts/tests/database-native-warm-helper-phases-test.rb scripts/tests/database-native-setting-capture-test.rb]
      compilation_source_hashes.merge(paths.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] })
    end

    def native_warm_pair(scenario, variant)
      source, role = variant == "reference" ? ["reference_proof", "postgres"] : [@database, @runtime]
      identity_case = native_fk_case!(wrapper_identity_cases, NATIVE_WARM_SCENARIOS.fetch(scenario))
      arguments = identity_case.fetch(:arguments)
      test_case = { name: "native-warm-#{scenario}", calls: [arguments, arguments], finish_setting: true }
      arms = %w[plain observed].to_h do |arm|
        previous = [@correction_evidence, @native_warm_context]
        directory = File.join(@native_warm_evidence, "#{scenario}-#{variant}-#{arm}")
        Dir.mkdir(directory, 0o700)
        @correction_evidence = directory
        @native_warm_context = { name: "#{scenario}-#{variant}-#{arm}", directory:, database: "ingestion_correction_#{variant}",
          role:, variant:, observed: arm == "observed", calls: 0 }
        first = @checks.length
        evidence = metadata_json_parse(JSON.generate(correction_isolated(test_case, variant, source, role)))
        raise Failure, "native warm canonical application oracle failed" unless @checks.drop(first).all? { |row| row.fetch(:passed) }
        raise Failure, "native warm application capture absent or duplicated" unless @native_warm_context.fetch(:calls) == 1

        native_warm_identity!(evidence.fetch("frames"), identity_case.fetch(:identity))
        frames = evidence.fetch("frames")
        native_warm_scope!(frames, variant)
        comparable = compilation_comparable("fixture" => nil, "frames" => frames)
        native_policy_record(directory, "application.json", evidence)
        native_policy_record(directory, "comparable.json", comparable)
        if arm == "observed"
          phases = NativeWarmHelperPhases.new.validate!(events: @native_warm_context.fetch(:trace).fetch(:events), frames:, scenario:, variant:)
          native_policy_record(directory, "readback.json", { scenario:, variant:, phases:, d3_complete: false })
        end
        [arm, comparable]
      ensure
        @correction_evidence, @native_warm_context = previous
      end
      equal = arms.fetch("plain") == arms.fetch("observed")
      check("native warm #{scenario}-#{variant} complete plain/observed application pair", equal)
      raise Failure, "native warm observation changed application behavior" unless equal

      arms.fetch("plain")
    end

    def native_warm_identity!(frames, expected)
      frames.select { |frame| frame.fetch("state") == "00000" }.each do |frame|
        tables = frame.fetch("tables_after")
        canonical = tables.fetch("canonical_torrent")
        sources = tables.fetch("canonical_torrent_source")
        observations = tables.fetch("search_request_source_observation")
        unless canonical.length == 1 && sources.length == 1 && observations.length == 1 &&
            canonical.first.values_at("identity_strategy", "identity_confidence") == expected.values_at(:strategy, :confidence) &&
            canonical.first.values_at("infohash_v1", "infohash_v2", "magnet_hash", "title_size_hash") == expected.fetch(:hashes) &&
            sources.first.values_at("infohash_v1", "infohash_v2", "magnet_hash") == expected.fetch(:source_hashes) &&
            wrapper_identity_observation?(expected, sources.first, observations.first)
          raise Failure, "native warm exact identity or observation answers changed"
        end
      end
    end

    def native_warm_scope!(frames, variant)
      unless %w[reference final].include?(variant) && frames.length == 2 && frames.all? do |frame|
        frame.values_at("before", "after", "finished_setting", "within", "outside") ==
          ["error", "error", "error", "true", (variant == "reference").to_s]
      end
        raise Failure, "native warm caller scope or approved D4 lifetime changed"
      end
    end

    def native_warm_result(query, role:, database:)
      context = @native_warm_context
      raise Failure, "native warm transport target changed" unless context.values_at(:database, :role) == [database, role]
      raise Failure, "native warm operation captured twice" unless context.fetch(:calls).zero?

      context[:calls] += 1
      before = policy_read_snapshot(database)
      capture = @native_warm_sessions.execute(command: command(role, database), query:, name: context.fetch(:name), observed: context.fetch(:observed))
      after = policy_read_snapshot(database)
      native_policy_record(context.fetch(:directory), "read-inputs.json", { before:, after: })
      raise Failure, "native warm read-only inputs changed" unless before == after && before.keys.sort == IngestionPolicy::POLICY_READ_TABLES.sort
      raise Failure, "native warm exact D4 diagnostic changed" unless capture.application.stderr == (context.fetch(:variant) == "reference" ? policy_d4_expected : "")

      if capture.debugger
        trace = NativeTrace.new(query: ->(statement) { sql(statement, role: "postgres", database:) }, directory: context.fetch(:directory),
          observations: :warm_helpers).collect!(capture.debugger)
        native_bind_catalog_sources!(trace.fetch(:catalog), context.fetch(:variant), context.fetch(:directory))
        context[:trace] = trace
      end
      capture.application
    end
  end
end
