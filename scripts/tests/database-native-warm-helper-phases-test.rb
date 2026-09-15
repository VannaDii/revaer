# frozen_string_literal: true

require_relative "../database_rebaseline/native_warm_helper_phases"

module RevaerDatabaseRebaseline
  class NativeWarmHelperPhasesTest
    def run!
      @assertions = 0
      %w[magnet-uri title-size].product(%w[reference final]).each do |scenario, variant|
        request = fixture(scenario, variant)
        original = Marshal.dump(request)
        phases = validate(request)
        assert(Marshal.dump(request) == original, "validator does not mutate evidence")
        assert(phases.length == 2, "two operations retained")
        assert(phases.map { |phase| phase.fetch(:frame) } == request.fetch(:frames), "complete canonical frames retained")
        retained = phases.flat_map { |phase| [phase.fetch(:snapshots).first, *phase.fetch(:events), *phase.fetch(:snapshots).drop(1)] }
        assert(retained == request.fetch(:events), "all readback metadata retained")
        assert(phases.map { |phase| phase.fetch(:callback_count) } == [1, 0], "cold and zero warm observed counts")
        assert(phases.all? { |phase| phase.fetch(:callback_count_basis) == "observed-only" }, "no invented expected count")
        assert(phases.all? { |phase| phase.values_at(:scenario, :variant, :scope) == [scenario, variant, "bounded-readback-not-whole-D3"] }, "bounded context retained")
        assert(phases.map { |phase| phase.values_at(:before, :after, :finish) } == snapshot_indices(request).each_slice(3).to_a, "exact interval coordinates")
        test_events(request)
        test_frames(request)
        test_intervals(request)
        %i[scenario variant].each { |key| rejected(request, "unknown #{key}") { |copy| copy[key] = "unknown" } }
        [nil, {}, "bad"].each do |bad|
          %i[events frames].each { |key| rejected(request, "malformed #{key}") { |copy| copy[key] = bad } }
        end
        observed = deep_copy(request)
        position = snapshot_indices(observed).fetch(4)
        3.times { observed.fetch(:events).insert(position, callback(variant == "reference" ? 2 : 0)) }
        renumber(observed)
        assert(validate(observed).last.fetch(:callback_count) == 3, "warm count remains observed, not constrained")
        update = deep_copy(request)
        update.fetch(:events).find { |event| event.fetch("kind") == "trigger" }["function"] = "RI_FKey_check_upd"
        assert(validate(update).first.fetch(:callback_count) == 1, "bound update callback supported")
        known = deep_copy(request)
        position = snapshot_indices(known).fetch(1)
        (IngestionProof::INGESTION_HELPERS - ["search_result_ingest_v1"]).each do |helper|
          known.fetch(:events).insert(position, call(helper, variant == "reference" ? 2 : 0))
        end
        renumber(known)
        assert((IngestionProof::INGESTION_HELPERS - validate(known).first.fetch(:helpers)).empty?, "known public helpers supported")
        if variant == "reference"
          rejected(request, "frozen error entering policy") do |copy|
            copy.fetch(:events).insert(snapshot_indices(copy).fetch(4), call("policy_text_match_v1", 2))
            renumber(copy)
          end
        end
      end
      puts "database-native-warm-helper-phases-test: #{@assertions} assertions passed"
    end

    private

    def validate(request)
      NativeWarmHelperPhases.new.validate!(**request)
    end

    def assert(value, label)
      raise Failure, "native warm helper phases: #{label}" unless value

      @assertions += 1
    end

    def deep_copy(value)
      Marshal.load(Marshal.dump(value))
    end

    def rejected(request, label)
      copy = deep_copy(request)
      yield copy
      begin
        validate(copy)
      rescue Failure
        @assertions += 1
      else
        raise Failure, "native warm helper phases accepted #{label}"
      end
    end

    def test_events(request)
      request.fetch(:events).each_index do |index|
        rejected(request, "missing event #{index}") { |copy| copy.fetch(:events).delete_at(index) }
        rejected(request, "nil event #{index}") { |copy| copy.fetch(:events)[index] = nil }
        %w[kind compiler_setting native_line].each do |key|
          rejected(request, "missing event #{key}") { |copy| copy.fetch(:events).fetch(index).delete(key) }
        end
        [nil, "0", 0.0, 1, 3].each do |setting|
          rejected(request, "invalid compiler #{setting}") { |copy| copy.fetch(:events).fetch(index)["compiler_setting"] = setting }
        end
        rejected(request, "wrong valid scope") do |copy|
          event = copy.fetch(:events).fetch(index)
          event["compiler_setting"] = event.fetch("compiler_setting") == 0 ? 2 : 0
        end
        [0, -1, "1", 1.5, nil].each do |line|
          rejected(request, "invalid line") { |copy| copy.fetch(:events).fetch(index)["native_line"] = line }
        end
        rejected(request, "foreign protocol") { |copy| copy.fetch(:events).fetch(index)["kind"] = "regex" }
        next unless index.positive?

        rejected(request, "duplicate line") { |copy| copy.fetch(:events).fetch(index)["native_line"] = copy.fetch(:events).fetch(index - 1).fetch("native_line") }
        rejected(request, "reversed order") { |copy| copy.fetch(:events)[index - 1, 2] = copy.fetch(:events)[index - 1, 2].reverse }
      end
      request.fetch(:events).each_with_index do |event, index|
        next unless event.fetch("kind") == "call"

        %w[schema name].each do |key|
          rejected(request, "unknown call #{key}") { |copy| copy.fetch(:events).fetch(index)[key] = "foreign" }
        end
      end
      trigger_index = request.fetch(:events).index { |event| event.fetch("kind") == "trigger" }
      callback(0).each_key do |key|
        rejected(request, "missing callback #{key}") { |copy| copy.fetch(:events).fetch(trigger_index).delete(key) }
      end
      { "language" => "c", "internal" => false, "enabled" => "D", "constraint_type" => "p",
        "function" => "RI_FKey_noaction_upd", "parent_oid" => 1, "table" => " ", "constraint" => "",
        "function_oid" => 0, "trigger_oid" => "42", "constraint_oid" => -1 }.each do |key, value|
        rejected(request, "changed callback #{key}") { |copy| copy.fetch(:events).fetch(trigger_index)[key] = value }
      end
      rejected(request, "noninteger parent OID") { |copy| copy.fetch(:events).fetch(trigger_index)["parent_oid"] = 0.0 }
    end

    def test_frames(request)
      [[], [request.fetch(:frames).first], request.fetch(:frames) * 2, [nil, nil]].each do |frames|
        rejected(request, "frame cardinality or shape") { |copy| copy[:frames] = frames }
      end
      request.fetch(:frames).each_with_index do |frame, index|
        (IngestionCorrections::CORRECTION_RECORDS + ["finished_setting"]).each do |key|
          rejected(request, "missing frame #{key}") { |copy| copy.fetch(:frames).fetch(index).delete(key) }
        end
        %w[before after finished_setting].each do |key|
          rejected(request, "setting drift #{key}") { |copy| copy.fetch(:frames).fetch(index)[key] = "use_column" }
        end
        ["XX000", "00000", "42P07"].reject { |state| state == frame.fetch("state") }.each do |state|
          rejected(request, "outcome drift") { |copy| copy.fetch(:frames).fetch(index)["state"] = state }
        end
        [nil, 0, "0", "-1", "1x", "", " 42"].each do |backend|
          rejected(request, "invalid backend") { |copy| copy.fetch(:frames).fetch(index)["backend"] = backend }
        end
        [nil, "", " ", 1].each do |clock|
          rejected(request, "invalid clock") { |copy| copy.fetch(:frames).fetch(index)["clock"] = clock }
        end
        %w[within outside].each do |key|
          rejected(request, "transaction lifetime") { |copy| copy.fetch(:frames).fetch(index)[key] = "not-committed" }
        end
        rejected(request, "role malformed") { |copy| copy.fetch(:frames).fetch(index)["role"] = {} }
        rejected(request, "result malformed") { |copy| copy.fetch(:frames).fetch(index)["result"] = nil }
        if frame.key?("result")
          rejected(request, "missing result") { |copy| copy.fetch(:frames).fetch(index).delete("result") }
        end
        %w[tables_before tables_after tables_finish].each do |key|
          [nil, {}, [], { "foreign" => [] }].each do |image|
            rejected(request, "incomplete image") { |copy| copy.fetch(:frames).fetch(index)[key] = image }
          end
          rejected(request, "missing table") { |copy| copy.fetch(:frames).fetch(index).fetch(key).delete("canonical_torrent") }
          rejected(request, "bad table rows") { |copy| copy.fetch(:frames).fetch(index).fetch(key)["canonical_torrent"] = [nil] }
          rejected(request, "nonstring table key") { |copy| copy.fetch(:frames).fetch(index).fetch(key)[:foreign] = [] }
        end
      end
      rejected(request, "different backend") { |copy| copy.fetch(:frames).last["backend"] = "43" }
      rejected(request, "same clock") { |copy| copy.fetch(:frames).last["clock"] = copy.fetch(:frames).first.fetch("clock") }
      rejected(request, "role drift") { |copy| copy.fetch(:frames).last.fetch("role")["current"] = "other" }
      rejected(request, "first rollback masquerades as commit") { |copy| copy.fetch(:frames).first["tables_finish"] = copy.fetch(:frames).first.fetch("tables_before") }
      rejected(request, "broken continuity") { |copy| copy.fetch(:frames).last["tables_before"] = tables(99) }
      rejected(request, "warm rollback masquerades as commit") { |copy| copy.fetch(:frames).last["tables_finish"] = tables(99) }
      rejected(request, "coherent empty rollback") do |copy|
        copy.fetch(:frames).each { |frame| %w[tables_before tables_after tables_finish].each { |key| frame[key] = tables(0) } }
      end
      return unless request.fetch(:variant) == "reference"

      rejected(request, "failed operation writes") do |copy|
        %w[tables_after tables_finish].each { |key| copy.fetch(:frames).last[key] = tables(99) }
      end
    end

    def test_intervals(request)
      boundaries = snapshot_indices(request)
      [0, boundaries.fetch(1) + 1, boundaries.fetch(2) + 1, boundaries.fetch(4) + 1, request.fetch(:events).length].each do |position|
        rejected(request, "prefix suffix or escaped helper") do |copy|
          copy.fetch(:events).insert(position, call("normalize_title_v1", 0))
          renumber(copy)
        end
        rejected(request, "escaped callback") do |copy|
          copy.fetch(:events).insert(position, callback(0))
          renumber(copy)
        end
      end
      [boundaries.fetch(0) + 1, boundaries.fetch(3) + 1].each do |position|
        rejected(request, "duplicate root") do |copy|
          copy.fetch(:events).insert(position, deep_copy(copy.fetch(:events).fetch(position)))
          renumber(copy)
        end
        rejected(request, "helper precedes root") do |copy|
          copy.fetch(:events)[position, 2] = copy.fetch(:events)[position, 2].reverse
          renumber(copy)
        end
      end
      rejected(request, "extra snapshot") do |copy|
        copy.fetch(:events) << call("snapshot", 0, "ingestion_observation")
        renumber(copy)
      end
    end

    def snapshot_indices(request)
      request.fetch(:events).each_index.select { |index| request.fetch(:events).fetch(index)["schema"] == "ingestion_observation" }
    end

    def renumber(request)
      request.fetch(:events).each_with_index { |event, index| event["native_line"] = index * 2 + 1 }
    end

    def call(name, setting, schema = "public")
      { "kind" => "call", "schema" => schema, "name" => name, "compiler_setting" => setting, "source_readback" => "retained" }
    end

    def callback(setting)
      { "kind" => "trigger", "function" => "RI_FKey_check_ins", "language" => "internal",
        "internal" => true, "enabled" => "O", "constraint_type" => "f", "parent_oid" => 0,
        "function_oid" => 100, "trigger_oid" => 101, "constraint_oid" => 102,
        "table" => "canonical_torrent_source", "constraint" => "source_canonical_fk", "compiler_setting" => setting }
    end

    def tables(value)
      IngestionProof::INGESTION_TABLES.to_h { |table| [table, value.zero? ? [] : [{ "fixture_value" => value }]] }
    end

    def fixture(scenario, variant)
      states = variant == "reference" ? %w[00000 42P07] : %w[00000 00000]
      setting = variant == "reference" ? 2 : 0
      events = []
      frames = states.each_with_index.map do |state, index|
        events << call("snapshot", 0, "ingestion_observation")
        helpers = scenario == "magnet-uri" ? %w[normalize_magnet_uri_v1 derive_magnet_hash_v1] : %w[normalize_title_v1 compute_title_size_hash_v1]
        (["search_result_ingest_v1"] + helpers).each { |helper| events << call(helper, setting) }
        events << callback(setting) if index.zero?
        2.times { events << call("snapshot", 0, "ingestion_observation") }
        after = state == "00000" ? index + 1 : index
        frame = { "backend" => "42", "clock" => "2026-09-14T00:00:0#{index}+00:00",
          "role" => { "session" => "runtime", "current" => "runtime", "superuser" => false, "create_role" => false, "bypass_rls" => false },
          "state" => state, "before" => "error", "after" => "error", "finished_setting" => "error",
          "within" => "true", "outside" => (variant == "reference").to_s,
          "tables_before" => tables(index), "tables_after" => tables(after), "tables_finish" => tables(after),
          "retained_metadata" => { "note" => "not whole D3" } }
        frame["result"] = { "fixture_result" => index } if state == "00000"
        frame["diagnostic"] = { "state" => state, "message" => "retained frozen failure" } if state == "42P07"
        frame
      end
      request = { events:, frames:, scenario:, variant: }
      renumber(request)
      request
    end
  end
end

RevaerDatabaseRebaseline::NativeWarmHelperPhasesTest.new.run! if $PROGRAM_NAME == __FILE__
