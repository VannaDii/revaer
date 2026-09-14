# frozen_string_literal: true

require_relative "support"
require_relative "ingestion_proof"

module RevaerDatabaseRebaseline
  class NativeFkRemainingPhases
    LOGGER_TABLES = %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].freeze

    def self.operations(oracle, scenario, variant)
      names = { "imdb-upsert" => "imdb-upsert", "v2-conflict-warm-rollback" => "existing-v2-hash-conflict-warm-rollback" }
      entries = oracle.fetch("cases").select { |entry| entry.fetch("name") == names.fetch(scenario) }
      raise Failure, "independent case missing or duplicated" unless entries.length == 1 && %w[reference final].include?(variant)

      entries.first.fetch("variants").fetch(variant).fetch("operations").map do |operation|
        rows = operation.fetch("multiset").map do |entry|
          owners = { "v1_direct" => "public.search_result_ingest_v1", "v1_nested_logger" => "public.log_source_metadata_conflict_v1", "wrapper_post_v1" => "public.search_result_ingest" }
          scope = entry.fetch("scope")
          wrapper = scope == "wrapper_post_v1"
          ambient = wrapper || variant == "final" ? "error" : "use_column"
          table = entry.fetch("table").delete_prefix("public.")
          required_scope = if table == "canonical_torrent_best_source_context"
                             "wrapper_post_v1"
                           elsif LOGGER_TABLES.include?(table)
                             "v1_nested_logger"
                           else
                             "v1_direct"
                           end
          compile_mode = scope == "v1_direct" || (scope == "v1_nested_logger" && variant == "reference") ? "use_column" : "error"
          raise Failure, "independent ownership or ambient scope changed" unless owners.key?(scope) && entry.fetch("sql_owner") == owners.fetch(scope) &&
            scope == required_scope && entry.fetch("owner_compile_resolution") == compile_mode &&
            entry.fetch("v1_active") == !wrapper && entry.fetch("ambient_variable_conflict") == ambient && entry.fetch("table").match?(/\Apublic\.[a-z_]+\z/)

          entry.slice("function", "constraint", "count").merge("table" => entry.fetch("table").delete_prefix("public."), "scope" => wrapper ? "wrapper" : "ingestion")
        end
        raise Failure, "independent callback total changed" unless rows.all? { |row| row.fetch("count").is_a?(Integer) && row.fetch("count").positive? } &&
          rows.sum { |row| row.fetch("count") } == operation.fetch("total_callback_entries")
        totals = %w[wrapper_pre_v1 v1_direct v1_nested_logger wrapper_post_v1].to_h { |scope| [scope, 0] }
        operation.fetch("multiset").each { |row| totals[row.fetch("scope")] += row.fetch("count") }
        raise Failure, "independent callback scope totals changed" unless operation.fetch("scope_totals") == totals

        { "ordinal" => operation.fetch("ordinal"), "state" => operation.fetch("expected_sqlstate"), "rows" => rows }
      end
    end

    def validate!(events:, frames:, scenario:, variant:, operations:)
      wrapped = scenario == "v2-conflict-warm-rollback"
      raise Failure, "unknown remaining-FK context" unless %w[imdb-upsert v2-conflict-warm-rollback].include?(scenario) && %w[reference final].include?(variant)
      state = scenario == "imdb-upsert" && variant == "reference" ? "42P10" : "00000"
      raise Failure, "remaining-FK frame inventory changed" unless frames.length == 2 && operations.length == 2 &&
        frames.all? { |frame| frame.fetch("state") == state && frame.values_at("before", "after") == %w[error error] }
      raise Failure, "remaining-FK transaction lifetime changed" unless frames.each_with_index.all? do |frame, index|
        frame.values_at("within", "outside") == [(state == "00000").to_s, (wrapped && variant == "reference" && index == 1).to_s]
      end

      lines = events.map { |event| event.fetch("native_line") }
      raise Failure, "native ordering or kind changed" unless events.all? { |event| %w[call trigger].include?(event.fetch("kind")) } &&
        lines.all? { |line| line.is_a?(Integer) && line.positive? } && lines.each_cons(2).all? { |left, right| left < right }

      snapshots = events.each_index.select { |index| snapshot?(events.fetch(index)) }
      raise Failure, "remaining-FK snapshot phases changed" unless snapshots.length == 6 && snapshots.all? { |index| events.fetch(index).fetch("compiler_setting") == 0 }

      consumed = snapshots.dup
      result = operations.each_with_index.map do |operation, ordinal|
        before, after, finish = snapshots.slice(ordinal * 3, 3)
        raise Failure, "remaining-FK transaction ordering changed" unless before < after && after < finish
        raise Failure, "independent operation ordering or state changed" unless operation.fetch("ordinal") == ordinal + 1 && operation.fetch("state") == state

        indices = ((before + 1)...after).to_a
        consumed.concat(indices)
        phase = events[(before + 1)...after]
        calls = phase.select { |event| event.fetch("kind") == "call" }
        names = calls.map { |event| event.fetch("name") }
        roots = wrapped ? %w[search_result_ingest search_result_ingest_v1] : %w[search_result_ingest_v1]
        expected_helpers = roots + %w[derive_magnet_hash_v1 normalize_title_v1] + (wrapped ? ["log_source_metadata_conflict_v1"] * 2 : [])
        raise Failure, "remaining-FK root path changed" unless phase.take(roots.length).map { |event| event.values_at("kind", "schema", "name") } == roots.map { |name| ["call", "public", name] } && roots.all? { |name| names.count(name) == 1 } &&
          names == expected_helpers && calls.all? { |event| event.fetch("schema") == "public" && IngestionProof::INGESTION_HELPERS.include?(event.fetch("name")) }

        setting = variant == "reference" ? 2 : 0
        raise Failure, "authored helper compiler scope changed" unless calls.all? do |event|
          event.fetch("compiler_setting") == (event.fetch("name") == "search_result_ingest" ? 0 : setting)
        end

        expected = {}
        operation.fetch("rows").each do |row|
          count = row.fetch("count")
          raise Failure, "unresolved independent callback row" unless count.is_a?(Integer) && count >= 0 && %w[ingestion wrapper].include?(row.fetch("scope"))
          next if count.zero?

          identity = row.values_at("function", "table", "constraint")
          raise Failure, "ambiguous independent callback ownership" if expected.key?(identity)
          raise Failure, "unexpected wrapper-owned callback" if row.fetch("scope") == "wrapper" && !wrapped

          expected[identity] = row
        end
        callbacks = phase.select { |event| event.fetch("kind") == "trigger" }
        actual = callbacks.map { |event| event.values_at("function", "table", "constraint") }.tally
        raise Failure, "remaining-FK multiset differs" unless actual == expected.transform_values { |row| row.fetch("count") }

        wrapper_started = false
        phase.each do |event|
          if event.fetch("kind") == "call"
            raise Failure, "helper entered after wrapper-owned callbacks" if wrapper_started
            next
          end
          raise Failure, "native callback binding changed" unless event.values_at("language", "internal", "enabled", "constraint_type") == ["internal", true, "O", "f"] &&
            %w[function_oid trigger_oid constraint_oid].all? { |field| event.fetch(field).is_a?(Integer) && event.fetch(field).positive? }

          scope = expected.fetch(event.values_at("function", "table", "constraint")).fetch("scope")
          raise Failure, "ingestion callback entered after wrapper phase" if wrapper_started && scope == "ingestion"

          wrapper_started ||= scope == "wrapper"
          raise Failure, "native callback compiler scope changed" unless event.fetch("compiler_setting") == (scope == "wrapper" ? 0 : setting)
        end
        validate_loggers!(phase, expected, wrapped ? 2 : 0)
        { "ordinal" => ordinal + 1, "state" => state, "callbacks" => callbacks.length,
          "ingestion_setting" => setting, "wrapper_setting" => wrapped ? 0 : nil,
          "scope_counts" => expected.values.group_by { |row| row.fetch("scope") }.transform_values { |rows| rows.sum { |row| row.fetch("count") } } }
      end
      raise Failure, "native activity escaped recorded operations" unless consumed.sort == (0...events.length).to_a

      result
    end

    private

    def snapshot?(event)
      event.values_at("kind", "schema", "name") == ["call", "ingestion_observation", "snapshot"]
    end

    def validate_loggers!(phase, expected, count)
      positions = phase.each_index.select { |index| phase.fetch(index).values_at("kind", "name") == ["call", "log_source_metadata_conflict_v1"] }
      raise Failure, "remaining-FK logger inventory changed" unless positions.length == count

      rows = expected.select { |identity, _row| LOGGER_TABLES.include?(identity.fetch(1)) }
      if count.zero?
        raise Failure, "unowned logger callback" unless rows.empty?
        return
      end
      raise Failure, "independent logger counts are not separable" unless rows.values.all? { |row| row.fetch("scope") == "ingestion" && (row.fetch("count") % count).zero? }

      per_call = rows.transform_values { |row| row.fetch("count") / count }
      positions.each do |position|
        entries = phase.slice(position + 1, per_call.values.sum)
        raise Failure, "logger callback ordering or ownership changed" unless entries && entries.all? { |event| event.fetch("kind") == "trigger" } &&
          entries.map { |event| event.values_at("function", "table", "constraint") }.tally == per_call
      end
    end
  end
end
