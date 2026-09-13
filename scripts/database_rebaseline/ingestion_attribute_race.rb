# frozen_string_literal: true

require_relative "ingestion_guid"

module RevaerDatabaseRebaseline
  # K2 only. The administrator writes children of a committed source; neither
  # ingestion definitions nor constraints are changed to schedule the conflict.
  module IngestionAttributeRace
    include IngestionGuid
    ATTRIBUTE_RACE_KINDS = %w[eight-non-id eleven-all-id].freeze
    ATTRIBUTE_RACE_MODES = %w[cold helpers-first].freeze
    ATTRIBUTE_RACE_ORDER = %w[tracker_name tracker_category tracker_subcategory size_bytes_reported files_count imdb_id tmdb_id tvdb_id season episode year].freeze
    ATTRIBUTE_RACE_SEEDED_SEQUENCES = %w[canonical_torrent canonical_torrent_source canonical_torrent_source_context_score canonical_torrent_best_source_context search_request_source_observation search_request_canonical search_page search_page_item].freeze

    private

    def attribute_race_shape(kind, fixture: false)
      raise Failure, "attribute race unknown kind" unless ATTRIBUTE_RACE_KINDS.include?(kind)

      shape = metadata_cases.first.fetch(:fixtures).first.merge(title: "Proof", normalized: "proof", release: nil,
        inputs: [], answers: [], signals: [], conflicts: [])
      return shape if fixture

      ids = kind == "eleven-all-id" ? [["imdb_id", :text, "tt7654321"], ["tmdb_id", :int, 234_567], ["tvdb_id", :int, 765_432]] : []
      shape.merge(inputs: IngestionMetadata::METADATA_INPUTS.first(8) + ids,
        answers: IngestionMetadata::METADATA_ANSWERS.first(8) + ids, external_ids: !ids.empty?,
        observed: "2026-09-11T00:00:00+00:00", seeders: 9, leechers: 4,
        signals: [["year", nil, 2027], ["season", nil, 3], ["episode", nil, 8]])
    end

    def attribute_race_writer_rows(kind)
      raise Failure, "attribute race unknown kind" unless ATTRIBUTE_RACE_KINDS.include?(kind)

      ids = kind == "eleven-all-id" ? [["imdb_id", :text, "tt1234567"], ["tmdb_id", :int, 123_456], ["tvdb_id", :int, 654_321]] : []
      (IngestionAttributes::ATTRIBUTE_ANSWERS.first(8) + ids).each_with_index.map do |(key, type, value), index|
        attributes_value_columns(type, value, uuid: false).merge("canonical_torrent_source_attr_id" => index + 1,
          "canonical_torrent_source_id" => 1, "attr_key" => key)
      end
    end

    def attribute_race_query(kind, mode, actor)
      raise Failure, "attribute race unknown mode or actor" unless ATTRIBUTE_RACE_MODES.include?(mode) && %w[fixture a b].include?(actor)

      session = guid_session(metadata_arguments(attribute_race_shape(kind, fixture: actor == "fixture")), helpers: actor == "a" && mode == "helpers-first")
      query = correction_session(session) do |arguments|
        if actor == "b"
          columns = %w[canonical_torrent_source_id attr_key value_text value_int value_bigint value_numeric value_bool]
          tuples = attribute_race_writer_rows(kind).map { |row| "(#{columns.map { |column| policy_sql_value(row.fetch(column)) }.join(',')})" }
          "WITH inserted AS (INSERT INTO public.canonical_torrent_source_attr (#{columns.join(',')}) VALUES #{tuples.join(',')} RETURNING canonical_torrent_source_attr_id) SELECT json_build_object('writer', 'direct-admin-durable-attrs', 'rows', count(*)) FROM inserted;"
        else
          ingestion_call(arguments)
        end
      end
      anchor = "SELECT 'backend:' || pg_backend_pid();"
      raise Failure, "attribute race session boundary changed" unless query.scan(anchor).one?

      context = "SELECT 'race_context:' || json_build_object('database', current_database(), 'application', current_setting('application_name'), 'isolation', current_setting('transaction_isolation'))::text;"
      "\\set SHOW_CONTEXT always\nSET application_name = 'attribute-race-#{actor}';\n" + query.sub(anchor, "#{context}\n#{anchor}")
    end

    def attribute_race_parse(raw, mode, actor)
      stdout = raw.fetch("stdout")
      metadata_transport_json!(stdout)
      lines = stdout.lines
      index = actor == "a" && mode == "helpers-first" ? 1 : 0
      context = lines.delete_at(index)
      raise Failure, "attribute race entry context missing or reordered" unless context&.start_with?("race_context:")

      frame = correction_frames(lines.join, guid_session({}, helpers: index == 1)).fetch(0)
      parsed = { "context" => metadata_json_parse(context.delete_prefix("race_context:")), "frame" => frame }
      raise Failure, "attribute race raw and declared evidence differ" if raw.key?("parsed") && raw.fetch("parsed") != parsed

      parsed
    end

    def attribute_race_execute(kind, mode, actor, database, role, name)
      raw = metadata_transport(attribute_race_query(kind, mode, actor), database, role, name)
      raw.merge("parsed" => attribute_race_parse(raw, mode, actor))
    end

    def attribute_race_read(database, name)
      # Reuse the metadata read-input/sequence transport and extend its seven
      # counters to every identity sequence belonging to the 18 write tables.
      record = metadata_read(database, name)
      tables = IngestionProof::INGESTION_TABLES - IngestionMetadata::METADATA_SEQUENCES.keys
      pairs = tables.map do |table|
        column = table == "search_request_source_observation" ? "observation_id" : "#{table}_id"
        "#{literal(table)}, (SELECT last_value FROM pg_catalog.pg_sequences WHERE schemaname = 'public' AND format('%I.%I', schemaname, sequencename)::regclass = pg_get_serial_sequence(#{literal("public.#{table}")}, #{literal(column)})::regclass)"
      end
      extra = metadata_transport("SELECT json_build_object(#{pairs.join(',')})::text;", database, "postgres", "#{name}-remaining-sequences")
      record.merge("remaining" => extra, "tables" => ingestion_snapshot(database))
    end

    def attribute_race_read_validate!(record, clock, counters, tables)
      data = metadata_read_parse(record)
      extra = metadata_read_parse(record.fetch("remaining"))
      unless record.fetch("data") == data && data.keys.sort == %w[inputs sequences] &&
          data.fetch("sequences").keys.sort == IngestionMetadata::METADATA_SEQUENCES.keys.sort &&
          extra.keys.sort == (IngestionProof::INGESTION_TABLES - IngestionMetadata::METADATA_SEQUENCES.keys).sort &&
          data.fetch("sequences").merge(extra) == counters && metadata_tables_equal?(data.fetch("inputs"), metadata_read_tables(clock)) &&
          metadata_tables_equal?(record.fetch("tables"), tables)
        raise Failure, "attribute race complete read inputs, sequences or table image changed"
      end
    end

    def attribute_race_wait(database, writer, name)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 10
      query = <<~SQL
        SELECT COALESCE(json_agg(row_to_json(r)), '[]') FROM (
          SELECT a.pid, a.usename, a.application_name, a.state, a.wait_event_type, a.wait_event,
            pg_blocking_pids(a.pid) AS blockers, l.locktype, l.mode, l.granted, l.transactionid::text AS xid,
            b.pid AS writer_pid, b.usename AS writer_role, b.application_name AS writer_application,
            b.state AS writer_state, b.backend_xid::text AS writer_xid, w.mode AS writer_mode, w.granted AS writer_granted,
            left(a.query, 69) AS query_prefix
          FROM pg_stat_activity a JOIN pg_locks l ON l.pid=a.pid
          JOIN pg_stat_activity b ON b.pid=#{Integer(writer.fetch('backend'))}
          JOIN pg_locks w ON w.pid=b.pid AND w.locktype='transactionid' AND w.transactionid=l.transactionid
          WHERE a.datname=#{literal(database)} AND b.datname=a.datname AND a.application_name='attribute-race-a'
            AND l.locktype='transactionid' AND NOT l.granted
        ) r;
      SQL
      loop do
        raw = metadata_transport(query, database, "postgres", "#{name}-wait-#{Process.clock_gettime(Process::CLOCK_MONOTONIC, :nanosecond)}")
        rows = metadata_read_parse(raw)
        return raw if rows.one?

        raise Failure, "attribute race exact owned lock wait absent" if !rows.empty? || Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline

        sleep 0.02
      end
    end

    def attribute_race_wait_validate!(raw, writer, backend, role)
      expected = { "pid" => Integer(backend), "usename" => role, "application_name" => "attribute-race-a", "state" => "active",
        "wait_event_type" => "Lock", "wait_event" => "transactionid", "blockers" => [Integer(writer.fetch("backend"))],
        "locktype" => "transactionid", "mode" => "ShareLock", "granted" => false, "xid" => writer.fetch("xid"),
        "writer_pid" => Integer(writer.fetch("backend")), "writer_role" => "postgres", "writer_application" => "attribute-race-b",
        "writer_state" => "idle in transaction", "writer_xid" => writer.fetch("xid"), "writer_mode" => "ExclusiveLock", "writer_granted" => true,
        "query_prefix" => "SELECT row_to_json(r) FROM public.search_result_ingest(search_request" }
      raise Failure, "attribute race exact owned lock wait changed" unless JSON.generate(metadata_read_parse(raw)) == JSON.generate([expected])
    end

    def attribute_race_interleave(kind, mode, database, role, name)
      transcript = +""
      paused = nil
      evidence = {}
      query = attribute_race_query(kind, mode, "b")
      pieces = query.split("COMMIT;", -1)
      raise Failure, "attribute race writer commit boundary changed" unless pieces.length == 2

      held = pieces.first + "SELECT 'writer_xid:' || pg_current_xact_id()::text; SELECT 'held';\n"
      release = "COMMIT; SELECT 'released';\n"
      finish = "#{pieces.last}\nSELECT 'finished';\n"
      metadata_write("#{name}-controller.sql", held + release + finish)
      Open3.popen3(*command("postgres", database)) do |input, output, error, waiter|
        error_reader = Thread.new { error.read }
        begin
          input.write(held)
          input.flush
          guid_controller_read(output, "held", transcript)
          writer = %w[backend writer_xid].to_h do |key|
            values = transcript.lines.grep(/^#{key}:/).map { |line| line.delete_prefix("#{key}:").strip }
            raise Failure, "attribute race writer identity missing or duplicated" unless values.one? && values.first.match?(/\A[1-9][0-9]*\z/)

            [key == "writer_xid" ? "xid" : key, values.first]
          end
          evidence["writer_identity"] = writer
          evidence["held"] = attribute_race_read(database, "#{name}-held")
          paused = Thread.new { attribute_race_execute(kind, mode, "a", database, role, "#{name}-a") }
          evidence["wait"] = attribute_race_wait(database, writer, name)
          backend = metadata_read_parse(evidence.fetch("wait")).fetch(0).fetch("pid").to_s
          attribute_race_wait_validate!(evidence.fetch("wait"), writer, backend, role)
          evidence["waiting"] = attribute_race_read(database, "#{name}-waiting")
          # Only A's first durable INSERT has allocated an identity at this point.
          # This distinguishes tracker_name from an unrelated wait on B's xid.
          count = attribute_race_writer_rows(kind).length
          data = evidence.fetch("waiting").fetch("data").fetch("sequences")
          unless data.fetch("canonical_torrent_source_attr") == count + 1 && data.fetch("search_request_source_observation_attr") == count && data.fetch("canonical_torrent_signal").nil?
            raise Failure, "attribute race tracker-name wait allocation changed"
          end
          metadata_write("#{name}-release-checkpoint.json", JSON.pretty_generate(evidence) + "\n")
          input.write(release)
          input.flush
          guid_controller_read(output, "released", transcript)
          raise Failure, "attribute race wrapper did not settle" unless paused.join(125)

          evidence["a"] = paused.value
          input.write(finish)
          input.flush
          guid_controller_read(output, "finished", transcript)
        ensure
          input.close unless input.closed?
          transcript << output.read
          stderr = error_reader.value
          success = waiter.value.success?
          metadata_write("#{name}-controller.json", JSON.pretty_generate({ stdout: transcript, stderr:, success: }) + "\n")
          raise Failure, "attribute race wrapper cleanup did not settle" if paused && !paused.join(125)
          raise Failure, "attribute race controller transport failed" unless success && stderr.empty?
        end
      end
      raise Failure, "attribute race controller markers changed" unless %w[held released finished].all? { |marker| transcript.lines.count("#{marker}\n") == 1 }

      raw = { "stdout" => transcript.lines.reject { |line| line.start_with?("writer_xid:") || %w[held released finished].include?(line.strip) }.join, "stderr" => "" }
      evidence.merge("b" => raw.merge("parsed" => attribute_race_parse(raw, mode, "b")), "controller" => transcript)
    end

    def attribute_race_observer!(database, name)
      query = <<~SQL
        CREATE FUNCTION ingestion_observation.trace_attribute_update() RETURNS trigger
        LANGUAGE plpgsql SET search_path TO pg_catalog AS $observer$
        BEGIN
          RAISE NOTICE 'ingestion-attribute-update:%', json_build_object(
            'schema', TG_TABLE_SCHEMA, 'relation', TG_TABLE_NAME, 'operation', TG_OP,
            'backend', pg_backend_pid()::text, 'session', session_user, 'current', current_user,
            'setting', current_setting('plpgsql.variable_conflict'), 'clock', transaction_timestamp(),
            'old', row_to_json(OLD), 'new', row_to_json(NEW));
          RETURN NEW;
        END;
        $observer$;
        REVOKE ALL ON FUNCTION ingestion_observation.trace_attribute_update() FROM PUBLIC;
        CREATE TRIGGER ingestion_attribute_update AFTER UPDATE ON public.canonical_torrent_source_attr
          FOR EACH ROW EXECUTE FUNCTION ingestion_observation.trace_attribute_update();
      SQL
      metadata_write("#{name}-trace.sql", query)
      sql(query, role: "postgres", database:)
    end

    def attribute_race_stack(key, variant)
      routine = @ingestion_inventory.fetch(variant == "reference" ? "reference_proof" : @database).fetch("routines").find { |row| row.fetch("name") == "search_result_ingest_v1" }
      source = routine.fetch("source")
      statements = source.scan(/INSERT INTO canonical_torrent_source_attr \(\n.*?;/m).select { |statement| statement.include?("'#{key}',") }
      raise Failure, "attribute race exact conflict UPDATE site changed" unless statements.one? && statements.first.include?("ON CONFLICT (canonical_torrent_source_id, attr_key)\n                DO UPDATE SET")

      statement = statements.first
      line = source[0...source.index(statement)].count("\n") + 1
      "SQL statement \"#{statement.delete_suffix(';')}\"\nPL/pgSQL function #{routine.fetch('signature')} line #{line} at SQL statement\n#{disambiguation_wrapper_stack}"
    end

    def attribute_race_diagnostics!(stderr, frame, kind, variant, role, observed:)
      failed = kind == "eleven-all-id" && variant == "reference"
      records = stderr.split(/(?=^(?:NOTICE|ERROR):  )/)
      error = failed ? records.pop : ""
      if failed
        wrapper = disambiguation_wrapper_stack
        raise Failure, "attribute race D5 wrapper stack changed" unless error && error.scan(wrapper).one?

        diagnostics = ingestion_diagnostics(error.sub(wrapper, ""), role:)
        unless ingestion_approved_rejection?({ "errors" => diagnostics.map { |entry| entry.fetch("error") }, "details" => [], "hints" => [], "diagnostics" => diagnostics }, "D5")
          raise Failure, "attribute race exact frozen D5 changed"
        end
      end
      expected_keys = observed ? ATTRIBUTE_RACE_ORDER & attribute_race_writer_rows(kind).map { |row| row.fetch("attr_key") } : []
      raise Failure, "attribute race UPDATE notice count changed" unless records.length == expected_keys.length

      records.zip(expected_keys).each do |record, key|
        match = record.match(/\ANOTICE:  00000: ingestion-attribute-update:(?<event>\{[^\n]+\})\nCONTEXT:  PL\/pgSQL function ingestion_observation.trace_attribute_update\(\) line 3 at RAISE\n(?<stack>.+)LOCATION:  exec_stmt_raise, pl_exec.c:3897\n\z/m)
        raise Failure, "attribute race unexpected UPDATE diagnostic" unless match

        row = attribute_race_writer_rows(kind).find { |item| item.fetch("attr_key") == key }
        event = { "schema" => "public", "relation" => "canonical_torrent_source_attr", "operation" => "UPDATE",
          "backend" => frame.fetch("backend"), "session" => role, "current" => variant == "reference" ? "postgres" : @owner,
          "setting" => variant == "reference" ? "use_column" : "error", "clock" => frame.fetch("clock"), "old" => row, "new" => row }
        unless metadata_json_parse(match[:event]) == event && match[:stack] == attribute_race_stack(key, variant)
          raise Failure, "attribute race UPDATE event, retained row or exact stack changed"
        end
      end
    end

    def attribute_race_validate!(evidence, kind, mode, variant, role, observed: false)
      raise Failure, "attribute race unknown scenario" unless ATTRIBUTE_RACE_KINDS.include?(kind) && ATTRIBUTE_RACE_MODES.include?(mode) &&
        %w[reference final].include?(variant) && role == (variant == "reference" ? "postgres" : @runtime)

      parsed = %w[fixture b a].to_h { |actor| [actor, attribute_race_parse(evidence.fetch(actor), mode, actor)] }
      frames = parsed.transform_values { |value| value.fetch("frame") }
      seed = evidence.fetch("seed_clock")
      clocks = [seed] + frames.values.map { |frame| frame.fetch("clock") }
      backends = frames.values.map { |frame| frame.fetch("backend") }
      unless clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == 4 && clocks.each_cons(2).all? { |left, right| DateTime.iso8601(left) < DateTime.iso8601(right) } &&
          backends.all? { |pid| pid.match?(/\A[1-9][0-9]*\z/) } && backends.uniq.length == 3
        raise Failure, "attribute race transaction or backend provenance changed"
      end
      failed = kind == "eleven-all-id" && variant == "reference"
      frames.each do |actor, frame|
        direct = actor == "b" ? "postgres" : role
        capabilities = { "session" => direct, "current" => direct, "superuser" => direct == "postgres", "create_role" => direct == "postgres", "bypass_rls" => direct == "postgres" }
        expected_context = { "database" => evidence.fetch("database"), "application" => "attribute-race-#{actor}", "isolation" => "read committed" }
        lifetime = actor == "b" || (actor == "a" && failed) ? %w[false false] : ["true", (variant == "reference").to_s]
        unless parsed.fetch(actor).fetch("context") == expected_context && frame.fetch("role") == capabilities &&
            frame.values_at("before", "after", "finished_setting") == %w[error error error] && frame.values_at("within", "outside") == lifetime &&
            frame.fetch("state") == (actor == "a" && failed ? "42P10" : "00000")
          raise Failure, "attribute race direct role, settings, lifetime or required outcome changed"
        end
        raise Failure, "attribute race unexpected fixture/writer diagnostic" unless actor == "a" || evidence.fetch(actor).fetch("stderr").empty?
      end
      fixture, b, a = frames.values_at("fixture", "b", "a")
      identities = attributes_identities(fixture.fetch("tables_after"))
      initial = attributes_initial_tables(attribute_race_shape(kind, fixture: true), fixture.fetch("clock"), identities)
      initial["canonical_torrent_best_source_context"] = [{ "canonical_torrent_best_source_context_id" => 1, "context_key_type" => "search_request", "context_key_id" => 569001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "computed_at" => fixture.fetch("clock") }]
      written = initial.merge("canonical_torrent_source_attr" => attribute_race_writer_rows(kind))
      counters = IngestionProof::INGESTION_TABLES.to_h { |table| [table, nil] }
      stages = [counters.dup]
      ATTRIBUTE_RACE_SEEDED_SEQUENCES.each { |table| counters[table] = 1 }
      stages << counters.dup
      count = attribute_race_writer_rows(kind).length
      counters["canonical_torrent_source_attr"] = count
      stages << counters.dup
      counters.merge!("canonical_torrent_source_attr" => count + 1, "search_request_source_observation_attr" => count, "canonical_torrent_source_context_score" => 2)
      stages << counters.dup
      counters.merge!("canonical_torrent_source_attr" => count * 2, "canonical_torrent_signal" => 3)
      unless failed
        counters.merge!("canonical_torrent_best_source_context" => 2, "search_request_canonical" => 2)
        counters["canonical_external_id"] = 3 if kind == "eleven-all-id"
      end
      stages << counters.dup
      finished = if failed
                   written
                 else
                   changed = metadata_changed_tables(written, attribute_race_shape(kind), a.fetch("clock"), IngestionMetadata::METADATA_SEQUENCES.transform_values { 0 })
                   metadata_bind_attrs!(changed, a.fetch("tables_after"), 0, count, [])
                   changed.fetch("canonical_torrent_best_source_context").first["computed_at"] = a.fetch("clock")
                   changed
                 end
      [[fixture, attributes_empty, initial, initial], [b, initial, written, finished], [a, initial, finished, finished]].each do |frame, before, after, committed|
        unless metadata_tables_equal?(frame.fetch("tables_before"), before) && metadata_tables_equal?(frame.fetch("tables_after"), after) &&
            metadata_tables_equal?(frame.fetch("tables_finish"), committed)
          raise Failure, "attribute race independent 18-table transition changed"
        end
      end
      unless attributes_result?(fixture, identities, first: true) && b.fetch("result") == { "writer" => "direct-admin-durable-attrs", "rows" => count } &&
          (failed ? !a.key?("result") : attributes_result?(a, identities, first: false))
        raise Failure, "attribute race independently bound result changed"
      end
      %w[seeded before held waiting after].zip(stages, [attributes_empty, initial, initial, initial, finished]).each do |stage, sequence, tables|
        attribute_race_read_validate!(evidence.fetch(stage), seed, sequence, tables)
      end
      writer = evidence.fetch("writer_identity")
      unless writer.keys.sort == %w[backend xid] && writer.fetch("backend") == b.fetch("backend") && writer.fetch("xid").match?(/\A[1-9][0-9]*\z/)
        raise Failure, "attribute race writer transaction changed"
      end
      transcript = evidence.fetch("controller")
      expected_controller = evidence.fetch("b").fetch("stdout").split("outside:", 2)
      unless expected_controller.length == 2 && transcript == "#{expected_controller.first}writer_xid:#{writer.fetch('xid')}\nheld\nreleased\noutside:#{expected_controller.last}finished\n"
        raise Failure, "attribute race controller commit provenance changed"
      end
      attribute_race_wait_validate!(evidence.fetch("wait"), writer, a.fetch("backend"), role)
      attribute_race_diagnostics!(evidence.fetch("a").fetch("stderr"), a, kind, variant, role, observed:)
      { "application" => compilation_comparable("fixture" => nil, "frames" => frames.values), "sequences" => stages,
        "inputs" => policy_comparable_inputs("inputs_before" => evidence.fetch("seeded").fetch("data").fetch("inputs"), "seed_clocks" => { "race" => seed }) }
    rescue Failure, KeyError, ArgumentError, TypeError => error
      raise Failure, "attribute race evidence rejected: #{error.message}"
    end

    def attribute_race_isolated(kind, mode, variant, source, role, observed:)
      name = "#{kind}-#{mode}-#{variant}-#{observed ? 'observed' : 'plain'}"
      database = "ingestion_attribute_race_#{variant}_#{SecureRandom.hex(6)}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\nUPDATE public.trust_tier SET created_at = transaction_timestamp();\nUPDATE public.media_domain SET created_at = transaction_timestamp();"
        clock = policy_setup!(seed, database, File.join(@metadata_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        evidence = { "database" => database, "seed_clock" => clock, "seeded" => attribute_race_read(database, "#{name}-seeded") }
        evidence["fixture"] = attribute_race_execute(kind, mode, "fixture", database, role, "#{name}-fixture")
        evidence["before"] = attribute_race_read(database, "#{name}-before")
        attribute_race_observer!(database, name) if observed
        evidence.merge!(attribute_race_interleave(kind, mode, database, role, name))
        evidence["after"] = attribute_race_read(database, "#{name}-after")
        metadata_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        attribute_race_validate!(evidence, kind, mode, variant, role, observed:)
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def attribute_race_source_hashes
      metadata_source_hashes.merge(%w[scripts/database_rebaseline/ingestion_attribute_race.rb scripts/tests/database-ingestion-attribute-race-test.rb
        scripts/database_rebaseline/ingestion_guid.rb scripts/database_rebaseline/ingestion_guid_trace.rb].to_h do |path|
        [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest]
      end)
    end

    def verify_ingestion_attribute_race!
      directory = File.join(@contract.output_path, "ingestion-attribute-race")
      raise Failure, "attribute race evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = Dir.mktmpdir("run-", directory)
      hashes = attribute_race_source_hashes
      cases = []
      completed = false
      begin
        metadata_write("routine-inventory.json", JSON.pretty_generate(@ingestion_inventory) + "\n")
        ATTRIBUTE_RACE_KINDS.product(ATTRIBUTE_RACE_MODES).each do |kind, mode|
          pair = { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.to_h do |variant, (source, role)|
            plain = attribute_race_isolated(kind, mode, variant, source, role, observed: false)
            observed = attribute_race_isolated(kind, mode, variant, source, role, observed: true)
            raise Failure, "attribute race NOTICE observation changed application evidence" unless plain == observed

            [variant, plain]
          end
          equivalent = pair.fetch("reference") == pair.fetch("final")
          raise Failure, "attribute race eight-attribute parity changed" if kind == "eight-non-id" && !equivalent
          raise Failure, "attribute race frozen ID success fabricated" if kind == "eleven-all-id" && equivalent

          cases << { kind:, mode:, equivalent:, accepted: true, approved_delta: kind == "eleven-all-id" ? "ADR 588 D5: exact frozen rollback, independently specified final success" : nil }
        end
        raise Failure, "attribute race source bytes changed" unless hashes == attribute_race_source_hashes

        completed = true
      ensure
        report = { completed:, passed: completed, d3_complete: false, cases:, source_sha256: hashes,
          candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256, postgres_image: @contract.postgres_image,
          limitations: ["K2 cold and pure-helper-first only; repeated committed/warm and mutating-helper-first paths remain unqualified.",
            "Test-only AFTER UPDATE NOTICE observations are not native/RI callback closure or actual Rust wrapper qualification.",
            "Frozen all-ID ingestion fails D5 and rolls back A only; B's committed rows remain. Parent owns live qualification and integration."] }
        metadata_write("report.json", JSON.pretty_generate(report) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end
  end
end
