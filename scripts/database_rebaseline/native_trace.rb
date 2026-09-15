# frozen_string_literal: true

require "json"
require_relative "ingestion_metadata"

module RevaerDatabaseRebaseline
  # Bind read-only native observations to the catalog of the same live clone.
  class NativeTrace
    def initialize(query:, directory:, observations: :trust_rank, trigger_expectation: :present)
      raise Failure, "unknown native observation protocol" unless %i[trust_rank policy fk settings].include?(observations)
      unless %i[present absent].include?(trigger_expectation) && (observations == :policy || trigger_expectation == :present)
        raise Failure, "invalid native trigger expectation"
      end

      @query = query
      @directory = directory
      @observations = observations
      @trigger_expectation = trigger_expectation
    end

    def collect!(debugger)
      stdout_path = debugger.fetch(:stdout_path)
      lines = File.binread(stdout_path).lines
      stderr = File.binread(debugger.fetch(:stderr_path))
      raise Failure, "native debugger diagnostic retained" unless stderr.empty?
      forbidden = case @observations
                  when :trust_rank then %w[PARENT_RI: REGEX: CAST_EXECUTOR:]
                  when :policy then %w[PARENT_RI: K1:]
                  else %w[PARENT_RI: K1: REGEX: CAST_EXECUTOR:]
                  end
      if lines.any? { |line| line.match?(/K1_ERROR|CAST_EXECUTOR_ERROR|Python Exception|Traceback|Error in sourced|Cannot access memory|No symbol|exited with code|received signal/) || line.start_with?(*forbidden) }
        raise Failure, "native debugger failure or unexpected callback retained"
      end

      calls = parse_calls(lines)
      catalog = call_catalog(calls)
      write(File.join(@directory, "call-catalog.json"), catalog)
      mapped = calls.map do |call|
        routine = catalog.find { |row| row.fetch("oid") == call.fetch("oid") }
        raise Failure, "native call language mismatch" unless routine.fetch("language") == call.fetch("language")

        call.merge(routine)
      end
      triggers = parse_triggers(lines)
      if @trigger_expectation == :absent ? !triggers.empty? : triggers.empty?
        raise Failure, "native trigger presence differs from expected execution path"
      end
      bindings = trigger_catalog(triggers)
      write("#{stdout_path}.trigger-catalog.json", bindings)
      triggers = triggers.map do |trigger|
        binding = bindings.find { |row| row.fetch("trigger_oid") == trigger.fetch("trigger_oid") }
        unless binding.values_at("function_oid", "function", "language", "internal", "constraint_type") ==
               [trigger.fetch("function_oid"), trigger.fetch("function"), "internal", true, "f"]
          raise Failure, "native trigger/function/constraint binding mismatch"
        end
        trigger.merge(binding)
      end
      ri = lines.grep(/\ARI:/).map(&:strip)
      expected = triggers.map { |trigger| "RI:#{trigger.fetch('function')}:#{trigger.fetch('compiler_setting')}" }
      raise Failure, "native trigger records do not cover callbacks" unless ri == expected

      write("#{stdout_path}.triggers.json", triggers.map { |trigger| trigger.reject { |key, _value| %w[kind native_line].include?(key) } })
      native = (mapped + triggers).sort_by { |event| event.fetch("native_line") }
      write(File.join(@directory, "native-events.json"), native)
      events, filename = case @observations
                         when :trust_rank then [trust_rank_events(lines), "trust-rank-events.json"]
                         when :policy then [policy_events(lines, mapped), "policy-events.json"]
                         when :settings then [native, "settings-events.json"]
                         else [native, "fk-events.json"]
                         end
      write(File.join(@directory, filename), events)
      { events:, native:, catalog:, triggers:, debugger_stderr: stderr }
    rescue KeyError, TypeError, NoMethodError => error
      raise Failure, "malformed native trace: #{error.message}"
    rescue SystemCallError, IOError => error
      raise Failure, "native trace evidence unavailable: #{error.message}"
    end

    private

    def trust_rank_events(lines)
      events = lines.each_with_index.filter_map do |line, index|
        next unless line.start_with?("K1:")

        event = json(line.delete_prefix("K1:"))
        raise Failure, "native event line identity injected" unless event.is_a?(Hash) && !event.key?("native_line")

        event.merge("native_line" => index + 1)
      end
      raise Failure, "native trust-rank observations absent" if events.empty?

      events
    end

    def policy_events(lines, calls)
      extra = lines.each_with_index.filter_map do |line, index|
        if line.start_with?("REGEX:")
          match = line.match(/\AREGEX:(textregexeq|texticregexeq):([012])\n\z/)
          raise Failure, "unrecognized native regex observation" unless match

          { "kind" => "regex", "name" => match[1], "compiler_setting" => Integer(match[2]), "native_line" => index + 1 }
        elsif line.start_with?("CAST_EXECUTOR:")
          event = json(line.delete_prefix("CAST_EXECUTOR:"))
          unless event.is_a?(Hash) && event.fetch("kind") == "executor_end" && !event.key?("native_line")
            raise Failure, "native executor event identity changed"
          end

          event.merge("native_line" => index + 1)
        end
      end
      (calls + extra).sort_by { |event| event.fetch("native_line") }
    end

    def json(bytes)
      JSON.parse(bytes, object_class: IngestionMetadata::UniqueObject, allow_duplicate_key: false)
      JSON.parse(bytes, allow_duplicate_key: false)
    rescue JSON::ParserError => error
      raise Failure, "invalid native trace JSON: #{error.message}"
    end

    def write(path, data)
      File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(JSON.pretty_generate(data) + "\n") }
    end

    def parse_calls(lines)
      lines.each_with_index.filter_map do |line, index|
        next unless line.start_with?("CALL:")

        match = line.match(/\ACALL:(plpgsql|sql):([1-9][0-9]*):([012])\n\z/)
        raise Failure, "unrecognized native call observation" unless match

        { "kind" => "call", "language" => match[1], "oid" => Integer(match[2]),
          "compiler_setting" => Integer(match[3]), "native_line" => index + 1 }
      end
    end

    def parse_triggers(lines)
      lines.each_with_index.filter_map do |line, index|
        next unless line.start_with?("TRIGGER:")

        match = line.match(/\ATRIGGER:(RI_FKey_check_ins|RI_FKey_check_upd):([1-9][0-9]*):([1-9][0-9]*):([012])\n\z/)
        raise Failure, "unrecognized native trigger observation" unless match
        unless index.positive? && lines.fetch(index - 1) == "RI:#{match[1]}:#{match[4]}\n"
          raise Failure, "native trigger is not adjacent to its callback entry"
        end

        { "kind" => "trigger", "function" => match[1], "function_oid" => Integer(match[2]),
          "trigger_oid" => Integer(match[3]), "compiler_setting" => Integer(match[4]), "native_line" => index + 1 }
      end
    end

    def call_catalog(calls)
      oids = calls.map { |call| call.fetch("oid") }.uniq
      raise Failure, "native helper calls missing" if oids.empty?

      query = <<~SQL
        SELECT json_agg(json_build_object('oid', p.oid::bigint, 'schema', n.nspname, 'name', p.proname,
          'signature', p.oid::regprocedure::text, 'language', l.lanname,
          'source_sha256', encode(public.digest(p.prosrc, 'sha256'), 'hex'), 'config', p.proconfig))
        FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace JOIN pg_language l ON l.oid=p.prolang
        WHERE p.oid IN (#{oids.join(',')});
      SQL
      rows = json(@query.call(query))
      raise Failure, "native helper OID mapping incomplete" unless rows.is_a?(Array) && rows.map { |row| row.fetch("oid") }.sort == oids.sort

      rows
    end

    def trigger_catalog(triggers)
      oids = triggers.map { |trigger| trigger.fetch("trigger_oid") }.uniq
      return [] if oids.empty?

      query = <<~SQL
        SELECT json_agg(json_build_object('trigger_oid', t.oid::bigint, 'function_oid', t.tgfoid::bigint,
          'function', p.proname, 'language', l.lanname, 'trigger', t.tgname, 'table', t.tgrelid::regclass::text,
          'internal', t.tgisinternal, 'enabled', t.tgenabled, 'parent_oid', t.tgparentid::bigint,
          'constraint_oid', c.oid::bigint, 'constraint', c.conname, 'constraint_type', c.contype,
          'referenced_table', c.confrelid::regclass::text, 'trigger_definition', pg_get_triggerdef(t.oid, true),
          'constraint_definition', pg_get_constraintdef(c.oid, true)))
        FROM pg_trigger t JOIN pg_proc p ON p.oid=t.tgfoid JOIN pg_language l ON l.oid=p.prolang
        JOIN pg_constraint c ON c.oid=t.tgconstraint WHERE t.oid IN (#{oids.join(',')});
      SQL
      rows = json(@query.call(query))
      raise Failure, "native trigger OID mapping incomplete" unless rows.is_a?(Array) && rows.map { |row| row.fetch("trigger_oid") }.sort == oids.sort

      rows
    end
  end
end
