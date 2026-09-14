# frozen_string_literal: true

require_relative "database-ingestion-attributes-test"
require_relative "../database_rebaseline/native_trust_rank_readback"

module RevaerDatabaseRebaseline
  # Generated in memory from committed builders, not an independent capture or
  # a replacement for the separate positive rank-40 calibration qualification.
  class NativeTrustRankReadbackFixture < IngestionAttributesTest
    def initialize(root:, source:)
      @contract = NativeTrustRankApplicationOracle::Root.new(root:)
      @source = source
      @runtime = "proof_runtime_#{'a' * 16}"
      @ingestion_inventory = NativeTrustRankReadback::VARIANTS.to_h do |variant|
        names = ["search_result_ingest_v1"] + source.helper_plan.uniq
        routines = names.map do |name|
          root_routine = name == "search_result_ingest_v1"
          { "name" => name, "signature" => root_routine ? source.signature : "#{name}()",
            "source" => root_routine ? source.bodies.fetch(variant) : "synthetic helper body: #{name}",
            "settings" => root_routine && variant == "reference" ? ["plpgsql.variable_conflict=use_column"] : ["search_path=pg_catalog, public"],
            "definer" => variant == "final", "owner" => "proof_owner_#{'a' * 16}" }
        end
        ["#{variant}_proof", { "routines" => routines }]
      end
    end

    def bundle(producers, source_commit)
      hashes = producers.to_h { |name, bytes| ["scripts/database_rebaseline/#{name}", Digest::SHA256.hexdigest(bytes)] }
      contexts = NativeTrustRankReadback::CASES.product(NativeTrustRankReadback::MODES, NativeTrustRankReadback::VARIANTS).map do |case_name, mode, variant|
        context = { "case" => case_name, "mode" => mode, "variant" => variant,
                    "plain" => arm(case_name, mode, variant), "observed" => arm(case_name, mode, variant) }
        native_context!(context)
        context["debugger_script"] = producers.fetch("native-trust-rank.gdb") + 'printf "READY:%d\\n", 0' + "\n"
        context
      end
      { "inventory" => @ingestion_inventory, "contexts" => contexts, "producers" => producers.dup,
        "provenance" => { "source_commit" => source_commit, "source_before" => hashes, "source_after" => hashes.dup } }
    end

    def refresh_arm!(arm, mode)
      evidence = arm.fetch("application")
      attributes_refresh_transport!(evidence, mode)
      %w[stdout stderr].each do |stream|
        arm[stream] = evidence.fetch(stream)
        arm["fixture_#{stream}"] = evidence.fetch("fixture_transport").fetch(stream)
      end
      arm["comparable"] = { "application" => compilation_comparable("fixture" => evidence.fetch("fixture"), "frames" => evidence.fetch("frames")),
                            "inputs" => validation_comparable(evidence, { site: nil }).fetch("inputs") }
    end

    private

    def arm(case_name, mode, variant)
      spec = attributes_cases.find { |item| item.fetch(:name) == case_name }
      evidence = attributes_test_evidence(spec, mode, variant)
      { "application" => evidence, "stdout" => evidence.fetch("stdout"), "stderr" => evidence.fetch("stderr"),
        "sql" => attributes_query(attributes_session(spec, mode)), "fixture_sql" => attributes_query(calls: [attributes_arguments(spec)]),
        "fixture_stdout" => evidence.fetch("fixture_transport").fetch("stdout"), "fixture_stderr" => evidence.fetch("fixture_transport").fetch("stderr"),
        "comparable" => { "application" => compilation_comparable("fixture" => evidence.fetch("fixture"), "frames" => evidence.fetch("frames")),
                          "inputs" => validation_comparable(evidence, { site: nil }).fetch("inputs") } }
    end

    def catalog(variant, mode)
      names = %w[search_result_ingest_v1 derive_magnet_hash_v1 normalize_title_v1]
      names += @source.helper_plan if mode == "helpers-first"
      names.uniq.each_with_index.map do |name, index|
        source = @ingestion_inventory.fetch("#{variant}_proof").fetch("routines").find { |row| row.fetch("name") == name }
        { "oid" => 1001 + index, "schema" => "public", "name" => name, "signature" => source.fetch("signature"),
          "language" => name == "policy_action_to_decision_type" ? "sql" : "plpgsql",
          "source_sha256" => Digest::SHA256.hexdigest(source.fetch("source")), "config" => source.fetch("settings") }
      end
    end

    def snapshot_source
      pairs = IngestionProof::INGESTION_TABLES.map do |table|
        "'#{table}', (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)->(SELECT a.attname FROM pg_attribute a WHERE a.attrelid = 'public.#{table}'::regclass AND a.attnum = 1)), '[]') FROM public.\"#{table}\" t)"
      end
      " SELECT json_build_object(#{pairs.join(',')}); "
    end

    def native_context!(context)
      variant = context.fetch("variant")
      source = snapshot_source
      snapshot = { "oid" => 1000, "schema" => "ingestion_observation", "name" => "snapshot", "signature" => "ingestion_observation.snapshot()",
                   "language" => "sql", "source_sha256" => Digest::SHA256.hexdigest(source), "config" => ["search_path=pg_catalog"] }
      context["catalog"] = [snapshot] + catalog(variant, context.fetch("mode"))
      context["snapshot_sql"] = "CREATE FUNCTION ingestion_observation.snapshot() RETURNS json LANGUAGE sql AS $$#{source}$$;\n"
      rows = context.fetch("catalog").to_h { |row| [row.fetch("name"), row] }
      trigger = { "function" => "RI_FKey_check_ins", "function_oid" => 2001, "trigger_oid" => 2002,
                  "table" => "canonical_torrent_signal", "internal" => true }
      lines, native, events, triggers = ["READY:0"], [], [], []
      call = lambda do |name, setting|
        row = rows.fetch(name)
        lines << "CALL:#{row.fetch('language')}:#{row.fetch('oid')}:#{setting}"
        native << row.merge("kind" => "call", "compiler_setting" => setting, "native_line" => lines.length)
      end
      @source.helper_plan.each { |name| call.call(name, 0) } if context.fetch("mode") == "helpers-first"
      setting = variant == "reference" ? 2 : 0
      frames = context.fetch("observed").fetch("application").fetch("frames")
      frames.each do |frame|
        call.call("snapshot", 0)
        call.call("search_result_ingest_v1", setting)
        @source.expected_events(case_name: context.fetch("case"), variant:, backend: Integer(frame.fetch("backend"), 10), oid: rows.fetch("search_result_ingest_v1").fetch("oid")).each do |event|
          lines << "K1:#{JSON.generate(event)}"
          events << event.merge("native_line" => lines.length)
        end
        %w[derive_magnet_hash_v1 normalize_title_v1].each { |name| call.call(name, setting) }
        if frame.fetch("state") == "00000"
          lines << "RI:RI_FKey_check_ins:#{setting}"
          lines << "TRIGGER:RI_FKey_check_ins:2001:2002:#{setting}"
          triggers << trigger.merge("compiler_setting" => setting)
          native << triggers.last.merge("kind" => "trigger", "native_line" => lines.length)
        end
        2.times { call.call("snapshot", 0) }
      end
      lines << "[Inferior 1 (process #{frames.first.fetch('backend')}) exited normally]"
      context.merge!("events" => events, "native" => native, "triggers" => triggers, "trigger_catalog" => [trigger],
                     "debugger_stdout" => lines.join("\n") + "\n", "debugger_stderr" => "")
    end
  end

  class NativeTrustRankReadbackTest
    SOURCE_COMMIT = ("a" * 40).freeze
    PRODUCERS = %w[native_sessions.rb native_processes.rb native_tooling.rb native_trace.rb native_trust_rank_proof.rb
                   native_trust_rank_source.rb native_trust_rank_readback.rb native-trust-rank.gdb].to_h do |name|
      [name, "# synthetic unit-test #{name} for exec_eval_boolean; never executed\n"]
    end.freeze

    def initialize
      @root = File.expand_path("../..", __dir__)
      @producer_sha256 = PRODUCERS.transform_values { |bytes| Digest::SHA256.hexdigest(bytes) }
      @validator = NativeTrustRankReadback.new(root: @root, source_commit: SOURCE_COMMIT, producer_sha256: @producer_sha256)
      source = NativeTrustRankSource.new(NativeTrustRankSource::PINS.to_h { |path, _digest| [path, File.binread(File.join(@root, path))] })
      @fixture = NativeTrustRankReadbackFixture.new(root: @root, source:)
      @bundle = @fixture.bundle(PRODUCERS, SOURCE_COMMIT)
      @assertions = 0
      @mutations = 0
    end

    def run!
      result = @validator.validate!(@bundle)
      assert(result.fetch("contexts").length == 8 && result.fetch("calls") == 24 && result.fetch("events") == 384, "synthetic scope, not live evidence")
      assert(result.fetch("warning_free") && !result.fetch("canonical_proof") && !result.fetch("d3_complete"), "empty diagnostics do not certify D3")
      assert(result.fetch("contexts").all? { |item| item.fetch("debugger_stderr").empty? }, "all debugger diagnostics empty")
      result.fetch("contexts").each do |item|
        assert(item.fetch("intervals").map { |call| call.fetch("state") } == (item.fetch("variant") == "reference" ? %w[00000 00000 42P07] : %w[00000 00000 00000]), "three synthetic intervals including exact frozen D4")
      end
      assert(result.values_at("source_commit", "producer_sha256") == [SOURCE_COMMIT, @producer_sha256], "independently supplied registry returned")
      source_tests!
      raw_tests!
      sequence_tests!
      identity_tests!
      application_tests!
      producer_tests!
      constructor_tests!
      read_tests!
      puts "database-native-trust-rank-readback-test: #{@assertions} assertions passed (#{@mutations} rejected mutations); synthetic only; calibration and live readback still required; d3_complete=false"
    end

    private

    def assert(value, message)
      raise Failure, "test failed: #{message}" unless value

      @assertions += 1
    end

    def reject(label, pattern = /K1|attribute|ingestion|validation|metadata/)
      yield
    rescue Failure => error
      assert(error.message.match?(pattern), "#{label}: unexpected rejection #{error.message}")
      @mutations += 1
    else
      raise Failure, "mutation accepted: #{label}"
    end

    def mutate(label, index: 0, pattern: /K1|attribute|ingestion|validation|metadata/)
      bundle = Marshal.load(Marshal.dump(@bundle))
      yield(bundle.fetch("contexts").fetch(index), bundle)
      reject(label, pattern) { @validator.validate!(bundle) }
    end

    def sync_k1(context)
      lines = context.fetch("debugger_stdout").lines(chomp: true)
      context.fetch("events").each do |event|
        lines[event.fetch("native_line") - 1] = "K1:#{JSON.generate(event.reject { |key, _value| key == 'native_line' })}"
      end
      context["debugger_stdout"] = lines.join("\n") + "\n"
    end

    # Rebuild indices after a raw-line edit so failures cannot depend merely on
    # stale offsets or divergence between the serialized and raw K1 records.
    def edit_lines(context)
      original = context.fetch("debugger_stdout").lines(chomp: true)
      tagged = original.each_with_index.map { |line, index| [index + 1, line] }
      yield tagged
      positions = tagged.each_with_index.to_h { |(old, _line), index| [old, index + 1] }
      context["debugger_stdout"] = tagged.map(&:last).join("\n") + "\n"
      context["events"] = tagged.each_with_index.filter_map do |(_old, line), index|
        JSON.parse(line.delete_prefix("K1:")).merge("native_line" => index + 1) if line.start_with?("K1:")
      end
      context["native"] = context.fetch("native").filter_map do |event|
        line = positions[event.fetch("native_line")]
        event.merge("native_line" => line) if line
      end.sort_by { |event| event.fetch("native_line") }
    end

    def source_tests!
      files = NativeTrustRankSource::PINS.to_h { |path, _digest| [path, File.binread(File.join(@root, path))] }
      source = NativeTrustRankSource.new(files)
      NativeTrustRankReadback::CASES.product(NativeTrustRankReadback::VARIANTS).each do |case_name, variant|
        events = source.expected_events(case_name:, variant:, backend: 123, oid: 456)
        missing = case_name == "missing-public-trust-tier"
        offset = variant == "reference" ? 0 : 1
        assert(events.length == (missing ? 18 : 14), "source-derived event count")
        assert(events.first.fetch("before").values_at("line", "statement_id") == [128 + offset, 462], "source block location")
        assert(events.last.fetch("after").fetch("locals").values_at("instance_trust_rank", "trust_bucket").all? { |local| local.values_at("isnull", "value") == [false, 0] }, "source-derived zero values")
        outer = events.find { |event| event["kind"] == "boolean" && event["phase"] == "return" }
        assert(outer.fetch("answer") == (missing ? 1 : 0), "independent branch answer")
      end
      NativeTrustRankSource::PINS.each_key do |path|
        reject("changed source #{path}", /frozen source changed/) { NativeTrustRankSource.new(files.merge(path => files.fetch(path) + "\n")) }
      end
      helpers = %w[policy_action_to_decision_type] * 6 + %w[normalize_title_v1] * 2 +
        %w[normalize_magnet_uri_v1] * 8 + %w[derive_magnet_hash_v1] * 4 + %w[normalize_magnet_uri_v1] +
        %w[compute_title_size_hash_v1] * 3 + %w[policy_text_match_v1] * 11 + %w[policy_uuid_match_v1] * 4 +
        %w[policy_int_match_v1] * 4 + %w[policy_release_group_match_v1 policy_text_match_v1 policy_release_group_match_v1 policy_text_match_v1 policy_release_group_match_v1]
      assert(source.helper_plan == helpers, "pinned helper source includes folding and nested calls in order")
      reject("unknown source case") { source.expected_events(case_name: "typed-rank-40", variant: "reference", backend: 123, oid: 456) }
    end

    def raw_tests!
      %w[PARENT_RI:RI_FKey_cascade_del:2 REGEX:textregexeq:2].each do |marker|
        mutate("unexpected native marker #{marker}") do |context, _bundle|
          edit_lines(context) do |lines|
            ready = lines.index { |_index, line| line == "READY:0" }
            lines.insert(ready + 1, [-1, marker])
          end
        end
      end
      mutate("JSON-only event change") { |context, _bundle| context.fetch("events").first["query"] = "1" }
      mutate("raw-only event change") { |context, _bundle| context["debugger_stdout"] = context.fetch("debugger_stdout").sub('"query":"0"', '"query":"1"') }
      mutate("duplicate raw JSON field") { |context, _bundle| context["debugger_stdout"] = context.fetch("debugger_stdout").sub('"phase":"entry"', '"phase":"entry","phase":"entry"') }
      mutate("duplicate nested raw JSON field") { |context, _bundle| context["debugger_stdout"] = context.fetch("debugger_stdout").sub('"isnull":true', '"isnull":true,"isnull":true') }
      mutate("escaped duplicate raw JSON field") { |context, _bundle| context["debugger_stdout"] = context.fetch("debugger_stdout").sub('"phase":"entry"', '"phase":"entry","ph\u0061se":"entry"') }
      mutate("raw native_line injection") { |context, _bundle| context["debugger_stdout"] = context.fetch("debugger_stdout").sub('K1:{', 'K1:{"native_line":19,') }
      mutate("missing JSON event") { |context, _bundle| context.fetch("events").shift }
      mutate("duplicated JSON event") { |context, _bundle| context.fetch("events").insert(0, context.fetch("events").first.dup) }
      mutate("JSON order") { |context, _bundle| context.fetch("events")[0, 2] = context.fetch("events").first(2).reverse }
      %w[K1_ERROR: Python\ Exception Traceback Cannot\ access\ memory].each do |error|
        %w[debugger_stdout debugger_stderr].each do |stream|
          mutate("#{error} in #{stream}") { |context, _bundle| context[stream] += "#{error} test\n" }
        end
      end
      mutate("unknown warning") { |context, _bundle| context["debugger_stderr"] += "warning: unknown observer condition\n" }
      mutate("old missing-source warning", pattern: /debugger stderr must be empty/) do |context, _bundle|
        context["debugger_stderr"] = "warning: 30\tsrc/thread/aarch64/syscall_cp.s: No such file or directory\n"
      end
      mutate("whitespace stderr", pattern: /debugger stderr must be empty/) { |context, _bundle| context["debugger_stderr"] = "\n" }
      mutate("missing READY") { |context, _bundle| context["debugger_stdout"] = context.fetch("debugger_stdout").sub("READY:0", "READY:2") }
      mutate("duplicate READY") { |context, _bundle| context["debugger_stdout"] += "READY:0\n" }
      mutate("changed exited backend") { |context, _bundle| context["debugger_stdout"] = context.fetch("debugger_stdout").sub(/\(process \d+\) exited/, "(process 999999) exited") }
    end

    def sequence_tests!
      @bundle.fetch("contexts").each_index do |index|
        mutate("synchronized missing pair #{index}", index:) do |context, _bundle|
          edit_lines(context) { |lines| start = lines.index { |_old, line| line.start_with?("K1:") }; lines.slice!(start, 2) }
        end
        mutate("synchronized duplicate pair #{index}", index:) do |context, _bundle|
          edit_lines(context) do |lines|
            start = lines.index { |_old, line| line.start_with?("K1:") }
            lines.insert(start, *lines.slice(start, 2).each_with_index.map { |(_old, line), n| [-n - 1, line] })
          end
        end
        mutate("changed branch answer #{index}", index:) do |context, _bundle|
          event = context.fetch("events").find { |item| item.key?("answer") }
          event["answer"] = 1 - event.fetch("answer")
          sync_k1(context)
        end
      end
      mutate("synchronized reordered pair") do |context, _bundle|
        edit_lines(context) { |lines| start = lines.index { |_old, line| line.start_with?("K1:") }; lines[start, 4] = lines.slice(start + 2, 2) + lines.slice(start, 2) }
      end
      mutate("reordered phase") do |context, _bundle|
        edit_lines(context) { |lines| start = lines.index { |_old, line| line.start_with?("K1:") }; lines[start, 2] = lines.slice(start, 2).reverse }
      end
      mutate("K1 outside root interval") do |context, _bundle|
        edit_lines(context) { |lines| start = lines.index { |_old, line| line.start_with?("K1:") }; pair = lines.slice!(start, 2); lines.insert(start - 1, *pair) }
      end
      mutate("synchronized missing snapshot") do |context, _bundle|
        line = context.fetch("native").first.fetch("native_line")
        edit_lines(context) { |lines| lines.reject! { |old, _text| old == line } }
      end
      mutate("synchronized missing helper-first call", index: 2) do |context, _bundle|
        line = context.fetch("native").first.fetch("native_line")
        edit_lines(context) { |lines| lines.reject! { |old, _text| old == line } }
      end
      mutate("synchronized swapped root and snapshot") do |context, _bundle|
        first, second = context.fetch("native").first(2).map { |event| event.fetch("native_line") - 1 }
        edit_lines(context) { |lines| lines[first], lines[second] = lines[second], lines[first] }
      end
      mutate("unexpected >=30 success") do |context, _bundle|
        context.fetch("events").find { |event| event["query"] == "instance_trust_rank >= 30" && event["phase"] == "return" }["answer"] = 1
        sync_k1(context)
      end
      mutate("missing-row never NULL", index: 4) do |context, _bundle|
        context.fetch("events").each do |event|
          next unless event["query"] == "instance_trust_rank IS NULL"

          %w[before after].each { |key| event.fetch(key).fetch("locals").fetch("instance_trust_rank").merge!("isnull" => false, "value" => 0) if event.key?(key) }
        end
        sync_k1(context)
      end
      mutate("missing-row fallback leaves NULL", index: 4) do |context, _bundle|
        event = context.fetch("events").find { |item| item["query"] == "instance_trust_rank := 0" && item["phase"] == "return" }
        event.fetch("after").fetch("locals").fetch("instance_trust_rank").merge!("isnull" => true, "value" => nil)
        sync_k1(context)
      end
      mutate("third frozen call reduced to initialization") do |context, _bundle|
        native = context.fetch("native")
        last_root = native.select { |item| item["name"] == "search_result_ingest_v1" }.last.fetch("native_line")
        edit_lines(context) do |lines|
          seen = 0
          lines.reject! do |old, line|
            next false unless old > last_root && line.start_with?("K1:")

            seen += 1
            seen > 4
          end
        end
      end
    end

    def identity_tests!
      { "backend" => 999, "function_oid" => 999, "signature" => "search_result_ingest_v1(uuid)",
        "compiler_setting" => 1, "resolve_option" => 0, "line" => 471, "statement_id" => 999 }.each do |key, value|
        mutate("coordinated context #{key}") do |context, _bundle|
          context.fetch("events").each { |event| %w[before after].each { |phase| event.fetch(phase)[key] = value if event.key?(phase) } }
          sync_k1(context)
        end
      end
      NativeTrustRankSource::LOCALS.each do |name|
        { "dno" => 999, "declaration_line" => 999, "isnull" => "false" }.each do |key, value|
          mutate("local #{name} #{key}") do |context, _bundle|
            context.fetch("events").each do |event|
              %w[before after].each { |phase| event.fetch(phase).fetch("locals").fetch(name)[key] = value if event.key?(phase) }
            end
            sync_k1(context)
          end
        end
      end
      [nil, 1, 40, 0.0, "0"].each do |value|
        mutate("incorrect bucket #{value.inspect}") do |context, _bundle|
          context.fetch("events").last.fetch("after").fetch("locals").fetch("trust_bucket")["value"] = value
          sync_k1(context)
        end
      end
      mutate("extra local") do |context, _bundle|
        context.fetch("events").first.fetch("before").fetch("locals")["unexpected"] = {}
        sync_k1(context)
      end
      mutate("missing return context") { |context, _bundle| context.fetch("events")[1].delete("after"); sync_k1(context) }
      mutate("assignment target changed") { |context, _bundle| context.fetch("events").first["target"] = "trust_bucket"; sync_k1(context) }
      mutate("nonzero initialization expression") { |context, _bundle| context.fetch("events").first["query"] = "40"; sync_k1(context) }
      mutate("extra assignment answer") { |context, _bundle| context.fetch("events")[1]["answer"] = 0; sync_k1(context) }
      mutate("duplicate catalog") { |context, _bundle| context.fetch("catalog") << context.fetch("catalog").first.dup }
      mutate("source inventory") { |_context, bundle| bundle.fetch("inventory").fetch("reference_proof").fetch("routines").find { |row| row["name"] == "search_result_ingest_v1" }["source"] += "\n" }
      mutate("catalog source digest") { |context, _bundle| context.fetch("catalog").find { |row| row["name"] == "search_result_ingest_v1" }["source_sha256"] = "0" * 64 }
      mutate("snapshot source") { |context, _bundle| context["snapshot_sql"] = context.fetch("snapshot_sql").sub("json_build_object", "json_build_array") }
      mutate("native root offset") { |context, _bundle| context.fetch("native").find { |event| event["name"] == "search_result_ingest_v1" }["native_line"] += 1 }
      mutate("missing native snapshot") { |context, _bundle| context.fetch("native").shift }
      mutate("duplicate native root") { |context, _bundle| context.fetch("native").insert(1, context.fetch("native")[1].dup) }
      mutate("coordinated native trigger metadata") do |context, _bundle|
        context.fetch("triggers").each { |trigger| trigger["internal"] = false }
        context.fetch("native").select { |event| event["kind"] == "trigger" }.each { |event| event["internal"] = false }
      end
      mutate("trigger catalog duplicate") { |context, _bundle| context.fetch("trigger_catalog") << context.fetch("trigger_catalog").first.dup }
      mutate("helper ambient scope", index: 2) do |context, _bundle|
        native = context.fetch("native").first
        native["compiler_setting"] = 2
        lines = context.fetch("debugger_stdout").lines(chomp: true)
        lines[native.fetch("native_line") - 1] = lines.fetch(native.fetch("native_line") - 1).sub(/:0\z/, ":2")
        context["debugger_stdout"] = lines.join("\n") + "\n"
      end
      mutate("helper context relabeled cold", index: 2) { |context, _bundle| context["mode"] = "cold" }
      mutate("missing context") { |_context, bundle| bundle.fetch("contexts").pop }
      mutate("duplicate context") { |context, bundle| bundle.fetch("contexts") << context }
    end

    def application_tests!
      %w[plain observed].each do |arm|
        mutate("#{arm} canonical state") { |context, _bundle| context.fetch(arm).fetch("application").fetch("frames").last["state"] = "00000" }
        mutate("#{arm} canonical scalar") { |context, _bundle| context.fetch(arm).fetch("application").fetch("frames").first.fetch("tables_after").fetch("canonical_torrent_signal").first["confidence"] = 0.6 }
        mutate("#{arm} raw application") { |context, _bundle| context.fetch(arm)["stdout"] += "extra\n" }
        mutate("#{arm} fixture raw") { |context, _bundle| context.fetch(arm)["fixture_stdout"] += "extra\n" }
        mutate("#{arm} statement") { |context, _bundle| context.fetch(arm)["sql"] += "SELECT 1;\n" }
        mutate("#{arm} fixture statement") { |context, _bundle| context.fetch(arm)["fixture_sql"] += "SELECT 1;\n" }
        mutate("#{arm} comparable") { |context, _bundle| context.fetch(arm)["comparable"] = {} }
        mutate("#{arm} duplicate application JSON key") do |context, _bundle|
          data = context.fetch(arm)
          data["stdout"] = data.fetch("stdout").sub(/"backend"\s*:\s*"(\d+)"/) { |entry| "#{entry},#{entry}" }
          data.fetch("application")["stdout"] = data.fetch("stdout")
        end
        mutate("#{arm} synchronized raw outcome") do |context, _bundle|
          data = context.fetch(arm)
          data.fetch("application").fetch("frames").first.fetch("result")["observation_created"] = true
          data["stdout"] = data.fetch("stdout").sub(/"observation_created"\s*:\s*false/, '"observation_created":true')
          data.fetch("application")["stdout"] = data.fetch("stdout")
        end
        mutate("#{arm} changed exact D4") do |context, _bundle|
          data = context.fetch(arm)
          data["stderr"] = data.fetch("stderr").sub("createas.c:406", "createas.c:407")
          data.fetch("application")["stderr"] = data.fetch("stderr")
        end
      end
      mutate("helper result changed", index: 2) do |context, _bundle|
        data = context.fetch("observed")
        data["stdout"] = data.fetch("stdout").sub("helpers:", "other:")
        data.fetch("application")["stdout"] = data.fetch("stdout")
      end
      mutate("aggregate pairs equal but wrong") do |context, _bundle|
        %w[plain observed].each do |arm|
          data = context.fetch(arm)
          data.fetch("application").fetch("trust_rank_expectation")["rank"] = 40
        end
      end
      %w[inputs_before inputs_fixture inputs_after].each do |phase|
        mutate("missing input table #{phase}", pattern: /attribute read inputs changed/) do |context, _bundle|
          context.fetch("observed").fetch("application").fetch(phase).delete("trust_tier")
        end
      end
      mutate("coherent paired wrong public rank", pattern: /attribute read inputs changed/) do |context, _bundle|
        %w[plain observed].each do |arm|
          evidence = context.fetch(arm).fetch("application")
          %w[inputs_before inputs_fixture inputs_after].each do |phase|
            evidence.fetch(phase).fetch("trust_tier").find { |row| row.fetch("trust_tier_key") == "public" }["rank"] = 10
          end
          @fixture.refresh_arm!(context.fetch(arm), context.fetch("mode"))
        end
      end
      mutate("coherent paired wrong table value", pattern: /independently specified first results changed/) do |context, _bundle|
        %w[plain observed].each do |arm|
          context.fetch(arm).fetch("application").fetch("fixture").fetch("tables_after").fetch("canonical_torrent_signal").first["confidence"] = 0.6
          @fixture.refresh_arm!(context.fetch(arm), context.fetch("mode"))
        end
      end
      mutate("coherent wrong result", pattern: /independently specified repeat results changed/) do |context, _bundle|
        context.fetch("observed").fetch("application").fetch("frames").first.fetch("result")["canonical_changed"] = true
        @fixture.refresh_arm!(context.fetch("observed"), context.fetch("mode"))
      end
      mutate("coherent application backend substitution", pattern: /debugger.application backend changed/) do |context, _bundle|
        context.fetch("observed").fetch("application").fetch("frames").each { |frame| frame["backend"] = "777" }
        @fixture.refresh_arm!(context.fetch("observed"), context.fetch("mode"))
      end
      mutate("coherent duplicate transaction clock", pattern: /cold backend or role.GUC.clock changed/) do |context, _bundle|
        frames = context.fetch("observed").fetch("application").fetch("frames")
        frames.last["clock"] = frames.first.fetch("clock")
        @fixture.refresh_arm!(context.fetch("observed"), context.fetch("mode"))
      end
    end

    def producer_tests!
      PRODUCERS.each_key do |name|
        mutate("retained producer bytes #{name}") { |_context, bundle| bundle.fetch("producers")[name] += "\n" }
        %w[source_before source_after].each do |phase|
          mutate("retained producer seal #{name} #{phase}") do |_context, bundle|
            hashes = bundle.fetch("provenance").fetch(phase)
            path = hashes.keys.find { |key| File.basename(key) == name }
            hashes[path] = "0" * 64
          end
        end
      end
      mutate("amended producer checkpoint") { |_context, bundle| bundle.fetch("provenance")["source_commit"] = "f" * 40 }
      mutate("unbound actual debugger probe") { |context, _bundle| context["debugger_script"] = context.fetch("debugger_script").sub("exec_eval_boolean", "another_symbol") }
      mutate("missing probe insertion") { |context, _bundle| context["debugger_script"] = 'printf "READY:%d\\n", 0' }
      mutate("duplicate probe insertion") { |context, _bundle| context["debugger_script"] += PRODUCERS.fetch("native-trust-rank.gdb") }
      mutate("probe inserted after readiness") do |context, _bundle|
        context["debugger_script"] = 'printf "READY:%d\\n", 0' + "\n" + PRODUCERS.fetch("native-trust-rank.gdb")
      end
      mutate("missing readiness command") { |context, _bundle| context["debugger_script"] = PRODUCERS.fetch("native-trust-rank.gdb") }
      mutate("extra producer") { |_context, bundle| bundle.fetch("producers")["unregistered.rb"] = "extra" }
      mutate("missing producer") { |_context, bundle| bundle.fetch("producers").delete("native_sessions.rb") }
      mutate("old probe basename") do |_context, bundle|
        bundle.fetch("producers")["trust-rank-probe.gdb"] = bundle.fetch("producers").delete("native-trust-rank.gdb")
      end
      mutate("coherently resealed attacker producer") do |context, bundle|
        name = "native-trust-rank.gdb"
        changed = bundle.fetch("producers").fetch(name) + "# substitution\n"
        context["debugger_script"] = context.fetch("debugger_script").sub(PRODUCERS.fetch(name), changed)
        bundle.fetch("producers")[name] = changed
        %w[source_before source_after].each do |phase|
          bundle.fetch("provenance").fetch(phase)["scripts/database_rebaseline/#{name}"] = Digest::SHA256.hexdigest(changed)
        end
      end
      %w[source_before source_after].each do |phase|
        mutate("missing source phase #{phase}") { |_context, bundle| bundle.fetch("provenance").delete(phase) }
        mutate("malformed source phase #{phase}") { |_context, bundle| bundle.fetch("provenance")[phase] = [] }
      end
      mutate("duplicate producer basename in both source phases") do |_context, bundle|
        %w[source_before source_after].each do |phase|
          bundle.fetch("provenance").fetch(phase)["other/native-trust-rank.gdb"] = @producer_sha256.fetch("native-trust-rank.gdb")
        end
      end
      mutate("unrelated source registry drift") { |_context, bundle| bundle.fetch("provenance").fetch("source_after")["other.rb"] = "a" * 64 }
      [nil, [], {}, { "provenance" => nil }].each { |value| reject("malformed bundle") { @validator.validate!(value) } }
    end

    def constructor_tests!
      [nil, "", "a" * 39, "A" * 40, "g" * 40].each do |commit|
        reject("invalid trusted checkpoint") { NativeTrustRankReadback.new(root: @root, source_commit: commit, producer_sha256: @producer_sha256) }
      end
      [nil, [], {}, { "old-probe.gdb" => "a" * 64 }, { "native-trust-rank.gdb" => "x" },
       @producer_sha256.merge("../producer.rb" => "a" * 64), @producer_sha256.merge("" => "a" * 64),
       @producer_sha256.merge("producer.rb" => nil)].each do |hashes|
        reject("invalid trusted producer map") { NativeTrustRankReadback.new(root: @root, source_commit: SOURCE_COMMIT, producer_sha256: hashes) }
      end
      commit = SOURCE_COMMIT.dup
      hashes = @producer_sha256.transform_values(&:dup)
      validator = NativeTrustRankReadback.new(root: @root, source_commit: commit, producer_sha256: hashes)
      commit.replace("b" * 40)
      hashes.each_value { |digest| digest.replace("b" * 64) }
      hashes.clear
      assert(validator.validate!(@bundle).fetch("source_commit") == SOURCE_COMMIT, "trusted registry copied before caller mutation")

      bundle = Marshal.load(Marshal.dump(@bundle))
      bundle.fetch("provenance")["source_commit"] = "b" * 40
      fresh = NativeTrustRankReadback.new(root: @root, source_commit: "b" * 40, producer_sha256: @producer_sha256)
      assert(fresh.validate!(bundle).fetch("source_commit") == "b" * 40, "no historical source checkpoint hardcoded")
      producers = PRODUCERS.merge("native_sessions.rb" => "# distinct independently supplied synthetic producer\n")
      hashes = producers.transform_values { |bytes| Digest::SHA256.hexdigest(bytes) }
      fresh = NativeTrustRankReadback.new(root: @root, source_commit: SOURCE_COMMIT, producer_sha256: hashes)
      assert(fresh.validate!(@fixture.bundle(producers, SOURCE_COMMIT)).fetch("producer_sha256") == hashes, "exact current registry replaces historical producer pins")
    end

    # Minimal temporary wire-layout fixtures exercise read_run! without writing
    # complete synthetic captures, retaining target evidence, or running a server.
    def read_tests!
      Dir.mktmpdir("native-trust-rank-readback-test-") do |directory|
        paths = wire_files
        paths.each do |name, bytes|
          path = File.join(directory, name)
          FileUtils.mkdir_p(File.dirname(path))
          File.binwrite(path, bytes)
        end
        loaded = @validator.read_run!(directory)
        assert(loaded.fetch("contexts").map { |context| context.values_at("case", "mode", "variant") } == NativeTrustRankReadback::CASES.product(NativeTrustRankReadback::MODES, NativeTrustRankReadback::VARIANTS), "canonical wire context order")
        assert(loaded.fetch("producers") == PRODUCERS && loaded.fetch("provenance") == @bundle.fetch("provenance"), "wire registry and producer bytes retained")
        reject("wire placeholders cannot qualify") { @validator.validate!(loaded) }
        paths.select { |name, _bytes| name.end_with?(".json") }.each_key do |name|
          path = File.join(directory, name)
          File.binwrite(path, '{"nested":{"value":1,"value":1}}')
          reject("duplicate wire JSON #{name}", /invalid or duplicate JSON/) { @validator.read_run!(directory) }
          File.binwrite(path, paths.fetch(name))
        end
        report = File.join(directory, "report.json")
        ["{", "[]", "null"].each do |bytes|
          File.binwrite(report, bytes)
          reject("malformed report") { @validator.read_run!(directory) }
        end
        File.unlink(report)
        reject("missing report", /cannot read evidence/) { @validator.read_run!(directory) }
        File.symlink(File.join(directory, "reference_proof-inventory.json"), report)
        reject("symlink report", /regular non-symlink file/) { @validator.read_run!(directory) }
        File.unlink(report)
        File.binwrite(report, paths.fetch("report.json"))
        Dir.mkdir(File.join(directory, "unexpected-context"))
        reject("extra context directory", /context directory set changed/) { @validator.read_run!(directory) }
      end
    end

    def wire_files
      paths = PRODUCERS.merge("report.json" => JSON.generate(@bundle.fetch("provenance")))
      %w[reference_proof final_proof].each { |variant| paths["#{variant}-inventory.json"] = "{}" }
      @bundle.fetch("contexts").each do |context|
        prefix = context.values_at("case", "mode", "variant").join("-")
        %w[plain traced].each do |arm|
          %w[application comparable].each { |name| paths["#{prefix}-#{arm}/#{name}.json"] = "{}" }
          %w[stdout stderr sql].each do |extension|
            ["", "-fixture"].each { |fixture| paths["#{prefix}-#{arm}/#{prefix}#{fixture}.#{extension}"] = "" }
          end
        end
        %w[trust-rank-events native-events call-catalog].each { |name| paths["#{prefix}-traced/#{name}.json"] = "[]" }
        paths["#{prefix}-traced/observer-#{context.fetch('variant')}.sql"] = ""
        %w[stdout stderr gdb stdout.triggers.json stdout.trigger-catalog.json].each do |suffix|
          paths["#{prefix}-traced-observed-debugger.#{suffix}"] = suffix.end_with?(".json") ? "[]" : ""
        end
      end
      paths
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    raise RevaerDatabaseRebaseline::Failure, "no evidence arguments accepted; synthetic unit tests only" unless ARGV.empty?

    RevaerDatabaseRebaseline::NativeTrustRankReadbackTest.new.run!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn error.message
    exit 1
  end
end
