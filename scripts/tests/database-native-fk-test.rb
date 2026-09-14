# frozen_string_literal: true

require_relative "../database_rebaseline/native_fk_phases"
require_relative "../database_rebaseline/native_fk_remaining_phases"

module RevaerDatabaseRebaseline
  # Synthetic contracts exercise validators only; they are not native execution proof.
  class NativeFkTest
    LOGGER_TABLES = %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].freeze

    def run!
      @assertions = 0
      compilation_test!
      remaining_test!
      oracle_test!
      puts "database-native-fk-test: #{@assertions} assertions passed (synthetic only)"
    end

    private

    def assert(condition, label)
      raise Failure, "native FK test failed: #{label}" unless condition

      @assertions += 1
    end

    def rejected(label)
      yield
    rescue Failure => error
      assert(error.message.include?(label), "expected #{label}, got #{error.message}")
    else
      raise Failure, "native FK test accepted #{label}"
    end

    def copy(value)
      Marshal.load(Marshal.dump(value))
    end

    def call(name, setting = 0, schema = "public")
      { "kind" => "call", "schema" => schema, "name" => name, "compiler_setting" => setting }
    end

    def snapshot
      call("snapshot", 0, "ingestion_observation")
    end

    def callback(table, setting, function = "RI_FKey_check_ins", constraint = "#{table}_fk")
      { "kind" => "trigger", "function" => function, "table" => table, "constraint" => constraint,
        "compiler_setting" => setting, "language" => "internal", "internal" => true, "enabled" => "O",
        "constraint_type" => "f", "function_oid" => 11, "trigger_oid" => 12, "constraint_oid" => 13 }
    end

    def number!(events)
      events.each_with_index { |event, index| event["native_line"] = index + 1 }
    end

    def logger(setting)
      [call("log_source_metadata_conflict_v1", setting)] + LOGGER_TABLES.map { |table| callback(table, setting) }
    end

    def compilation_fixture(scenario = "logger-first-setting", variant = "reference")
      count = scenario == "cold-logger-setting" ? 1 : 3
      expectations = { "multisets" => {
        "logger" => LOGGER_TABLES.map { |table| { "function" => "RI_FKey_check_ins", "table" => table, "constraints" => ["#{table}_fk"], "count_each" => 1 } },
        "new_canonical_existing_source" => [{ "function" => "RI_FKey_check_upd", "table" => "canonical_torrent_source",
                                              "constraints" => %w[source_canonical_fk source_instance_fk], "count_each" => 1 }]
      }, "operations" => [] }
      events = []
      count.times do |index|
        ingest = index.positive?
        settings = { "reference" => ingest ? 2 : 0, "final" => 0 }
        setting = settings.fetch(variant)
        multisets = ingest ? { "new_canonical_existing_source" => 1, "logger" => 2 } : { "logger" => 1 }
        expectations.fetch("operations") << { "scenario" => scenario, "operation_ordinal" => index + 1,
                                              "compiler_setting" => settings, "multisets" => multisets }
        phase = if ingest
                  [call("search_result_ingest_v1", setting), call("derive_magnet_hash_v1", setting)] +
                    %w[source_canonical_fk source_instance_fk].map { |constraint| callback("canonical_torrent_source", setting, "RI_FKey_check_upd", constraint) } +
                    logger(setting) + logger(setting)
                else
                  logger(setting)
                end
        events.concat([snapshot] + phase + [snapshot, snapshot])
      end
      number!(events)
      { expectations:, input: { events:, frames: Array.new(count) { { "state" => "00000", "before" => "error", "after" => "error" } }, scenario:, variant: } }
    end

    def validate_compilation(fixture)
      NativeFkPhases.new(fixture.fetch(:expectations)).validate!(**fixture.fetch(:input))
    end

    def compilation_test!
      %w[cold-logger-setting logger-first-setting].product(%w[reference final]).each do |scenario, variant|
        fixture = compilation_fixture(scenario, variant)
        original = copy(fixture)
        result = validate_compilation(fixture)
        counts = scenario == "cold-logger-setting" ? [3] : [3, 8, 8]
        assert(result.map { |row| row.fetch("callback_count") } == counts, "compilation callback counts #{scenario}/#{variant}")
        assert(result.map { |row| row.fetch("ordinal") } == (1..counts.length).to_a, "compilation ordinals")
        assert(result.map { |row| row.fetch("compiler_setting") } == (variant == "reference" && counts.length == 3 ? [0, 2, 2] : [0] * counts.length), "compilation settings")
        assert(result.first.fetch("callbacks") == LOGGER_TABLES.map { |table| { "identity" => ["RI_FKey_check_ins", table, "#{table}_fk"], "count" => 1 } }, "callback identities retained")
        assert(fixture == original, "compilation input unchanged")
      end
      mutations = {
        "unknown FK compilation context" => ->(f) { f[:input][:scenario] = "unknown" },
        "unknown native FK event" => ->(f) { f[:input][:events][1]["kind"] = "return" },
        "native event ordering changed" => ->(f) { f[:input][:events][1]["native_line"] = 1 },
        "operation or frame inventory changed" => ->(f) { f[:input][:frames][0]["after"] = "use_column" },
        "native snapshot boundaries changed" => ->(f) { f[:input][:events][0]["compiler_setting"] = 2 },
        "in-call compiler setting changed" => ->(f) { f[:input][:events][1]["compiler_setting"] = 2 },
        "native operation root or logger inventory changed" => ->(f) { f[:input][:events][1]["name"] = "normalize_title_v1" },
        "unknown authored helper in FK phase" => ->(f) { f[:input][:events].find { |e| e["name"] == "derive_magnet_hash_v1" }["name"] = "unknown" },
        "native trigger binding changed" => ->(f) { f[:input][:events][2]["function_oid"] = 0 },
        "native callback multiset differs" => ->(f) { f[:input][:events][2]["constraint"] = "unowned_fk" },
        "logger callback ordering or ownership changed" => ->(f) { f[:input][:events].insert(3, call("normalize_title_v1")); number!(f[:input][:events]) },
        "native helper or callback escaped" => ->(f) { f[:input][:events] << call("normalize_title_v1"); number!(f[:input][:events]) },
        "invalid independent callback multiplier" => ->(f) { f[:expectations]["operations"][0]["multisets"]["logger"] = 0 },
        "unresolved independent FK count" => ->(f) { f[:expectations]["multisets"]["logger"][0]["count_each"] = nil }
      }
      mutations.each do |label, mutate|
        fixture = compilation_fixture
        mutate.call(fixture)
        rejected(label) { validate_compilation(fixture) }
      end
      %w[language internal enabled constraint_type function trigger_oid constraint_oid].zip(["sql", false, "D", "u", "RI_FKey_cascade_del", -1, "13"]).each do |field, value|
        fixture = compilation_fixture
        fixture[:input][:events][2][field] = value
        rejected("native trigger binding changed") { validate_compilation(fixture) }
      end
      fixture = compilation_fixture
      fixture[:expectations]["multisets"]["logger"] << { "function" => "RI_FKey_check_ins", "table" => "unused", "constraints" => ["unused_fk"], "count_each" => 0 }
      assert(validate_compilation(fixture).length == 3, "resolved zero counts remain valid")
      fixture = compilation_fixture
      events = fixture[:input][:events]
      events[2], events[3] = events[3], events[2]
      number!(events)
      assert(validate_compilation(fixture).first.fetch("callback_count") == 3, "logger callbacks are a multiset, not a fixed internal order")
    end

    def oracle_row(table, scope, variant, count)
      wrapper = scope == "wrapper_post_v1"
      owner = { "v1_direct" => "public.search_result_ingest_v1", "v1_nested_logger" => "public.log_source_metadata_conflict_v1", "wrapper_post_v1" => "public.search_result_ingest" }.fetch(scope)
      { "function" => "RI_FKey_check_ins", "table" => "public.#{table}", "constraint" => "#{table}_fk", "count" => count,
        "scope" => scope, "sql_owner" => owner, "v1_active" => !wrapper,
        "owner_compile_resolution" => scope == "v1_direct" || (scope == "v1_nested_logger" && variant == "reference") ? "use_column" : "error",
        "ambient_variable_conflict" => wrapper || variant == "final" ? "error" : "use_column" }
    end

    def remaining_fixture(scenario = "v2-conflict-warm-rollback", variant = "reference")
      wrapped = scenario == "v2-conflict-warm-rollback"
      state = !wrapped && variant == "reference" ? "42P10" : "00000"
      setting = variant == "reference" ? 2 : 0
      rows = [oracle_row("canonical_torrent_source", "v1_direct", variant, 1)]
      if wrapped
        rows.concat(LOGGER_TABLES.map { |table| oracle_row(table, "v1_nested_logger", variant, 2) })
        rows << oracle_row("canonical_torrent_best_source_context", "wrapper_post_v1", variant, 1)
      end
      entries = Array.new(2) do |index|
        { "ordinal" => index + 1, "expected_sqlstate" => state, "multiset" => copy(rows), "total_callback_entries" => wrapped ? 8 : 1,
          "scope_totals" => { "wrapper_pre_v1" => 0, "v1_direct" => 1, "v1_nested_logger" => wrapped ? 6 : 0, "wrapper_post_v1" => wrapped ? 1 : 0 } }
      end
      oracle = { "cases" => [{ "name" => wrapped ? "existing-v2-hash-conflict-warm-rollback" : "imdb-upsert", "variants" => { variant => { "operations" => entries } } }] }
      events = []
      frames = Array.new(2) do |index|
        phase = wrapped ? [call("search_result_ingest")] : []
        phase.concat(%w[search_result_ingest_v1 derive_magnet_hash_v1 normalize_title_v1].map { |name| call(name, setting) })
        phase << callback("canonical_torrent_source", setting)
        phase.concat(logger(setting) + logger(setting) + [callback("canonical_torrent_best_source_context", 0)]) if wrapped
        events.concat([snapshot] + phase + [snapshot, snapshot])
        { "state" => state, "before" => "error", "after" => "error", "within" => (state == "00000").to_s,
          "outside" => (wrapped && variant == "reference" && index == 1).to_s }
      end
      number!(events)
      operations = NativeFkRemainingPhases.operations(oracle, scenario, variant)
      { oracle:, input: { events:, frames:, scenario:, variant:, operations: } }
    end

    def validate_remaining(fixture)
      NativeFkRemainingPhases.new.validate!(**fixture.fetch(:input))
    end

    def remaining_test!
      %w[imdb-upsert v2-conflict-warm-rollback].product(%w[reference final]).each do |scenario, variant|
        fixture = remaining_fixture(scenario, variant)
        original = copy(fixture)
        wrapped = scenario == "v2-conflict-warm-rollback"
        expected = (1..2).map do |ordinal|
          { "ordinal" => ordinal, "state" => scenario == "imdb-upsert" && variant == "reference" ? "42P10" : "00000",
            "callbacks" => wrapped ? 8 : 1, "ingestion_setting" => variant == "reference" ? 2 : 0,
            "wrapper_setting" => wrapped ? 0 : nil, "scope_counts" => wrapped ? { "ingestion" => 7, "wrapper" => 1 } : { "ingestion" => 1 } }
        end
        assert(validate_remaining(fixture) == expected, "remaining summary #{scenario}/#{variant}")
        assert(fixture == original, "remaining input unchanged")
      end
      mutations = {
        "unknown remaining-FK context" => ->(f) { f[:input][:variant] = "unknown" },
        "remaining-FK frame inventory changed" => ->(f) { f[:input][:frames][0]["state"] = "42P10" },
        "remaining-FK transaction lifetime changed" => ->(f) { f[:input][:frames][1]["outside"] = "false" },
        "native ordering or kind changed" => ->(f) { f[:input][:events][0]["native_line"] = "1" },
        "remaining-FK snapshot phases changed" => ->(f) { f[:input][:events][0]["compiler_setting"] = 2 },
        "independent operation ordering or state changed" => ->(f) { f[:input][:operations][0]["ordinal"] = 2 },
        "remaining-FK root path changed" => ->(f) { f[:input][:events][1]["schema"] = "other" },
        "authored helper compiler scope changed" => ->(f) { f[:input][:events][1]["compiler_setting"] = 2 },
        "unresolved independent callback row" => ->(f) { f[:input][:operations][0]["rows"][0]["count"] = -1 },
        "ambiguous independent callback ownership" => ->(f) { f[:input][:operations][0]["rows"] << copy(f[:input][:operations][0]["rows"][0]) },
        "remaining-FK multiset differs" => ->(f) { f[:input][:events][5]["constraint"] = "other_fk" },
        "native callback binding changed" => ->(f) { f[:input][:events][5]["internal"] = false },
        "native callback compiler scope changed" => ->(f) { f[:input][:events][5]["compiler_setting"] = 0 },
        "helper entered after wrapper-owned callbacks" => ->(f) { e = f[:input][:events]; e.insert(6, e.delete_at(14)); number!(e) },
        "ingestion callback entered after wrapper phase" => ->(f) { e = f[:input][:events]; e[13], e[14] = e[14], e[13] },
        "logger callback ordering or ownership changed" => ->(f) { e = f[:input][:events]; e[5], e[7] = e[7], e[5]; number!(e) },
        "native activity escaped recorded operations" => ->(f) { f[:input][:events] << call("normalize_title_v1"); number!(f[:input][:events]) }
      }
      mutations.each do |label, mutate|
        fixture = remaining_fixture
        mutate.call(fixture)
        number!(fixture[:input][:events]) if label == "ingestion callback entered after wrapper phase"
        rejected(label) { validate_remaining(fixture) }
      end
      fixture = remaining_fixture("imdb-upsert", "final")
      fixture[:input][:operations][0]["rows"][0]["scope"] = "wrapper"
      rejected("unexpected wrapper-owned callback") { validate_remaining(fixture) }
      fixture = remaining_fixture("imdb-upsert", "reference")
      fixture[:input][:frames][0]["within"] = "true"
      rejected("remaining-FK transaction lifetime changed") { validate_remaining(fixture) }
      fixture = remaining_fixture
      fixture[:input][:operations][0]["rows"] << { "count" => 0, "scope" => "ingestion" }
      assert(validate_remaining(fixture).length == 2, "resolved zero callback rows remain valid")
      fixture = remaining_fixture
      fixture[:input][:operations][0]["rows"][1]["count"] = 3
      fixture[:input][:events].insert(8, copy(fixture[:input][:events][7]))
      number!(fixture[:input][:events])
      rejected("independent logger counts are not separable") { validate_remaining(fixture) }
      fixture = remaining_fixture("imdb-upsert", "final")
      row = fixture[:input][:operations][0]["rows"][0]
      row["table"] = LOGGER_TABLES.first
      row["constraint"] = "#{LOGGER_TABLES.first}_fk"
      fixture[:input][:events][4] = callback(LOGGER_TABLES.first, 0)
      number!(fixture[:input][:events])
      rejected("unowned logger callback") { validate_remaining(fixture) }
      fixture = remaining_fixture
      events = fixture[:input][:events]
      events[7], events[8] = events[8], events[7]
      number!(events)
      assert(validate_remaining(fixture).first.fetch("callbacks") == 8, "remaining logger callbacks retain multiset semantics")
    end

    def oracle_test!
      %w[reference final].each do |variant|
        fixture = remaining_fixture("v2-conflict-warm-rollback", variant)
        oracle = fixture.fetch(:oracle)
        %w[sql_owner scope owner_compile_resolution v1_active ambient_variable_conflict table].each do |field|
          [0, 1, 4].each do |index|
            changed = copy(oracle)
            row = changed["cases"][0]["variants"][variant]["operations"][0]["multiset"][index]
            row[field] = field == "v1_active" ? !row.fetch(field) : "invalid"
            rejected("independent ownership or ambient scope changed") { NativeFkRemainingPhases.operations(changed, "v2-conflict-warm-rollback", variant) }
          end
        end
        { "total_callback_entries" => 99, "scope_totals" => {}, "count" => 0 }.each do |field, value|
          changed = copy(oracle)
          operation = changed["cases"][0]["variants"][variant]["operations"][0]
          (field == "count" ? operation["multiset"][0] : operation)[field] = value
          label = field == "scope_totals" ? "independent callback scope totals changed" : "independent callback total changed"
          rejected(label) { NativeFkRemainingPhases.operations(changed, "v2-conflict-warm-rollback", variant) }
        end
        changed = copy(oracle)
        changed["cases"] *= 2
        rejected("independent case missing or duplicated") { NativeFkRemainingPhases.operations(changed, "v2-conflict-warm-rollback", variant) }
        rejected("independent case missing or duplicated") { NativeFkRemainingPhases.operations({ "cases" => [] }, "imdb-upsert", variant) }
      end
    end
  end
end

RevaerDatabaseRebaseline::NativeFkTest.new.run! if $PROGRAM_NAME == __FILE__
