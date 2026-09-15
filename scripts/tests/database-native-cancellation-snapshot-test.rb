# frozen_string_literal: true

require_relative "../database_rebaseline/native_cancellation_snapshot"

module RevaerDatabaseRebaseline
  class NativeCancellationSnapshotTest
    Snapshot = NativeCancellationSnapshot

    def run!
      count = 0
      [0, 2].each do |setting|
        snapshot = Snapshot.new(pid: 123, database_oid: 456, compiler_setting: setting)
        receipt = "CANCEL_SETTING:123:456:#{setting}\n"
        stdout = "Reading symbols from /proc/123/root/usr/local/bin/postgres...\n#{receipt}[Inferior 1 (process 123) detached]\nDETACHED\n"
        assert(snapshot.validate!(stdout: stdout, stderr: "") == { pid: 123, database_oid: 456, compiler_setting: setting }, "bound observed setting #{setting}")
        script = snapshot.script(musl_source_directory: "/observer-source")
        expected = ["set pagination off", "set confirm off", "set auto-load off", "set debuginfod enabled off",
                    "set may-call-functions off", "set may-write-memory off", "set may-write-registers off",
                    "set print thread-events off", "set sysroot /proc/123/root", "file /proc/123/root/usr/local/bin/postgres",
                    "directory /observer-source", "attach 123",
                    'printf "CANCEL_SETTING:%d:%u:%d\n", (int)MyProcPid, (unsigned int)MyDatabaseId, (int)plpgsql_variable_conflict',
                    "detach", 'printf "DETACHED\n"', "quit"].join("\n") + "\n"
        assert(script == expected, "exact read-only script")
        mutations = [stdout.delete_prefix(stdout), stdout.sub(receipt, ""), stdout + receipt,
                     stdout.sub(":123:", ":124:"), stdout.sub(":456:", ":457:"),
                     stdout.sub(receipt, "CANCEL_SETTING:123:456:#{2 - setting}\n"),
                     stdout.sub(receipt, receipt.sub("123", "0123")), stdout.sub(receipt, " #{receipt}"),
                     stdout.sub(receipt, receipt.sub("456", "bad")), stdout.sub(receipt, receipt.strip),
                     stdout.sub("DETACHED\n", ""), stdout + "DETACHED\n", stdout.sub("DETACHED", "DETACHED:1"),
                     stdout.sub("DETACHED\n", "DETACHED"), "DETACHED\n#{receipt}",
                     stdout + "\x00", stdout + "\e[31m", stdout + "CANCEL_SETTING broken\n"]
        diagnostics = ["warning: missing source", "Error in sourced command file", "Cannot access memory",
                       "No symbol in current context", "No such file or directory", "source unavailable",
                       "probe invalid", "Program received signal SIGINT", "Traceback", "Python Exception",
                       "process exited", "permission denied", "failed to attach", "Undefined command",
                       "Inferior stopped", "source not available", "missing debug symbols", "Aborted"]
        mutations.concat(diagnostics.map { |diagnostic| stdout + diagnostic + "\n" })
        mutations.each_with_index do |mutation, index|
          rejects("receipt mutation #{setting}/#{index}") { snapshot.validate!(stdout: mutation, stderr: "") }
          count += 1
        end
        ["warning\n", "\n", " ", nil, 1].each do |stderr|
          rejects("stderr #{stderr.inspect}") { snapshot.validate!(stdout: stdout, stderr: stderr) }
          count += 1
        end
        [nil, 1, "\xff".dup.force_encoding("UTF-8")].each do |invalid|
          rejects("invalid stdout") { snapshot.validate!(stdout: invalid, stderr: "") }
          count += 1
        end
      end

      [:pid, :database_oid, :compiler_setting].each do |field|
        invalid = [nil, true, false, "2", 2.0, 0.5, -1]
        invalid.concat(field == :compiler_setting ? [1, 3] : [0])
        invalid.each do |value|
          args = { pid: 123, database_oid: 456, compiler_setting: 0 }.merge(field => value)
          rejects("#{field}=#{value.inspect}") { Snapshot.new(**args) }
          count += 1
        end
      end

      snapshot = Snapshot.new(pid: 123, database_oid: 456, compiler_setting: 0)
      [nil, 1, "", "relative", "/tmp/source\ncall evil()", "/tmp/a;b", "/tmp/a b", "/tmp/../source", "/tmp/a\""].each do |path|
        rejects("source path #{path.inspect}") { snapshot.script(musl_source_directory: path) }
        count += 1
      end
      assert(snapshot.script(musl_source_directory: "/pinned/musl-1.2.5/src").include?("directory /pinned/musl-1.2.5/src\nattach 123\n"), "injected source directory")
      puts "native cancellation snapshot: success cases and #{count} rejection cases passed"
    end

    private

    def assert(condition, label)
      raise "assertion failed: #{label}" unless condition
    end

    def rejects(label)
      yield
    rescue Snapshot::Error
      return
    else
      raise "accepted invalid input: #{label}"
    end
  end
end

RevaerDatabaseRebaseline::NativeCancellationSnapshotTest.new.run! if $PROGRAM_NAME == __FILE__
