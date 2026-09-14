# frozen_string_literal: true

require_relative "native_sessions"
require_relative "native_tooling"
require_relative "native_trace"

module RevaerDatabaseRebaseline
  module NativeTrustRankProof
    NATIVE_PRODUCERS = %w[native_sessions.rb native_processes.rb native_tooling.rb native_trace.rb
                          native_trust_rank_proof.rb native_trust_rank_source.rb native_trust_rank_readback.rb
                          native-trust-rank.gdb].freeze
    NATIVE_HEADERS = %w[plpgsql.h fmgr.h pg_config.h pg_config_manual.h postgres_ext.h commands/trigger.h utils/reltrigger.h].freeze

    private

    def verify_ingestion_native_trust_rank!
      require_relative "native_trust_rank_readback"
      @contract.validate_output_path!
      parent = File.join(@contract.output_path, "ingestion-native-trust-rank")
      raise Failure, "native proof parent must not be a symlink" if File.symlink?(parent)

      FileUtils.mkdir_p(parent, mode: 0o700)
      @native_evidence = Dir.mktmpdir("run-", parent)
      @native_primary_evidence = @native_evidence
      @attributes_validated_evidence ||= {}
      source_before = native_source_hashes
      source_commit = @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip
      producer_sha256 = NATIVE_PRODUCERS.to_h do |name|
        path = "scripts/database_rebaseline/#{name}"
        native_write(name, File.binread(File.join(@contract.root, path)))
        [name, source_before.fetch(path)]
      end
      first = @checks.length
      completed = false
      begin
        native_prepare!(parent)
        native_before = dependency_native_identity
        raise Failure, "native proof requires pinned arm64 PostgreSQL" unless native_before.fetch("architecture") == "arm64" && sql("SHOW server_version_num", role: "postgres") == "160014"

        @ingestion_inventory.each { |name, inventory| native_write("#{name}-inventory.json", JSON.pretty_generate(inventory) + "\n") }
        headers = @runner.run!(["docker", "exec", @container, "tar", "-cf", "-", "-C", "/usr/local/include/postgresql/server", *NATIVE_HEADERS])
        native_write("target-abi-headers.tar", headers)
        NativeTrustRankReadback::CASES.product(NativeTrustRankReadback::MODES, NativeTrustRankReadback::VARIANTS).each do |case_name, mode, variant|
          native_trust_pair(case_name, mode, variant)
        end
        native_write("report.json", JSON.pretty_generate({ source_commit:, source_before:, source_after: native_source_hashes,
          native_before:, native_after: dependency_native_identity, complete_d3: false, target_abi_headers_sha256: Digest::SHA256.hexdigest(headers) }) + "\n")
        reader = NativeTrustRankReadback.new(root: @contract.root, source_commit:, producer_sha256:)
        qualified = reader.validate!(reader.read_run!(@native_evidence))
        native_write("readback.json", JSON.pretty_generate(qualified) + "\n")
        check("native K1 source-bound paired application and local-variable readback", true)
        native_trust_calibration!(parent)
        check("native K1 source bytes unchanged", source_before == native_source_hashes)
        check("native K1 target bytes unchanged", native_before == dependency_native_identity)
        completed = true
      ensure
        @native_evidence = @native_primary_evidence
        @native_context = nil
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
                   cleanup_required: "cleanup.json must independently confirm owned resources and clients were removed",
                   scope: "Two K1 cases in cold/helper-first reference/final pairs, plus nonzero rank-40 calibration",
                   source_commit:, source_sha256: source_before, checks: }
        path = native_write("proof-result.json", JSON.pretty_generate(report) + "\n")
        @native_trust_rank_result = { path:, sha256: Digest::SHA256.file(path).hexdigest, passed: report.fetch(:passed) }
      end
      raise Failure, "native K1 proof failed; evidence retained" unless @checks.drop(first).all? { |entry| entry.fetch(:passed) }
    end

    def native_source_hashes
      paths = IngestionAttributes::ATTRIBUTE_SOURCE_FILES + NATIVE_PRODUCERS.map { |name| "scripts/database_rebaseline/#{name}" }
      paths.uniq.sort.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] }
    end

    def native_write(name, bytes)
      raise Failure, "invalid native evidence name" unless name.match?(/\A[a-zA-Z0-9_.-]+\z/)

      path = File.join(@native_evidence, name)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(bytes) }
      path
    end

    def native_prepare!(parent)
      @native_tooling_parent = Dir.mktmpdir("tooling-", parent)
      @native_tooling = NativeTooling.new(runner: @runner, inputs: EnvFile.load(@contract.build_inputs_path),
                                         directory: @native_tooling_parent, postgres_image: @contract.postgres_image).prepare!
      native_write("tooling-receipt.json", File.binread(@native_tooling.receipt_path))
      @native_sessions = NativeSessions.new(runner: @runner, processes: NativeProcesses.new(directory: @native_evidence),
        directory: @native_evidence, container: @container, tooling: @native_tooling,
        probe: File.binread(File.join(@contract.root, "scripts/database_rebaseline/native-trust-rank.gdb")))
    end

    def native_trust_pair(case_name, mode, variant)
      test_case = attributes_cases.find { |entry| entry.fetch(:name) == case_name }
      raise Failure, "native trust-rank canonical case absent" unless test_case

      source, role = variant == "reference" ? ["reference_proof", "postgres"] : [@database, @runtime]
      prefix = [case_name, mode, variant].join("-")
      pairs = %w[plain traced].to_h do |arm|
        previous = [@attributes_evidence, @correction_evidence, @native_context]
        directory = File.join(@native_evidence, "#{prefix}-#{arm}")
        Dir.mkdir(directory, 0o700)
        @attributes_evidence = directory
        @correction_evidence = directory
        @native_context = { name: "#{prefix}-#{arm}-#{arm == 'traced' ? 'observed' : 'plain'}", observed: arm == "traced", directory: }
        raw = attributes_isolated(test_case, mode, variant, source, role)
        evidence = JSON.parse(JSON.generate(raw), allow_duplicate_key: false)
        comparable = { "application" => compilation_comparable("fixture" => evidence.fetch("fixture"), "frames" => evidence.fetch("frames")),
                       "inputs" => validation_comparable(evidence, { site: nil }).fetch("inputs") }
        attributes_write("application.json", JSON.pretty_generate(evidence) + "\n")
        attributes_write("comparable.json", JSON.pretty_generate(comparable) + "\n")
        [arm, { application: evidence, comparable:, trace: @native_context[:trace] }]
      ensure
        @attributes_evidence, @correction_evidence, @native_context = previous
      end
      check("native #{prefix} complete plain/observed application pair", pairs.fetch("plain").fetch(:comparable) == pairs.fetch("traced").fetch(:comparable))
      pairs
    end

    def attributes_execute(session, database, role, name)
      previous = @native_target
      @native_target = @native_context&.merge(database:, role:) unless name.end_with?("-fixture")
      super
    ensure
      @native_target = previous
    end

    def native_result(query, role:, database:)
      raise Failure, "native transport target changed" unless @native_target.values_at(:database, :role) == [database, role]

      capture = @native_sessions.execute(command: command(role, database), query:, name: @native_target.fetch(:name), observed: @native_target.fetch(:observed))
      if capture.debugger
        trace = NativeTrace.new(query: ->(statement) { sql(statement, role: "postgres", database:) }, directory: @native_target.fetch(:directory)).collect!(capture.debugger)
        @native_context[:trace] = trace
      end
      capture.application
    end

    def native_trust_calibration!(parent)
      primary = @native_evidence
      @native_evidence = Dir.mktmpdir("calibration-", parent)
      results = %w[reference final].map do |variant|
        pair = native_trust_pair("typed-rank-40", "cold", variant)
        events = pair.fetch("traced").fetch(:trace).fetch(:events)
        boolean = events.select { |event| event.values_at("kind", "phase") == %w[boolean return] }
        answers = boolean.map { |event| event.values_at("query", "answer") }.tally
        expected = { ["instance_trust_tier_key IS NOT NULL", 1] => 3,
                     ["instance_trust_rank IS NULL", 0] => 3, ["instance_trust_rank >= 40", 1] => 3 }
        buckets = events.select do |event|
          event.values_at("kind", "phase", "target") == %w[assignment return trust_bucket] &&
            event.fetch("after").fetch("locals").fetch("trust_bucket").fetch("value") == 3
        end
        unless answers == expected && buckets.length == 3 && buckets.all? { |event| event.fetch("after").fetch("locals").fetch("instance_trust_rank").fetch("value") == 40 }
          raise Failure, "native nonzero trust-rank calibration changed"
        end
        check("native #{variant} positive rank/bucket and Boolean calibration", true)
        { variant:, answers: answers.map { |(query, answer), count| { query:, answer:, count: } }, nonzero_reads: buckets.length }
      end
      native_write("calibration.json", JSON.pretty_generate({ results:, d3_complete: false, source_sha256: native_source_hashes }) + "\n")
    ensure
      @native_evidence = primary
    end

    def cleanup_native_resources!
      failures = []
      operations = [-> { @native_sessions&.remove_containers! }, -> { yield },
                    -> { @native_sessions&.close_after_owner_cleanup! }, -> { cleanup_native_tooling! }]
      operations.each do |operation|
        operation.call
      rescue StandardError => error
        failures << "#{error.class}: #{error.message}"
      end
      if @native_primary_evidence
        @native_evidence = @native_primary_evidence
        native_write("cleanup.json", JSON.pretty_generate({ passed: failures.empty?, failures: }) + "\n")
      end
      return if failures.empty?

      @failures << "disposable native/container cleanup failed"
      raise Failure, "proof cleanup failed: #{failures.join('; ')}"
    end

    def cleanup_native_tooling!
      if @native_tooling
        receipt = JSON.parse(File.binread(@native_tooling.receipt_path), allow_duplicate_key: false)
        tag = receipt.fetch("owned_image_tag")
        if tag
          raise Failure, "native cleanup tag is not owned" unless tag.match?(/\Arevaer-native-observer:[a-zA-Z0-9-]+\z/)

          images = JSON.parse(@runner.run!(["docker", "image", "inspect", tag]), allow_duplicate_key: false)
          raise Failure, "owned native image identity changed" unless images.length == 1 && images.first.fetch("Id") == @native_tooling.debugger_image

          @runner.run!(["docker", "image", "rm", "--no-prune", tag])
        end
      end
    ensure
      if @native_tooling_parent
        Find.find(@native_tooling_parent) { |entry| File.chmod(0o700, entry) if File.lstat(entry).directory? }
        FileUtils.remove_entry_secure(@native_tooling_parent)
      end
    end
  end
end
