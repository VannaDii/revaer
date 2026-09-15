# frozen_string_literal: true

require_relative "native_cancellation_snapshot"
require_relative "native_cancellation_clocks"
require_relative "native_cancellation_comparison"

module RevaerDatabaseRebaseline
  module NativeCancellationProof
    include NativeCancellationClocks

    NATIVE_CANCELLATION_PRODUCERS = %w[native_cancellation_proof.rb native_cancellation_clocks.rb
      native_cancellation_comparison.rb native_cancellation_snapshot.rb native_sessions.rb
      native_processes.rb native_tooling.rb native_trust_rank_proof.rb].freeze

    private

    def verify_ingestion_cancellation!
      @contract.validate_output_path!
      parent = File.join(@contract.output_path, "ingestion-native-cancellation")
      raise Failure, "native cancellation parent must not be a symlink" if File.symlink?(parent)

      FileUtils.mkdir_p(parent, mode: 0o700)
      @native_cancellation_evidence = Dir.mktmpdir("run-", parent)
      source_before = cancellation_source_hashes
      source_commit = @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip
      NATIVE_CANCELLATION_PRODUCERS.each do |name|
        native_write(name, File.binread(File.join(@contract.root, "scripts/database_rebaseline", name)), directory: @native_cancellation_evidence)
      end
      first = @checks.length
      completed = false
      @native_cancellation_pairs = []
      begin
        native_prepare_tooling!(parent)
        native_write("tooling-receipt.json", File.binread(@native_tooling.receipt_path), directory: @native_cancellation_evidence)
        @native_cancellation_sessions = NativeSessions.new(runner: @runner,
          processes: NativeProcesses.new(directory: @native_cancellation_evidence), directory: @native_cancellation_evidence,
          container: @container, tooling: @native_tooling, probe: "# Read-only cancellation snapshot.\n")
        native_before = dependency_native_identity
        raise Failure, "native cancellation requires pinned arm64 PostgreSQL" unless
          native_before.fetch("architecture") == "arm64" && sql("SHOW server_version_num", role: "postgres") == "160014"

        native_policy_record(@native_cancellation_evidence, "routine-inventory.json", @ingestion_inventory)
        @native_cancellation_active = true
        super
        check("native cancellation four exact plain/observed contexts", @native_cancellation_pairs.length == 4)
        check("native cancellation source bytes unchanged", source_before == cancellation_source_hashes)
        native_after = dependency_native_identity
        check("native cancellation target bytes unchanged", native_before == native_after)
        native_policy_record(@native_cancellation_evidence, "identity.json", { source_commit:, source_before:,
          source_after: cancellation_source_hashes, native_before:, native_after: })
        completed = true
      ensure
        @native_cancellation_active = false
        @native_cancellation_context = nil
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |row| row.fetch(:passed) }, d3_complete: false,
          source_commit:, source_sha256: source_before, contexts: @native_cancellation_pairs, checks:,
          cleanup_required: "cleanup.json must independently confirm removal of owned resources and clients",
          scope: "Cold and committed-warm server cancellation at the owned Rust source-INSERT wait, four plain/observed pairs; exact frozen D4 recovery remains a failure",
          limits: "Not client-future cancellation, every interruption site, every helper or complete D3" }
        path = native_policy_record(@native_cancellation_evidence, "proof-result.json", report)
        @native_cancellation_result = { path:, sha256: Digest::SHA256.file(path).hexdigest, passed: report.fetch(:passed) }
      end
      raise Failure, "native cancellation proof failed; evidence retained" unless @checks.drop(first).all? { |row| row.fetch(:passed) }
    end

    def cancellation_source_hashes
      paths = NATIVE_CANCELLATION_PRODUCERS.map { |name| "scripts/database_rebaseline/#{name}" }
      paths += %w[snapshot comparison capture].map { |name| "scripts/tests/database-native-cancellation-#{name}-test.rb" }
      super.merge(paths.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] })
    end

    def cancellation_isolated!(variant, port, cache_state)
      return super unless @native_cancellation_active

      previous = @cancellation_evidence
      name = "#{variant}-#{cache_state.tr('_', '-')}"
      arms = %w[plain observed].to_h do |arm|
        directory = File.join(@native_cancellation_evidence, "#{name}-#{arm}")
        Dir.mkdir(directory, 0o700)
        @cancellation_evidence = directory
        @native_cancellation_context = { name: "#{name}-#{arm}", variant:, cache_state:, directory:,
          observed: arm == "observed", clocks: {}, witnesses: {}, wait_seen: false }
        result = super(variant, port, cache_state)
        raw = metadata_json_parse(File.binread(File.join(directory, "#{variant}-#{cache_state}.json")))
        raise Failure, "native cancellation owned wait was not observed" unless @native_cancellation_context.fetch(:wait_seen)

        native_cancellation_validate_clocks!(raw)
        context = @native_cancellation_context
        raise Failure, "native cancellation snapshot was not validated" if arm == "observed" && !context.key?(:snapshot)

        [arm, { raw:, clocks: context.fetch(:clocks), result:, snapshot: context[:snapshot] }]
      end
      comparison = CancellationPairComparison.new(variant:, cache_state:).compare!(
        arms.fetch("plain").fetch(:raw), arms.fetch("observed").fetch(:raw),
        plain_clocks: arms.fetch("plain").fetch(:clocks), observed_clocks: arms.fetch("observed").fetch(:clocks))
      path = native_policy_record(@native_cancellation_evidence, "#{name}-comparison.json", comparison)
      check("native cancellation #{name} complete plain/observed application pair", true)
      @native_cancellation_pairs << { variant:, cache_state:, comparison: path,
        plain_equals_observed: true, snapshot: arms.fetch("observed").fetch(:snapshot) }
      arms.fetch("plain").fetch(:result).merge(native_pair: path, native_observation: arms.fetch("observed").fetch(:snapshot))
    ensure
      if @native_cancellation_active
        @cancellation_evidence = previous
        @native_cancellation_context = nil
      end
    end

    def cancellation_validate_wait!(blocked, locker, database, role)
      super
      context = @native_cancellation_context
      return unless context && !context.fetch(:wait_seen)

      context[:wait_seen] = true
      native_cancellation_record_clock!("cancelled", blocked, locker, database, role)
      native_cancellation_observe!(blocked, locker, database, role) if context.fetch(:observed)
    end

    def native_cancellation_observe!(blocked, locker, database, role)
      context = @native_cancellation_context
      prefix = File.join(context.fetch(:directory), "#{context.fetch(:variant)}-#{context.fetch(:cache_state)}")
      prepared = metadata_json_parse(File.binread("#{prefix}-frames.json.prepared.json"))
      pid = blocked.fetch("pid")
      raise Failure, "native cancellation PID differs from prepared Rust backend" unless prepared.fetch("session").fetch("pid") == pid

      query = "SELECT json_build_object('pid', a.pid, 'database_oid', d.oid::bigint, 'database', d.datname, 'role', a.usename, 'application', a.application_name, 'backend_start', a.backend_start)::text FROM pg_stat_activity a JOIN pg_database d ON d.oid=a.datid WHERE a.pid=#{Integer(pid)} AND d.datname=#{literal(database)} AND a.usename=#{literal(role)} AND a.application_name=#{literal(IngestionCancellation::CANCELLATION_APPLICATION)};"
      native_write("identity.sql", query, directory: context.fetch(:directory))
      identity = metadata_json_parse(sql(query, role: "postgres", database:))
      native_policy_record(context.fetch(:directory), "identity-before.json", identity)
      raise Failure, "native cancellation backend identity changed" unless identity.keys.sort ==
        %w[application backend_start database database_oid pid role] &&
        identity.values_at("pid", "database", "role", "application") == [pid, database, role, IngestionCancellation::CANCELLATION_APPLICATION]

      expected = context.fetch(:variant) == "reference" ? 2 : 0
      observer = NativeCancellationSnapshot.new(pid:, database_oid: identity.fetch("database_oid"), compiler_setting: expected)
      snapshot = @native_cancellation_sessions.snapshot(name: context.fetch(:name), observer:)
      after_identity = metadata_json_parse(sql(query, role: "postgres", database:))
      native_policy_record(context.fetch(:directory), "identity-after.json", after_identity)
      raise Failure, "native cancellation backend identity changed across detach" unless identity == after_identity

      after = metadata_json_parse(sql(cancellation_wait_query(database, role, locker.fetch("pid")), role: "postgres", database:))
      native_policy_record(context.fetch(:directory), "wait-after-detach.json", after)
      IngestionCancellation.instance_method(:cancellation_validate_wait!).bind(self).call(after, locker, database, role)
      raise Failure, "native cancellation owned wait changed across detach" unless after == blocked

      context[:snapshot] = snapshot.merge(detached_and_owned_wait_revalidated: true)
      native_policy_record(context.fetch(:directory), "snapshot.json", context.fetch(:snapshot))
    end
  end
end
