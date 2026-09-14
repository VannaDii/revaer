# frozen_string_literal: true

require_relative "support"
require_relative "ingestion_proof"

module RevaerDatabaseRebaseline
  # Consumes independently qualified inlined-cast plan events.
  class NativePolicyCastPhases
    CASES = %w[populated-all-fields release-token-match release-token-false-persisted-true
               release-regex-error-token release-regex-error-persisted-signal].freeze
    POLICY = %w[policy_text_match_v1 policy_uuid_match_v1 policy_int_match_v1
                policy_release_group_match_v1 policy_action_to_decision_type].freeze

    def validate!(events, frames, name:, variant:, mode:)
      raise Failure, "unknown policy phase request" unless CASES.include?(name) && %w[reference final].include?(variant) && %w[cold helpers-first].include?(mode)

      error = name.start_with?("release-regex-error-")
      setting = variant == "reference" ? 2 : 0
      raise Failure, "unknown native policy event kind" unless events.all? { |event| %w[call regex inlined_cast].include?(event.fetch("kind")) }
      states = error ? ["2201B"] * 3 : ["00000", "00000", variant == "reference" ? "42P07" : "00000"]
      raise Failure, "policy phase outcomes changed" unless frames.map { |frame| frame.fetch("state") } == states

      calls = events.select { |event| helper?(event) }
      raise Failure, "unexpected mapped helper" unless calls.all? do |call|
        snapshot?(call) || (call.fetch("schema") == "public" && IngestionProof::INGESTION_HELPERS.include?(call.fetch("name")))
      end
      snapshots = events.each_index.select { |index| snapshot?(events.fetch(index)) }
      raise Failure, "policy snapshot boundaries changed" unless snapshots.length == 9 && snapshots.all? { |index| events.fetch(index).fetch("compiler_setting").zero? }

      starts = events.each_index.select { |index| ingestion?(events.fetch(index)) }
      raise Failure, "expected exactly three actual ingestion calls" unless starts.length == 3

      ranges = starts.each_with_index.map { |start, index| start...snapshots.fetch(index * 3 + 1) }
      events.each_with_index do |event, index|
        next if snapshot?(event) || ranges.any? { |range| range.cover?(index) }
        next if index < starts.first && helper?(event)
        if index < starts.first && mode == "helpers-first" && error && event.fetch("kind") == "regex"
          previous = index.positive? ? events.fetch(index - 1) : {}
          next if previous.values_at("kind", "schema", "name", "compiler_setting") == ["call", "public", "policy_text_match_v1", 0] &&
            %w[textregexeq texticregexeq].include?(event.fetch("name")) && event.fetch("compiler_setting").zero?
        end
        raise Failure, "native helper or regex event escaped its declared interval"
      end

      controls = events.take(starts.first).select { |event| helper?(event) && !snapshot?(event) }
      if mode == "helpers-first"
        raise Failure, "helper-first controls incomplete or setting changed" unless (POLICY - controls.map { |call| call.fetch("name") }).empty? &&
          controls.all? { |call| POLICY.include?(call.fetch("name")) && call.fetch("compiler_setting").zero? }
      else
        raise Failure, "cold policy session invoked a helper first" unless controls.empty?
      end

      starts.each_with_index.map do |start, index|
        before, after, finish = snapshots.slice(index * 3, 3)
        raise Failure, "snapshot/ingestion ordering changed" unless before < start && start < after && after < finish &&
          (index == 2 || finish < starts.fetch(index + 1))

        phase = events.slice(start, after - start)
        helpers = phase.select { |event| helper?(event) }
        raise Failure, "in-call helper setting changed" unless helpers.all? { |call| call.fetch("compiler_setting") == setting }
        required = %w[policy_text_match_v1 policy_release_group_match_v1]
        required << "policy_action_to_decision_type" unless error
        required += %w[policy_uuid_match_v1 policy_int_match_v1] if name == "populated-all-fields"
        if states.fetch(index) == "42P07"
          raise Failure, "frozen scratch failure reached downstream policy helpers" if helpers.any? { |call| POLICY.include?(call.fetch("name")) }
        else
          raise Failure, "required in-call helper missing" unless (required - helpers.map { |call| call.fetch("name") }).empty?
        end
        regex = phase.select { |event| event.fetch("kind") == "regex" }
        if error
          first_policy = phase.index { |event| helper?(event) && POLICY.include?(event.fetch("name")) }
          policy_regex = phase.drop(first_policy).select { |event| event.fetch("kind") == "regex" }
          tail = phase.last(2)
          raise Failure, "nested error operator order or setting changed" unless tail.length == 2 &&
            tail.first.values_at("kind", "schema", "name", "compiler_setting") == ["call", "public", "policy_text_match_v1", setting] &&
            tail.last.values_at("kind", "name", "compiler_setting") == ["regex", "texticregexeq", setting] && policy_regex.length == 1 &&
            regex.all? { |event| %w[textregexeq texticregexeq].include?(event.fetch("name")) && event.fetch("compiler_setting") == setting }
        elsif !regex.empty?
          raise Failure, "unexpected regex error observation"
        end
        { ordinal: index, state: states.fetch(index), entry: start, snapshot_after: after, helpers: }
      end
    end

    private

    def helper?(event)
      event.fetch("kind") == "call" || event.values_at("kind", "schema", "name", "language") ==
        ["inlined_cast", "public", "policy_action_to_decision_type", "sql_inlined"]
    end

    def snapshot?(event)
      event.values_at("kind", "schema", "name") == ["call", "ingestion_observation", "snapshot"]
    end

    def ingestion?(event)
      event.values_at("kind", "schema", "name") == ["call", "public", "search_result_ingest_v1"]
    end
  end
end
