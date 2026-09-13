# frozen_string_literal: true

require "rbconfig"
require_relative "database-ingestion-metadata-test"
require_relative "../database_rebaseline/ingestion_attribute_race"

module RevaerDatabaseRebaseline
  class IngestionAttributeRaceTest < IngestionMetadataTest
    include IngestionAttributeRace

    def run_tests!
      @assertions = 0
      @runtime = "attribute_race_unit_runtime"
      race_test_inventory!
      race_test_queries!
      ATTRIBUTE_RACE_KINDS.product(ATTRIBUTE_RACE_MODES, %w[reference final]).each do |kind, mode, variant|
        role = variant == "reference" ? "postgres" : @runtime
        value = race_test_evidence(kind, mode, variant)
        plain = attribute_race_validate!(value, kind, mode, variant, role)
        observed = copy(value)
        observed.fetch("a")["stderr"] = race_test_notices(kind, variant, role) + value.fetch("a").fetch("stderr")
        assert(plain == attribute_race_validate!(observed, kind, mode, variant, role, observed: true), "plain and observed evidence agree only after independent validation")
        race_test_mutations!(observed, kind, mode, variant, role)
      end
      race_test_transport!
      race_test_controller!
      race_test_lifecycle!
      puts "database-ingestion-attribute-race-test: #{@assertions} assertions passed (Ruby fixtures only; no database/native qualification)"
    end

    private

    def race_test_inventory!
      routines = { "search_result_ingest_v1" => "0052_indexer_search_result_ingest_proc.sql", "search_result_ingest" => "0120_search_result_ingest_seed_best_source_context.sql" }.map do |name, file|
        text = File.binread(File.join(@contract.root, "crates/revaer-data/migrations", file))
        source = text.split("CREATE OR REPLACE FUNCTION #{name}(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
        { "name" => name, "signature" => "#{name}(uuid)", "source" => source }
      end
      final = File.binread(File.join(@contract.root, "crates/revaer-data/init.sql"))
      source = final.split("CREATE FUNCTION public.search_result_ingest_v1(", 2).fetch(1).split("AS $_$", 2).fetch(1).split("$_$;", 2).first
      @ingestion_inventory = { "reference_proof" => { "routines" => routines }, @database => { "routines" => [routines.first.merge("source" => source), routines.last] } }
    end

    # Independently authored full fixtures: no race model or changed-table oracle
    # is used to manufacture the records that the validator accepts.
    def race_test_initial
      clock = "2026-09-12T00:00:01+00:00"
      magnet = "750837cbeaadaf72ab0a852ee3e0517760ca56eb6b8fc3c08b2d9e73348b6bea"
      tables = %w[canonical_torrent canonical_torrent_source canonical_torrent_source_attr canonical_torrent_source_context_score
        canonical_torrent_best_source_context search_request_source_observation search_request_source_observation_attr canonical_torrent_signal
        canonical_external_id canonical_size_sample canonical_size_rollup search_request_canonical search_page search_page_item search_filter_decision
        source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].to_h { |table| [table, []] }
      tables["canonical_torrent"] = [{ "canonical_torrent_id" => 1, "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000090",
        "identity_confidence" => 1.0, "identity_strategy" => "infohash_v1", "infohash_v1" => "a" * 40, "infohash_v2" => nil, "magnet_hash" => magnet,
        "title_size_hash" => nil, "imdb_id" => nil, "tmdb_id" => nil, "tvdb_id" => nil, "ids_confidence" => nil, "title_display" => "Proof",
        "title_normalized" => "proof", "size_bytes" => nil, "created_at" => clock, "updated_at" => clock }]
      tables["canonical_torrent_source"] = [{ "canonical_torrent_source_id" => 1, "indexer_instance_id" => 569001,
        "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-000000000091", "source_guid" => "ingestion-proof-source",
        "infohash_v1" => "a" * 40, "infohash_v2" => nil, "magnet_hash" => magnet, "title_normalized" => "proof", "size_bytes" => nil,
        "last_seen_at" => "2026-09-10T00:00:00+00:00", "last_seen_seeders" => 5, "last_seen_leechers" => 2, "last_seen_published_at" => nil,
        "last_seen_download_url" => nil, "last_seen_magnet_uri" => nil, "last_seen_details_url" => nil, "last_seen_uploader" => nil, "created_at" => clock, "updated_at" => clock }]
      tables["canonical_torrent_source_context_score"] = [{ "canonical_torrent_source_context_score_id" => 1, "context_key_type" => "search_request", "context_key_id" => 569001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "score_total_context" => 0, "score_policy_adjust" => 0, "score_tag_adjust" => 0, "is_dropped" => false, "computed_at" => clock }]
      tables["canonical_torrent_best_source_context"] = [{ "canonical_torrent_best_source_context_id" => 1, "context_key_type" => "search_request", "context_key_id" => 569001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "computed_at" => clock }]
      tables["search_request_source_observation"] = [{ "observation_id" => 1, "search_request_id" => 569001, "indexer_instance_id" => 569001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "observed_at" => "2026-09-10T00:00:00+00:00", "seeders" => 5, "leechers" => 2,
        "published_at" => nil, "uploader" => nil, "source_guid" => "ingestion-proof-source", "details_url" => nil, "download_url" => nil, "magnet_uri" => nil,
        "title_raw" => "Proof", "size_bytes" => nil, "infohash_v1" => "a" * 40, "infohash_v2" => nil, "magnet_hash" => magnet,
        "guid_conflict" => false, "was_downranked" => false, "was_flagged" => false }]
      tables["search_request_canonical"] = [{ "search_request_canonical_id" => 1, "search_request_id" => 569001, "canonical_torrent_id" => 1, "first_seen_at" => clock }]
      tables["search_page"] = [{ "search_page_id" => 1, "search_request_id" => 569001, "page_number" => 1, "sealed_at" => nil }]
      tables["search_page_item"] = [{ "search_page_item_id" => 1, "search_page_id" => 1, "search_request_canonical_id" => 1, "position" => 1 }]
      tables
    end

    def race_test_values(kind, incoming: false)
      values = incoming ? [["tracker_name", "text", "Changed Tracker"], ["tracker_category", "int", 4000], ["tracker_subcategory", "int", 4040],
        ["size_bytes_reported", "bigint", 8_589_934_592], ["files_count", "int", 6], ["season", "int", 3], ["episode", "int", 8], ["year", "int", 2027]] :
        [["tracker_name", "text", "Proof Tracker"], ["tracker_category", "int", 2000], ["tracker_subcategory", "int", 2040],
          ["size_bytes_reported", "bigint", 4_294_967_296], ["files_count", "int", 3], ["season", "int", 2], ["episode", "int", 7], ["year", "int", 2026]]
      if kind == "eleven-all-id"
        values += incoming ? [["imdb_id", "text", "tt7654321"], ["tmdb_id", "int", 234_567], ["tvdb_id", "int", 765_432]] :
          [["imdb_id", "text", "tt1234567"], ["tmdb_id", "int", 123_456], ["tvdb_id", "int", 654_321]]
      end
      values.each_with_index.map do |(key, type, value), index|
        row = %w[text int bigint numeric bool].to_h { |channel| ["value_#{channel}", channel == type ? value : nil] }.merge("attr_key" => key)
        if incoming
          row.merge("observation_attr_id" => index + 1, "observation_id" => 1, "value_uuid" => nil, "created_at" => "2026-09-12T00:00:03+00:00")
        else
          row.merge("canonical_torrent_source_attr_id" => index + 1, "canonical_torrent_source_id" => 1)
        end
      end
    end

    def race_test_finished(kind, variant)
      tables = race_test_initial.merge("canonical_torrent_source_attr" => race_test_values(kind))
      return tables if kind == "eleven-all-id" && variant == "reference"

      clock = "2026-09-12T00:00:03+00:00"
      observed = "2026-09-11T00:00:00+00:00"
      tables.fetch("canonical_torrent").first["updated_at"] = clock
      tables.fetch("canonical_torrent_source").first.merge!("updated_at" => clock, "last_seen_at" => observed, "last_seen_seeders" => 9, "last_seen_leechers" => 4)
      tables.fetch("search_request_source_observation").first.merge!("observed_at" => observed, "seeders" => 9, "leechers" => 4)
      %w[canonical_torrent_source_context_score canonical_torrent_best_source_context].each { |table| tables.fetch(table).first["computed_at"] = clock }
      tables["search_request_source_observation_attr"] = race_test_values(kind, incoming: true)
      tables["canonical_torrent_signal"] = [["year", 2027], ["season", 3], ["episode", 8]].each_with_index.map do |(key, value), index|
        { "canonical_torrent_signal_id" => index + 1, "canonical_torrent_id" => 1, "signal_key" => key, "value_text" => nil, "value_int" => value, "confidence" => 0.5, "parser_version" => 1 }
      end
      if kind == "eleven-all-id"
        tables.fetch("canonical_torrent").first.merge!("imdb_id" => "tt7654321", "tmdb_id" => 234_567, "tvdb_id" => 765_432)
        tables["canonical_external_id"] = [["imdb", "tt7654321", nil], ["tmdb", nil, 234_567], ["tvdb", nil, 765_432]].each_with_index.map do |(type, text, int), index|
          { "canonical_external_id_id" => index + 1, "canonical_torrent_id" => 1, "id_type" => type, "id_value_text" => text, "id_value_int" => int,
            "source_canonical_torrent_source_id" => 1, "trust_tier_rank" => 10, "first_seen_at" => observed, "last_seen_at" => observed }
        end
      end
      tables
    end

    def race_test_frame(actor, variant, kind)
      initial = race_test_initial
      written = initial.merge("canonical_torrent_source_attr" => race_test_values(kind))
      failed = actor == "a" && variant == "reference" && kind == "eleven-all-id"
      number = { "fixture" => 1, "b" => 2, "a" => 3 }.fetch(actor)
      direct = variant == "reference" || actor == "b" ? "postgres" : @runtime
      lifetime = failed || actor == "b" ? %w[false false] : ["true", (variant == "reference").to_s]
      result = if actor == "b"
                 { "writer" => "direct-admin-durable-attrs", "rows" => kind == "eleven-all-id" ? 11 : 8 }
               else
                 { "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000090", "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-000000000091",
                   "observation_created" => actor == "fixture", "durable_source_created" => actor == "fixture", "canonical_changed" => actor == "fixture" }
               end
      frame = { "backend" => (100 + number).to_s, "clock" => "2026-09-12T00:00:0#{number}+00:00",
        "role" => { "session" => direct, "current" => direct, "superuser" => direct == "postgres", "create_role" => direct == "postgres", "bypass_rls" => direct == "postgres" },
        "before" => "error", "after" => "error", "finished_setting" => "error", "state" => failed ? "42P10" : "00000", "within" => lifetime.first, "outside" => lifetime.last,
        "tables_before" => actor == "fixture" ? initial.transform_values { [] } : initial,
        "tables_after" => actor == "b" ? written : (actor == "fixture" ? initial : race_test_finished(kind, variant)),
        "tables_finish" => actor == "fixture" ? initial : race_test_finished(kind, variant) }
      frame["result"] = result unless failed
      copy(frame)
    end

    def race_test_raw(parsed, mode, actor, stderr = "")
      lines = actor == "a" && mode == "helpers-first" ? ["helpers:#{JSON.generate(ingestion_helper_expectations)}"] : []
      lines << "race_context:#{JSON.generate(parsed.fetch('context'))}"
      frame = parsed.fetch("frame")
      %w[backend role clock before tables_before state within after tables_after outside tables_finish finished_setting].each do |key|
        lines << JSON.generate(frame.fetch("result")) if key == "state" && frame.key?("result")
        value = frame.fetch(key)
        lines << "#{key}:#{%w[role clock tables_before tables_after tables_finish].include?(key) ? JSON.generate(value) : value}"
      end
      { "stdout" => lines.join("\n") + "\n", "stderr" => stderr, "parsed" => copy(parsed) }
    end

    def race_test_refresh!(value, mode)
      %w[fixture b a].each do |actor|
        raw = value.fetch(actor)
        value[actor] = race_test_raw(raw.fetch("parsed"), mode, actor, raw.fetch("stderr"))
      end
      before, after = value.fetch("b").fetch("stdout").split("outside:", 2)
      value["controller"] = "#{before}writer_xid:900\nheld\nreleased\noutside:#{after}finished\n"
      value
    end

    def race_test_read(sequence, tables)
      data = { "inputs" => metadata_read_tables("2026-09-12T00:00:00+00:00"), "sequences" => sequence.slice(*IngestionMetadata::METADATA_SEQUENCES.keys) }
      { "data" => data, "stdout" => JSON.generate(data) + "\n", "stderr" => "", "tables" => copy(tables),
        "remaining" => { "stdout" => JSON.generate(sequence.reject { |table, _| IngestionMetadata::METADATA_SEQUENCES.key?(table) }) + "\n", "stderr" => "" } }
    end

    def race_test_evidence(kind, mode, variant)
      role = variant == "reference" ? "postgres" : @runtime
      value = { "database" => "unit_attribute_race", "seed_clock" => "2026-09-12T00:00:00+00:00", "writer_identity" => { "backend" => "102", "xid" => "900" } }
      %w[fixture b a].each do |actor|
        parsed = { "context" => { "database" => "unit_attribute_race", "application" => "attribute-race-#{actor}", "isolation" => "read committed" }, "frame" => race_test_frame(actor, variant, kind) }
        stderr = actor == "a" && variant == "reference" && kind == "eleven-all-id" ? metadata_test_d5_stderr.sub("LOCATION:", "#{race_test_wrapper}LOCATION:") : ""
        value[actor] = race_test_raw(parsed, mode, actor, stderr)
      end
      count = kind == "eleven-all-id" ? 11 : 8
      sequence = race_test_initial.transform_values { nil }
      value["seeded"] = race_test_read(sequence, race_test_initial.transform_values { [] })
      %w[canonical_torrent canonical_torrent_source canonical_torrent_source_context_score canonical_torrent_best_source_context search_request_source_observation search_request_canonical search_page search_page_item].each { |table| sequence[table] = 1 }
      value["before"] = race_test_read(sequence, race_test_initial)
      sequence["canonical_torrent_source_attr"] = count
      value["held"] = race_test_read(sequence, race_test_initial)
      sequence.merge!("canonical_torrent_source_context_score" => 2, "canonical_torrent_source_attr" => count + 1, "search_request_source_observation_attr" => count)
      value["waiting"] = race_test_read(sequence, race_test_initial)
      sequence.merge!("canonical_torrent_source_attr" => count * 2, "canonical_torrent_signal" => 3)
      unless kind == "eleven-all-id" && variant == "reference"
        sequence.merge!("canonical_torrent_best_source_context" => 2, "search_request_canonical" => 2)
        sequence["canonical_external_id"] = 3 if kind == "eleven-all-id"
      end
      value["after"] = race_test_read(sequence, race_test_finished(kind, variant))
      wait = { "pid" => 103, "usename" => role, "application_name" => "attribute-race-a", "state" => "active", "wait_event_type" => "Lock", "wait_event" => "transactionid",
        "blockers" => [102], "locktype" => "transactionid", "mode" => "ShareLock", "granted" => false, "xid" => "900", "writer_pid" => 102,
        "writer_role" => "postgres", "writer_application" => "attribute-race-b", "writer_state" => "idle in transaction", "writer_xid" => "900", "writer_mode" => "ExclusiveLock", "writer_granted" => true,
        "query_prefix" => "SELECT row_to_json(r) FROM public.search_result_ingest(search_request" }
      value["wait"] = { "stdout" => JSON.generate([wait]) + "\n", "stderr" => "" }
      race_test_refresh!(value, mode)
    end

    def race_test_wrapper
      source = @ingestion_inventory.fetch("reference_proof").fetch("routines").last.fetch("source")
      statement = source.split("    SELECT *\n", 2).fetch(1).split("    );", 2).first
      target, arguments = statement.split("    FROM search_result_ingest_v1(", 2)
      # Keep the leading indentation outside PL/pgSQL's replaced INTO target.
      spaces = "    " + " " * (target.bytesize - 4 + 4)
      "SQL statement \"SELECT *\n#{spaces}FROM search_result_ingest_v1(#{arguments}    )\"\nPL/pgSQL function search_result_ingest(uuid) line 9 at SQL statement\n"
    end

    def race_test_notices(kind, variant, role)
      source = @ingestion_inventory.fetch("reference_proof").fetch("routines").first.fetch("source")
      sites = { "tracker_name" => 1729, "tracker_category" => 1756, "tracker_subcategory" => 1783, "size_bytes_reported" => 1810, "files_count" => 1828,
        "imdb_id" => 1846, "tmdb_id" => 1873, "tvdb_id" => 1900, "season" => 1927, "episode" => 1945, "year" => 1963 }
      rows = race_test_values(kind)
      sites.filter_map do |key, line|
        row = rows.find { |item| item.fetch("attr_key") == key }
        next unless row

        statement = source.lines.drop(line - 1).join.split(";", 2).first.lstrip
        event = { "schema" => "public", "relation" => "canonical_torrent_source_attr", "operation" => "UPDATE", "backend" => "103", "session" => role,
          "current" => variant == "reference" ? "postgres" : @owner, "setting" => variant == "reference" ? "use_column" : "error", "clock" => "2026-09-12T00:00:03+00:00", "old" => row, "new" => row }
        "NOTICE:  00000: ingestion-attribute-update:#{JSON.generate(event)}\nCONTEXT:  PL/pgSQL function ingestion_observation.trace_attribute_update() line 3 at RAISE\n" \
          "SQL statement \"#{statement}\"\nPL/pgSQL function search_result_ingest_v1(uuid) line #{line + (variant == 'final' ? 1 : 0)} at SQL statement\n#{race_test_wrapper}LOCATION:  exec_stmt_raise, pl_exec.c:3897\n"
      end.join
    end

    def race_test_queries!
      assert(IngestionProof::INGESTION_TABLES.sort == race_test_initial.keys.sort, "independent 18-table inventory")
      ATTRIBUTE_RACE_KINDS.each do |kind|
        assert(attribute_race_writer_rows(kind) == race_test_values(kind), "independent typed B rows and IDs")
        shape = attribute_race_shape(kind)
        assert(shape.fetch(:answers).length == (kind == "eleven-all-id" ? 11 : 8) && shape.fetch(:conflicts).empty?, "bounded differing metadata inputs")
        assert(shape.fetch(:answers).all? { |key, type, value| race_test_values(kind, incoming: true).find { |row| row.fetch("attr_key") == key }.fetch("value_#{type}") == value }, "independent incoming channels and values")
        ATTRIBUTE_RACE_MODES.each do |mode|
          %w[fixture a b].each do |actor|
            query = attribute_race_query(kind, mode, actor)
            assert(query.scan("COMMIT;").length == 1 && query.scan("BEGIN;").length == 1, "one real transaction per actor")
            assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|DISABLE|advisory|\\connect/), "no SQL, constraint, compiler or scheduling repair")
            assert(query.include?("FROM public.search_result_ingest(") == (actor != "b"), "only A and the non-ID fixture invoke the real wrapper")
            assert(query.include?("INSERT INTO public.canonical_torrent_source_attr") == (actor == "b"), "only B directly inserts child attributes")
            assert(query.include?("helpers:") == (actor == "a" && mode == "helpers-first"), "same-backend helpers precede A only")
            if actor == "a" && mode == "helpers-first"
              assert(query.index("helpers:") < query.index("FROM public.search_result_ingest("), "helpers are not after ingestion")
            end
          end
        end
      end
      rejected("unknown") { attribute_race_query("bad", "cold", "a") }
      rejected("unknown") { attribute_race_query("eight-non-id", "warm", "a") }
      rejected("unknown") { attribute_race_writer_rows("bad") }
      assert(ingestion_call({}).sub("_v1(", "(")[0, 69] == "SELECT row_to_json(r) FROM public.search_result_ingest(search_request", "wait prefix uses exact actual wrapper bytes")
    end

    def race_test_mutations!(original, kind, mode, variant, role)
      validate = ->(value) { attribute_race_validate!(value, kind, mode, variant, role, observed: true) }
      %w[fixture b a].each do |actor|
        %w[before after finished_setting within outside backend clock state].each do |key|
          changed = copy(original)
          changed.fetch(actor).fetch("parsed").fetch("frame")[key] = "altered"
          race_test_refresh!(changed, mode)
          rejected(key == "state" ? "ingestion correction" : "attribute race") { validate.call(changed) }
        end
        %w[session current superuser create_role bypass_rls].each do |key|
          changed = copy(original)
          changed.fetch(actor).fetch("parsed").fetch("frame").fetch("role")[key] = "altered"
          race_test_refresh!(changed, mode)
          rejected("attribute race") { validate.call(changed) }
        end
        %w[database application isolation].each do |key|
          changed = copy(original)
          changed.fetch(actor).fetch("parsed").fetch("context")[key] = "altered"
          race_test_refresh!(changed, mode)
          rejected("attribute race") { validate.call(changed) }
        end
      end
      # Alter every stored column wherever the row appears, including raw frames
      # and external snapshots. Agreement between copies is not an oracle.
      original.fetch("after").fetch("tables").each do |table, rows|
        columns = rows.empty? ? [nil] : rows.first.keys
        columns.each do |column|
          changed = copy(original)
          images = %w[fixture b a].flat_map { |actor| changed.fetch(actor).fetch("parsed").fetch("frame").values_at("tables_before", "tables_after", "tables_finish") }
          images += %w[seeded before held waiting after].map { |stage| changed.fetch(stage).fetch("tables") }
          images.each do |tables|
            if column
              tables.fetch(table).each { |row| row[column] = "altered" }
            else
              tables.fetch(table) << { "unexpected" => true }
            end
          end
          race_test_refresh!(changed, mode)
          rejected("attribute") { validate.call(changed) }
        end
      end
      %w[seeded before held waiting after].each do |stage|
        IngestionProof::INGESTION_TABLES.each do |table|
          changed = copy(original)
          record = changed.fetch(stage)
          if record.fetch("data").fetch("sequences").key?(table)
            record.fetch("data").fetch("sequences")[table] = 999
            record["stdout"] = JSON.generate(record.fetch("data")) + "\n"
          else
            sequence = JSON.parse(record.fetch("remaining").fetch("stdout"))
            sequence[table] = 999
            record.fetch("remaining")["stdout"] = JSON.generate(sequence) + "\n"
          end
          rejected("sequences") { validate.call(changed) }
        end
      end
      IngestionPolicy::POLICY_READ_TABLES.each do |table|
        changed = copy(original)
        %w[seeded before held waiting after].each do |stage|
          record = changed.fetch(stage)
          record.fetch("data").fetch("inputs").fetch(table) << { "unexpected" => true }
          record["stdout"] = JSON.generate(record.fetch("data")) + "\n"
        end
        rejected("read inputs") { validate.call(changed) }
      end
      wait = JSON.parse(original.fetch("wait").fetch("stdout")).first
      wait.each_key do |key|
        changed = copy(original)
        changed.fetch("wait")["stdout"] = JSON.generate([wait.merge(key => "altered")])
        rejected("owned lock wait") { validate.call(changed) }
      end
      records = original.fetch("a").fetch("stderr").split(/(?=^(?:NOTICE|ERROR):  )/)
      [records.drop(1), records.reverse, records + [records.first], []].each do |altered|
        changed = copy(original)
        changed.fetch("a")["stderr"] = altered.join
        rejected("attribute race") { validate.call(changed) }
      end
      records.each_with_index do |record, index|
        next unless record.start_with?("NOTICE:")

        event = JSON.parse(record.lines.first.split("ingestion-attribute-update:", 2).last)
        event.each_key do |key|
          altered = records.dup
          altered[index] = record.sub(/ingestion-attribute-update:.*\n/, "ingestion-attribute-update:#{JSON.generate(event.merge(key => 'altered'))}\n")
          changed = copy(original)
          changed.fetch("a")["stderr"] = altered.join
          rejected("UPDATE event") { validate.call(changed) }
        end
        changed = copy(original)
        changed.fetch("a")["stderr"] = records.each_with_index.map { |item, offset| offset == index ? item.sub("at SQL statement", "at PERFORM") : item }.join
        rejected("exact stack") { validate.call(changed) }
      end
      %w[controller writer_identity seed_clock database].each do |key|
        changed = copy(original)
        changed.delete(key)
        rejected("attribute race") { validate.call(changed) }
      end
      changed = copy(original)
      changed.fetch("a")["stdout"] = changed.fetch("a").fetch("stdout").sub('role:{', 'role:{"session":"duplicate",')
      rejected("duplicate metadata JSON") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("a")["stderr"] = changed.fetch("a").fetch("stderr").sub('"operation":', '"operation":"INSERT","operation":')
      rejected("duplicate metadata JSON") { validate.call(changed) }
      changed = copy(original)
      changed.fetch("a").fetch("parsed").fetch("frame")["after"] = "use_column"
      rejected("raw and declared") { validate.call(changed) }
      return unless kind == "eleven-all-id" && variant == "reference"

      %w[42P10 2107 plancat.c:920].each do |text|
        changed = copy(original)
        changed.fetch("a")["stderr"] = changed.fetch("a").fetch("stderr").sub(text, "wrong")
        rejected("attribute race") { validate.call(changed) }
      end
      changed = copy(original)
      frame = changed.fetch("a").fetch("parsed").fetch("frame")
      frame["tables_after"] = frame["tables_finish"] = race_test_initial
      changed.fetch("after")["tables"] = race_test_initial
      race_test_refresh!(changed, mode)
      rejected("18-table transition") { validate.call(changed) }
    end

    def race_test_transport!
      sample = race_test_evidence("eight-non-id", "cold", "final")
      probe = dup
      writes = {}
      probe.define_singleton_method(:metadata_write) { |name, bytes| writes[name] = bytes }
      probe.define_singleton_method(:result) { |_query, **_options| Struct.new(:stdout, :stderr, :success).new(sample.fetch("a").fetch("stdout"), "", true) }
      raw = probe.send(:attribute_race_execute, "eight-non-id", "cold", "a", "unit_attribute_race", @runtime, "unit")
      assert(raw == sample.fetch("a") && writes.keys.sort == %w[unit.sql unit.stderr unit.stdout], "actual wrapper transport retains raw bytes")
      statements = []
      probe.define_singleton_method(:sql) { |query, **_options| statements << query; "" }
      probe.send(:attribute_race_observer!, "unit_attribute_race", "unit")
      assert(statements.one? && statements.first.include?("AFTER UPDATE ON public.canonical_torrent_source_attr") && statements.first.include?("row_to_json(OLD)") &&
        !statements.first.include?("CREATE OR REPLACE") && !statements.first.include?("search_result_ingest"), "disposable trace observes UPDATE without editing ingestion")
      probe.define_singleton_method(:metadata_read) { |_database, _name| sample.fetch("waiting").reject { |key, _| %w[remaining tables].include?(key) } }
      probe.define_singleton_method(:metadata_transport) { |query, *_args| statements << query; sample.fetch("waiting").fetch("remaining") }
      probe.define_singleton_method(:ingestion_snapshot) { |_database| sample.fetch("waiting").fetch("tables") }
      assert(probe.send(:attribute_race_read, "unit_attribute_race", "unit") == sample.fetch("waiting"), "read transport combines all 18 sequences and tables")
      assert(statements.last.include?("observation_id") && statements.last.include?("canonical_size_rollup_id"), "remaining identity sequences use actual identity columns")
      polls = 0
      probe.define_singleton_method(:metadata_transport) do |query, *_args|
        statements << query
        polls += 1
        polls == 1 ? { "stdout" => "[]", "stderr" => "" } : sample.fetch("wait")
      end
      assert(probe.send(:attribute_race_wait, "unit_attribute_race", sample.fetch("writer_identity"), "unit") == sample.fetch("wait") && polls == 2, "owned transaction-lock poll retains successful observation")
      assert(statements.last.include?("b.pid=102") && statements.last.include?("b.datname=a.datname") && statements.last.include?("pg_blocking_pids"), "wait is constrained to the owned database and writer")
      probe.define_singleton_method(:metadata_transport) { |*_args| { "stdout" => "[{},{}]", "stderr" => "" } }
      rejected("owned lock wait absent") { probe.send(:attribute_race_wait, "unit_attribute_race", sample.fetch("writer_identity"), "unit") }
    end

    def race_test_controller!
      sample = race_test_evidence("eight-non-id", "cold", "final")
      before, after = sample.fetch("b").fetch("stdout").split("outside:", 2)
      script = <<~RUBY
        STDOUT.sync = true
        STDIN.each_line do |line|
          if line.include?("SELECT 'writer_xid:'")
            STDOUT.write(#{(before + "writer_xid:900\nheld\n").inspect})
          elsif line.start_with?('COMMIT;')
            STDOUT.write("released\\n")
          elsif line.include?("SELECT 'finished'")
            STDOUT.write(#{("outside:" + after + "finished\n").inspect})
          end
        end
      RUBY
      [false, true].each do |bad_wait|
        probe = dup
        writes = {}
        release = Queue.new
        started = Queue.new
        probe.define_singleton_method(:command) { |_role, _database| [RbConfig.ruby, "--disable-gems", "-e", script] }
        probe.define_singleton_method(:metadata_write) { |name, bytes| writes[name] = bytes }
        probe.define_singleton_method(:guid_controller_read) do |output, marker, transcript|
          super(output, marker, transcript)
          release << true if marker == "released"
        end
        probe.define_singleton_method(:attribute_race_execute) do |*_args|
          started << true
          release.pop unless bad_wait
          sample.fetch("a")
        end
        probe.define_singleton_method(:attribute_race_read) { |_database, name| sample.fetch(name.end_with?("held") ? "held" : "waiting") }
        probe.define_singleton_method(:attribute_race_wait) do |*_args|
          started.pop
          raise Failure, "attribute race injected wrong wait" if bad_wait

          sample.fetch("wait")
        end
        if bad_wait
          rejected("injected wrong wait") { probe.send(:attribute_race_interleave, "eight-non-id", "cold", "unit_attribute_race", @runtime, "unit") }
          assert(!writes.key?("unit-release-checkpoint.json") && !JSON.parse(writes.fetch("unit-controller.json")).fetch("stdout").include?("released"), "failed wait closes writer without COMMIT")
        else
          actual = probe.send(:attribute_race_interleave, "eight-non-id", "cold", "unit_attribute_race", @runtime, "unit")
          assert(actual == sample.slice("a", "b", "controller", "writer_identity", "wait", "held", "waiting"), "controller preserves actual split transaction evidence")
          assert(writes.keys.index("unit-release-checkpoint.json") < writes.keys.index("unit-controller.json"), "wait checkpoint precedes release and final controller record")
        end
        controller = JSON.parse(writes.fetch("unit-controller.json"))
        assert(controller.values_at("success", "stderr") == [true, ""], "owned controller is reaped and raw diagnostics retained")
      end
    end

    def race_test_lifecycle!
      sample = race_test_evidence("eight-non-id", "cold", "final")
      probe = dup
      statements = []
      probe.define_singleton_method(:sql) { |query, **_args| statements << query; "" }
      probe.define_singleton_method(:policy_setup!) { |*_args| sample.fetch("seed_clock") }
      probe.define_singleton_method(:correction_observer!) { |*_args| true }
      probe.define_singleton_method(:attribute_race_observer!) { |*_args| true }
      probe.define_singleton_method(:attribute_race_read) { |_database, name| sample.fetch(name.split("-").last) }
      probe.define_singleton_method(:attribute_race_execute) { |*_args| sample.fetch("fixture") }
      probe.define_singleton_method(:attribute_race_interleave) { |*_args| sample.slice("a", "b", "writer_identity", "controller", "wait", "held", "waiting") }
      probe.define_singleton_method(:metadata_write) { |*_args| true }
      probe.define_singleton_method(:attribute_race_validate!) do |evidence, *_args, **_kwargs|
        raise Failure, "attribute race injected validation failure" unless evidence.fetch("database").start_with?("ingestion_attribute_race_final_")

        evidence
      end
      probe.instance_variable_set(:@metadata_evidence, "/private/tmp")
      result = probe.send(:attribute_race_isolated, "eight-non-id", "cold", "final", "source_unit", @runtime, observed: true)
      assert(result.fetch("a") == sample.fetch("a") && statements.last.start_with?("DROP DATABASE") && statements.last.include?("WITH (FORCE)"), "isolated driver exercises fixture, race and forced cleanup")
      probe.define_singleton_method(:attribute_race_interleave) { |*_args| raise Failure, "injected controller failure" }
      rejected("injected controller failure") { probe.send(:attribute_race_isolated, "eight-non-id", "cold", "final", "source_unit", @runtime, observed: false) }
      assert(statements.last.start_with?("DROP DATABASE"), "isolated failure still removes only its clone")
      hashes = attribute_race_source_hashes
      assert(hashes.keys.include?("scripts/database_rebaseline/ingestion_attribute_race.rb") && hashes.keys.include?("scripts/tests/database-ingestion-attribute-race-test.rb") && hashes.values.all? { |digest| digest.match?(/\A[0-9a-f]{64}\z/) }, "current source provenance includes both owned files")
      Dir.mktmpdir("attribute-race-unit-") do |directory|
        producer = dup
        contract = @contract.dup
        contract.define_singleton_method(:output_path) { directory }
        producer.instance_variable_set(:@contract, contract)
        producer.instance_variable_set(:@metadata_evidence, "previous-metadata")
        producer.instance_variable_set(:@correction_evidence, "previous-correction")
        calls = []
        producer.define_singleton_method(:attribute_race_isolated) do |kind, mode, variant, _source, role, observed:|
          calls << [kind, mode, variant, observed]
          value = race_test_evidence(kind, mode, variant)
          value.fetch("a")["stderr"] = race_test_notices(kind, variant, role) + value.fetch("a").fetch("stderr") if observed
          attribute_race_validate!(value, kind, mode, variant, role, observed:)
        end
        producer.send(:verify_ingestion_attribute_race!)
        reports = Dir.glob(File.join(directory, "ingestion-attribute-race/run-*/report.json"))
        report = JSON.parse(File.binread(reports.fetch(0)))
        assert(calls.length == 16 && reports.one? && report.values_at("passed", "d3_complete") == [true, false], "producer covers every paired cold/helper case without D3 closure")
        assert(report.fetch("cases").select { |item| item.fetch("kind") == "eleven-all-id" }.all? { |item| !item.fetch("equivalent") && item.fetch("approved_delta").include?("D5") }, "frozen failures remain distinct from independently accepted final results")
        assert(producer.instance_variable_get(:@metadata_evidence) == "previous-metadata" && producer.instance_variable_get(:@correction_evidence) == "previous-correction", "producer restores parent evidence owners")
        producer.define_singleton_method(:attribute_race_isolated) { |*_args, **_kwargs| raise Failure, "injected producer failure" }
        rejected("injected producer failure") { producer.send(:verify_ingestion_attribute_race!) }
        reports = Dir.glob(File.join(directory, "ingestion-attribute-race/run-*/report.json"))
        assert(reports.length == 2 && reports.map { |path| JSON.parse(File.binread(path)).fetch("passed") }.sort_by(&:to_s) == [false, true], "failed run has separate retained report, never overwrites success")
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    raise RevaerDatabaseRebaseline::Failure, "expected no arguments; parent owns live qualification" unless ARGV.empty?

    RevaerDatabaseRebaseline::IngestionAttributeRaceTest.new.run_tests!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-attribute-race-test: #{error.message}"
    exit 1
  end
end
