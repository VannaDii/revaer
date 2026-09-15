# frozen_string_literal: true

require_relative "support"
require_relative "ingestion_proof"
require_relative "ingestion_corrections"

module RevaerDatabaseRebaseline
  # Bounded readback only: catalog/source binding and application oracles remain
  # the producer's responsibility. Observed warm counts do not certify whole D3.
  class NativeWarmHelperPhases
    # Both fixtures have no explicit hashes, policy rules or metadata conflicts.
    # The v1 identity block calls derive before title normalization; only the
    # URI path enters normalize_magnet, and only fallback computes title-size.
    HELPER_PATHS = {
      "magnet-uri" => %w[search_result_ingest_v1 derive_magnet_hash_v1 normalize_magnet_uri_v1 normalize_title_v1].freeze,
      "title-size" => %w[search_result_ingest_v1 derive_magnet_hash_v1 normalize_title_v1 compute_title_size_hash_v1].freeze
    }.freeze

    def validate!(events:, frames:, scenario:, variant:)
      unless HELPER_PATHS.key?(scenario) && %w[reference final].include?(variant)
        raise Failure, "unknown native warm helper context"
      end
      unless events.is_a?(Array) && events.all? { |event| event.is_a?(Hash) } &&
          frames.is_a?(Array) && frames.length == 2 && frames.all? { |frame| frame.is_a?(Hash) }
        raise Failure, "native warm helper evidence shape changed"
      end
      validate_frames!(frames, variant)
      events.each { |event| validate_event!(event) }
      lines = events.map { |event| event.fetch("native_line") }
      unless lines.all? { |line| positive_integer?(line) } && lines.each_cons(2).all? { |left, right| left < right }
        raise Failure, "native warm helper event order changed"
      end
      snapshots = events.each_index.select { |index| snapshot?(events.fetch(index)) }
      unless snapshots.length == 6 && snapshots.all? { |index| events.fetch(index).fetch("compiler_setting") == 0 }
        raise Failure, "native warm helper snapshot boundaries or caller scope changed"
      end
      consumed = snapshots.dup
      phases = frames.each_with_index.map do |frame, index|
        before, after, finish = snapshots.slice(index * 3, 3)
        indices = ((before + 1)...after).to_a
        consumed.concat(indices)
        phase = events[(before + 1)...after]
        setting = variant == "reference" ? 2 : 0
        helpers = phase.select { |event| event.fetch("kind") == "call" }.map { |event| event.fetch("name") }
        unless phase.first && phase.first.values_at("kind", "schema", "name") == ["call", "public", "search_result_ingest_v1"] &&
            helpers.count("search_result_ingest_v1") == 1 && phase.all? { |event| event.fetch("compiler_setting") == setting }
          raise Failure, "native warm helper operation root or compiler scope changed"
        end
        unless helpers == HELPER_PATHS.fetch(scenario)
          raise Failure, "native warm exact identity helper path changed"
        end
        callbacks = phase.select { |event| event.fetch("kind") == "trigger" }
        raise Failure, "native warm cold callbacks missing" if index.zero? && callbacks.empty?
        { ordinal: index + 1, scenario:, variant:, scope: "bounded-readback-not-whole-D3",
          state: frame.fetch("state"), compiler_setting: setting,
          before:, after:, finish:, helpers:, callback_count: callbacks.length,
          callback_count_basis: "observed-only", frame:, events: phase,
          snapshots: [before, after, finish].map { |position| events.fetch(position) } }
      end
      unless consumed.sort == (0...events.length).to_a
        raise Failure, "native warm helper event escaped its operation"
      end
      phases
    rescue KeyError, TypeError, NoMethodError => error
      raise Failure, "malformed native warm helper evidence: #{error.message}"
    end

    private

    def positive_integer?(value)
      value.is_a?(Integer) && value.positive?
    end

    def nonempty_string?(value)
      value.is_a?(String) && !value.strip.empty?
    end

    def validate_frames!(frames, variant)
      states = variant == "reference" ? %w[00000 42P07] : %w[00000 00000]
      backend = frames.first.fetch("backend")
      unless backend.is_a?(String) && backend.match?(/\A[1-9][0-9]*\z/) &&
          frames.map { |frame| frame.fetch("state") } == states
        raise Failure, "native warm backend or application outcomes changed"
      end
      frames.each do |frame|
        (IngestionCorrections::CORRECTION_RECORDS + ["finished_setting"]).each { |key| frame.fetch(key) }
        unless frame.fetch("backend") == backend && nonempty_string?(frame.fetch("clock")) &&
            frame.values_at("before", "after", "finished_setting") == %w[error error error] &&
            frame.values_at("within", "outside") == ["true", (variant == "reference").to_s]
          raise Failure, "native warm transaction provenance or settings changed"
        end
        validate_role!(frame.fetch("role"))
        success = frame.fetch("state") == "00000"
        unless frame.key?("result") == success && (!success || frame.fetch("result").is_a?(Hash))
          raise Failure, "native warm result framing changed"
        end
        %w[tables_before tables_after tables_finish].each { |key| validate_image!(frame.fetch(key)) }
        unless frame.fetch("tables_finish") == frame.fetch("tables_after") &&
            (success || frame.fetch("tables_after") == frame.fetch("tables_before"))
          raise Failure, "native warm commit or failed-call images changed"
        end
      end
      first, second = frames
      unless first.fetch("clock") != second.fetch("clock") && first.fetch("role") == second.fetch("role") &&
          first.fetch("tables_finish") == second.fetch("tables_before") && first.fetch("tables_before") != first.fetch("tables_after")
        raise Failure, "native warm committed transaction continuity changed"
      end
    end

    def validate_role!(role)
      unless role.is_a?(Hash) && %w[session current].all? { |key| nonempty_string?(role.fetch(key)) } &&
          %w[superuser create_role bypass_rls].all? { |key| [true, false].include?(role.fetch(key)) }
        raise Failure, "native warm role readback malformed"
      end
    end

    def validate_image!(image)
      unless image.is_a?(Hash) && image.keys.all? { |key| key.is_a?(String) } &&
          image.keys.sort == IngestionProof::INGESTION_TABLES.sort &&
          image.values.all? { |rows| rows.is_a?(Array) && rows.all? { |row| row.is_a?(Hash) } }
        raise Failure, "native warm complete table image malformed"
      end
    end

    def snapshot?(event)
      event.values_at("kind", "schema", "name") == ["call", "ingestion_observation", "snapshot"]
    end

    def validate_event!(event)
      unless event.fetch("compiler_setting").is_a?(Integer) && [0, 2].include?(event.fetch("compiler_setting"))
        raise Failure, "unknown native warm compiler value"
      end
      case event.fetch("kind")
      when "call"
        unless snapshot?(event) || (event.fetch("schema") == "public" && IngestionProof::INGESTION_HELPERS.include?(event.fetch("name")))
          raise Failure, "unknown native warm helper"
        end
      when "trigger"
        unless event.values_at("language", "internal", "enabled", "constraint_type", "parent_oid") == ["internal", true, "O", "f", 0] &&
            event.fetch("parent_oid").is_a?(Integer) &&
            %w[RI_FKey_check_ins RI_FKey_check_upd].include?(event.fetch("function")) &&
            %w[function_oid trigger_oid constraint_oid].all? { |key| positive_integer?(event.fetch(key)) } &&
            %w[table constraint].all? { |key| nonempty_string?(event.fetch(key)) }
          raise Failure, "unknown or unbound native warm callback"
        end
      else
        raise Failure, "unknown native warm event"
      end
    end
  end
end
