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
      before = prior
      helper = nil
      session = metadata_session(spec, mode)
      if session.key?(:helper_finish)
        helper = attributes_test_frame(role, fixtures.length + 1, prior, prior)
        helper.merge!("backend" => "200", "within" => "false", "outside" => "false",
          "result" => { "helper" => "log_source_metadata_conflict_v1" })
        changed = metadata_helper_tables(prior, helper.fetch("clock"), counters)
        helper["tables_after"] = changed
        helper["tables_finish"] = session.fetch(:helper_finish) == "commit" ? changed : prior
        %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].each { |table| counters[table] += 1 }
        prior = helper.fetch("tables_finish")
      end
      frames = (0..2).map do |index|
        frame = attributes_test_frame(role, fixtures.length + index + (helper ? 2 : 1), prior, prior)
        frame["backend"] = "200"
        frame["outside"] = (variant == "reference" && index.positive?).to_s
        if variant == "reference" && index == 2
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
      metadata_refresh!(evidence, mode)
      evidence
    end

    def metadata_refresh!(evidence, mode)
      evidence.merge!(attributes_test_transport(evidence.fetch("frames"), mode))
      if evidence.key?("mutating_helper")
        helper = attributes_test_transport([evidence.fetch("mutating_helper")], "cold").fetch("stdout").split("\n", 2).last
        evidence["stdout"] = "#{helper}metadata-helper-finish\n#{evidence.fetch('stdout')}"
      end
      evidence.fetch("fixtures").each { |record| record.merge!(attributes_test_transport(record.fetch("frames"), "cold")) }
      evidence.fetch("reads").each { |record| record["stdout"] = JSON.generate(record.fetch("data")) + "\n" }
    end

    def metadata_case_tests!
      specs = metadata_cases
      assert(specs.map { |spec| spec.fetch(:name) } == %w[replace-typed extend-typed existing-conflicts-stale-long], "finite missing-path inventory")
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
            assert(first_retry.fetch("source_metadata_conflict").map { |row| row.fetch("source_metadata_conflict_id") } == expected, "conflict rollback gaps plus retained previous rows")
            assert(first_retry.fetch("source_metadata_conflict_audit_log").map { |row| row.fetch("conflict_id") } == expected, "audit FK follows nontransactional conflict IDs")
            assert(first_retry.fetch("canonical_torrent_source_attr").map { |row| row.fetch("canonical_torrent_source_attr_id") } == (1..8).to_a, "durable first-wins retains all identities")
          end
          assert(metadata_comparable(metadata_test_evidence(spec, mode, "reference")) == metadata_comparable(metadata_test_evidence(spec, mode, "final")), "paired outcomes retain every value outside validated identities clocks and exact D4")
        end
      end
      long = metadata_test_evidence(specs.last, "cold", "final")
      tables = long.fetch("after")
      assert(tables.fetch("canonical_torrent_source").first.fetch("last_seen_seeders") == 9 && tables.fetch("search_request_source_observation").first.fetch("seeders") == 1, "stale observations are not durable source truth")
      assert(tables.fetch("source_metadata_conflict").last(3).first.fetch("incoming_value") == "Z" * 256, "conflict is truncated independently of observation")
      assert(tables.fetch("search_request_source_observation_attr").find { |row| row.fetch("attr_key") == "tracker_name" }.fetch("value_text").length == 512, "observation keeps full validated tracker text")
      assert(metadata_test_evidence(specs.first, "cold", "final").fetch("after").fetch("canonical_torrent_signal").map { |row| row.fetch("canonical_torrent_signal_id") } == (1..6).to_a + (13..24).to_a, "signal gaps and duplicate NULL-distinct keys remain")
      rejected("unknown metadata") { metadata_session(specs.first, "reconnect") }
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
          assert(metadata_comparable(metadata_test_evidence(spec, mode, "reference")) == metadata_comparable(metadata_test_evidence(spec, mode, "final")), "mutating-helper parity outside exact D4")
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
            metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("frames").first.fetch("result")["canonical_changed"] = true }
            metadata_reject_mutation(evidence, spec, mode, variant) { |changed| changed.fetch("frames").first["tables_finish"] = changed.fetch("frames").first.fetch("tables_after") }
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
      spec = metadata_cases.first
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
          evidence["stdout"] = evidence.fetch("stdout").sub('"value_int":4000', '"value_int":99,"value_int":4000')
        elsif kind == "impossible-clock"
          metadata_invalid_clocks!(evidence)
          metadata_refresh!(evidence, "cold")
        end
        reads = evidence.fetch("reads").dup
        snapshots = [evidence.fetch("before"), evidence.fetch("after")]
        outcomes = [evidence.fetch("fixtures").first, evidence].map do |record|
          CommandRunner::Result.new(stdout: record.fetch("stdout"), stderr: record.fetch("stderr"), success: true)
        end
        outcomes[-1] = CommandRunner::Result.new(stdout: "partial native stdout\n", stderr: "native producer failure\n", success: false) if kind == "failed"
        commands = []
        define_singleton_method(:sql) { |query, **_options| commands << query; "" }
        define_singleton_method(:policy_setup!) { |*_args| evidence.fetch("seed_clock") }
        define_singleton_method(:correction_observer!) { |*_args| nil }
        define_singleton_method(:metadata_read) { |*_args| reads.shift }
        define_singleton_method(:ingestion_snapshot) { |*_args| snapshots.shift }
        define_singleton_method(:result) { |*_args, **_options| outcomes.shift }
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
        assert(File.exist?(File.join(@metadata_evidence, "replace-typed-cold-final.stdout")), "#{kind} producer raw attempt retained")
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
      schema = File.binread(File.join(@contract.root, "crates/revaer-data/migrations/0022_indexer_canonicalization.sql"))
      assert(schema.include?("(attr_key IN ('tracker_name', 'imdb_id') AND value_text IS NOT NULL)") && source.include?("IF existing_tracker_name IS NULL THEN"), "serial coalesce-upsert reachability counterexample remains source-backed")
      proof = File.binread(File.join(@contract.root, "scripts/database_rebaseline/ingestion_proof.rb"))
      assert(proof.include?("verify_ingestion_metadata!") && proof.include?("complete ingestion branch and helper execution matrix"), "canonical matrix wired without removing incomplete D3")
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
