# frozen_string_literal: true

require_relative "support"
require_relative "ingestion_proof"

module RevaerDatabaseRebaseline
  # Scope readback for the existing settings cases; not callback-count closure.
  class NativeSettingPhases
    CONTROLS = %w[normalize_title_v1 normalize_magnet_uri_v1 derive_magnet_hash_v1
      compute_title_size_hash_v1 policy_text_match_v1 policy_uuid_match_v1
      policy_int_match_v1 policy_release_group_match_v1 policy_action_to_decision_type].freeze

    def validate!(events:, frames:, name:, mode:, variant:)
      unless IngestionSettingPaths::SETTING_PATH_NAMES.include?(name) && %w[cold helpers-first].include?(mode) && %w[reference final].include?(variant)
        raise Failure, "unknown native settings context"
      end
      raise Failure, "native settings evidence shape changed" unless events.is_a?(Array) && frames.is_a?(Array)

      states = case name
               when "success-rollback" then ["00000", "00000", variant == "reference" ? "42P07" : "00000"]
               when "late-validation-error" then %w[P0001 P0001]
               when "policy-regex-error" then %w[2201B 2201B]
               end
      unless frames.map { |frame| frame.fetch("state") } == states && frames.all? { |frame| frame.values_at("before", "after", "finished_setting") == %w[error error error] }
        raise Failure, "native settings application outcomes or restored settings changed"
      end
      lines = events.map { |event| event.fetch("native_line") }
      unless lines.all? { |line| line.is_a?(Integer) && line.positive? } && lines.each_cons(2).all? { |a, b| a < b }
        raise Failure, "native settings event order changed"
      end
      events.each { |event| validate_event!(event) }
      snapshots = events.each_index.select { |index| snapshot?(events.fetch(index)) }
      unless snapshots.length == states.length * 3 && snapshots.all? { |index| events.fetch(index).fetch("compiler_setting").zero? }
        raise Failure, "native settings snapshot boundaries or caller scope changed"
      end
      controls = events.take(snapshots.first)
      if mode == "helpers-first"
        unless controls.all? { |event| event.fetch("kind") == "call" && CONTROLS.include?(event.fetch("name")) && event.fetch("compiler_setting").zero? } &&
            (CONTROLS - controls.map { |event| event.fetch("name") }).empty?
          raise Failure, "native settings helper-first controls changed"
        end
      elsif !controls.empty?
        raise Failure, "cold native settings session invoked helpers first"
      end

      ranges = []
      phases = states.each_with_index.map do |state, index|
        before, after, finish = snapshots.slice(index * 3, 3)
        range = (before + 1)...after
        ranges << range
        phase = events[range]
        setting = variant == "reference" ? 2 : 0
        roots = phase.select { |event| event.values_at("kind", "schema", "name") == ["call", "public", "search_result_ingest_v1"] }
        unless roots.length == 1 && phase.first == roots.first && phase.all? { |event| event.fetch("compiler_setting") == setting }
          raise Failure, "native settings operation root or in-call scope changed"
        end
        helpers = phase.select { |event| event.fetch("kind") == "call" }.map { |event| event.fetch("name") }
        # Both first calls insert a source before the year and policy guards;
        # the first transaction rolls back, so the second inserts it again.
        if index < 2 && phase.none? { |event| event.fetch("kind") == "trigger" }
          raise Failure, "native settings pre-guard callback missing"
        end
        unless helpers.include?("derive_magnet_hash_v1") && helpers.include?("normalize_title_v1")
          raise Failure, "native settings required identity helpers missing"
        end
        if name == "policy-regex-error" && !helpers.include?("policy_text_match_v1")
          raise Failure, "native settings regex helper missing"
        end
        if state == "42P07" && helpers.any? { |helper| helper.start_with?("policy_") }
          raise Failure, "frozen settings scratch failure reached policy helpers"
        end
        { ordinal: index + 1, state:, compiler_setting: setting, before:, after:, finish:, helpers:,
          callback_count: phase.count { |event| event.fetch("kind") == "trigger" } }
      end
      events.each_with_index do |event, index|
        next if index < snapshots.first || snapshot?(event) || ranges.any? { |range| range.cover?(index) }

        raise Failure, "native settings event escaped its operation"
      end
      phases
    rescue KeyError, TypeError, NoMethodError => error
      raise Failure, "malformed native settings evidence: #{error.message}"
    end

    private

    def snapshot?(event)
      event.values_at("kind", "schema", "name") == ["call", "ingestion_observation", "snapshot"]
    end

    def validate_event!(event)
      case event.fetch("kind")
      when "call"
        unless snapshot?(event) || (event.fetch("schema") == "public" && IngestionProof::INGESTION_HELPERS.include?(event.fetch("name")))
          raise Failure, "unknown native settings helper"
        end
      when "trigger"
        unless event.values_at("language", "internal", "enabled", "constraint_type") == ["internal", true, "O", "f"] &&
            %w[RI_FKey_check_ins RI_FKey_check_upd].include?(event.fetch("function"))
          raise Failure, "unknown native settings callback"
        end
      else
        raise Failure, "unknown native settings event"
      end
      unless event.fetch("compiler_setting").is_a?(Integer) && [0, 2].include?(event.fetch("compiler_setting"))
        raise Failure, "unknown native settings compiler value"
      end
    end
  end
end
