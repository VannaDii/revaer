# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionGuidTrace
    GUID_TRACE_TABLES = %w[source_metadata_conflict source_metadata_conflict_audit_log indexer_health_event].freeze

    private

    def guid_trace_observer!(database, name)
      query = <<~SQL
        CREATE FUNCTION ingestion_observation.trace_guid() RETURNS trigger
        LANGUAGE plpgsql SET search_path TO pg_catalog AS $observer$
        BEGIN
          RAISE NOTICE 'ingestion-guid-setting:%', json_build_object(
            'schema', TG_TABLE_SCHEMA, 'relation', TG_TABLE_NAME, 'operation', TG_OP,
            'backend', pg_backend_pid()::text, 'session', session_user, 'current', current_user,
            'setting', current_setting('plpgsql.variable_conflict'), 'clock', transaction_timestamp());
          RETURN NEW;
        END;
        $observer$;
        REVOKE ALL ON FUNCTION ingestion_observation.trace_guid() FROM PUBLIC;
        #{GUID_TRACE_TABLES.map { |table| "CREATE TRIGGER ingestion_guid_trace BEFORE INSERT ON public.#{identifier(table)} FOR EACH ROW EXECUTE FUNCTION ingestion_observation.trace_guid();" }.join("\n")}
      SQL
      metadata_write("#{name}-trace.sql", query)
      sql(query, role: "postgres", database:)
    end

    def guid_logger_stack(table)
      raise Failure, "GUID unknown logger relation" unless GUID_TRACE_TABLES.include?(table)

      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |row| row.fetch("name") == "log_source_metadata_conflict_v1" }
      source = routine.fetch("source")
      statements = source.scan(/INSERT INTO #{Regexp.escape(table)} \(\n.*?;/m)
      line = source.lines.index { |value| value.strip == "INSERT INTO #{table} (" }
      raise Failure, "GUID logger statement changed" unless statements.one? && line

      statement = statements.first.delete_suffix(" INTO conflict_id;").delete_suffix(";")
      "SQL statement \"#{statement}\"\nPL/pgSQL function #{routine.fetch('signature')} line #{line + 1} at SQL statement\n"
    end

    def guid_caller_stack(kind, variant)
      database = variant == "reference" ? "reference_proof" : @database
      routine = @ingestion_inventory.fetch(database).fetch("routines").find { |row| row.fetch("name") == "search_result_ingest_v1" }
      source = guid_instrument(routine.fetch("source"), kind)
      value = kind == "changed-selected-guid" ? "source_public_id::TEXT" : "existing_source_guid"
      statements = source.scan(/PERFORM log_source_metadata_conflict_v1\(\n.*?;/m).select do |statement|
        statement.include?("'source_guid',") && statement.lines.any? { |line| line.strip == "#{value}," }
      end
      raise Failure, "GUID exact logger callsite changed" unless statements.one?

      statement = statements.first
      line = source[0...source.index(statement)].count("\n") + 1
      "SQL statement \"#{statement.sub('PERFORM ', 'SELECT ').delete_suffix(';')}\"\n" \
        "PL/pgSQL function #{routine.fetch('signature')} line #{line} at PERFORM\n#{disambiguation_wrapper_stack}"
    end

    def guid_trace_records(stderr)
      stderr.split(/(?=^NOTICE:  )/).map do |record|
        match = record.match(/\ANOTICE:  00000: ingestion-guid-setting:(?<event>\{[^\n]+\})\nCONTEXT:  PL\/pgSQL function ingestion_observation.trace_guid\(\) line 3 at RAISE\n(?<stack>.+)LOCATION:  exec_stmt_raise, pl_exec.c:3897\n\z/m)
        raise Failure, "GUID unexpected diagnostic or trace framing" unless match

        { "event" => metadata_json_parse(match[:event]), "stack" => match[:stack] }
      end
    end

    def guid_trace_expected(frames, trace, warm_logger:)
      return [] unless trace.fetch(:logger)

      frames.each_with_index.flat_map do |frame, index|
        warm = warm_logger && index.zero?
        GUID_TRACE_TABLES.map do |table|
          event = { "schema" => "public", "relation" => table, "operation" => "INSERT",
            "backend" => frame.fetch("backend"), "session" => trace.fetch(:role),
            "current" => trace.fetch(:variant) == "reference" ? "postgres" : @owner,
            "setting" => trace.fetch(:variant) == "reference" && !warm ? "use_column" : "error", "clock" => frame.fetch("clock") }
          { "event" => event, "stack" => guid_logger_stack(table) + (warm ? "" : guid_caller_stack(trace.fetch(:kind), trace.fetch(:variant))) }
        end
      end
    end

    def guid_trace_validate!(stderr, frames, trace, warm_logger:)
      if trace.nil?
        raise Failure, "GUID unexpected diagnostic" unless stderr.empty?

        return
      end
      actual = guid_trace_records(stderr)
      expected = guid_trace_expected(frames, trace, warm_logger:)
      unless JSON.generate(actual) == JSON.generate(expected)
        raise Failure, "GUID in-call settings, statement stack or event order changed"
      end
    end

    def guid_warm_operation
      { operation: :logger, existing: "NULL::text", incoming: "NULL::text", observed_at: "NULL::timestamptz", type: "tracker_name" }
    end

    def guid_warm_validate!(warm, tested, before, capabilities)
      expected = before.merge(guid_expected_records(1, "", warm.fetch("clock"), type: "tracker_name", incoming: "", observed: warm.fetch("clock")))
      unless warm.fetch("backend") == tested.fetch("backend") && warm.fetch("role") == capabilities &&
             warm.values_at("before", "after", "finished_setting") == %w[error error error] &&
             warm.values_at("within", "outside") == %w[false false] && warm.fetch("result") == { "logger" => "" } &&
             size_tables_equal?(warm.fetch("tables_before"), before) && size_tables_equal?(warm.fetch("tables_after"), expected) &&
             size_tables_equal?(warm.fetch("tables_finish"), before)
        raise Failure, "GUID mutating logger-first scope or complete rollback changed"
      end
    end
  end
end
