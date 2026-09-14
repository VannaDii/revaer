# frozen_string_literal: true

require "tmpdir"

module RevaerDatabaseRebaseline
  # Disposable observers never replace an application routine or its compiler setting.
  module IngestionCompilation
    COMPILATION_OBSERVERS = %w[canonical_torrent source_metadata_conflict search_filter_decision].freeze
    COMPILATION_CLOCKS = IngestionExisting::EXISTING_CLOCK_COLUMNS.merge(
      "source_metadata_conflict" => %w[observed_at], "indexer_health_event" => %w[occurred_at]
    ).freeze

    private

    def verify_ingestion_compilation!
      @compilation_validated_evidence = {}
      directory = File.join(@contract.output_path, "ingestion-compilation")
      raise Failure, "ingestion compilation evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @compilation_evidence = Dir.mktmpdir("run-", directory)
      checks_start = @checks.length
      source_hashes = compilation_source_hashes
      cases = []
      completed = false
      begin
        compilation_cases.each do |test_case|
          variants = {}
          { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
            plain = compilation_isolated(test_case, variant, source, role, observed: false)
            observed = compilation_isolated(test_case, variant, source, role, observed: true)
            check("#{test_case.fetch(:name)} #{variant} observers preserve application behavior", plain == observed)
            variants[variant] = observed
          end
          equivalent = variants.fetch("reference") == variants.fetch("final")
          check("#{test_case.fetch(:name)} exact frozen/final application parity", equivalent)
          cases << { name: test_case.fetch(:name), equivalent:, variants: }
        end
        check("compilation source bytes unchanged during matrix", source_hashes == compilation_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(checks_start)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) },
                   d3_complete: false, observations: "committed successful writes only",
                   approved_metadata_delta: "ADR 588 D4 temporary-table lifetime", postgres_image: @contract.postgres_image,
                   candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
                   source_sha256: source_hashes, checks:, cases: }
        path = File.join(@compilation_evidence, "report.json")
        bytes = JSON.pretty_generate(report) + "\n"
        File.binwrite(path, bytes)
        @compilation_validated_evidence[path] = Digest::SHA256.hexdigest(bytes) if report.fetch(:passed)
      end
    end

    def compilation_source_hashes
      paths = %w[scripts/tests/database-ingestion-compilation-test.rb
                 scripts/tests/database-ingestion-corrections-test.rb
                 scripts/tests/database-ingestion-corrections-seed.sql
                 scripts/tests/database-ingestion-dependencies-test.rb]
      wrapper_source_hashes.merge(paths.to_h { |path| [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest] })
    end

    def compilation_cases
      ingest = { operation: :ingest, arguments: {} }
      conflict = { operation: :ingest, arguments: { infohash_v1_input: "repeat('b',40)::char(40)" } }
      [
        { name: "cold-canonical-setting", calls: [ingest] },
        { name: "cold-logger-setting", fixture: true, calls: [conflict] },
        { name: "logger-first-setting", fixture: true, calls: [
          { operation: :logger, existing: "NULL::text", incoming: "NULL::text", observed_at: "NULL::timestamptz", type: "tracker_name" },
          { operation: :logger, existing: "repeat('A',257)", incoming: "repeat('B',258)", observed_at: "'2026-09-10T00:02:00Z'::timestamptz", type: "tracker_category" },
          conflict
        ] },
        { name: "cold-decision-setting", policy: true, calls: [
          { operation: :ingest, arguments: { search_request_public_id_input: "'58800000-0000-4000-8000-000000000003'::uuid" } }
        ] }
      ]
    end

    def compilation_call(operation, source_id)
      return ingestion_call(operation.fetch(:arguments)) if operation.fetch(:operation) == :ingest
      raise Failure, "unknown compilation proof operation" unless operation.fetch(:operation) == :logger
      raise Failure, "logger proof requires a real persisted source" unless source_id.is_a?(Integer) && source_id.positive?

      <<~SQL
        SELECT json_build_object('logger', public.log_source_metadata_conflict_v1(
          #{source_id}, 569001, #{literal(operation.fetch(:type))}::public.conflict_type,
          #{operation.fetch(:existing)}, #{operation.fetch(:incoming)}, #{operation.fetch(:observed_at)}));
      SQL
    end

    def compilation_isolated(test_case, variant, source, role, observed:)
      database = "ingestion_compilation_#{variant}"
      name = "#{test_case.fetch(:name)}-#{variant}-#{observed ? 'observed' : 'plain'}"
      prefix = File.join(@compilation_evidence, name)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        sql(File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")), role: "postgres", database:)
        if test_case[:policy]
          sql(File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-corrections-seed.sql")), role: "postgres", database:)
        end
        correction_observer!(database, variant)
        fixture = compilation_fixture(test_case, database, role, prefix)
        before = ingestion_snapshot(database)
        source_id = fixture&.fetch("tables_after")&.fetch("canonical_torrent_source")&.first&.fetch("canonical_torrent_source_id")
        compilation_observer!(database, prefix) if observed
        query = correction_session(test_case) { |operation| compilation_call(operation, source_id) }
        File.binwrite("#{prefix}.sql", query)
        outcome = result(query, role:, database:)
        File.binwrite("#{prefix}.stdout", outcome.stdout)
        File.binwrite("#{prefix}.stderr", outcome.stderr)
        raise Failure, "compilation proof transport failed" unless outcome.success

        frames = correction_parse(outcome.stdout, outcome.stderr, test_case)
        after = ingestion_snapshot(database)
        events = observed ? compilation_events(database) : []
        evidence = { "fixture" => fixture, "before" => before, "frames" => frames, "after" => after, "events" => events }
        bytes = JSON.pretty_generate(evidence) + "\n"
        File.binwrite("#{prefix}.json", bytes)
        first = @checks.length
        compilation_verify!(name, test_case, evidence, variant, role, observed:)
        if @checks.drop(first).all? { |entry| entry.fetch(:passed) }
          @compilation_validated_evidence["#{prefix}.json"] = Digest::SHA256.hexdigest(bytes)
        end
        compilation_comparable(evidence)
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def compilation_fixture(test_case, database, role, prefix)
      return nil unless test_case[:fixture]

      fixture_case = { name: "compilation-fixture", calls: [{}] }
      query = correction_session(fixture_case)
      outcome = result(query, role:, database:)
      File.binwrite("#{prefix}-fixture.sql", query)
      File.binwrite("#{prefix}-fixture.stdout", outcome.stdout)
      File.binwrite("#{prefix}-fixture.stderr", outcome.stderr)
      raise Failure, "compilation fixture transport failed" unless outcome.success

      frames = correction_parse(outcome.stdout, outcome.stderr, fixture_case)
      frame = frames.fetch(0)
      unless frame.fetch("state") == "00000" && correction_results?(fixture_case, frames, frame.fetch("tables_finish"))
        raise Failure, "compilation fixture must commit real successful ingestion"
      end
      frame
    end

    def compilation_observer!(database, prefix)
      query = <<~SQL
        CREATE TABLE ingestion_observation.events (
          event_id bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
          relation_name text NOT NULL, operation text NOT NULL,
          backend integer NOT NULL, session_role text NOT NULL, current_role_name text NOT NULL,
          variable_conflict text NOT NULL, transaction_clock timestamptz NOT NULL
        );
        ALTER TABLE ingestion_observation.events OWNER TO #{identifier(@owner)};
        CREATE FUNCTION ingestion_observation.record_setting() RETURNS trigger
        LANGUAGE plpgsql SET search_path TO pg_catalog AS $observer$
        BEGIN
          INSERT INTO ingestion_observation.events (
            relation_name, operation, backend, session_role, current_role_name, variable_conflict, transaction_clock
          ) VALUES (TG_TABLE_NAME, TG_OP, pg_backend_pid(), session_user, current_user,
                    current_setting('plpgsql.variable_conflict'), transaction_timestamp());
          RETURN NEW;
        END;
        $observer$;
        REVOKE ALL ON FUNCTION ingestion_observation.record_setting() FROM PUBLIC;
        #{COMPILATION_OBSERVERS.map { |table| "CREATE TRIGGER ingestion_setting_observer BEFORE INSERT ON public.#{identifier(table)} FOR EACH ROW EXECUTE FUNCTION ingestion_observation.record_setting();" }.join("\n")}
      SQL
      File.binwrite("#{prefix}-observer.sql", query)
      sql(query, role: "postgres", database:)
    end

    def compilation_events(database)
      JSON.parse(sql("SELECT COALESCE(json_agg(row_to_json(e) ORDER BY event_id), '[]') FROM ingestion_observation.events e", role: "postgres", database:))
    end

    def compilation_verify!(name, test_case, evidence, variant, role, observed:)
      frames = evidence.fetch("frames")
      fixture = evidence.fetch("fixture")
      all_frames = fixture ? [fixture] + frames : frames
      capabilities = { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }
      check("#{name} all operations succeed without hidden errors", all_frames.all? { |frame| frame.fetch("state") == "00000" })
      check("#{name} exact direct role and caller setting", all_frames.all? { |frame| frame.fetch("role") == capabilities && frame.values_at("before", "after") == %w[error error] })
      backends = frames.map { |frame| frame.fetch("backend") }
      check("#{name} cold tested backend then same-backend commits", backends.uniq.length == 1 && all_frames.all? { |frame| frame.fetch("backend").match?(/\A[1-9][0-9]*\z/) } && (!fixture || fixture.fetch("backend") != backends.first))
      clocks = all_frames.map { |frame| frame.fetch("clock") }
      check("#{name} distinct transaction clocks", clocks.uniq.length == all_frames.length && clocks.all? { |clock| clock.is_a?(String) && clock.match?(/\A\d{4}-\d\d-\d\dT.*\+00:00\z/) })
      fixture_valid = !fixture || (fixture.fetch("tables_before").values.all?(&:empty?) && correction_snapshots?({}, fixture.fetch("tables_before"), [fixture], evidence.fetch("before")))
      check("#{name} complete persisted-state continuity", fixture_valid && correction_snapshots?(test_case, evidence.fetch("before"), frames, evidence.fetch("after")))
      check("#{name} exact approved temporary-table lifetime", compilation_temp_lifetime?(test_case, evidence, variant))
      check("#{name} exact helper results and stored relationships", compilation_outcomes?(test_case, evidence))
      check("#{name} exact in-call setting scope", compilation_settings?(test_case, frames, evidence.fetch("events"), variant, role, observed:))
    end

    def compilation_temp_lifetime?(test_case, evidence, variant)
      pairs = test_case.fetch(:calls).zip(evidence.fetch("frames"))
      pairs.unshift([{ operation: :ingest }, evidence.fetch("fixture")]) if evidence.fetch("fixture")
      pairs.all? do |operation, frame|
        ingest = operation.fetch(:operation) == :ingest
        frame.fetch("within") == ingest.to_s && frame.fetch("outside") == (ingest && variant == "reference").to_s
      end
    end

    def compilation_settings?(test_case, frames, events, variant, role, observed:)
      return events.empty? unless observed

      expected = []
      test_case.fetch(:calls).zip(frames).each do |operation, frame|
        logger = operation.fetch(:operation) == :logger
        relations = logger ? ["source_metadata_conflict"] : ["canonical_torrent"]
        # Changed infohash and its derived magnet hash are separate logger calls.
        relations.concat(%w[source_metadata_conflict source_metadata_conflict]) if !logger && test_case[:fixture]
        relations << "search_filter_decision" if test_case[:policy]
        relations.each do |relation|
          expected << { "event_id" => expected.length + 1, "relation_name" => relation, "operation" => "INSERT",
                        "backend" => Integer(frame.fetch("backend")), "session_role" => role,
                        "current_role_name" => variant == "reference" ? "postgres" : @owner,
                        "variable_conflict" => variant == "reference" && !logger ? "use_column" : "error",
                        "transaction_clock" => frame.fetch("clock") }
        end
      end
      events == expected
    end

    def compilation_outcomes?(test_case, evidence)
      frames = evidence.fetch("frames")
      test_case.fetch(:calls).zip(frames).all? do |operation, frame|
        tables = frame.fetch("tables_after")
        if operation.fetch(:operation) == :logger
          compilation_logger_outcome?(operation, frame)
        else
          result = frame.fetch("result")
          canonical = tables.fetch("canonical_torrent").find { |row| row.fetch("canonical_torrent_public_id") == result.fetch("canonical_torrent_public_id") }
          source = tables.fetch("canonical_torrent_source").find { |row| row.fetch("canonical_torrent_source_public_id") == result.fetch("canonical_torrent_source_public_id") }
          flags = test_case[:fixture] ? [false, false, true] : [true, true, true]
          canonical && source && result.keys.sort == %w[canonical_changed canonical_torrent_public_id canonical_torrent_source_public_id durable_source_created observation_created] &&
            result.values_at("observation_created", "durable_source_created", "canonical_changed") == flags &&
            canonical.fetch("infohash_v1") == (test_case[:fixture] ? "b" : "a") * 40 && source.fetch("infohash_v1") == "a" * 40 &&
            ingestion_existing_conflict_links?(tables) && (!test_case[:policy] || compilation_decision?(tables))
        end
      end
    end

    def compilation_logger_outcome?(operation, frame)
      before = frame.fetch("tables_before")
      after = frame.fetch("tables_after")
      conflict = after.fetch("source_metadata_conflict").last
      truncated = operation.fetch(:type) == "tracker_category"
      values = truncated ? ["A" * 256, "B" * 256, "2026-09-10T00:02:00+00:00"] : ["", "", frame.fetch("clock")]
      changed = %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event]
      conflict_id = before.fetch("source_metadata_conflict").length + 1
      source_id = before.fetch("canonical_torrent_source").first.fetch("canonical_torrent_source_id")
      expected_conflict = {
        "source_metadata_conflict_id" => conflict_id, "canonical_torrent_source_id" => source_id,
        "conflict_type" => operation.fetch(:type), "existing_value" => values[0], "incoming_value" => values[1], "observed_at" => values[2],
        "resolved_at" => nil, "resolved_by_user_id" => nil, "resolution" => nil, "resolution_note" => nil
      }
      expected_audit = {
        "source_metadata_conflict_audit_log_id" => conflict_id, "conflict_id" => conflict_id,
        "action" => "created", "actor_user_id" => 0, "occurred_at" => frame.fetch("clock"), "note" => nil
      }
      expected_health = {
        "indexer_health_event_id" => conflict_id, "indexer_instance_id" => 569001, "occurred_at" => values[2],
        "event_type" => "identity_conflict", "latency_ms" => nil, "http_status" => nil, "error_class" => nil, "detail" => operation.fetch(:type)
      }
      frame.fetch("result") == { "logger" => "" } &&
        changed.all? { |table| after.fetch(table).length == before.fetch(table).length + 1 && after.fetch(table)[0...-1] == before.fetch(table) } &&
        (IngestionProof::INGESTION_TABLES - changed).all? { |table| after.fetch(table) == before.fetch(table) } &&
        conflict == expected_conflict && after.fetch("source_metadata_conflict_audit_log").last == expected_audit &&
        after.fetch("indexer_health_event").last == expected_health
    end

    def compilation_decision?(tables)
      decisions = tables.fetch("search_filter_decision")
      decisions == [{
          "search_filter_decision_id" => 1, "search_request_id" => 588003,
          "policy_rule_public_id" => "58800000-0000-4000-8000-000000000005", "policy_snapshot_id" => 588002,
          "observation_id" => 1, "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1,
          "decision" => "flag", "decision_detail" => nil, "decided_at" => tables.fetch("canonical_torrent").first.fetch("created_at")
        }] && tables.fetch("search_request_source_observation").first.fetch("was_flagged") == true
    end

    def compilation_comparable(evidence)
      frames = evidence.fetch("fixture") ? [evidence.fetch("fixture")] : []
      frames += evidence.fetch("frames")
      images = frames.each_with_index.to_h { |frame, index| [index, { "before" => frame.fetch("tables_before"), "after" => frame.fetch("tables_after") }] }
      identities = ingestion_existing_identities(images)
      clocks = frames.each_with_index.to_h { |frame, index| [frame.fetch("clock"), "<transaction:#{index}>"] }
      frames.map do |frame|
        normalized = frame.merge("backend" => "<validated-backend>", "role" => "<validated-direct-role>", "outside" => "<validated-ADR-588-D4-lifetime>")
        normalized["clock"] = clocks.fetch(frame.fetch("clock"))
        if frame.key?("result")
          normalized["result"] = frame.fetch("result").to_h do |key, value|
            [key, IngestionExisting::EXISTING_IDENTITIES.any? { |table| key == "#{table}_public_id" } ? identities.fetch(value) : value]
          end
        end
        %w[tables_before tables_after tables_finish].each do |key|
          normalized[key] = frame.fetch(key).to_h do |table, rows|
            [table, rows.map do |row|
              row.to_h do |column, value|
                replacement = if IngestionExisting::EXISTING_IDENTITIES.include?(table) && column == "#{table}_public_id"
                                identities.fetch(value)
                              elsif COMPILATION_CLOCKS.fetch(table, []).include?(column)
                                clocks.fetch(value, value)
                              else
                                value
                              end
                [column, replacement]
              end
            end]
          end
        end
        normalized
      end
    end
  end
end
