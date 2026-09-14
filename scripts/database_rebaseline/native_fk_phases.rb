# frozen_string_literal: true

require_relative "support"
require_relative "ingestion_proof"

module RevaerDatabaseRebaseline
  class NativeFkPhases
    def initialize(expectations)
      @expectations = expectations
    end

    def validate!(events:, frames:, scenario:, variant:)
      raise Failure, "unknown FK compilation context" unless %w[cold-logger-setting logger-first-setting].include?(scenario) && %w[reference final].include?(variant)
      raise Failure, "unknown native FK event" unless events.all? { |event| %w[call trigger].include?(event.fetch("kind")) }

      lines = events.map { |event| event.fetch("native_line") }
      raise Failure, "native event ordering changed" unless lines.all? { |line| line.is_a?(Integer) && line.positive? } && lines.each_cons(2).all? { |a, b| a < b }

      operations = @expectations.fetch("operations").select { |row| row.fetch("scenario") == scenario }
      count = scenario == "cold-logger-setting" ? 1 : 3
      raise Failure, "operation or frame inventory changed" unless frames.length == count && operations.map { |row| row.fetch("operation_ordinal") } == (1..count).to_a &&
        frames.all? { |frame| frame.fetch("state") == "00000" && frame.values_at("before", "after") == %w[error error] }

      snapshots = events.each_index.select { |index| snapshot?(events.fetch(index)) }
      raise Failure, "native snapshot boundaries changed" unless snapshots.length == count * 3 && snapshots.all? { |index| events.fetch(index).fetch("compiler_setting").zero? }

      ranges = []
      phases = operations.each_with_index.map do |operation, index|
        before, after, finish = snapshots.slice(index * 3, 3)
        raise Failure, "native transaction-frame ordering changed" unless before < after && after < finish &&
          (index + 1 == count || finish < snapshots.fetch((index + 1) * 3))

        range = (before + 1)...after
        ranges << range
        phase = events[range]
        setting = operation.fetch("compiler_setting").fetch(variant)
        raise Failure, "in-call compiler setting changed" unless phase.all? { |event| event.fetch("compiler_setting") == setting }

        ingest = operation.fetch("multisets").key?("new_canonical_existing_source")
        calls = phase.select { |event| event.fetch("kind") == "call" }
        roots = calls.select { |event| event.fetch("schema") == "public" && event.fetch("name") == "search_result_ingest_v1" }
        loggers = calls.select { |event| event.fetch("schema") == "public" && event.fetch("name") == "log_source_metadata_conflict_v1" }
        first = phase.first
        expected_first = ingest ? "search_result_ingest_v1" : "log_source_metadata_conflict_v1"
        raise Failure, "native operation root or logger inventory changed" unless first && first.values_at("kind", "schema", "name") == ["call", "public", expected_first] &&
          roots.length == (ingest ? 1 : 0) && loggers.length == (ingest ? 2 : 1)
        raise Failure, "unknown authored helper in FK phase" unless calls.all? { |event| event.fetch("schema") == "public" && IngestionProof::INGESTION_HELPERS.include?(event.fetch("name")) }

        actual = phase.select { |event| event.fetch("kind") == "trigger" }.map do |event|
          raise Failure, "native trigger binding changed" unless event.values_at("language", "internal", "enabled", "constraint_type") == ["internal", true, "O", "f"] &&
            %w[RI_FKey_check_ins RI_FKey_check_upd].include?(event.fetch("function")) &&
            %w[function_oid trigger_oid constraint_oid].all? { |field| event.fetch(field).is_a?(Integer) && event.fetch(field).positive? }

          event.values_at("function", "table", "constraint")
        end.tally
        expected = expected_counts(operation)
        raise Failure, "native callback multiset differs from independent expectation" unless actual == expected

        logger_expected = expected_counts({ "multisets" => { "logger" => 1 } })
        phase.each_with_index do |event, position|
          next unless event.values_at("kind", "schema", "name") == ["call", "public", "log_source_metadata_conflict_v1"]

          callbacks = phase.slice(position + 1, logger_expected.values.sum)
          raise Failure, "logger callback ordering or ownership changed" unless callbacks && callbacks.all? { |entry| entry.fetch("kind") == "trigger" } &&
            callbacks.map { |entry| entry.values_at("function", "table", "constraint") }.tally == logger_expected
        end

        { "ordinal" => index + 1, "compiler_setting" => setting, "callback_count" => actual.values.sum,
          "callbacks" => actual.map { |key, value| { "identity" => key, "count" => value } } }
      end
      events.each_with_index do |event, index|
        next if snapshot?(event) || ranges.any? { |range| range.cover?(index) }

        raise Failure, "native helper or callback escaped its operation"
      end
      phases
    end

    private

    def snapshot?(event)
      event.values_at("kind", "schema", "name") == ["call", "ingestion_observation", "snapshot"]
    end

    def expected_counts(operation)
      counts = Hash.new(0)
      operation.fetch("multisets").each do |name, multiplier|
        raise Failure, "invalid independent callback multiplier" unless multiplier.is_a?(Integer) && multiplier.positive?

        @expectations.fetch("multisets").fetch(name).each do |row|
          count = row.fetch("count_each")
          raise Failure, "unresolved independent FK count: #{row.fetch('constraints').join(',')}" unless count.is_a?(Integer) && count >= 0

          row.fetch("constraints").each do |constraint|
            counts[[row.fetch("function"), row.fetch("table"), constraint]] += count * multiplier if count.positive?
          end
        end
      end
      counts
    end
  end
end
