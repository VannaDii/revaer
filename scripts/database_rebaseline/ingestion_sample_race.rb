# frozen_string_literal: true

require "open3"

module RevaerDatabaseRebaseline
  # K3 only: a direct writer removes locked samples after ingestion skips a duplicate.
  module IngestionSampleRace
    SAMPLE_RACE_IDS = {
      "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000090",
      "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-000000000091"
    }.freeze
    SAMPLE_RACE_PRIOR = "2026-09-09T00:00:00+00:00"
    SAMPLE_RACE_GUARD = "            IF sample_count IS NOT NULL AND sample_count > 0 THEN"
    SAMPLE_RACE_SITE = <<~'SQL'.freeze
                  ON CONFLICT DO NOTHING;

                  DELETE FROM canonical_size_sample
                  WHERE canonical_torrent_id = canonical_id
                    AND canonical_size_sample_id IN (
                        SELECT canonical_size_sample_id
                        FROM canonical_size_sample
                        WHERE canonical_torrent_id = canonical_id
                        ORDER BY observed_at DESC
                        OFFSET 25
                    );

                  SELECT COUNT(*), percentile_cont(0.5) WITHIN GROUP (ORDER BY size_bytes),
                         MIN(size_bytes), MAX(size_bytes)
                  INTO sample_count, sample_median, sample_min, sample_max
                  FROM canonical_size_sample
                  WHERE canonical_torrent_id = canonical_id;
    SQL

    private

    def sample_race_require!(condition, label)
      raise Failure, "sample race #{label}" unless condition
    end

    def sample_race_session(mode)
      sample_race_require!(IngestionSampling::SAMPLING_MODES.include?(mode), "unknown compilation mode")
      { calls: [wrapper_arguments("size-source", minute: 25, title: IngestionSize::SIZE_TITLE)],
        wrapper: true, helpers: mode == "helpers-first", finish_setting: true }
    end

    def sample_race_call
      ingestion_call(sample_race_session("cold").fetch(:calls).first).sub("public.search_result_ingest_v1(", "public.search_result_ingest(")
    end

    def sample_race_database(variant)
      "ingestion_sample_race_#{Process.pid}_#{variant}"
    end

    def sample_race_context_sql
      <<~SQL.strip
        json_build_object('pid', pg_backend_pid(), 'database', current_database(),
          'session', session_user, 'current', current_user,
          'transaction', pg_current_xact_id()::text, 'clock', transaction_timestamp(),
          'isolation', current_setting('transaction_isolation'),
          'setting', current_setting('plpgsql.variable_conflict'), 'application', current_setting('application_name'))
      SQL
    end

    def sample_race_seed_sql
      base = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
      <<~SQL
        \\set VERBOSITY verbose
        SET statement_timeout = '20s';
        DO $$ BEGIN NULL; END $$;
        SET application_name = 'sample-race-seed';
        BEGIN;
        #{base}
        UPDATE public.trust_tier SET created_at = transaction_timestamp();
        UPDATE public.media_domain SET created_at = transaction_timestamp();
        INSERT INTO public.canonical_torrent (canonical_torrent_public_id, identity_confidence, identity_strategy,
          infohash_v1, magnet_hash, title_display, title_normalized, size_bytes)
        VALUES (#{literal(SAMPLE_RACE_IDS.fetch('canonical_torrent_public_id'))}, 1.0, 'infohash_v1',
          repeat('a', 40), #{literal(IngestionAttributes::ATTRIBUTE_MAGNET)}, 'Size proof', 'size proof', 1024);
        INSERT INTO public.canonical_torrent_source (indexer_instance_id, canonical_torrent_source_public_id,
          source_guid, infohash_v1, magnet_hash, title_normalized, size_bytes, last_seen_at, last_seen_seeders, last_seen_leechers)
        VALUES (569001, #{literal(SAMPLE_RACE_IDS.fetch('canonical_torrent_source_public_id'))}, 'size-source',
          repeat('a', 40), #{literal(IngestionAttributes::ATTRIBUTE_MAGNET)}, 'size proof', 1024, '2026-09-10T00:00:00Z', 5, 2);
        INSERT INTO public.canonical_size_sample (canonical_torrent_id, observed_at, size_bytes)
        SELECT 1, '2026-09-10T00:00:00Z'::timestamptz + n * interval '1 minute', 1024
        FROM generate_series(0, 25) n ORDER BY n;
        INSERT INTO public.canonical_size_rollup (canonical_torrent_id, sample_count, size_median, size_min, size_max, updated_at)
        VALUES (1, 26, 1024, 1024, 1024, #{literal(SAMPLE_RACE_PRIOR)});
        SELECT #{sample_race_context_sql};
        COMMIT;
      SQL
    end

    def sample_race_samples
      (0..25).map do |minute|
        { "canonical_size_sample_id" => minute + 1, "canonical_torrent_id" => 1,
          "observed_at" => sampling_observed(minute), "size_bytes" => 1024 }
      end
    end

    def sample_race_tables(seed_clock, call_clock = nil)
      tables = size_expected_tables({ bytes: 1024, sampled: true, fallback: false },
        { "clock" => call_clock || seed_clock, "result" => SAMPLE_RACE_IDS }, 0)
      %w[canonical_torrent canonical_torrent_source].each { |table| tables.fetch(table).first["created_at"] = seed_clock }
      tables["canonical_size_sample"] = call_clock ? [] : sample_race_samples
      tables.fetch("canonical_size_rollup").first.merge!("sample_count" => 26, "updated_at" => SAMPLE_RACE_PRIOR)
      if call_clock
        tables.fetch("canonical_torrent_source").first["last_seen_at"] = sampling_observed(25)
        tables.fetch("search_request_source_observation").first["observed_at"] = sampling_observed(25)
      else
        retained = %w[canonical_torrent canonical_torrent_source canonical_size_sample canonical_size_rollup]
        tables.each_key { |table| tables[table] = [] unless retained.include?(table) }
      end
      tables
    end

    def sample_race_routine(variant)
      sample_race_require!(%w[reference final].include?(variant), "unknown variant")
      @ingestion_inventory.fetch(variant == "reference" ? "reference_proof" : @database).fetch("routines")
        .find { |routine| routine.fetch("name") == "search_result_ingest_v1" } || raise(Failure, "sample race missing routine")
    end

    def sample_race_instrument(source)
      site = SAMPLE_RACE_SITE.lines.map { |line| line.strip.empty? ? line : "            #{line}" }.join
      sample_race_require!(source.scan(site + "\n" + SAMPLE_RACE_GUARD).one?, "count/prune source site changed")
      hook = "RAISE NOTICE 'ingestion-sample-race:%', json_build_object(" \
        "'sample_count', sample_count, 'median', sample_median, 'minimum', sample_min, 'maximum', sample_max, " \
        "'canonical_id', canonical_id, 'allowed', size_sample_allowed, 'observed_at', observed_at_value, " \
        "'size', size_bytes_input, 'backend', pg_backend_pid(), 'transaction', pg_current_xact_id()::text, " \
        "'session', session_user, 'current', current_user, 'setting', current_setting('plpgsql.variable_conflict'), " \
        "'clock', transaction_timestamp()); "
      # Keeping the hook on the guard's line preserves all original error coordinates, including D4.
      source.sub(SAMPLE_RACE_GUARD, "            #{hook}#{SAMPLE_RACE_GUARD.strip}")
    end

    def sample_race_definition!(database, variant, name, observed:)
      query = "SELECT json_build_object('source', p.prosrc, 'definition', pg_get_functiondef(p.oid)) FROM pg_proc p WHERE p.pronamespace='public'::regnamespace AND p.proname='search_result_ingest_v1';"
      raw = metadata_transport(query, database, "postgres", "#{name}-original")
      original = metadata_read_parse(raw)
      source = sample_race_routine(variant).fetch("source")
      sample_race_require!(original.fetch("source") == source && original.fetch("definition").scan(source).one?, "exact installed source changed")
      changed = sample_race_instrument(source)
      definition = original.fetch("definition").sub(source) { changed }
      if observed
        install = metadata_transport(definition, database, "postgres", "#{name}-observer")
        sample_race_require!(install.values.all?(&:empty?), "observer install diagnostic")
      end
      after = metadata_read_parse(metadata_transport(query, database, "postgres", "#{name}-definition-check"))
      expected = observed ? { "source" => changed, "definition" => definition } : original
      sample_race_require!(after == expected, "observer altered unrelated definition bytes")
      { "original_sha256" => Digest::SHA256.hexdigest(source),
        "tested_sha256" => Digest::SHA256.hexdigest(observed ? changed : source) }
    end

    def sample_race_trace!(stderr, context, variant, observed:)
      unless observed
        sample_race_require!(stderr.empty?, "unexpected plain diagnostic (including D4)")
        return
      end
      match = stderr.match(/\ANOTICE:  00000: ingestion-sample-race:(\{[^\n]+\})\nCONTEXT:  (.+)LOCATION:  exec_stmt_raise, pl_exec.c:3897\n\z/m)
      sample_race_require!(match, "missing or additional in-call diagnostic")
      routine = sample_race_routine(variant)
      line = routine.fetch("source").lines.index { |text| text.chomp == SAMPLE_RACE_GUARD }
      sample_race_require!(line, "missing guard line")
      stack = "PL/pgSQL function #{routine.fetch('signature')} line #{line + 1} at RAISE\n#{disambiguation_wrapper_stack}"
      event = { "sample_count" => 0, "median" => nil, "minimum" => nil, "maximum" => nil,
        "canonical_id" => 1, "allowed" => true, "observed_at" => sampling_observed(25), "size" => 1024,
        "backend" => context.fetch("pid"), "transaction" => context.fetch("transaction"),
        "session" => context.fetch("session"), "current" => variant == "reference" ? "postgres" : @owner,
        "setting" => variant == "reference" ? "use_column" : "error", "clock" => context.fetch("clock") }
      sample_race_require!(match[2] == stack && JSON.generate(metadata_json_parse(match[1])) == JSON.generate(event), "exact count/stack/transaction trace changed")
    end

    def sample_race_context!(context, database, role, application)
      sample_race_require!(context.keys.sort == %w[application clock current database isolation pid session setting transaction] &&
        context.values_at("database", "session", "current", "application", "isolation", "setting") ==
          [database, role, role, application, "read committed", "error"] &&
        context.fetch("pid").is_a?(Integer) && context.fetch("pid").positive? &&
        context.fetch("transaction").is_a?(String) && context.fetch("transaction").match?(/\A[1-9][0-9]*\z/) &&
        metadata_clock?(context.fetch("clock")), "direct role/backend/transaction/caller provenance changed")
    end

    def sample_race_locked!(locked, database)
      sample_race_context!(locked.fetch("context"), database, "postgres", "sample-race-writer")
      rows = locked.fetch("rows")
      sample_race_require!(locked.keys.sort == %w[context rows] && rows.length == 26 &&
        size_tables_equal?({ "samples" => rows.map { |row| row.reject { |key, _| %w[tuple relation].include?(key) } } },
          { "samples" => sample_race_samples }) && rows.map { |row| row.fetch("canonical_size_sample_id") } == (1..26).to_a &&
        rows.all? { |row| row.fetch("tuple").match?(/\A\([0-9]+,[1-9][0-9]*\)\z/) } &&
        rows.map { |row| row.fetch("tuple") }.uniq.length == 26 &&
        rows.map { |row| row.fetch("relation") }.uniq.one? && rows.first.fetch("relation").is_a?(Integer) &&
        rows.first.fetch("relation").positive?, "26 exact locked tuples changed")
    end

    def sample_race_wait_sql(database, caller, locked)
      writer = locked.fetch("context")
      <<~SQL
        \\set VERBOSITY verbose
        DO $$ BEGIN NULL; END $$;
        SET statement_timeout = '5s';
        SELECT json_build_object('reader', #{sample_race_context_sql}, 'query_bytes', pg_size_bytes(current_setting('track_activity_query_size')),
          'activity', (SELECT row_to_json(r) FROM (SELECT a.pid, a.datname AS database, a.usename AS role,
            a.application_name AS application, a.state, a.wait_event_type AS wait_type, a.wait_event,
            a.backend_xid::text AS transaction, a.query, pg_blocking_pids(a.pid) AS blockers
            FROM pg_stat_activity a WHERE a.pid = #{Integer(caller.fetch('pid'))} AND a.datname = #{literal(database)}) r),
          'locks', (SELECT COALESCE(json_agg(row_to_json(r) ORDER BY to_jsonb(r)::text), '[]') FROM
            (SELECT pid, locktype, mode, granted, relation::bigint AS relation, page, tuple, transactionid::text AS transaction
             FROM pg_locks WHERE pid IN (#{Integer(caller.fetch('pid'))}, #{Integer(writer.fetch('pid'))})) r))::text;
      SQL
    end

    def sample_race_wait!(wait, caller, locked, database, role)
      sample_race_context!(wait.fetch("reader"), database, "postgres", "psql")
      sample_race_locked!(locked, database)
      sample_race_context!(caller, database, role, "sample-race-ingest")
      writer = locked.fetch("context")
      sample_race_require!(%w[pid transaction clock].all? do |key|
        [caller, writer, wait.fetch("reader")].map { |context| context.fetch(key) }.uniq.length == 3
      end, "wait backend/transaction ownership changed")
      row = wait.fetch("activity")
      bytes = wait.fetch("query_bytes")
      sample_race_require!(wait.keys.sort == %w[activity locks query_bytes reader] && bytes.eql?(1024) &&
        row.keys.sort == %w[application blockers database pid query role state transaction wait_event wait_type] &&
        row.values_at("pid", "database", "role", "application", "state", "wait_type", "wait_event", "transaction", "blockers", "query") ==
          [caller.fetch("pid"), database, role, "sample-race-ingest", "active", "Lock", "transactionid",
           caller.fetch("transaction"), [writer.fetch("pid")], sample_race_call.strip.byteslice(0, bytes - 1)], "exact owned pruning wait changed")
      victim = locked.fetch("rows").first
      page, tuple = victim.fetch("tuple").delete("()").split(",").map { |value| Integer(value) }
      shape = { "relation" => nil, "page" => nil, "tuple" => nil, "transaction" => nil }
      expected = [
        shape.merge("pid" => caller.fetch("pid"), "locktype" => "transactionid", "mode" => "ShareLock", "granted" => false, "transaction" => writer.fetch("transaction")),
        shape.merge("pid" => writer.fetch("pid"), "locktype" => "transactionid", "mode" => "ExclusiveLock", "granted" => true, "transaction" => writer.fetch("transaction")),
        shape.merge("pid" => caller.fetch("pid"), "locktype" => "tuple", "mode" => "AccessExclusiveLock", "granted" => true, "relation" => victim.fetch("relation"), "page" => page, "tuple" => tuple),
        shape.merge("pid" => caller.fetch("pid"), "locktype" => "relation", "mode" => "RowExclusiveLock", "granted" => true, "relation" => victim.fetch("relation")),
        shape.merge("pid" => writer.fetch("pid"), "locktype" => "relation", "mode" => "RowShareLock", "granted" => true, "relation" => victim.fetch("relation"))
      ]
      locks = wait.fetch("locks")
      sample_race_require!(locks.all? { |lock| lock.keys.sort == expected.first.keys.sort && [caller.fetch("pid"), writer.fetch("pid")].include?(lock.fetch("pid")) } &&
        expected.all? { |lock| locks.count(lock) == 1 } && locks.select { |lock| lock.fetch("granted") == false } == [expected.first] &&
        locks.select { |lock| lock.fetch("locktype") == "tuple" } == [expected.fetch(2)], "oldest tuple/transaction lock witness changed")
    end

    def sample_race_line(transcript, prefix)
      lines = transcript.lines(chomp: true).select { |line| line.start_with?("#{prefix}:") }
      sample_race_require!(lines.one?, "missing or duplicate #{prefix} record")
      metadata_json_parse(lines.first.delete_prefix("#{prefix}:"))
    end

    def sample_race_send(process, query)
      process.fetch(:sql) << query
      process.fetch(:input).write(query)
      process.fetch(:input).flush
    end

    def sample_race_marker(process, marker)
      guid_controller_read(process.fetch(:output), marker, process.fetch(:stdout))
    end

    def sample_race_reap(waiter)
      return false if waiter.join(2)

      begin
        Process.kill("TERM", -waiter.pid)
        Process.kill("KILL", -waiter.pid) unless waiter.join(2)
      rescue Errno::ESRCH
        sample_race_require!(waiter.join(2), "owned process disappeared without reaping")
      end
      sample_race_require!(waiter.join(5), "owned process did not reap")
      true
    end

    def sample_race_process(database, role, name)
      Open3.popen3(*command(role, database), pgroup: true) do |input, output, error, waiter|
        process = { input:, output:, waiter:, sql: +"", stdout: +"" }
        stderr = Thread.new { error.read }
        begin
          yield process
        ensure
          primary = $!
          input.close unless input.closed?
          tail = Thread.new { output.read }
          forced = sample_race_reap(waiter)
          sample_race_require!(tail.join(5) && stderr.join(5), "owned process pipes did not settle")
          process.fetch(:stdout) << tail.value
          status = waiter.value
          process[:record] = { "stdout" => process.fetch(:stdout), "stderr" => stderr.value }
          metadata_write("#{name}.sql", process.fetch(:sql))
          metadata_write("#{name}.stdout", process.fetch(:stdout))
          metadata_write("#{name}.stderr", stderr.value)
          metadata_write("#{name}-status.json", JSON.generate({ success: status.success?, exitstatus: status.exitstatus,
            termsig: status.termsig, forced:, controller_error: primary&.message }) + "\n")
          sample_race_require!(status.success? && !forced, "owned process failed or required termination") unless primary
        end
      end
    end

    def sample_race_interleave(database, role, mode, name, evidence)
      writer_process = nil
      sample_race_process(database, "postgres", "#{name}-writer") do |writer|
        writer_process = writer
        sample_race_send(writer, <<~SQL)
          \\set VERBOSITY verbose
          DO $$ BEGIN NULL; END $$;
          SET application_name = 'sample-race-writer';
          SET statement_timeout = '20s';
          SET idle_in_transaction_session_timeout = '30s';
          BEGIN;
          WITH locked AS MATERIALIZED (SELECT s.*, s.ctid::text AS tuple, s.tableoid::bigint AS relation
            FROM public.canonical_size_sample s WHERE canonical_torrent_id = 1 ORDER BY observed_at FOR UPDATE)
          SELECT 'locked:' || json_build_object('context', #{sample_race_context_sql},
            'rows', (SELECT json_agg(row_to_json(locked) ORDER BY observed_at) FROM locked))::text;
          \\echo sample_race_locked
        SQL
        sample_race_marker(writer, "sample_race_locked")
        locked = sample_race_line(writer.fetch(:stdout), "locked")
        sample_race_locked!(locked, database)
        caller_process = nil
        sample_race_process(database, role, "#{name}-caller") do |caller|
          caller_process = caller
          query = correction_session(sample_race_session(mode)).sub("SAVEPOINT operation;",
            "SELECT 'race_context:' || #{sample_race_context_sql}::text;\n\\echo sample_race_ready\nSAVEPOINT operation;")
          sample_race_send(caller, "\\set SHOW_CONTEXT always\nSET application_name = 'sample-race-ingest';\n#{query}\n\\echo sample_race_finished\n")
          sample_race_marker(caller, "sample_race_ready")
          context = sample_race_line(caller.fetch(:stdout), "race_context")
          deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 10
          attempt = 0
          loop do
            raw = metadata_transport(sample_race_wait_sql(database, context, locked), database, "postgres", "#{name}-wait-#{attempt}")
            wait = metadata_read_parse(raw)
            if wait.fetch("activity")&.fetch("wait_type") == "Lock"
              sample_race_wait!(wait, context, locked, database, role)
              evidence["wait"] = raw.merge("data" => wait)
              break
            end
            sample_race_require!(caller.fetch(:waiter).alive? && Process.clock_gettime(Process::CLOCK_MONOTONIC) < deadline, "caller did not reach pruning wait")
            attempt += 1
            sleep 0.02
          end
          evidence["waiting"] = sampling_read(database, "#{name}-waiting")
          seed_clock = metadata_read_parse(evidence.fetch("seed")).fetch("clock")
          sample_race_require!(sampling_read?(evidence.fetch("waiting"), sample_race_tables(seed_clock), metadata_read_tables(seed_clock), 27), "pre-release state or duplicate sequence changed")
          sample_race_send(writer, <<~SQL)
            WITH deleted AS (DELETE FROM public.canonical_size_sample WHERE canonical_torrent_id = 1 RETURNING *)
            SELECT 'deleted:' || json_build_object('context', #{sample_race_context_sql},
              'rows', (SELECT json_agg(row_to_json(deleted) ORDER BY canonical_size_sample_id) FROM deleted))::text;
            SELECT 'writer_tables:' || ingestion_observation.snapshot()::text;
            COMMIT;
            SELECT 'released:' || #{sample_race_context_sql}::text;
            \\echo sample_race_released
          SQL
          sample_race_marker(writer, "sample_race_released")
          sample_race_marker(caller, "sample_race_finished")
        end
        evidence["caller"] = caller_process.fetch(:record)
      end
      evidence["writer"] = writer_process.fetch(:record)
    end

    def sample_race_validate!(evidence, mode, variant, role, observed:)
      database = evidence.fetch("database")
      sample_race_require!(database == sample_race_database(variant), "owned database changed")
      sample_race_require!(role == (variant == "reference" ? "postgres" : @runtime), "variant role changed")
      seed = metadata_read_parse(evidence.fetch("seed"))
      sample_race_context!(seed, database, "postgres", "sample-race-seed")
      source = sample_race_routine(variant).fetch("source")
      sample_race_require!(evidence.fetch("definition") == { "original_sha256" => Digest::SHA256.hexdigest(source),
        "tested_sha256" => Digest::SHA256.hexdigest(observed ? sample_race_instrument(source) : source) }, "source identity changed")
      raw = evidence.fetch("caller")
      context = sample_race_line(raw.fetch("stdout"), "race_context")
      sample_race_context!(context, database, role, "sample-race-ingest")
      lines = raw.fetch("stdout").lines(chomp: true)
      context_index = lines.index { |line| line.start_with?("race_context:") }
      sample_race_require!(lines.count("sample_race_ready") == 1 && lines.count("sample_race_finished") == 1 &&
        lines.last == "sample_race_finished" && context_index.positive? &&
        lines.fetch(context_index - 1).start_with?("tables_before:") && lines.fetch(context_index + 1) == "sample_race_ready", "caller markers changed")
      stdout = raw.fetch("stdout").lines.reject { |line| line.start_with?("race_context:") || %w[sample_race_ready sample_race_finished].include?(line.strip) }.join
      metadata_transport_json!(stdout)
      frames = correction_frames(stdout, sample_race_session(mode))
      frame = frames.fetch(0)
      sample_race_require!(validation_context?(frames, role) && frame.fetch("backend") == context.fetch("pid").to_s &&
        frame.fetch("clock") == context.fetch("clock") && frame.fetch("finished_setting") == "error", "caller frame provenance changed")
      sample_race_require!(frame.fetch("state") == "00000", "required ingestion success failed; D4 is not success")
      sample_race_trace!(raw.fetch("stderr"), context, variant, observed:)
      before = sample_race_tables(seed.fetch("clock"))
      after = sample_race_tables(seed.fetch("clock"), context.fetch("clock"))
      result = SAMPLE_RACE_IDS.merge("canonical_changed" => false, "observation_created" => true, "durable_source_created" => false)
      sample_race_require!(frame.fetch("result") == result && frame.values_at("within", "outside") == ["true", (variant == "reference").to_s] &&
        size_tables_equal?(frame.fetch("tables_before"), before) && size_tables_equal?(frame.fetch("tables_after"), after) &&
        size_tables_equal?(frame.fetch("tables_finish"), after), "complete 18-table transition or rollup changed")
      writer = evidence.fetch("writer")
      sample_race_require!(writer.fetch("stderr").empty?, "writer diagnostic")
      lines = writer.fetch("stdout").lines(chomp: true)
      sample_race_require!(lines.map { |line| line.split(":", 2).first } ==
        %w[locked sample_race_locked deleted writer_tables released sample_race_released], "writer schedule framing changed")
      locked = sample_race_line(writer.fetch("stdout"), "locked")
      deleted = sample_race_line(writer.fetch("stdout"), "deleted")
      released = sample_race_line(writer.fetch("stdout"), "released")
      sample_race_locked!(locked, database)
      sample_race_context!(released, database, "postgres", "sample-race-writer")
      sample_race_require!(deleted == { "context" => locked.fetch("context"), "rows" => sample_race_samples } &&
        released.fetch("pid") == locked.fetch("context").fetch("pid") &&
        size_tables_equal?(sample_race_line(writer.fetch("stdout"), "writer_tables"), before.merge("canonical_size_sample" => [])), "writer deletion/commit state changed")
      actors = [seed, locked.fetch("context"), context, released]
      sample_race_require!(actors.map { |actor| actor.fetch("transaction") }.uniq.length == 4 &&
        actors.map { |actor| actor.fetch("clock") }.uniq.length == 4 &&
        [seed, locked.fetch("context"), context].map { |actor| actor.fetch("pid") }.uniq.length == 3, "transaction separation changed")
      wait = metadata_read_parse(evidence.fetch("wait"))
      sample_race_require!(JSON.generate(wait) == JSON.generate(evidence.fetch("wait").fetch("data")), "raw wait differs")
      sample_race_wait!(wait, context, locked, database, role)
      inputs = metadata_read_tables(seed.fetch("clock"))
      sample_race_require!(sampling_read?(evidence.fetch("initial"), before, inputs, 26) &&
        sampling_read?(evidence.fetch("waiting"), before, inputs, 27) && sampling_read?(evidence.fetch("after"), after, inputs, 27), "independent state/inputs/sequence changed")
      { "result" => frame.fetch("result"), "before" => sample_race_normalize(frame.fetch("tables_before"), seed, context),
        "after" => sample_race_normalize(frame.fetch("tables_after"), seed, context),
        "finish" => sample_race_normalize(frame.fetch("tables_finish"), seed, context),
        "inputs" => policy_comparable_inputs("inputs_before" => metadata_read_parse(evidence.fetch("after")).fetch("inputs"),
          "seed_clocks" => { "sample-race" => seed.fetch("clock") }), "state" => frame.fetch("state"), "sample_sequence" => [26, 27, 27] }
    rescue KeyError, TypeError, NoMethodError, ArgumentError => error
      raise Failure, "sample race malformed evidence: #{error.message}"
    end

    def sample_race_normalize(tables, seed, context)
      columns = { "canonical_torrent" => %w[created_at updated_at], "canonical_torrent_source" => %w[created_at updated_at],
        "canonical_torrent_source_context_score" => %w[computed_at], "canonical_torrent_best_source_context" => %w[computed_at],
        "search_request_canonical" => %w[first_seen_at] }
      tables.to_h do |table, rows|
        [table, rows.map do |row|
          row.to_h do |key, value|
            if columns.fetch(table, []).include?(key)
              value = "seed-clock" if value == seed.fetch("clock")
              value = "call-clock" if value == context.fetch("clock")
            end
            [key, value]
          end
        end]
      end
    end

    def sample_race_isolated(mode, variant, source, role, observed:)
      name = "#{mode}-#{variant}-#{observed ? 'observed' : 'plain'}"
      database = sample_race_database(variant)
      created = false
      evidence = { "database" => database }
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        evidence["seed"] = metadata_transport(sample_race_seed_sql, database, "postgres", "#{name}-seed")
        correction_observer!(database, variant)
        evidence["initial"] = sampling_read(database, "#{name}-initial")
        evidence["definition"] = sample_race_definition!(database, variant, name, observed:)
        sample_race_interleave(database, role, mode, name, evidence)
        evidence["after"] = sampling_read(database, "#{name}-after")
        sample_race_validate!(evidence, mode, variant, role, observed:)
      ensure
        primary = $!
        metadata_write("#{name}.json", JSON.pretty_generate({ evidence:, failure: primary&.message }) + "\n")
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_sample_race!
      directory = File.join(@contract.output_path, "ingestion-sample-race")
      sample_race_require!(!File.symlink?(directory), "evidence directory is a symlink")
      FileUtils.mkdir_p(directory, mode: 0o700)
      @sample_race_evidence = Dir.mktmpdir("run-", directory)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = @sample_race_evidence
      hashes = wrapper_source_hashes.merge("scripts/tests/database-ingestion-sample-race-test.rb" =>
        Digest::SHA256.file(File.join(@contract.root, "scripts/tests/database-ingestion-sample-race-test.rb")).hexdigest)
      completed = false
      cases = []
      begin
        IngestionSampling::SAMPLING_MODES.each do |mode|
          pair = { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.map do |variant, (source, role)|
            plain = sample_race_isolated(mode, variant, source, role, observed: false)
            traced = sample_race_isolated(mode, variant, source, role, observed: true)
            sample_race_require!(JSON.generate(plain) == JSON.generate(traced), "observer changed validated application evidence")
            plain
          end
          sample_race_require!(JSON.generate(pair.first) == JSON.generate(pair.last), "frozen/final full-state comparison failed")
          check("sample race #{mode} exact wait, zero in-call count and full-state parity", true)
          cases << { mode:, equivalent: true, plain_observed_equivalent: true }
        end
        sample_race_require!(hashes.all? { |path, digest| Digest::SHA256.file(File.join(@contract.root, path)).hexdigest == digest }, "source bytes changed")
        completed = true
      ensure
        primary = $!
        metadata_write("report.json", JSON.pretty_generate({ completed:, passed: completed, d3_complete: false,
          failure: primary&.message, cases:, source_sha256: hashes, candidate_sha256: @contract.expected_candidate_sha256,
          final_sha256: @contract.final_sha256, postgres_image: @contract.postgres_image,
          limitations: ["K3 only; direct administrator writer in disposable clones, not a runtime writer API.",
            "Cold/helper-first first ingestion calls only; any D4 failure fails this case and is retained.",
            "Plain/NOTICE pairs do not establish native callbacks, other sequence behavior or complete D3."] }) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end
  end
end
