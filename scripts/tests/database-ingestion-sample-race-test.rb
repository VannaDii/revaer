# frozen_string_literal: true

require "rbconfig"
require_relative "database-ingestion-sampling-test"
require_relative "../database_rebaseline/ingestion_sample_race"

module RevaerDatabaseRebaseline
  class IngestionSampleRaceTest < IngestionSamplingTest
    include IngestionSampleRace

    def run_tests!
      @assertions = 0
      @runtime = "sample_race_runtime"
      @owner = "sample_race_owner"
      sample_test_inventory!
      sample_test_sources!
      comparables = IngestionSampling::SAMPLING_MODES.product(%w[reference final], [false, true]).map do |mode, variant, observed|
        role = variant == "reference" ? "postgres" : @runtime
        evidence = sample_test_evidence(mode, variant, role, observed:)
        value = sample_race_validate!(evidence, mode, variant, role, observed:)
        assert(value.fetch("state") == "00000", "K3 #{mode}/#{variant}/#{observed} synthetic oracle")
        assert(value.fetch("after").keys.sort == IngestionProof::INGESTION_TABLES.sort, "all 18 tables compared")
        assert(value.fetch("after").fetch("canonical_size_sample").empty?, "no surviving samples")
        assert(value.fetch("before").fetch("canonical_size_rollup") == value.fetch("after").fetch("canonical_size_rollup"), "explicit prior rollup unchanged")
        value
      end
      assert(comparables.uniq.one?, "exact cold/helper-first frozen/final and plain/NOTICE comparisons")
      sample_test_mutations!
      sample_test_schedule!
      sample_test_processes!
      puts "database-ingestion-sample-race-test: #{@assertions} assertions passed (synthetic/source/process tests; no database execution)"
    end

    private

    def sample_test_inventory!
      routines = { "search_result_ingest_v1" => "0052_indexer_search_result_ingest_proc.sql",
        "search_result_ingest" => "0120_search_result_ingest_seed_best_source_context.sql" }.map do |name, path|
        sql = File.binread(File.join(@contract.root, "crates/revaer-data/migrations", path))
        body = sql.split("CREATE OR REPLACE FUNCTION #{name}(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
        { "name" => name, "signature" => "#{name}(uuid)", "source" => body }
      end
      final = copy(routines)
      source = FinalSql.new(@contract).approved_ingestion_body(final.first.fetch("source"))
      final.first["source"] = "\n#variable_conflict use_column\n#{source.delete_prefix("\n")}"
      @ingestion_inventory = { "reference_proof" => { "routines" => routines }, @database => { "routines" => final } }
    end

    def sample_test_sources!
      %w[reference final].each do |variant|
        source = sample_race_routine(variant).fetch("source")
        changed = sample_race_instrument(source)
        assert(source.lines.length == changed.lines.length, "observer preserves original native line coordinates")
        assert(changed.lines.zip(source.lines).count { |left, right| left != right } == 1, "exact single-line disposable NOTICE insertion")
        assert(changed.sub(/RAISE NOTICE 'ingestion-sample-race:.*?; /, "") == source, "observer restores exact source bytes")
        %w[OFFSET\ 25 sample_count\ >\ 0 percentile_cont(0.5) ON\ CONFLICT\ DO\ NOTHING].each do |token|
          rejected("source site changed") { sample_race_instrument(source.sub(token, "altered")) }
        end
        rejected("source site changed") { sample_race_instrument(source + source) }
      end
      query = correction_session(sample_race_session("helpers-first"))
      assert(query.scan("FROM public.search_result_ingest(").one?, "one actual application-wrapper call on fresh backend")
      assert(query.index("helpers:") < query.index("FROM public.search_result_ingest("), "real helper-first answers precede call")
      assert(query.include?("'2026-09-10T00:25:00Z'::timestamptz") && query.include?("size_bytes_input => 1024::bigint"), "newest exact positive-size tuple")
      assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no warm compiler/temp repair")
      assert(sample_race_seed_sql.include?("generate_series(0, 25)") && sample_race_seed_sql.include?("VALUES (1, 26, 1024, 1024, 1024,"), "26 distinct samples and explicit prior rollup")
      assert(!sample_race_seed_sql.match?(/DISABLE|session_replication_role|SET CONSTRAINTS|INSERT INTO public\.canonical_[^;]*OVERRIDING/m), "no disabled constraints or replaced identities")
      wait_query = sample_race_wait_sql("numeric-oid-control", { "pid" => 11 }, { "context" => { "pid" => 12 } })
      assert(wait_query.include?("relation::bigint AS relation"), "lock relation OIDs must use the same numeric JSON representation as tuples")
      rejected("unknown compilation mode") { sample_race_session("warm-fixed") }
    end

    def sample_test_context(pid, transaction, second, role, application)
      { "pid" => pid, "database" => @sample_test_database, "session" => role, "current" => role,
        "transaction" => transaction.to_s, "clock" => "2026-09-12T00:00:0#{second}+00:00",
        "isolation" => "read committed", "setting" => "error", "application" => application }
    end

    # Independently authored golden rows, never copied from the validator's state builder.
    def sample_test_tables(seed, clock = nil)
      tables = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      magnet = "750837cbeaadaf72ab0a852ee3e0517760ca56eb6b8fc3c08b2d9e73348b6bea"
      canonical_uuid = "56900000-0000-4000-8000-000000000090"
      source_uuid = "56900000-0000-4000-8000-000000000091"
      observed = clock ? "2026-09-10T00:25:00+00:00" : "2026-09-10T00:00:00+00:00"
      tables["canonical_torrent"] = [{ "canonical_torrent_id" => 1, "canonical_torrent_public_id" => canonical_uuid,
        "identity_confidence" => 1.0, "identity_strategy" => "infohash_v1", "infohash_v1" => "a" * 40,
        "infohash_v2" => nil, "magnet_hash" => magnet, "title_size_hash" => nil, "imdb_id" => nil, "tmdb_id" => nil,
        "tvdb_id" => nil, "ids_confidence" => nil, "title_display" => "Size proof", "title_normalized" => "size proof",
        "size_bytes" => 1024, "created_at" => seed, "updated_at" => clock || seed }]
      tables["canonical_torrent_source"] = [{ "canonical_torrent_source_id" => 1, "indexer_instance_id" => 569001,
        "canonical_torrent_source_public_id" => source_uuid, "source_guid" => "size-source", "infohash_v1" => "a" * 40,
        "infohash_v2" => nil, "magnet_hash" => magnet, "title_normalized" => "size proof", "size_bytes" => 1024,
        "last_seen_at" => observed, "last_seen_seeders" => 5, "last_seen_leechers" => 2, "last_seen_published_at" => nil,
        "last_seen_download_url" => nil, "last_seen_magnet_uri" => nil, "last_seen_details_url" => nil, "last_seen_uploader" => nil,
        "created_at" => seed, "updated_at" => clock || seed }]
      tables["canonical_size_rollup"] = [{ "canonical_size_rollup_id" => 1, "canonical_torrent_id" => 1,
        "sample_count" => 26, "size_median" => 1024, "size_min" => 1024, "size_max" => 1024, "updated_at" => "2026-09-09T00:00:00+00:00" }]
      unless clock
        tables["canonical_size_sample"] = (1..26).map do |id|
          { "canonical_size_sample_id" => id, "canonical_torrent_id" => 1,
            "observed_at" => "2026-09-10T00:#{format('%02d', id - 1)}:00+00:00", "size_bytes" => 1024 }
        end
        return tables
      end
      tables["canonical_torrent_source_context_score"] = [{ "canonical_torrent_source_context_score_id" => 1,
        "context_key_type" => "search_request", "context_key_id" => 569001, "canonical_torrent_id" => 1,
        "canonical_torrent_source_id" => 1, "score_total_context" => 0.0, "score_policy_adjust" => 0.0,
        "score_tag_adjust" => 0.0, "is_dropped" => false, "computed_at" => clock }]
      tables["canonical_torrent_best_source_context"] = [{ "canonical_torrent_best_source_context_id" => 1,
        "context_key_type" => "search_request", "context_key_id" => 569001, "canonical_torrent_id" => 1,
        "canonical_torrent_source_id" => 1, "computed_at" => clock }]
      tables["search_request_source_observation"] = [{ "observation_id" => 1, "search_request_id" => 569001, "indexer_instance_id" => 569001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "observed_at" => observed, "seeders" => 5, "leechers" => 2,
        "published_at" => nil, "uploader" => nil, "source_guid" => "size-source", "details_url" => nil, "download_url" => nil,
        "magnet_uri" => nil, "title_raw" => "Size proof", "size_bytes" => 1024, "infohash_v1" => "a" * 40, "infohash_v2" => nil,
        "magnet_hash" => magnet, "guid_conflict" => false, "was_downranked" => false, "was_flagged" => false }]
      tables["search_request_canonical"] = [{ "search_request_canonical_id" => 1, "search_request_id" => 569001,
        "canonical_torrent_id" => 1, "first_seen_at" => clock }]
      tables["search_page"] = [{ "search_page_id" => 1, "search_request_id" => 569001, "page_number" => 1, "sealed_at" => nil }]
      tables["search_page_item"] = [{ "search_page_item_id" => 1, "search_page_id" => 1, "search_request_canonical_id" => 1, "position" => 1 }]
      tables
    end

    def sample_test_notice(context, variant)
      event = { "sample_count" => 0, "median" => nil, "minimum" => nil, "maximum" => nil, "canonical_id" => 1,
        "allowed" => true, "observed_at" => "2026-09-10T00:25:00+00:00", "size" => 1024,
        "backend" => context.fetch("pid"), "transaction" => context.fetch("transaction"), "session" => context.fetch("session"),
        "current" => variant == "reference" ? "postgres" : @owner, "setting" => variant == "reference" ? "use_column" : "error", "clock" => context.fetch("clock") }
      routine = sample_race_routine(variant)
      line = routine.fetch("source").lines.index { |text| text.include?("IF sample_count IS NOT NULL AND sample_count > 0 THEN") } + 1
      "NOTICE:  00000: ingestion-sample-race:#{JSON.generate(event)}\n" \
        "CONTEXT:  PL/pgSQL function #{routine.fetch('signature')} line #{line} at RAISE\n" \
        "#{disambiguation_wrapper_stack}LOCATION:  exec_stmt_raise, pl_exec.c:3897\n"
    end

    def sample_test_frame_transport(frame, mode, context, stderr)
      lines = []
      lines << "helpers:#{JSON.generate(ingestion_helper_expectations)}" if mode == "helpers-first"
      correction_records(finish_setting: true).each do |key|
        if key == "state"
          lines << "race_context:#{JSON.generate(context)}" << "sample_race_ready"
          lines << JSON.generate(frame.fetch("result")) if frame.key?("result")
        end
        value = frame.fetch(key)
        lines << "#{key}:#{%w[role clock tables_before tables_after tables_finish].include?(key) ? JSON.generate(value) : value}"
      end
      { "stdout" => (lines + ["sample_race_finished"]).join("\n") + "\n", "stderr" => stderr }
    end

    def sample_test_evidence(mode, variant, role, observed:)
      @sample_test_database = sample_race_database(variant)
      seed = sample_test_context(90, 100, 0, "postgres", "sample-race-seed")
      writer = sample_test_context(91, 101, 1, "postgres", "sample-race-writer")
      context = sample_test_context(92, 102, 2, role, "sample-race-ingest")
      released = sample_test_context(91, 104, 4, "postgres", "sample-race-writer")
      before = sample_test_tables(seed.fetch("clock"))
      after = sample_test_tables(seed.fetch("clock"), context.fetch("clock"))
      frame = attributes_test_frame(role, 2, before, after).merge("backend" => "92", "finished_setting" => "error")
      frame.fetch("result")["observation_created"] = true
      rows = before.fetch("canonical_size_sample")
      locked = { "context" => writer, "rows" => rows.map { |row| row.merge("tuple" => "(0,#{row.fetch('canonical_size_sample_id')})", "relation" => 200) } }
      transcript = "locked:#{JSON.generate(locked)}\nsample_race_locked\n" \
        "deleted:#{JSON.generate({ 'context' => writer, 'rows' => rows })}\n" \
        "writer_tables:#{JSON.generate(before.merge('canonical_size_sample' => []))}\n" \
        "released:#{JSON.generate(released)}\nsample_race_released\n"
      wait = { "reader" => sample_test_context(93, 103, 3, "postgres", "psql"), "query_bytes" => 1024,
        "activity" => { "pid" => 92, "database" => @sample_test_database, "role" => role, "application" => "sample-race-ingest",
          "state" => "active", "wait_type" => "Lock", "wait_event" => "transactionid", "transaction" => "102",
          "query" => sample_race_call.byteslice(0, 1023), "blockers" => [91] },
        "locks" => [
          [92, "transactionid", "ShareLock", false, nil, nil, nil, "101"],
          [91, "transactionid", "ExclusiveLock", true, nil, nil, nil, "101"],
          [92, "tuple", "AccessExclusiveLock", true, 200, 0, 1, nil],
          [92, "relation", "RowExclusiveLock", true, 200, nil, nil, nil],
          [91, "relation", "RowShareLock", true, 200, nil, nil, nil]
        ].map { |values| %w[pid locktype mode granted relation page tuple transaction].zip(values).to_h } }
      source = sample_race_routine(variant).fetch("source")
      inputs = metadata_read_tables(seed.fetch("clock"))
      { "database" => @sample_test_database, "seed" => { "stdout" => JSON.generate(seed), "stderr" => "" },
        "initial" => sampling_test_read(before, inputs, 26), "waiting" => sampling_test_read(before, inputs, 27),
        "after" => sampling_test_read(after, inputs, 27), "writer" => { "stdout" => transcript, "stderr" => "" },
        "wait" => { "stdout" => JSON.generate(wait), "stderr" => "", "data" => wait },
        "definition" => { "original_sha256" => Digest::SHA256.hexdigest(source), "tested_sha256" => Digest::SHA256.hexdigest(observed ? sample_race_instrument(source) : source) },
        "caller" => sample_test_frame_transport(frame, mode, context, observed ? sample_test_notice(context, variant) : "") }
    end

    def sample_test_reject(original)
      changed = copy(original)
      yield changed
      rejected("") { sample_race_validate!(changed, "helpers-first", "final", @runtime, observed: true) }
    end

    def sample_test_mutations!
      original = sample_test_evidence("helpers-first", "final", @runtime, observed: true)
      sample_test_reject(original) { |e| e.fetch("wait").fetch("data").fetch("activity")["pid"] = 999 }
      sample_test_reject(original) { |e| e["database"] = "unowned" }
      sample_test_reject(original) do |e|
        e.fetch("wait").fetch("data")["query_bytes"] = 2048
        e.fetch("wait")["stdout"] = JSON.generate(e.fetch("wait").fetch("data"))
      end
      original.fetch("wait").fetch("data").fetch("reader").each_key do |key|
        sample_test_reject(original) do |e|
          data = e.fetch("wait").fetch("data")
          data.fetch("reader")[key] = "tampered"
          e.fetch("wait")["stdout"] = JSON.generate(data)
        end
      end
      sample_test_reject(original) do |e|
        e.fetch("caller")["stdout"] = "sample_race_ready\n" + e.fetch("caller").fetch("stdout").sub("sample_race_ready\n", "")
      end
      original.fetch("wait").fetch("data").fetch("activity").each_key do |key|
        sample_test_reject(original) do |e|
          data = e.fetch("wait").fetch("data")
          data.fetch("activity")[key] = "tampered"
          e.fetch("wait")["stdout"] = JSON.generate(data)
        end
      end
      original.fetch("wait").fetch("data").fetch("locks").each_with_index do |lock, index|
        lock.each_key do |key|
          sample_test_reject(original) do |e|
            data = e.fetch("wait").fetch("data")
            data.fetch("locks").fetch(index)[key] = "tampered"
            e.fetch("wait")["stdout"] = JSON.generate(data)
          end
        end
      end
      %w[initial waiting after].each do |phase|
        %w[tables inputs].each do |group|
          original.fetch(phase).fetch("data").fetch(group).each do |table, rows|
            mutations = [[nil, nil]] + rows.each_with_index.flat_map { |row, i| row.keys.map { |key| [i, key] } }
            mutations.each do |index, key|
              sample_test_reject(original) do |e|
                data = e.fetch(phase).fetch("data")
                target = data.fetch(group).fetch(table)
                index.nil? ? target.push({ "unexpected" => true }) : target.fetch(index)[key] = "tampered"
                e.fetch(phase)["stdout"] = JSON.generate(data)
              end
            end
          end
        end
        [nil, 0, 26.0, 28].each do |sequence|
          sample_test_reject(original) do |e|
            e.fetch(phase).fetch("data")["sample_sequence"] = sequence
            e.fetch(phase)["stdout"] = JSON.generate(e.fetch(phase).fetch("data"))
          end
        end
      end
      %w[locked deleted writer_tables released].each do |prefix|
        record = JSON.parse(JSON.generate(sample_race_line(original.fetch("writer").fetch("stdout"), prefix)))
        paths = prefix == "writer_tables" ? record.keys.map { |key| [key] } :
          (record.fetch("context", record).keys.map { |key| record.key?("context") ? ["context", key] : [key] })
        paths << ["rows"] if record.key?("rows")
        paths.each do |path|
          sample_test_reject(original) do |e|
            altered = copy(record)
            parent = path.length == 1 ? altered : altered.fetch(path.first)
            parent[path.last] = "tampered"
            e.fetch("writer")["stdout"] = e.fetch("writer").fetch("stdout").sub("#{prefix}:#{JSON.generate(record)}", "#{prefix}:#{JSON.generate(altered)}")
          end
        end
      end
      %w[backend before after finished_setting clock within outside state role tables_before tables_after tables_finish].each do |key|
        sample_test_reject(original) do |e|
          e.fetch("caller")["stdout"] = e.fetch("caller").fetch("stdout").sub(/^#{key}:.*$/, "#{key}:#{%w[role tables_before tables_after tables_finish].include?(key) ? '{}' : 'tampered'}")
        end
      end
      %w[pid database session current transaction clock isolation setting application].each do |key|
        sample_test_reject(original) do |e|
          context = sample_race_line(e.fetch("caller").fetch("stdout"), "race_context").to_h
          context[key] = "tampered"
          e.fetch("caller")["stdout"] = e.fetch("caller").fetch("stdout").sub(/^race_context:.*$/, "race_context:#{JSON.generate(context)}")
        end
      end
      notice = original.fetch("caller").fetch("stderr")
      event = metadata_json_parse(notice.lines.first.split("ingestion-sample-race:", 2).last)
      event.each_key do |key|
        sample_test_reject(original) do |e|
          altered = event.merge(key => "tampered")
          e.fetch("caller")["stderr"] = notice.sub(JSON.generate(event), JSON.generate(altered))
        end
      end
      [nil, 0.0, 1].each do |count|
        sample_test_reject(original) do |e|
          e.fetch("caller")["stderr"] = notice.sub(JSON.generate(event), JSON.generate(event.merge("sample_count" => count)))
        end
      end
      ["", notice * 2, notice.sub("at RAISE", "at SQL statement"), notice.sub("line 9", "line 10"),
       notice.sub("pl_exec.c:3897", "pl_exec.c:1"), notice + "WARNING: extra\n"].each do |stderr|
        sample_test_reject(original) { |e| e.fetch("caller")["stderr"] = stderr }
      end
      sample_test_reject(original) { |e| e.fetch("definition")["tested_sha256"] = "0" * 64 }
      %w[reference final].each do |variant|
        role = variant == "reference" ? "postgres" : @runtime
        evidence = sample_test_evidence("cold", variant, role, observed: false)
        evidence.fetch("caller")["stdout"] = evidence.fetch("caller").fetch("stdout").lines.reject { |line| line.start_with?("{") }.join.sub("state:00000", "state:42P07")
        evidence.fetch("caller")["stderr"] = sampling_diagnostic
        rejected("D4 is not success") { sample_race_validate!(evidence, "cold", variant, role, observed: false) }
      end
      duplicate = copy(original)
      duplicate.fetch("wait")["stdout"] = duplicate.fetch("wait").fetch("stdout").sub('"pid":92', '"pid":92,"pid":92')
      rejected("duplicate metadata JSON") { sample_race_validate!(duplicate, "helpers-first", "final", @runtime, observed: true) }
    end

    def sample_test_schedule!
      %w[valid wrong-wait early-delete].each do |case_name|
        original = sample_test_evidence("helpers-first", "final", @runtime, observed: true)
        events = []
        define_singleton_method(:sample_race_process) do |_database, _role, name, &operation|
          kind = name.end_with?("-writer") ? "writer" : "caller"
          process = { kind:, stdout: +"" }
          operation.call(process)
          process[:record] = original.fetch(kind)
        end
        define_singleton_method(:sample_race_send) do |process, query|
          if process.fetch(:kind) == "writer"
            if query.include?("DELETE FROM")
              assert(events.last == "validated-read", "writer deletion occurs only after exact wait and immutable read")
              assert(query.index("DELETE FROM") < query.index("COMMIT;"), "delete all samples before committing writer")
              events << "delete-commit"
            else
              assert(query.include?("ORDER BY observed_at FOR UPDATE") && !query.match?(/DELETE|UPDATE public/), "writer initially locks only")
              assert(query.include?("s.tableoid::bigint AS relation"), "writer relation OID must be encoded as a JSON number")
              events << "lock-all"
            end
          else
            assert(events.include?("sample_race_locked"), "ingestion starts after all tuple locks acquired")
            assert(query.include?(sample_race_call) && query.scan("FROM public.search_result_ingest(").one?, "controller invokes unchanged exact wrapper once")
            events << "ingest"
          end
        end
        define_singleton_method(:sample_race_marker) do |process, marker|
          transcript = original.fetch(process.fetch(:kind)).fetch("stdout")
          process[:stdout] = transcript.split("#{marker}\n", 2).first + "#{marker}\n"
          events << marker
        end
        define_singleton_method(:metadata_transport) do |query, _database, _role, _name|
          assert(query.include?("FROM pg_locks") && query.include?("a.pid = 92") && query.include?("91"), "monitor restricted to owned actors and actual lock inventory")
          assert(query.include?("DO $$ BEGIN NULL; END $$;") && query.include?("statement_timeout = '5s'"), "monitor loads caller GUC with bounded statement")
          record = copy(original.fetch("wait"))
          if case_name == "wrong-wait"
            record.fetch("data").fetch("activity")["wait_event"] = "advisory"
            record["stdout"] = JSON.generate(record.fetch("data"))
          end
          events << "observed-wait"
          record
        end
        define_singleton_method(:sampling_read) do |_database, _name|
          events << "validated-read"
          record = copy(original.fetch("waiting"))
          if case_name == "early-delete"
            record.fetch("data").fetch("tables")["canonical_size_sample"] = []
            record["stdout"] = JSON.generate(record.fetch("data"))
          end
          record
        end
        evidence = { "seed" => original.fetch("seed") }
        execute = -> { sample_race_interleave(original.fetch("database"), @runtime, "helpers-first", "unit", evidence) }
        if case_name == "valid"
          execute.call
          assert(events == %w[lock-all sample_race_locked ingest sample_race_ready observed-wait validated-read delete-commit sample_race_released sample_race_finished], "actual controller schedule and commit acknowledgement order")
          assert(evidence.fetch("caller") == original.fetch("caller") && evidence.fetch("writer") == original.fetch("writer"), "controller preserves exact raw transports")
        else
          rejected("sample race") { execute.call }
          assert(!events.include?("delete-commit"), "invalid wait or early deletion never releases writer")
        end
      ensure
        %i[sample_race_process sample_race_send sample_race_marker metadata_transport sampling_read].each { |method| singleton_class.remove_method(method) }
      end
    end

    def sample_test_processes!
      Dir.mktmpdir("sample-race-unit-") do |directory|
        @metadata_evidence = directory
        script = "STDOUT.sync = true; puts 'owned'; STDIN.read; warn 'retained failure'; exit 23"
        define_singleton_method(:command) { |_role, _database| [RbConfig.ruby, "--disable-gems", "-e", script] }
        rejected("owned process failed") do
          sample_race_process("unused", "unused", "failed") do |process|
            sample_race_marker(process, "owned")
          end
        end
        assert(File.binread(File.join(directory, "failed.stderr")) == "retained failure\n", "failed raw diagnostics retained")
        status = metadata_json_parse(File.binread(File.join(directory, "failed-status.json")))
        assert(status.fetch("exitstatus") == 23 && !status.fetch("success") && !status.fetch("forced"), "nonzero exit remains failure")
        script = "STDOUT.sync = true; trap('TERM') {}; puts 'owned'; sleep 300"
        define_singleton_method(:command) { |_role, _database| [RbConfig.ruby, "--disable-gems", "-e", script] }
        pid = nil
        started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        rejected("required termination") do
          sample_race_process("unused", "unused", "stalled") do |process|
            pid = process.fetch(:waiter).pid
            sample_race_marker(process, "owned")
          end
        end
        status = metadata_json_parse(File.binread(File.join(directory, "stalled-status.json")))
        assert(status.fetch("forced") && status.fetch("termsig") == Signal.list.fetch("KILL"), "only owned stalled process killed and retained")
        assert(Process.clock_gettime(Process::CLOCK_MONOTONIC) - started < 15, "owned process cleanup bounded")
        begin
          Process.waitpid(pid, Process::WNOHANG)
          raise Failure, "owned process was not reaped"
        rescue Errno::ECHILD
          assert(true, "owned process reaped")
        end
      ensure
        singleton_class.remove_method(:command)
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    raise RevaerDatabaseRebaseline::Failure, "unit tests take no arguments; parent owns live qualification" unless ARGV.empty?

    RevaerDatabaseRebaseline::IngestionSampleRaceTest.new.run_tests!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-sample-race-test: #{error.message}"
    exit 1
  end
end
