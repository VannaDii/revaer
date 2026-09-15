# frozen_string_literal: true

require_relative "../database_rebaseline/native_setting_phases"

module RevaerDatabaseRebaseline
  class NativeSettingPhasesTest
    def run!
      @assertions = 0
      IngestionSettingPaths::SETTING_PATH_NAMES.product(%w[cold helpers-first], %w[reference final]).each do |name, mode, variant|
        request = fixture(name, mode, variant)
        phases = NativeSettingPhases.new.validate!(**request)
        assert(phases.length == request.fetch(:frames).length, "all operation intervals retained")
        assert(phases.map { |phase| phase.fetch(:state) } == request.fetch(:frames).map { |frame| frame.fetch("state") }, "exact frozen/final states")
        request.fetch(:events).each_index do |index|
          rejected(request, "changed compiler scope #{index}") { |copy| copy.fetch(:events).fetch(index)["compiler_setting"] = 1 }
          rejected(request, "missing event #{index}") { |copy| copy.fetch(:events).delete_at(index) }
          rejected(request, "changed raw order #{index}") { |copy| copy.fetch(:events).fetch(index)["native_line"] = 0 }
          rejected(request, "unknown event #{index}") { |copy| copy.fetch(:events).fetch(index)["kind"] = "foreign" }
        end
        request.fetch(:frames).each_index do |index|
          %w[before after finished_setting].each do |field|
            rejected(request, "caller setting #{field}") { |copy| copy.fetch(:frames).fetch(index)[field] = "use_column" }
          end
          rejected(request, "application state") { |copy| copy.fetch(:frames).fetch(index)["state"] = "XX000" }
        end
        %i[name mode variant].each do |key|
          rejected(request, "unknown context #{key}") { |copy| copy[key] = "unknown" }
        end
        rejected(request, "unknown public helper") { |copy| copy.fetch(:events).find { |event| event["name"] == "derive_magnet_hash_v1" }["name"] = "foreign" }
        rejected(request, "escaped operation") do |copy|
          event = call("normalize_title_v1", 0)
          event["native_line"] = copy.fetch(:events).last.fetch("native_line") + 1
          copy.fetch(:events) << event
        end
        rejected(request, "malformed event") { |copy| copy.fetch(:events)[0] = nil }
        rejected(request, "malformed frames") { |copy| copy[:frames] = nil }
        rejected(request, "missing frame key") { |copy| copy.fetch(:frames).first.delete("state") }
      end
      puts "database-native-setting-phases-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, label)
      raise Failure, "native setting phases: #{label}" unless value

      @assertions += 1
    end

    def rejected(request, label)
      copy = Marshal.load(Marshal.dump(request))
      yield copy
      NativeSettingPhases.new.validate!(**copy)
    rescue Failure
      @assertions += 1
    else
      raise Failure, "native setting phases accepted #{label}"
    end

    def call(name, setting, schema = "public")
      { "kind" => "call", "schema" => schema, "name" => name, "compiler_setting" => setting }
    end

    def fixture(name, mode, variant)
      states = case name
               when "success-rollback" then ["00000", "00000", variant == "reference" ? "42P07" : "00000"]
               when "late-validation-error" then %w[P0001 P0001]
               when "policy-regex-error" then %w[2201B 2201B]
               end
      events = mode == "helpers-first" ? NativeSettingPhases::CONTROLS.map { |helper| call(helper, 0) } : []
      states.each_with_index do |_state, index|
        events << call("snapshot", 0, "ingestion_observation")
        setting = variant == "reference" ? 2 : 0
        %w[search_result_ingest_v1 derive_magnet_hash_v1 normalize_title_v1].each { |helper| events << call(helper, setting) }
        if index < 2
          events << { "kind" => "trigger", "function" => "RI_FKey_check_ins", "language" => "internal",
            "internal" => true, "enabled" => "O", "constraint_type" => "f", "compiler_setting" => setting }
        end
        events << call("policy_text_match_v1", setting) if name == "policy-regex-error"
        2.times { events << call("snapshot", 0, "ingestion_observation") }
      end
      events.each_with_index { |event, index| event["native_line"] = index + 1 }
      frames = states.map { |state| { "state" => state, "before" => "error", "after" => "error", "finished_setting" => "error" } }
      { events:, frames:, name:, mode:, variant: }
    end
  end
end

RevaerDatabaseRebaseline::NativeSettingPhasesTest.new.run! if $PROGRAM_NAME == __FILE__
