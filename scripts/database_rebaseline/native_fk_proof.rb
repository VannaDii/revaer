# frozen_string_literal: true

require_relative "native_fk_expectations"
require_relative "native_fk_phases"
require_relative "native_fk_remaining_phases"

module RevaerDatabaseRebaseline
  module NativeFkProof
    NATIVE_FK_CASES = %w[cold-logger-setting logger-first-setting imdb-upsert v2-conflict-warm-rollback].freeze
    NATIVE_FK_PRODUCERS = %w[native_fk_proof.rb native_fk_expectations.rb native_fk_phases.rb
      native_fk_remaining_phases.rb native_sessions.rb native_processes.rb native_trace.rb native_tooling.rb
      native_trust_rank_proof.rb native_policy_proof.rb].freeze
    NATIVE_FK_PROBE = "# Canonical helper and callback probes only.\n"

    private

    def verify_ingestion_native_fk!
      @contract.validate_output_path!
      parent = File.join(@contract.output_path, "ingestion-native-fk")
      raise Failure, "native FK parent must not be a symlink" if File.symlink?(parent)

      FileUtils.mkdir_p(parent, mode: 0o700)
      @native_fk_evidence = Dir.mktmpdir("run-", parent)
      source_before = native_fk_source_hashes
      source_commit = @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip
      NATIVE_FK_PRODUCERS.each do |name|
        native_write(name, File.binread(File.join(@contract.root, "scripts/database_rebaseline", name)), directory: @native_fk_evidence)
      end
      first = @checks.length
      contexts = []
      completed = false
      begin
        @compilation_validated_evidence ||= {}
        @native_fk_expectations = NativeFkExpectations.new(root: @contract.root, inventory: @ingestion_inventory)
        @native_fk_seen_constraints = []
        native_prepare_tooling!(parent)
        native_write("tooling-receipt.json", File.binread(@native_tooling.receipt_path), directory: @native_fk_evidence)
        @native_fk_sessions = NativeSessions.new(runner: @runner, processes: NativeProcesses.new(directory: @native_fk_evidence),
          directory: @native_fk_evidence, container: @container, tooling: @native_tooling, probe: NATIVE_FK_PROBE)
        native_before = dependency_native_identity
        raise Failure, "native FK requires pinned arm64 PostgreSQL" unless native_before.fetch("architecture") == "arm64" && sql("SHOW server_version_num", role: "postgres") == "160014"

        native_policy_record(@native_fk_evidence, "routine-inventory.json", @ingestion_inventory)
        headers = @runner.run!(["docker", "exec", @container, "tar", "-cf", "-", "-C", "/usr/local/include/postgresql/server", *NativeTrustRankProof::NATIVE_HEADERS])
        native_write("target-abi-headers.tar", headers, directory: @native_fk_evidence)
        NATIVE_FK_CASES.each do |name|
          variants = %w[reference final].to_h do |variant|
            comparable = native_fk_pair(name, variant)
            contexts << { name:, variant:, plain_equals_observed: true }
            [variant, comparable]
          end
          check("native FK #{name} exact frozen/final application parity", variants.fetch("reference") == variants.fetch("final")) unless name == "imdb-upsert"
        end
        check("native FK all eight source-bound plain/observed contexts", contexts.length == 8)
        targets = @native_fk_expectations.target_constraints
        check("native FK five targeted constraint bindings entered", targets.length == 5 && (targets - @native_fk_seen_constraints).empty?)
        check("native FK source bytes unchanged", source_before == native_fk_source_hashes)
        check("native FK target bytes unchanged", native_before == dependency_native_identity)
        native_policy_record(@native_fk_evidence, "identity.json", { source_commit:, source_before:, source_after: native_fk_source_hashes,
          native_before:, native_after: dependency_native_identity, target_abi_headers_sha256: Digest::SHA256.hexdigest(headers) })
        completed = true
      ensure
        @native_fk_context = nil
        @native_fk_fixture = false
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |row| row.fetch(:passed) }, d3_complete: false,
          source_commit:, source_sha256: source_before, contexts:, checks:,
          cleanup_required: "cleanup.json must independently confirm removal of owned resources and clients",
          scope: "Four canonical FK scenarios in reference/final plain/observed pairs; frozen D5 errors remain errors" }
        path = native_policy_record(@native_fk_evidence, "proof-result.json", report)
        @native_fk_result = { path:, sha256: Digest::SHA256.file(path).hexdigest, passed: report.fetch(:passed) }
      end
      raise Failure, "native FK proof failed; evidence retained" unless @checks.drop(first).all? { |row| row.fetch(:passed) }
    end

    def native_fk_source_hashes
      paths = NATIVE_FK_PRODUCERS.map { |name| "scripts/database_rebaseline/#{name}" }
      compilation_source_hashes.merge(paths.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] })
    end

    def native_fk_pair(name, variant)
      source, role = variant == "reference" ? ["reference_proof", "postgres"] : [@database, @runtime]
      database = case name
                 when "imdb-upsert" then "ingestion_correction_#{variant}"
                 when "v2-conflict-warm-rollback" then "ingestion_#{variant}_proof"
                 else "ingestion_compilation_#{variant}"
                 end
      arms = %w[plain observed].to_h do |arm|
        previous = [@ingestion_evidence, @compilation_evidence, @correction_evidence, @native_fk_context]
        directory = File.join(@native_fk_evidence, "#{name}-#{variant}-#{arm}")
        Dir.mkdir(directory, 0o700)
        @ingestion_evidence = @compilation_evidence = @correction_evidence = directory
        @native_fk_context = { name: "#{name}-#{variant}-#{arm}", directory:, database:, role:, variant:, observed: arm == "observed", calls: 0 }
        evidence, comparable = native_fk_application(name, variant, source, role, directory)
        raise Failure, "native FK application capture absent or duplicated" unless @native_fk_context.fetch(:calls) == 1

        native_policy_record(directory, "application.json", evidence)
        native_policy_record(directory, "comparable.json", comparable)
        if arm == "observed"
          trace = @native_fk_context.fetch(:trace)
          frames = evidence.fetch("frames")
          phases = if %w[cold-logger-setting logger-first-setting].include?(name)
                     NativeFkPhases.new(@native_fk_expectations.compilation).validate!(events: trace.fetch(:events), frames:, scenario: name, variant:)
                   else
                     operations = NativeFkRemainingPhases.operations(@native_fk_expectations.remaining, name, variant)
                     NativeFkRemainingPhases.new.validate!(events: trace.fetch(:events), frames:, scenario: name, variant:, operations:)
                   end
          @native_fk_seen_constraints.concat(trace.fetch(:triggers).map { |entry| entry.fetch("constraint") })
          native_policy_record(directory, "readback.json", { name:, variant:, phases:, d3_complete: false })
        end
        [arm, comparable]
      ensure
        @ingestion_evidence, @compilation_evidence, @correction_evidence, @native_fk_context = previous
      end
      equal = arms.fetch("plain") == arms.fetch("observed")
      check("native FK #{name}-#{variant} complete plain/observed application pair", equal)
      raise Failure, "native FK observation changed application behavior" unless equal

      arms.fetch("plain")
    end

    def native_fk_application(name, variant, source, role, directory)
      first = @checks.length
      if name == "imdb-upsert"
        test_case = native_fk_case!(correction_cases, name)
        evidence = metadata_json_parse(JSON.generate(correction_isolated(test_case, variant, source, role)))
        comparable = compilation_comparable("fixture" => nil, "frames" => evidence.fetch("frames"))
      elsif name == "v2-conflict-warm-rollback"
        evidence = existing_v2_isolated("warm-rollback", variant, source, role)
        check("native FK #{name}-#{variant} canonical identity conflict and recovery", existing_v2_evidence?(evidence, "warm-rollback", variant, role))
        comparable = { "application" => compilation_comparable("fixture" => evidence.fetch("fixtures").first, "frames" => evidence.fetch("frames")),
          "inputs" => wrapper_comparable_inputs(evidence) }
      else
        test_case = native_fk_case!(compilation_cases, name)
        comparable = compilation_isolated(test_case, variant, source, role, observed: false)
        evidence = metadata_json_parse(File.binread(File.join(directory, "#{name}-#{variant}-plain.json")))
      end
      raise Failure, "native FK canonical application oracle failed" unless @checks.drop(first).all? { |row| row.fetch(:passed) }

      [evidence, comparable]
    end

    def native_fk_case!(cases, name)
      matches = cases.select { |entry| entry.fetch(:name) == name }
      raise Failure, "canonical native FK case missing or duplicated" unless matches.length == 1

      matches.first
    end

    def compilation_fixture(...)
      previous = @native_fk_fixture
      @native_fk_fixture = true
      super
    ensure
      @native_fk_fixture = previous
    end

    def wrapper_execute(test_case, database, role, prefix)
      previous = @native_fk_fixture
      @native_fk_fixture = true if prefix.end_with?("-fixture")
      super
    ensure
      @native_fk_fixture = previous
    end

    def native_fk_result(query, role:, database:)
      context = @native_fk_context
      raise Failure, "native FK transport target changed" unless context.values_at(:database, :role) == [database, role]
      raise Failure, "native FK operation captured twice" unless context.fetch(:calls).zero?

      context[:calls] += 1
      before = policy_read_snapshot(database)
      capture = @native_fk_sessions.execute(command: command(role, database), query:, name: context.fetch(:name), observed: context.fetch(:observed))
      after = policy_read_snapshot(database)
      native_policy_record(context.fetch(:directory), "read-inputs.json", { before:, after: })
      raise Failure, "native FK read-only inputs changed" unless before == after && before.keys.sort == IngestionPolicy::POLICY_READ_TABLES.sort

      if capture.debugger
        trace = NativeTrace.new(query: ->(statement) { sql(statement, role: "postgres", database:) }, directory: context.fetch(:directory), observations: :fk).collect!(capture.debugger)
        native_bind_catalog_sources!(trace.fetch(:catalog), context.fetch(:variant), context.fetch(:directory))
        context[:trace] = trace
      end
      capture.application
    end
  end
end
