# frozen_string_literal: true

require_relative "database-ingestion-attributes-test"

module RevaerDatabaseRebaseline
  class IngestionMetadataTest < IngestionAttributesTest
    def run_tests!
      @assertions = 0
      @mutations = 0
      @runtime = "metadata_unit_runtime"
      source = File.binread(File.join(@contract.root, "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"))
      body = source.split("CREATE OR REPLACE FUNCTION search_result_ingest_v1(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
      @ingestion_inventory = { "reference_proof" => { "routines" => [{ "name" => "search_result_ingest_v1", "signature" => "search_result_ingest_v1(uuid)", "source" => body }] } }
      metadata_case_tests!
      metadata_external_tests!
      metadata_mutating_helper_tests!
      metadata_mutation_tests!
      metadata_transport_tests!
      metadata_json_tests!
      metadata_json_option_tests!
      metadata_clock_tests!
      metadata_identity_tests!
      metadata_producer_tests!
      metadata_isolated_tests!
      metadata_reachability_tests!(source)
      puts "database-ingestion-metadata-test: #{@assertions} assertions passed; #{@mutations} coherent semantic mutations rejected"
    end

    private

    # Reuse the uniquely owned, pinned, network-none attribute live driver. Its
    # mandatory D3/D4/D5 controls run first; only the bounded matrix is replaced.
    def verify_ingestion_attributes!
      verify_ingestion_metadata!
    ensure
      @attributes_evidence = @metadata_evidence
    end

    def attributes_source_hashes
      metadata_source_hashes
    end

    def attributes_cleanup!
      super
      [[@attributes_owned_container, "container", @container], [@attributes_owned_volume, "volume", @attributes_volume]].each do |owned, kind, name|
        next unless owned

        command = ["docker", kind, "ls"]
        command << "-a" if kind == "container"
        command += ["--format", "{{.Name#{kind == 'container' ? 's' : ''}}}", "--filter", "name=^#{kind == 'container' ? '/' : ''}#{name}$"]
        outcome = @runner.capture(command)
        @attributes_cleanup_records << { command:, stdout: outcome.stdout, stderr: outcome.stderr, success: outcome.success }
        raise Failure, "metadata owned resource absence not verified" unless outcome.success && outcome.stdout.strip.empty? && outcome.stderr.empty?
      end
    end

    def metadata_test_read(clock, counters)
      data = { "inputs" => metadata_read_tables(clock), "sequences" => counters.transform_values { |value| value.zero? ? nil : value } }
      { "data" => data, "stdout" => JSON.generate(data) + "\n", "stderr" => "" }
    end

    def metadata_test_assign!(tables, prior, shape, offset)
      rows = tables.fetch("search_request_source_observation_attr")
      old = prior.fetch("search_request_source_observation_attr").to_h { |row| [row.fetch("attr_key"), row.fetch("observation_attr_id")] }
      rows.each do |row|
        key = row.fetch("attr_key")
        row["observation_attr_id"] = old.fetch(key) { offset + shape.fetch(:answers).index { |answer| answer.first == key } + 1 }
      end
    end

    def metadata_test_evidence(spec, mode, variant)
      role = variant == "reference" ? "postgres" : @runtime
      identities = { "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000090",
                     "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-000000000091" }
      seed_clock = "2026-09-12T00:00:00+00:00"
      counters = METADATA_SEQUENCES.to_h { |table, _column| [table, 0] }
      reads = [metadata_test_read(seed_clock, counters)]
      prior = attributes_empty
      fixtures = spec.fetch(:fixtures).each_with_index.map do |shape, index|
        clock = "2026-09-12T00:00:0#{index + 1}+00:00"
        changed = index.zero? ? attributes_initial_tables(shape, clock, identities) : metadata_changed_tables(prior, shape, clock, counters)
        metadata_test_assign!(changed, prior, shape, counters.fetch("search_request_source_observation_attr"))
        frame = attributes_test_frame(role, index + 1, prior, changed, first: index.zero?)
        frame["backend"] = (100 + index).to_s
        metadata_advance!(counters, shape, first: index.zero?)
        reads << metadata_test_read(seed_clock, counters)
        prior = changed
        attributes_test_transport([frame], "cold")
      end
      external_fixture = nil
      extra = spec[:external_ids] ? 1 : 0
      if spec[:external_ids]
        changed = metadata_external_fixture_tables(prior)
        frame = attributes_test_frame("postgres", 2, prior, changed)
        frame.merge!("backend" => "150", "within" => "false", "outside" => "false",
          "result" => { "fixture" => "durable-external-ids", "rows" => 3 })
        stdout = attributes_test_transport([frame], "cold").fetch("stdout").split("\n", 2).last
        external_fixture = { "frames" => [frame], "stdout" => stdout, "stderr" => "" }
        counters["canonical_torrent_source_attr"] += 3
        reads << metadata_test_read(seed_clock, counters)
        prior = changed
      end
      before = prior
      helper = nil
      session = metadata_session(spec, mode)
      if session.key?(:helper_finish)
        helper = attributes_test_frame(role, fixtures.length + extra + 1, prior, prior)
        helper.merge!("backend" => "200", "within" => "false", "outside" => "false",
          "result" => { "helper" => "log_source_metadata_conflict_v1" })
        changed = metadata_helper_tables(prior, helper.fetch("clock"), counters)
        helper["tables_after"] = changed
        helper["tables_finish"] = session.fetch(:helper_finish) == "commit" ? changed : prior
        %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].each { |table| counters[table] += 1 }
        prior = helper.fetch("tables_finish")
      end
      frames = (0..2).map do |index|
        frame = attributes_test_frame(role, fixtures.length + extra + index + (helper ? 2 : 1), prior, prior)
        frame["backend"] = "200"
        frame["outside"] = (variant == "reference" && index.positive?).to_s
        if variant == "reference" && spec[:external_ids]
          frame.delete("result")
          frame.merge!("state" => "42P10", "within" => "false", "outside" => "false",
            "diagnostic" => ingestion_diagnostics(metadata_test_d5_stderr, role:).first)
          metadata_advance!(counters, spec.fetch(:incoming), external_success: false)
        elsif variant == "reference" && index == 2
          frame.delete("result")
          frame.merge!("state" => "42P07", "diagnostic" => ingestion_diagnostics(policy_d4_expected, role:).first)
        else
          shape = spec.fetch(:incoming)
          changed = metadata_changed_tables(prior, shape, frame.fetch("clock"), counters)
          metadata_test_assign!(changed, prior, shape, counters.fetch("search_request_source_observation_attr"))
          metadata_advance!(counters, shape)
          frame["tables_after"] = changed
          frame["tables_finish"] = index.zero? ? prior : changed
        end
        prior = frame.fetch("tables_finish")
        frame
      end
      reads << metadata_test_read(seed_clock, counters)
      evidence = { "frames" => frames, "fixtures" => fixtures, "before" => before, "after" => prior, "reads" => reads, "seed_clock" => seed_clock }
      evidence["mutating_helper"] = helper if helper
      evidence["external_fixture"] = external_fixture if external_fixture
      metadata_refresh!(evidence, mode)
      evidence
    end

    def metadata_refresh!(evidence, mode)
      evidence.merge!(attributes_test_transport(evidence.fetch("frames"), mode))
      failures = evidence.fetch("frames").count { |frame| frame.fetch("state") == "42P10" }
      evidence["stderr"] = metadata_test_d5_stderr * failures if failures.positive?
      if evidence.key?("mutating_helper")
        helper = attributes_test_transport([evidence.fetch("mutating_helper")], "cold").fetch("stdout").split("\n", 2).last
        evidence["stdout"] = "#{helper}metadata-helper-finish\n#{evidence.fetch('stdout')}"
      end
      evidence.fetch("fixtures").each { |record| record.merge!(attributes_test_transport(record.fetch("frames"), "cold")) }
      if evidence.key?("external_fixture")
        record = evidence.fetch("external_fixture")
        record["stdout"] = attributes_test_transport(record.fetch("frames"), "cold").fetch("stdout").split("\n", 2).last
      end
      evidence.fetch("reads").each { |record| record["stdout"] = JSON.generate(record.fetch("data")) + "\n" }
    end

    def metadata_case_tests!
      specs = metadata_cases
      assert(specs.map { |spec| spec.fetch(:name) } == %w[replace-typed extend-typed existing-conflicts-stale-long differing-external-ids], "finite missing-path inventory")
      base = specs.first.fetch(:fixtures).first.fetch(:answers).to_h { |key, _type, value| [key, value] }
      assert(METADATA_ANSWERS.all? { |key, _type, value| value != base.fetch(key) }, "every typed value really changes")
      assert(METADATA_INPUTS.map { |_key, type, _value| type }.uniq.sort == %i[bigint bool int numeric text], "all legal non-D5 channels")
      specs.each do |spec|
        %w[cold helpers-first].each do |mode|
          query = attributes_query(metadata_session(spec, mode))
          assert(query.scan("FROM public.search_result_ingest_v1(").length == 3 && query.scan("ROLLBACK;").length == 1 && query.scan("COMMIT;").length == 2, "real whole rollback then same-session commits")
          assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no hidden session or behavior repair")
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = metadata_test_evidence(spec, mode, variant)
            assert(metadata_validate!(evidence, spec, mode, variant, role), "#{spec.fetch(:name)} #{mode} #{variant}")
            first_retry = evidence.fetch("frames").fetch(1).fetch("tables_after")
            expected = spec.fetch(:fixtures).length == 2 ? [1, 2, 3, 7, 8, 9] : [4, 5, 6]
            expected = [] if spec[:external_ids] && variant == "reference"
            assert(first_retry.fetch("source_metadata_conflict").map { |row| row.fetch("source_metadata_conflict_id") } == expected, "conflict rollback gaps plus retained previous rows")
            assert(first_retry.fetch("source_metadata_conflict_audit_log").map { |row| row.fetch("conflict_id") } == expected, "audit FK follows nontransactional conflict IDs")
            assert(first_retry.fetch("canonical_torrent_source_attr").map { |row| row.fetch("canonical_torrent_source_attr_id") } == (1..(spec[:external_ids] ? 11 : 8)).to_a, "durable first-wins retains all identities")
          end
          assert(metadata_comparable(metadata_test_evidence(spec, mode, "reference"), fixtures_only: spec[:external_ids]) == metadata_comparable(metadata_test_evidence(spec, mode, "final"), fixtures_only: spec[:external_ids]), "paired fixtures and separately validated D4/D5 outcomes")
        end
      end
      long = metadata_test_evidence(specs.fetch(2), "cold", "final")
      tables = long.fetch("after")
      assert(tables.fetch("canonical_torrent_source").first.fetch("last_seen_seeders") == 9 && tables.fetch("search_request_source_observation").first.fetch("seeders") == 1, "stale observations are not durable source truth")
      assert(tables.fetch("source_metadata_conflict").last(3).first.fetch("incoming_value") == "Z" * 256, "conflict is truncated independently of observation")
      assert(tables.fetch("search_request_source_observation_attr").find { |row| row.fetch("attr_key") == "tracker_name" }.fetch("value_text").length == 512, "observation keeps full validated tracker text")
      assert(metadata_test_evidence(specs.first, "cold", "final").fetch("after").fetch("canonical_torrent_signal").map { |row| row.fetch("canonical_torrent_signal_id") } == (1..6).to_a + (13..24).to_a, "signal gaps and duplicate NULL-distinct keys remain")
      rejected("unknown metadata") { metadata_session(specs.first, "reconnect") }
    end

    def metadata_test_d5_stderr
      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").first
      statement = routine.fetch("source").match(/INSERT INTO canonical_external_id \(\n.*?;/m)
      raise Failure, "missing frozen IMDb D5 statement" unless statement

      "ERROR:  42P10: there is no unique or exclusion constraint matching the ON CONFLICT specification\n" \
        "CONTEXT:  SQL statement \"#{statement[0].delete_suffix(';')}\"\n" \
        "PL/pgSQL function #{routine.fetch('signature')} line 2107 at SQL statement\n" \
        "LOCATION:  infer_arbiter_indexes, plancat.c:920\n"
    end

    def metadata_external_tests!
      spec = metadata_cases.last
      query = metadata_external_fixture_query
      assert(query.scan("INSERT INTO public.canonical_torrent_source_attr").length == 1 && !query.include?("search_result_ingest"), "privileged fixture is not represented as frozen external-ID ingestion")
      assert(query.include?("(1, 'imdb_id', 'tt1234567', NULL)") && query.include?("(1, 'tmdb_id', NULL, 123456)") && query.include?("(1, 'tvdb_id', NULL, 654321)"), "explicit independently specified valid existing IDs")
      assert(query.scan("BEGIN;").length == 1 && query.scan("COMMIT;").length == 1 && query.include?("tables_finish:"), "fixture keeps transaction provenance and all table images")
      METADATA_MODES.each do |mode|
        %w[reference final].each do |variant|
          evidence = metadata_test_evidence(spec, mode, variant)
          role = variant == "reference" ? "postgres" : @runtime
          assert(metadata_validate!(evidence, spec, mode, variant, role), "three differing external-ID arms retain independent D5 expectations")
          fixture = evidence.fetch("external_fixture").fetch("frames").first
          assert(fixture.fetch("tables_before") == evidence.fetch("fixtures").last.fetch("frames").first.fetch("tables_finish"), "existing source provenance is real non-ID ingestion followed by separate admin fixture")
          frames = evidence.fetch("frames")
          helper = mode.start_with?("mutating-helper-first-") ? 1 : 0
          assert(evidence.fetch("reads").last.fetch("data").fetch("sequences") == {
            "search_request_source_observation_attr" => 73, "canonical_torrent_source_attr" => 11,
            "canonical_external_id" => variant == "reference" ? nil : 9, "canonical_torrent_signal" => 24,
            "source_metadata_conflict" => 9 + helper, "source_metadata_conflict_audit_log" => 9 + helper,
            "indexer_health_event" => 9 + helper
          }, "independent exact attempted sequence totals include rolled-back logger writes and final conflict-upsert gaps")
          frames.each do |frame|
            durable = frame.fetch("tables_after").fetch("canonical_torrent_source_attr").last(3)
            assert(durable.map { |row| row.values_at("canonical_torrent_source_attr_id", "attr_key", "value_text", "value_int") } == [
              [9, "imdb_id", "tt1234567", nil], [10, "tmdb_id", nil, 123456], [11, "tvdb_id", nil, 654321]
            ], "all three durable values and identities remain first-wins")
          end
          if variant == "reference"
            assert(frames.all? { |frame| frame.fetch("state") == "42P10" && !frame.key?("result") && frame.fetch("tables_before") == frame.fetch("tables_after") && frame.fetch("tables_after") == frame.fetch("tables_finish") }, "every frozen attempt including warm retry fails and rolls back all 18 tables")
            assert(evidence.fetch("after").fetch("canonical_external_id").empty?, "no invented frozen external-ID success")
          else
            first, retry_frame, last = frames
            assert(first.fetch("tables_after") != first.fetch("tables_before") && first.fetch("tables_finish") == first.fetch("tables_before"), "real final first mutation is rolled back before warm commit")
            assert(retry_frame.fetch("tables_after").fetch("canonical_external_id").map { |row| row.fetch("canonical_external_id_id") } == [4, 5, 6], "final external-ID rollback consumes exactly three IDs")
            assert(last.fetch("tables_after").fetch("canonical_external_id") == retry_frame.fetch("tables_after").fetch("canonical_external_id"), "third call retains the committed ID rows despite upsert sequence consumption")
            triples = [["external_id", "tt1234567", "tt7654321"], ["external_id", "123456", "234567"], ["external_id", "654321", "765432"]]
            frames.each do |frame|
              tables = frame.fetch("tables_after")
              assert(tables.fetch("source_metadata_conflict").last(3).map { |row| row.values_at("conflict_type", "existing_value", "incoming_value") } == triples, "logger order and payload distinguish all three arms")
              conflicts = tables.fetch("source_metadata_conflict").last(3)
              assert(tables.fetch("source_metadata_conflict_audit_log").last(3) == conflicts.map { |row| {
                "source_metadata_conflict_audit_log_id" => row.fetch("source_metadata_conflict_id"), "conflict_id" => row.fetch("source_metadata_conflict_id"),
                "action" => "created", "actor_user_id" => 0, "occurred_at" => frame.fetch("clock"), "note" => nil
              } }, "exact audit links actor action clock and null note")
              assert(tables.fetch("indexer_health_event").last(3) == conflicts.map { |row| {
                "indexer_health_event_id" => row.fetch("source_metadata_conflict_id"), "indexer_instance_id" => 569001,
                "occurred_at" => "2026-09-11T00:00:00+00:00", "event_type" => "identity_conflict", "latency_ms" => nil,
                "http_status" => nil, "error_class" => nil, "detail" => "external_id"
              } }, "exact health payload clock and nullable fields")
            end
          end
          metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.delete("external_fixture") }
          %w[backend clock before after state within outside result tables_before tables_after tables_finish].each do |key|
            metadata_reject_mutation(evidence, spec, mode, variant) do |changed|
              frame = changed.fetch("external_fixture").fetch("frames").first
              frame[key] = key.start_with?("tables_") ? attributes_empty : key == "result" ? { "fixture" => "changed" } : "changed"
            end
          end
          metadata_reject_mutation(evidence, spec, mode, variant) do |changed|
            changed.fetch("external_fixture").fetch("frames").first["backend"] = changed.fetch("fixtures").first.fetch("context").fetch("backend")
          end
          evidence.fetch("reads").last.fetch("data").fetch("sequences").each_key do |table|
            metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("reads").last.fetch("data").fetch("sequences")[table] = 0 }
          end
        end
      end
      evidence = metadata_test_evidence(spec, "cold", "reference")
      ["", metadata_test_d5_stderr * 2, metadata_test_d5_stderr.sub("plancat.c:920", "plancat.c:921") * 3,
       metadata_test_d5_stderr.sub("line 2107", "line 2132") * 3, metadata_test_d5_stderr.sub("lower(imdb_id_value)", "imdb_id_value") * 3,
       metadata_test_d5_stderr * 3 + "NOTICE: unexpected\n"].each do |stderr|
        changed = copy(evidence).merge("stderr" => stderr)
        rejected("") { metadata_validate!(changed, spec, "cold", "reference", "postgres") }
      end
      [metadata_test_d5_stderr.sub("plancat.c:920", "plancat.c:921"),
       metadata_test_d5_stderr.sub("line 2107", "line 2132"),
       metadata_test_d5_stderr.sub("lower(imdb_id_value)", "imdb_id_value"),
       metadata_test_d5_stderr.sub("search_result_ingest_v1(uuid)", "search_result_ingest_v1(text)"),
       metadata_test_d5_stderr.sub("there is no unique", "changed: there is no unique"),
       metadata_test_d5_stderr.sub("CONTEXT:", "DETAIL:  unexpected\nCONTEXT:")].each do |diagnostic|
        changed = copy(evidence).merge("stderr" => diagnostic * 3)
        rejected("") do
          # SQL/signature drift fails at the frozen-source parser; other fields
          # reach the D5 oracle with raw and parsed records changed together.
          changed.merge!(metadata_parse(changed.fetch("stdout"), changed.fetch("stderr"), metadata_session(spec, "cold"), "postgres"))
          metadata_validate!(changed, spec, "cold", "reference", "postgres")
        end
        @mutations += 1
      end
    end

    def metadata_mutating_helper_tests!
      assert(METADATA_MODES == %w[cold helpers-first mutating-helper-first-rollback mutating-helper-first-commit], "retain pure and mutating compilation orders")
      metadata_cases.each do |spec|
        METADATA_MODES.last(2).each do |mode|
          query = metadata_query(metadata_session(spec, mode))
          assert(query.scan("public.log_source_metadata_conflict_v1(").length == 1 && query.scan("FROM public.search_result_ingest_v1(").length == 3,
            "one actual mutating helper precedes three ingestion calls")
          assert(query.index("public.log_source_metadata_conflict_v1(") < query.index("FROM public.search_result_ingest_v1("), "mutating helper executes first in the same connection")
          assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no compilation reset or role/settings workaround")
          %w[reference final].each do |variant|
            evidence = metadata_test_evidence(spec, mode, variant)
            role = variant == "reference" ? "postgres" : @runtime
            assert(metadata_validate!(evidence, spec, mode, variant, role), "#{mode} independent state and sequence oracle")
            helper = evidence.fetch("mutating_helper")
            conflict = helper.fetch("tables_after").fetch("source_metadata_conflict").last
            assert(conflict.values_at("existing_value", "incoming_value", "observed_at") == ["O" * 256, "N" * 256, helper.fetch("clock")], "both truncation branches and default observed clock")
            retained = helper.fetch("tables_finish").fetch("source_metadata_conflict").include?(conflict)
            assert(retained == mode.end_with?("commit"), "helper rows commit or roll back without resetting compilation")
            %w[backend clock before after state within outside result tables_before tables_after tables_finish].each do |key|
              metadata_reject_mutation(evidence, spec, mode, variant) do |changed|
                row = changed.fetch("mutating_helper")
                row[key] = key == "result" ? { "helper" => "different" } : key.start_with?("tables_") ? attributes_empty : "changed"
              end
            end
            %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].each do |table|
              helper.fetch("tables_after").fetch(table).last.each_key do |column|
                metadata_reject_mutation(evidence, spec, mode, variant) do |changed|
                  changed.fetch("mutating_helper").fetch("tables_after").fetch(table).last[column] = "changed"
                end
              end
              metadata_reject_mutation(evidence, spec, mode, variant) do |changed|
                changed.fetch("reads").last.fetch("data").fetch("sequences")[table] -= 1
              end
            end
            changed = copy(evidence)
            changed["stdout"] = changed.fetch("stdout").sub("metadata-helper-finish\n", "")
            rejected("helper boundary") { metadata_validate!(changed, spec, mode, variant, role) }
            changed = copy(evidence)
            changed["stdout"] = "metadata-helper-finish\n" + changed.fetch("stdout")
            rejected("helper boundary") { metadata_validate!(changed, spec, mode, variant, role) }
          end
          assert(metadata_comparable(metadata_test_evidence(spec, mode, "reference"), fixtures_only: spec[:external_ids]) == metadata_comparable(metadata_test_evidence(spec, mode, "final"), fixtures_only: spec[:external_ids]), "mutating-helper and fixture parity outside independently checked D4/D5")
        end
      end
      rejected("helper finish") { metadata_helper_session("reconnect") }
    end

    def metadata_walk(value, &block)
      case value
      when Hash
        yield value
        value.each_value { |item| metadata_walk(item, &block) }
      when Array
        value.each { |item| metadata_walk(item, &block) }
      end
    end

    def metadata_reject_mutation(evidence, spec, mode, variant)
      changed = copy(evidence)
      yield changed
      metadata_refresh!(changed, mode)
      rejected("") { metadata_validate!(changed, spec, mode, variant, variant == "reference" ? "postgres" : @runtime) }
      @mutations += 1
    end

    def metadata_mutation_tests!
      metadata_cases.each do |spec|
        %w[cold helpers-first].each do |mode|
          %w[reference final].each do |variant|
            evidence = metadata_test_evidence(spec, mode, variant)
            IngestionProof::INGESTION_TABLES.each do |table|
              metadata_reject_mutation(evidence, spec, mode, variant) do |changed|
                metadata_walk(changed) { |value| value[table] = [] if value.key?(table) && value[table].is_a?(Array) }
                # An empty table needs a spurious row, not an unchanged deletion.
                if evidence.fetch("after").fetch(table).empty?
                  metadata_walk(changed) { |value| value[table] = [{ "unexpected" => true }] if value.key?(table) && value[table].is_a?(Array) }
                end
              end
            end
            evidence.fetch("after").each do |table, rows|
              rows.each do |row|
                row.each_key do |column|
                  next if %w[canonical_torrent_public_id canonical_torrent_source_public_id observation_attr_id].include?(column)

                  metadata_reject_mutation(evidence, spec, mode, variant) do |changed|
                    metadata_walk(changed) do |value|
                      next unless value[table].is_a?(Array)

                      value.fetch(table).each { |item| item[column] = "mutated" if item.key?(column) }
                    end
                  end
                end
              end
            end
            metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("frames").first["backend"] = "999" }
            if spec[:external_ids] && variant == "reference"
              metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("frames").first["state"] = "00000" }
              metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("frames").first["tables_finish"] = attributes_empty }
            else
              metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("frames").first.fetch("result")["canonical_changed"] = true }
              metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("frames").first["tables_finish"] = changed.fetch("frames").first.fetch("tables_after") }
            end
            metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("fixtures").first.fetch("frames").first["clock"] = changed.fetch("seed_clock") }
          end
        end
        evidence = metadata_test_evidence(spec, "cold", "final")
        evidence.fetch("reads").first.fetch("data").fetch("inputs").each do |table, rows|
          rows.each do |row|
            row.each_key do |column|
              metadata_reject_mutation(evidence, spec, "cold", "final") do |changed|
                changed.fetch("reads").each { |read| read.fetch("data").fetch("inputs").fetch(table).each { |item| item[column] = "mutated" } }
              end
            end
          end
          metadata_reject_mutation(evidence, spec, "cold", "final") { |changed| changed.fetch("reads").each { |read| read.fetch("data").fetch("inputs")[table] = [{ "extra" => true }] } }
        end
        METADATA_SEQUENCES.each_key do |table|
          metadata_reject_mutation(evidence, spec, "cold", "final") { |changed| changed.fetch("reads").last.fetch("data").fetch("sequences")[table] = 1 }
        end
      end
    end

    def metadata_transport_tests!
      spec = metadata_cases.first
      evidence = metadata_test_evidence(spec, "helpers-first", "reference")
      ["", evidence.fetch("stdout") + evidence.fetch("stdout"), evidence.fetch("stdout").sub(/^helpers:.*\n/, ""),
       evidence.fetch("stdout").sub(/^attribute_context:.*\n/, "attribute_context:{}\n")].each do |stdout|
        changed = copy(evidence).merge("stdout" => stdout)
        rejected("") { metadata_validate!(changed, spec, "helpers-first", "reference", "postgres") }
      end
      ["", evidence.fetch("stderr") + "NOTICE: extra\n", evidence.fetch("stderr").sub("createas.c:406", "createas.c:407")].each do |stderr|
        changed = copy(evidence).merge("stderr" => stderr)
        rejected("") { metadata_validate!(changed, spec, "helpers-first", "reference", "postgres") }
      end
      changed = copy(evidence)
      changed.fetch("reads").last["stdout"] = "{}"
      rejected("serialized read") { metadata_validate!(changed, spec, "helpers-first", "reference", "postgres") }
      changed.fetch("reads").last["stderr"] = "WARNING: extra\n"
      rejected("read diagnostic") { metadata_validate!(changed, spec, "helpers-first", "reference", "postgres") }
    end

    def metadata_json_tests!
      [
        '{"field":1,"field":2}',
        '{"field":null,"field":false}',
        '{"field":1,"field":1}',
        '{"outer":[{"inner":{"field":1,"field":2}}]}',
        '{"outer":[{"inner":{"field":1,"\\u0066ield":2}}]}'
      ].each { |json| rejected("duplicate metadata JSON") { metadata_json_parse(json) } }
      valid = JSON.generate("left" => { "field" => 1 }, "right" => { "field" => 2 }, "text" => '{"field":1}', "number" => 2.3456)
      assert(metadata_json_parse(valid) == JSON.parse(valid), "sibling fields, escaped text and numeric values are unchanged")
      assert(metadata_json_parse(valid).fetch("left").is_a?(IngestionMetadata::UniqueObject), "custom object class applies to nested objects")
      object = IngestionMetadata::UniqueObject.new
      object["field"] = nil
      begin
        object["field"] = false
        raise Failure, "duplicate setter accepted an existing NULL field"
      rescue JSON::ParserError => error
        assert(error.message.include?("duplicate metadata JSON"), "legacy parser setter rejects duplicate decoded keys")
      end

      spec = metadata_cases.first
      evidence = metadata_test_evidence(spec, "helpers-first", "final")
      (%w[attribute_context helpers role tables_before tables_after tables_finish] + [nil]).each do |prefix|
        changed = copy(evidence)
        lines = changed.fetch("stdout").lines(chomp: true)
        index = lines.index { |line| prefix ? line.start_with?("#{prefix}:") : line.start_with?("{") }
        json = prefix ? lines.fetch(index).delete_prefix("#{prefix}:") : lines.fetch(index)
        key = JSON.parse(json).keys.first
        lines[index] = "#{prefix ? "#{prefix}:" : ''}#{json.sub('{', "{#{JSON.generate(key)}:null,")}"
        changed["stdout"] = lines.join("\n") + "\n"
        rejected("duplicate metadata JSON") { metadata_validate!(changed, spec, "helpers-first", "final", @runtime) }
      end
      changed = copy(evidence)
      read = changed.fetch("reads").last
      read["stdout"] = read.fetch("stdout").sub('"display_name":"Ingestion proof"', '"display_name":"lost","display_name":"Ingestion proof"')
      rejected("duplicate metadata JSON") { metadata_validate!(changed, spec, "helpers-first", "final", @runtime) }
      changed = copy(evidence)
      fixture = changed.fetch("fixtures").first
      fixture["stdout"] = fixture.fetch("stdout").sub('"value_int":2000', '"value_int":99,"value_int":2000')
      rejected("duplicate metadata JSON") { metadata_validate!(changed, spec, "helpers-first", "final", @runtime) }
    end

    def metadata_invalid_clocks!(evidence)
      clocks = [evidence.fetch("seed_clock")] + (evidence.fetch("fixtures") + [evidence]).flat_map { |record| record.fetch("frames").map { |frame| frame.fetch("clock") } }
      clocks << evidence.fetch("external_fixture").fetch("frames").first.fetch("clock") if evidence.key?("external_fixture")
      replacements = clocks.each_with_index.to_h { |clock, index| [clock, "2026-99-99T25:61:0#{index}+00:00"] }
      metadata_walk(evidence) do |row|
        row.each { |key, value| row[key] = replacements.fetch(value, value) if value.is_a?(String) }
      end
    end

    def metadata_json_option_tests!
      parser = JSON.method(:parse)
      JSON.define_singleton_method(:parse) do |value, **options|
        parser.call(value, **options.reject { |key, _option| %i[object_class allow_duplicate_key].include?(key) })
      end
      rejected("lacks duplicate-field rejection") { metadata_json_parse('{}') }
    ensure
      JSON.define_singleton_method(:parse, parser) if parser
    end

    def metadata_clock_tests!
      valid = %w[0001-01-01T00:00:00+00:00 1500-02-28T00:00:00+00:00 1582-10-10T00:00:00+00:00
                 1600-02-29T23:59:59+00:00 2000-02-29T12:34:56+00:00 9999-12-31T23:59:59+00:00]
      (1..6).each { |digits| valid << "2026-09-12T01:02:03.#{'0' * digits}+00:00" }
      valid.concat(%w[2026-09-12T01:02:03.1+00:00 2026-09-12T01:02:03.01+00:00 2026-09-12T01:02:03.123456+00:00])
      valid.each do |clock|
        original = clock.dup
        assert(metadata_clock?(clock) && clock == original, "exact Gregorian timestamp domain without normalization")
      end
      invalid = [nil, 1, {}, "", "2026-09-12T01:02:03Z", "2026-09-12T01:02:03+01:00",
        "2026-09-12T01:02:03.1234567+00:00", "2026-09-12T01:02:03.+00:00", "2026-09-12 01:02:03+00:00",
        "2026-09-12T01:02:03+00:00\n", "0000-01-01T00:00:00+00:00", "1500-02-29T00:00:00+00:00",
        "1900-02-29T00:00:00+00:00", "2026-02-29T00:00:00+00:00", "2024-02-30T00:00:00+00:00",
        "2026-04-31T00:00:00+00:00", "2026-00-01T00:00:00+00:00", "2026-13-01T00:00:00+00:00",
        "2026-09-00T00:00:00+00:00", "2026-09-12T24:00:00+00:00", "2026-09-12T25:00:00+00:00",
        "2026-09-12T01:60:00+00:00", "2026-09-12T01:02:60+00:00"]
      invalid.each { |clock| assert(!metadata_clock?(clock), "invalid date/time or changed timestamp representation rejected") }
      metadata_cases.each do |spec|
        %w[cold helpers-first].each do |mode|
          %w[reference final].each do |variant|
            evidence = metadata_test_evidence(spec, mode, variant)
            metadata_invalid_clocks!(evidence)
            metadata_refresh!(evidence, mode)
            rejected("clocks changed") { metadata_validate!(evidence, spec, mode, variant, variant == "reference" ? "postgres" : @runtime) }
          end
        end
      end
    end

    def metadata_producer_tests!
      @contract.validate_output_path!
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      directory = Dir.mktmpdir("metadata-unit-", @contract.output_path)
      begin
        @metadata_evidence = directory
        spec = metadata_cases.first
        evidence = metadata_test_evidence(spec, "cold", "final")
        bytes = JSON.pretty_generate(evidence) + "\n"
        path = metadata_write("valid.json", bytes)
        registry = { path => Digest::SHA256.hexdigest(bytes) }
        assert(dependency_read_observation(path, registry).first == evidence, "exact serialized evidence registry")
        rejected("current-process") { dependency_read_observation(path, {}) }
        File.binwrite(path, bytes + " ")
        rejected("bytes changed") { dependency_read_observation(path, registry) }
        rejected("cannot retain") { metadata_write("valid.json", bytes) }
        rejected("invalid metadata") { metadata_write("../escaped", bytes) }
        File.symlink(path, File.join(directory, "link.json"))
        rejected("cannot retain") { metadata_write("link.json", bytes) }
        assert(File.stat(path).mode & 0o777 == 0o600, "private evidence bytes")
        define_singleton_method(:result) { |_query, **_options| CommandRunner::Result.new(stdout: "raw partial\n", stderr: "raw producer failure\n", success: false) }
        rejected("transport failed") { metadata_execute(metadata_session(spec, "cold"), "synthetic", @runtime, "producer-failure") }
        assert(File.binread(File.join(directory, "producer-failure.stderr")) == "raw producer failure\n", "failed native evidence retained")
        define_singleton_method(:result) { |_query, **_options| CommandRunner::Result.new(stdout: "invalid evidence\n", stderr: "", success: true) }
        rejected("context missing") { metadata_execute(metadata_session(spec, "cold"), "synthetic", @runtime, "invalid-evidence") }
        assert(File.binread(File.join(directory, "invalid-evidence.stdout")) == "invalid evidence\n", "invalid successful producer bytes retained")
      end
      singleton_class.remove_method(:result)
      @correction_evidence = "restore-this"
      define_singleton_method(:metadata_isolated) { |*_args| raise Failure, "synthetic isolated failure" }
      rejected("synthetic isolated") { verify_ingestion_metadata! }
      report = JSON.parse(File.binread(File.join(@metadata_evidence, "report.json")))
      assert(report.values_at("completed", "passed", "d3_complete") == [false, false, false], "producer failure cannot register passing completion")
      assert(@metadata_validated_evidence.empty? && @correction_evidence == "restore-this", "no invalid registration and evidence context restored")
    ensure
      %i[result metadata_isolated].each { |name| singleton_class.remove_method(name) if singleton_methods.include?(name) }
    end

    def metadata_identity_tests!
      spec = metadata_cases.fetch(1)
      evidence = metadata_test_evidence(spec, "cold", "final")
      [0, 2, "1"].each do |id|
        metadata_reject_mutation(evidence, spec, "cold", "final") do |changed|
          changed.fetch("fixtures").first.fetch("frames").first.fetch("tables_after").fetch("search_request_source_observation_attr").first["observation_attr_id"] = id
        end
      end
      metadata_reject_mutation(evidence, spec, "cold", "final") do |changed|
        changed.fetch("frames").fetch(1).fetch("tables_after").fetch("search_request_source_observation_attr").find { |row| row.fetch("attr_key") == "freeleech" }["observation_attr_id"] = 17
      end
      metadata_reject_mutation(evidence, spec, "cold", "final") do |changed|
        changed.fetch("frames").first.fetch("result")["canonical_torrent_source_public_id"] = "56900000-0000-4000-8000-000000000099"
      end
      metadata_reject_mutation(evidence, spec, "cold", "final") do |changed|
        changed.fetch("frames").first.fetch("role")["superuser"] = true
      end
      counters = METADATA_SEQUENCES.to_h { |table, _column| [table, 0] }
      read = metadata_test_read("2026-09-12T00:00:00+00:00", counters)
      captured = nil
      define_singleton_method(:metadata_transport) do |query, *_args|
        captured = query
        read.slice("stdout", "stderr")
      end
      assert(metadata_read("synthetic", "seed") == read, "read transport decoded without discarding raw bytes")
      assert(captured.include?("format('%I.%I', schemaname, sequencename)::regclass") && captured.start_with?("\\set VERBOSITY verbose\n"), "sequence observer resolves actual schema even before planner filtering and retains native diagnostics")
    ensure
      singleton_class.remove_method(:metadata_transport) if singleton_methods.include?(:metadata_transport)
    end

    def metadata_isolated_tests!
      [metadata_cases.first, metadata_cases.last].each { |spec| metadata_isolated_case_tests!(spec) }
    end

    def metadata_isolated_case_tests!(spec)
      %w[valid invalid failed duplicate-read duplicate-frame impossible-clock].each do |kind|
        @metadata_evidence = Dir.mktmpdir("metadata-producer-#{kind}-", @contract.output_path)
        @metadata_validated_evidence = {}
        evidence = metadata_test_evidence(spec, "cold", "final")
        if kind == "invalid"
          evidence.fetch("frames").first.fetch("result")["canonical_changed"] = true
          metadata_refresh!(evidence, "cold")
        elsif kind == "duplicate-read"
          read = evidence.fetch("reads").last
          read["stdout"] = read.fetch("stdout").sub('{', '{"inputs":{},')
        elsif kind == "duplicate-frame"
          evidence["stdout"] = evidence.fetch("stdout").sub('"value_int":2000', '"value_int":99,"value_int":2000')
        elsif kind == "impossible-clock"
          metadata_invalid_clocks!(evidence)
          metadata_refresh!(evidence, "cold")
        end
        reads = evidence.fetch("reads").dup
        snapshots = [evidence.fetch("before"), evidence.fetch("after")]
        records = evidence.fetch("fixtures") + (spec[:external_ids] ? [evidence.fetch("external_fixture")] : []) + [evidence]
        outcomes = records.map do |record|
          CommandRunner::Result.new(stdout: record.fetch("stdout"), stderr: record.fetch("stderr"), success: true)
        end
        outcomes[-1] = CommandRunner::Result.new(stdout: "partial native stdout\n", stderr: "native producer failure\n", success: false) if kind == "failed"
        commands = []
        roles = []
        define_singleton_method(:sql) { |query, **_options| commands << query; "" }
        define_singleton_method(:policy_setup!) { |*_args| evidence.fetch("seed_clock") }
        define_singleton_method(:correction_observer!) { |*_args| nil }
        define_singleton_method(:metadata_read) { |*_args| reads.shift }
        define_singleton_method(:ingestion_snapshot) { |*_args| snapshots.shift }
        define_singleton_method(:result) { |*_args, **options| roles << options.fetch(:role); outcomes.shift }
        if kind == "valid"
          assert(metadata_isolated(spec, "cold", "final", @database, @runtime) == evidence, "real producer path validates serialized evidence")
          assert(@metadata_validated_evidence.length == 1, "exact one valid observed result registered")
        else
          message = { "invalid" => "result flags", "failed" => "transport failed", "duplicate-read" => "duplicate metadata JSON",
                      "duplicate-frame" => "duplicate metadata JSON", "impossible-clock" => "clocks changed" }.fetch(kind)
          rejected(message) { metadata_isolated(spec, "cold", "final", @database, @runtime) }
          assert(@metadata_validated_evidence.empty?, "#{kind} producer cannot register evidence")
        end
        assert(commands.last == 'DROP DATABASE "ingestion_metadata_final" WITH (FORCE)', "#{kind} producer drops only its clone")
        assert(File.exist?(File.join(@metadata_evidence, "#{spec.fetch(:name)}-cold-final.stdout")), "#{kind} producer raw attempt retained")
        assert(roles == (spec[:external_ids] ? [@runtime, "postgres", @runtime] : [@runtime, @runtime]), "only the independently validated external-ID fixture uses the administrator connection")
      end
    ensure
      %i[sql policy_setup! correction_observer! metadata_read ingestion_snapshot result].each do |name|
        singleton_class.remove_method(name) if singleton_methods.include?(name)
      end
    end

    def metadata_reachability_tests!(source)
      helper = source.split("CREATE OR REPLACE FUNCTION log_source_metadata_conflict_v1(", 2).fetch(1).split("$$;", 2).first
      assert(helper.scan(/INSERT INTO (source_metadata_conflict|source_metadata_conflict_audit_log|indexer_health_event)\s/).flatten.sort == %w[indexer_health_event source_metadata_conflict source_metadata_conflict_audit_log], "frozen logger has exactly three creation writes")
      assert(!helper.match?(/\bUPDATE\b|ON CONFLICT/) && !source.match?(/PERFORM source_metadata_conflict_(resolve|reopen)/), "ingestion cannot update or resolve existing conflicts")
      assert(helper.include?("substring(incoming_value FROM 1 FOR 256)"), "frozen truncation branch retained")
      { 2314 => "imdb", 2341 => "tmdb", 2368 => "tvdb" }.each do |line, id|
        assert(source.lines.fetch(line - 1).strip == "ELSIF existing_#{id}_id <> #{id}_id_value THEN", "exact frozen #{id} differing-value logger arm remains pinned")
        assert(source.lines.fetch(line).strip == "PERFORM log_source_metadata_conflict_v1(", "actual logger call follows the pinned differing-ID guard")
      end
      schema = File.binread(File.join(@contract.root, "crates/revaer-data/migrations/0022_indexer_canonicalization.sql"))
      assert(schema.include?("(attr_key IN ('tracker_name', 'imdb_id') AND value_text IS NOT NULL)") && source.include?("IF existing_tracker_name IS NULL THEN"), "serial coalesce-upsert reachability counterexample remains source-backed")
      metadata_schema_facts_tests!(source, schema)
      proof = File.binread(File.join(@contract.root, "scripts/database_rebaseline/ingestion_proof.rb"))
      assert(proof.include?("verify_ingestion_metadata!") && proof.include?("complete ingestion branch and helper execution matrix"), "canonical matrix wired without removing incomplete D3")
    end

    # Pin the premises of the support note, not simulated PostgreSQL lock outcomes.
    def metadata_schema_facts_tests!(source, schema)
      tables = %w[canonical_torrent_source_attr canonical_torrent_signal canonical_size_sample].to_h do |name|
        [name, schema.split("CREATE TABLE IF NOT EXISTS #{name} (", 2).fetch(1).split("\n);", 2).first.gsub(/\s+/, " ")]
      end
      durable = tables.fetch("canonical_torrent_source_attr")
      assert(durable.include?("canonical_torrent_source_id BIGINT NOT NULL REFERENCES canonical_torrent_source (canonical_torrent_source_id) ON DELETE CASCADE"), "durable parent reference remains an immediate ordinary FK")
      assert(durable.include?("canonical_torrent_source_attr_uq UNIQUE ( canonical_torrent_source_id, attr_key )"), "durable conflict arbiter remains the non-null source/key pair")
      assert(durable.include?("attr_key durable_source_attr_key NOT NULL"), "durable key cannot bypass its type check with NULL")
      assert(durable.include?("CHECK ( ( (value_text IS NOT NULL)::INT + (value_int IS NOT NULL)::INT + (value_bigint IS NOT NULL)::INT + (value_numeric IS NOT NULL)::INT + (value_bool IS NOT NULL)::INT ) = 1 )"), "durable values retain exactly one populated channel")
      assert(durable.include?("(attr_key IN ('tracker_name', 'imdb_id') AND value_text IS NOT NULL) OR (attr_key = 'size_bytes_reported' AND value_bigint IS NOT NULL) OR ( attr_key IN ( 'tracker_category', 'tracker_subcategory', 'files_count', 'season', 'episode', 'year', 'tmdb_id', 'tvdb_id' ) AND value_int IS NOT NULL )"), "all eleven durable keys require their corresponding non-null channel")

      signal = tables.fetch("canonical_torrent_signal")
      assert(signal.include?("value_text VARCHAR(128), value_int INTEGER,"), "omitted signal channels have no non-null default")
      assert(signal.include?("canonical_torrent_signal_uq UNIQUE ( canonical_torrent_id, signal_key, value_text, value_int )"), "signal arbiter keeps ordinary NULL-distinct uniqueness")
      assert(signal.include?("CHECK ( ( (value_text IS NOT NULL)::INT + (value_int IS NOT NULL)::INT ) = 1 )"), "every legal signal retains exactly one NULL arbiter channel")
      signal_writes = source.scan(/INSERT INTO canonical_torrent_signal \(\s*canonical_torrent_id,\s*signal_key,\s*(value_text|value_int),\s*confidence\s*\)\s*VALUES \(\s*canonical_id,\s*'([^']+)'/)
      assert(signal_writes == [["value_text", "release_group"], ["value_text", "language"], ["value_text", "subtitles"], ["value_int", "year"], ["value_int", "season"], ["value_int", "episode"]], "all six actual signal inserts omit the other arbiter channel")

      refresh = source.match(/UPDATE canonical_torrent_source\s+SET last_seen_at = CASE.*?WHERE canonical_torrent_source_id = source_id;/m)
      assert(!refresh.nil?, "existing-source refresh retains its unconditional key predicate")
      assignments = refresh[0].scan(/^\s*(?:SET )?(\w+) =/).flatten
      assert(assignments == %w[last_seen_at last_seen_seeders last_seen_leechers last_seen_published_at last_seen_download_url last_seen_magnet_uri last_seen_details_url last_seen_uploader updated_at], "refresh changes no source identity or referenced unique-key column")
      assert(refresh.end(0) < source.index("INTO existing_tracker_name"), "source write precedes every durable-value lookup")
      lookups = source.split("        SELECT value_text\n        INTO existing_tracker_name", 2).fetch(1).split("        IF tracker_name_value IS NOT NULL THEN", 2).first
      assert(lookups.scan(/INTO existing_/).length == 10 && !lookups.match?(/\bFOR\b|\bLOCK\b/), "all eleven durable reads precede inserts and carry no explicit lock clause")
      assert(source.scan(/ON CONFLICT \(canonical_torrent_source_id, attr_key\)\s*DO UPDATE SET\s*value_(text|int|bigint) = COALESCE\(canonical_torrent_source_attr\.value_\1, EXCLUDED\.value_\1\);/).length == 11, "all eleven durable conflict updates preserve the stored typed value")

      sample = tables.fetch("canonical_size_sample")
      assert(sample.include?("canonical_size_sample_uq UNIQUE ( canonical_torrent_id, observed_at, size_bytes )"), "sample duplicate key is the complete incoming tuple")
      sampling = source.split("        IF size_sample_allowed THEN", 2).fetch(1).split("            IF sample_count IS NOT NULL", 2).first
      assert(sampling.match?(/INSERT INTO canonical_size_sample.*?ON CONFLICT DO NOTHING;.*?DELETE FROM canonical_size_sample.*?OFFSET 25.*?SELECT COUNT\(\*\)/m), "sample duplicate skip precedes pruning and a separate count query")

      migrations = File.join(@contract.root, "crates/revaer-data/migrations")
      instance = File.binread(File.join(migrations, "0014_indexer_instances.sql")).split("CREATE TABLE IF NOT EXISTS indexer_instance (", 2).fetch(1).split("\n);", 2).first
      assert(instance.include?("trust_tier_key trust_tier_key,") && !instance.match?(/REFERENCES trust_tier\b|FOREIGN KEY/), "instance trust key is nullable and has no trust-tier FK")
      trust = File.binread(File.join(migrations, "0012_indexer_core.sql")).split("CREATE TABLE IF NOT EXISTS trust_tier (", 2).fetch(1).split("\n);", 2).first
      assert(trust.include?("rank SMALLINT NOT NULL") && trust.include?("UNIQUE (trust_tier_key)"), "a present trust tier supplies one non-null rank")
      request = File.binread(File.join(migrations, "0023_indexer_search_requests.sql")).split("CREATE TABLE IF NOT EXISTS search_request (", 2).fetch(1).split("\n);", 2).first.gsub(/\s+/, " ")
      assert(request.include?("policy_snapshot_id BIGINT NOT NULL REFERENCES policy_snapshot (policy_snapshot_id)"), "request snapshot is mandatory, unlike the instance trust key")
    end
  end
end

if $PROGRAM_NAME == __FILE__
  test = RevaerDatabaseRebaseline::IngestionMetadataTest.new
  if ARGV.empty?
    test.run_tests!
  elsif ARGV.length == 2 && ARGV.first == "--live"
    test.run_live!(ARGV.last)
  else
    raise RevaerDatabaseRebaseline::Failure, "expected no arguments or --live owned-candidate-path"
  end
end
