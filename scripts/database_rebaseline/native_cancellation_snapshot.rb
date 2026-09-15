# frozen_string_literal: true

require_relative "support"

module RevaerDatabaseRebaseline
  # Builds and validates a read-only snapshot; transport and lock proof stay with the caller.
  class NativeCancellationSnapshot
    class Error < Failure; end

    DIAGNOSTIC = /warning|error|failed|failure|cannot|can't|could not|unable|unavailable|not available|not found|no such|no symbol|not defined|undefined|missing|syntax|not permitted|permission denied|probe|signal|traceback|exception|terminated|exited|stopped|aborted/i

    def initialize(pid:, database_oid:, compiler_setting:)
      unless [pid, database_oid].all? { |value| value.is_a?(Integer) && value.positive? }
        raise Error, "native snapshot PID and database OID must be positive integers"
      end
      unless compiler_setting.is_a?(Integer) && [0, 2].include?(compiler_setting)
        raise Error, "native snapshot compiler setting must be 0 or 2"
      end

      @pid = pid
      @database_oid = database_oid
      @compiler_setting = compiler_setting
    end

    def script(musl_source_directory:)
      unless musl_source_directory.is_a?(String) && musl_source_directory.match?(%r{\A/(?:[A-Za-z0-9_-][A-Za-z0-9_.-]*/)*[A-Za-z0-9_-][A-Za-z0-9_.-]*\z})
        raise Error, "native snapshot musl source directory must be an absolute command-safe path"
      end

      root = "/proc/#{@pid}/root"
      ["set pagination off", "set confirm off", "set auto-load off", "set debuginfod enabled off",
       "set may-call-functions off", "set may-write-memory off", "set may-write-registers off",
       "set print thread-events off", "set sysroot #{root}", "file #{root}/usr/local/bin/postgres",
       "directory #{musl_source_directory}", "attach #{@pid}",
       'printf "CANCEL_SETTING:%d:%u:%d\n", (int)MyProcPid, (unsigned int)MyDatabaseId, (int)plpgsql_variable_conflict',
       "detach", 'printf "DETACHED\n"', "quit"].join("\n") + "\n"
    end

    def validate!(stdout:, stderr:)
      unless stdout.is_a?(String) && stderr.is_a?(String) && stderr.empty? && stdout.valid_encoding?
        raise Error, "native snapshot requires valid stdout and empty stderr"
      end
      if stdout.match?(DIAGNOSTIC) || stdout.match?(/[\x00-\x08\x0b-\x1f\x7f]/)
        raise Error, "native snapshot contains diagnostics or invalid control bytes"
      end

      lines = stdout.lines
      records = lines.select { |line| line.include?("CANCEL_SETTING") }
      expected = "CANCEL_SETTING:#{@pid}:#{@database_oid}:#{@compiler_setting}\n"
      raise Error, "native snapshot receipt is missing, duplicated or mismatched" unless records == [expected]
      unless lines.select { |line| line.include?("DETACHED") } == ["DETACHED\n"] && lines.index(expected) < lines.index("DETACHED\n")
        raise Error, "native snapshot detach receipt is missing, duplicated, malformed or out of order"
      end

      pid, database_oid, compiler_setting = records.fetch(0).strip.split(":").drop(1).map { |value| Integer(value, 10) }
      { pid: pid, database_oid: database_oid, compiler_setting: compiler_setting }.freeze
    end
  end
end
