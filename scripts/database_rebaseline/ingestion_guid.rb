# frozen_string_literal: true

require "open3"
require_relative "ingestion_guid_trace"

module RevaerDatabaseRebaseline
  # Test-only barriers expose two real concurrent GUID-conflict callsites.
  module IngestionGuid
    include IngestionGuidTrace
    GUID_KINDS = %w[changed-selected-guid competing-guid].freeze
    GUID_MODES = %w[cold helpers-first logger-first].freeze

    private

    def guid_session(arguments, helpers: false, warm_logger: false)
      { calls: (warm_logger ? [guid_warm_operation] : []) + [arguments], wrapper: true, helpers:, finish_setting: true, rollback: warm_logger }
    end

    def guid_parse(record, helpers: false, warm_logger: false, trace: nil)
      metadata_transport_json!(record.fetch("stdout"))
      frames = correction_frames(record.fetch("stdout"), guid_session({}, helpers:, warm_logger:))
      unless frames.length == (warm_logger ? 2 : 1) && frames.all? { |frame| frame.fetch("state") == "00000" }
        raise Failure, "GUID real wrapper must succeed without diagnostics"
      end
      guid_trace_validate!(record.fetch("stderr"), frames, trace, warm_logger:)
      if record.key?("frames") && !size_tables_equal?({ "frames" => frames }, { "frames" => record.fetch("frames") })
        raise Failure, "GUID raw and declared frames differ"
      end

      frames
    end

    def guid_execute(arguments, database, role, name, helpers: false, paused: false, warm_logger: false, trace: nil)
      session = correction_session(guid_session(arguments, helpers:, warm_logger:)) do |call|
        call[:operation] == :logger ? compilation_call(call, 1) : ingestion_call(call)
      end
      query = "\\set SHOW_CONTEXT always\nSET application_name = '#{paused ? 'guid-race-paused' : 'guid-race-contender'}';\n" + session
      raw = metadata_transport(query, database, role, name)
      raw.merge("frames" => guid_parse(raw, helpers:, warm_logger:, trace:))
    end

    def guid_instrument(original, kind)
      anchors = {
        "changed-selected-guid" => "        SELECT source_guid, infohash_v1, infohash_v2, magnet_hash\n        INTO existing_source_guid, existing_infohash_v1, existing_infohash_v2, existing_magnet_hash",
        "competing-guid" => "        IF source_guid_value IS NOT NULL THEN\n            IF existing_source_guid IS NULL THEN"
      }
      anchor = anchors.fetch(kind)
      raise Failure, "GUID expected one exact observation site" unless original.scan(anchor).length == 1

      hook = "        IF pg_catalog.current_setting('application_name') = 'guid-race-paused' THEN\n" \
        "            PERFORM pg_catalog.pg_advisory_xact_lock(588, 1029);\n        END IF;\n"
      original.sub(anchor, hook + anchor)
    end

    def guid_instrument!(database, kind, name)
      query = "SELECT pg_get_functiondef(p.oid) FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public' AND p.proname='search_result_ingest_v1'"
      original = sql(query, role: "postgres", database:)
      changed = guid_instrument(original, kind)
      metadata_write("#{name}-original.sql", original)
      metadata_write("#{name}-instrumented.sql", changed)
      sql(changed, role: "postgres", database:)
      raise Failure, "GUID instrumented definition differs" unless sql(query, role: "postgres", database:) == changed

      { "original_sha256" => Digest::SHA256.hexdigest(original), "instrumented_sha256" => Digest::SHA256.hexdigest(changed) }
    end

    def guid_controller_read(output, marker, transcript)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 10
      loop do
        remaining = deadline - Process.clock_gettime(Process::CLOCK_MONOTONIC)
        raise Failure, "GUID controller output timeout" unless remaining.positive? && IO.select([output], nil, nil, remaining)

        line = output.gets
        raise Failure, "GUID controller exited early" unless line

        transcript << line
        return if line.strip == marker
      end
    end

    def guid_barrier(database)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 10
      loop do
        query = "SELECT COALESCE(json_agg(row_to_json(r)), '[]') FROM (SELECT a.pid,a.usename,a.application_name,l.classid,l.objid,l.objsubid,l.granted FROM pg_stat_activity a JOIN pg_locks l ON l.pid=a.pid WHERE a.datname=#{literal(database)} AND a.application_name='guid-race-paused' AND l.locktype='advisory' AND l.classid=588 AND l.objid=1029 AND l.objsubid=2 AND NOT l.granted) r"
        rows = JSON.parse(sql(query, role: "postgres", database:))
        return rows.first if rows.one?

        raise Failure, "GUID caller did not reach exact advisory boundary" if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline

        sleep 0.02
      end
    end

    def guid_interleave(database, role, a, b, mode, name, trace: nil)
      paused = nil
      transcript = +""
      Open3.popen3(*command("postgres", database)) do |input, output, error, waiter|
        error_reader = Thread.new { error.read }
        begin
          input.write("SELECT pg_advisory_lock(588,1029); SELECT 'held';\n")
          input.flush
          guid_controller_read(output, "held", transcript)
          paused = Thread.new { guid_execute(a, database, role, "#{name}-paused", paused: true, helpers: mode == "helpers-first", warm_logger: mode == "logger-first", trace: trace&.merge(logger: true)) }
          barrier = guid_barrier(database)
          contender = guid_execute(b, database, role, "#{name}-contender", helpers: mode == "helpers-first", trace: trace&.merge(logger: false))
          intermediate = ingestion_snapshot(database)
          input.write("SELECT pg_advisory_unlock(588,1029); SELECT 'released';\n")
          input.flush
          guid_controller_read(output, "released", transcript)
          raise Failure, "GUID paused wrapper did not settle" unless paused.join(125)

          { "paused" => paused.value, "contender" => contender, "barrier" => barrier, "intermediate" => intermediate }
        ensure
          input.close unless input.closed?
          transcript << output.read
          stderr = error_reader.value
          success = waiter.value.success?
          metadata_write("#{name}-controller.json", JSON.pretty_generate({ stdout: transcript, stderr:, success: }) + "\n")
          raise Failure, "GUID paused wrapper cleanup did not settle" if paused && !paused.join(125)
          raise Failure, "GUID controller transport failed" unless success && stderr.empty?
        end
      end
    end

    def guid_expected_records(source, existing, clock, id: 1, type: "source_guid", incoming: "wanted", observed: "2026-09-10T00:03:00+00:00")
      {
        "source_metadata_conflict" => [{ "source_metadata_conflict_id" => id, "canonical_torrent_source_id" => source,
          "conflict_type" => type, "existing_value" => existing, "incoming_value" => incoming, "observed_at" => observed,
          "resolved_at" => nil, "resolved_by_user_id" => nil, "resolution" => nil, "resolution_note" => nil }],
        "source_metadata_conflict_audit_log" => [{ "source_metadata_conflict_audit_log_id" => id, "conflict_id" => id, "action" => "created",
          "actor_user_id" => 0, "occurred_at" => clock, "note" => nil }],
        "indexer_health_event" => [{ "indexer_health_event_id" => id, "indexer_instance_id" => 569001, "occurred_at" => observed,
          "event_type" => "identity_conflict", "latency_ms" => nil, "http_status" => nil, "error_class" => nil, "detail" => type }]
      }
    end

    def guid_validate!(evidence, kind, mode, variant, role, observed: false)
      raise Failure, "GUID unknown scenario" unless GUID_KINDS.include?(kind) && GUID_MODES.include?(mode) && %w[reference final].include?(variant)

      trace = observed ? { kind:, variant:, role: } : nil
      fixtures = evidence.fetch("fixtures").map { |raw| guid_parse(raw).first }
      paused = guid_parse(evidence.fetch("paused"), helpers: mode == "helpers-first", warm_logger: mode == "logger-first", trace: trace&.merge(logger: true))
      a = paused.last
      b = guid_parse(evidence.fetch("contender"), helpers: mode == "helpers-first", trace: trace&.merge(logger: false)).first
      all = fixtures + [b, a]
      before, intermediate, after = evidence.values_at("before", "intermediate", "after")
      capabilities = { "session" => role, "current" => role, "superuser" => variant == "reference", "create_role" => variant == "reference", "bypass_rls" => variant == "reference" }
      unless all.all? { |f| f.fetch("role") == capabilities && f.values_at("before", "after", "finished_setting") == %w[error error error] && f.values_at("within", "outside") == ["true", (variant == "reference").to_s] }
        raise Failure, "GUID direct authority caller settings or D4 scope changed"
      end
      backends = all.map { |f| f.fetch("backend") }
      barrier = { "pid" => Integer(a.fetch("backend")), "usename" => role, "application_name" => "guid-race-paused", "classid" => "588", "objid" => "1029", "objsubid" => 2, "granted" => false }
      unless backends.all? { |pid| pid.match?(/\A[1-9][0-9]*\z/) } && backends.uniq.length == all.length && JSON.generate(evidence.fetch("barrier")) == JSON.generate(barrier)
        raise Failure, "GUID distinct backends or exact barrier changed"
      end
      warm = mode == "logger-first" ? [paused.first] : []
      warm.each { |frame| guid_warm_validate!(frame, a, before, capabilities) }
      clocks = [evidence.fetch("seed_clock"), *(all + warm).map { |f| f.fetch("clock") }]
      unless clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length
        raise Failure, "GUID transaction clock provenance changed"
      end
      unless fixtures.length == (kind == "competing-guid" ? 2 : 1) && correction_snapshots?({}, attributes_empty, fixtures, before) &&
             size_tables_equal?(a.fetch("tables_before"), before) && size_tables_equal?(b.fetch("tables_before"), before) &&
             size_tables_equal?(b.fetch("tables_after"), intermediate) && size_tables_equal?(b.fetch("tables_finish"), intermediate) &&
             size_tables_equal?(a.fetch("tables_after"), after) && size_tables_equal?(a.fetch("tables_finish"), after)
        raise Failure, "GUID complete committed transition changed"
      end
      unless all.all? { |f| %w[tables_before tables_after tables_finish].all? { |key| f.fetch(key).keys.sort == IngestionProof::INGESTION_TABLES.sort } }
        raise Failure, "GUID 18-table inventory changed"
      end
      inputs = evidence.fetch("inputs_before")
      unless size_tables_equal?(inputs, evidence.fetch("inputs_after")) && size_tables_equal?(inputs, metadata_read_tables(evidence.fetch("seed_clock")))
        raise Failure, "GUID read inputs or observed seed clock changed"
      end
      source = before.fetch("canonical_torrent_source").find { |row| row.fetch("infohash_v1") == "a" * 40 }
      other = before.fetch("canonical_torrent_source").find { |row| row.fetch("infohash_v1") == (kind == "competing-guid" ? "b" : "a") * 40 }
      unless source && other && [source, other].all? { |row| row.fetch("source_guid").nil? && IngestionProof::INGESTION_UUID.match?(row.fetch("canonical_torrent_source_public_id")) }
        raise Failure, "GUID seeded source identities changed"
      end
      expected = guid_expected_records(source.fetch("canonical_torrent_source_id"), other.fetch("canonical_torrent_source_public_id"), a.fetch("clock"), id: warm.empty? ? 1 : 2)
      raise Failure, "GUID independent conflict audit or health record changed" unless expected.all? { |table, rows| size_tables_equal?({ table => after.fetch(table) }, { table => rows }) }

      durable = after.fetch("canonical_torrent_source").find { |row| row.fetch("canonical_torrent_source_id") == source.fetch("canonical_torrent_source_id") }
      observations = after.fetch("search_request_source_observation").select { |row| row.fetch("source_guid") == "wanted" }
      unless durable && durable.fetch("source_guid") == (kind == "competing-guid" ? nil : "other") && observations.one? &&
             observations.first.values_at("guid_conflict", "canonical_torrent_source_id", "canonical_torrent_id") == [true, source.fetch("canonical_torrent_source_id"), 1] &&
             a.fetch("result").values_at("observation_created", "durable_source_created") == [kind != "competing-guid", false]
        raise Failure, "GUID conflicting identity or observation binding changed"
      end
      guid_comparable(fixtures + warm + [b, a], inputs, evidence.fetch("seed_clock"), expected.fetch("source_metadata_conflict").first)
    end

    def guid_comparable(frames, inputs, seed_clock, conflict)
      copied = Marshal.load(Marshal.dump(frames))
      copied.each do |frame|
        %w[tables_before tables_after tables_finish].each do |key|
          frame.fetch(key).fetch("source_metadata_conflict").each do |row|
            next unless row.fetch("conflict_type") == "source_guid"

            raise Failure, "GUID unexpected conflict text" unless row == conflict

            row["existing_value"] = "<validated-guid-conflict-source-public-id>"
          end
        end
      end
      { application: compilation_comparable("fixture" => nil, "frames" => copied),
        inputs: policy_comparable_inputs("inputs_before" => inputs, "seed_clocks" => { "guid" => seed_clock }) }
    end

    def guid_isolated(kind, mode, variant, source, role, observed:)
      database = "ingestion_guid_#{variant}"
      name = "#{kind}-#{mode}-#{variant}-#{observed ? 'observed' : 'plain'}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\nUPDATE public.trust_tier SET created_at = transaction_timestamp();\nUPDATE public.media_domain SET created_at = transaction_timestamp();"
        clock = policy_setup!(seed, database, File.join(@guid_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        fixture = wrapper_arguments("unused", title: "GUID target").merge(source_guid_input: "NULL::varchar")
        fixtures = [guid_execute(fixture, database, role, "#{name}-fixture-a")]
        if kind == "competing-guid"
          fixtures << guid_execute(fixture.merge(infohash_v1_input: "repeat('b',40)::char(40)", title_raw_input: "'GUID contender'::varchar"), database, role, "#{name}-fixture-b")
        end
        evidence = { "seed_clock" => clock, "fixtures" => fixtures, "before" => ingestion_snapshot(database), "inputs_before" => policy_read_snapshot(database),
          "instrumentation" => guid_instrument!(database, kind, name) }
        guid_trace_observer!(database, name) if observed
        a = wrapper_arguments("wanted", minute: 3, seeders: 17, title: "GUID target refreshed")
        b = wrapper_arguments(kind == "competing-guid" ? "wanted" : "other", hash: kind == "competing-guid" ? "b" : "a", minute: 2, seeders: 11, title: "GUID contender refreshed")
        trace = observed ? { kind:, variant:, role: } : nil
        evidence.merge!(guid_interleave(database, role, a, b, mode, name, trace:))
        evidence.merge!("after" => ingestion_snapshot(database), "inputs_after" => policy_read_snapshot(database))
        metadata_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        comparable = guid_validate!(evidence, kind, mode, variant, role, observed:)
        check("GUID #{name} independent conflict and committed path", true)
        { name:, comparable: }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_guid!
      directory = File.join(@contract.output_path, "ingestion-guid")
      raise Failure, "GUID evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @guid_evidence = Dir.mktmpdir("run-", directory)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = @guid_evidence
      hashes = guid_source_hashes
      first = @checks.length
      cases = []
      completed = false
      begin
        GUID_KINDS.product(GUID_MODES).each do |kind, mode|
          pair = { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.to_h do |variant, (source, role)|
            plain = guid_isolated(kind, mode, variant, source, role, observed: false)
            observed = guid_isolated(kind, mode, variant, source, role, observed: true)
            same = JSON.generate(plain.fetch(:comparable)) == JSON.generate(observed.fetch(:comparable))
            check("GUID #{kind}-#{mode}-#{variant} trace preserves complete application evidence", same)
            raise Failure, "GUID trace changed application evidence" unless same

            [variant, observed]
          end
          equivalent = JSON.generate(pair.fetch("reference").fetch(:comparable)) == JSON.generate(pair.fetch("final").fetch(:comparable))
          check("GUID #{kind}-#{mode} complete result and 18-table path parity", equivalent)
          cases << { kind:, mode:, equivalent: }
          raise Failure, "GUID path comparison failed" unless equivalent
        end
        check("GUID source bytes unchanged", hashes == guid_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
          candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
          postgres_image: @contract.postgres_image, source_sha256: hashes, checks:, cases:,
          limitations: ["Controlled scheduling uses one disposable-only barrier in each variant; no uninstrumented timing guarantee.",
            "GUID logger in-call and caller settings are verified; other helper/native closure remains unproven.",
            "Legacy competing-GUID observation reuse is retained, not repaired or newly approved."] }
        metadata_write("report.json", JSON.pretty_generate(report) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end

    def guid_source_hashes
      path = "scripts/tests/database-ingestion-guid-test.rb"
      wrapper_source_hashes.merge(path => Digest::SHA256.file(File.join(@contract.root, path)).hexdigest)
    end
  end
end
