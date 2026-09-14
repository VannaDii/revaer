# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"

module RevaerDatabaseRebaseline
  class NativeSessionsTest
    class Runner < CommandRunner
      attr_reader :commands
      attr_accessor :fail_cleanup

      def initialize
        @commands = []
      end

      def capture(command, **_options)
        @commands << command
        failed = @fail_cleanup && command.take(3) == %w[docker rm -fv]
        Result.new(stdout: "", stderr: failed ? "cleanup denied" : "", success: !failed)
      end
    end

    class Processes
      attr_reader :events
      attr_accessor :bad_header

      def initialize
        @events = []
      end

      def launch(command, name)
        @events << [:launch, command, name]
        { name:, stdout_path: "#{name}.stdout", stderr_path: "#{name}.stderr" }
      end

      def handshake(_process, bytes, pattern)
        @events << [:handshake, bytes, pattern.match?("native_backend:42\n")]
        "native_backend:42\n"
      end

      def ready(_process, pattern)
        @events << [:ready, pattern.match?("READY:0\n"), pattern.match?("READY:2\n")]
      end

      def finish(process, stdin_data: nil)
        @events << [:finish, process.fetch(:name), stdin_data]
        { stdout: "#{@bad_header ? 'wrong' : 'native_backend:42'}\napplication\n", stderr: "original diagnostic\n" }
      end

      def close_after_owner_cleanup!
        @events << [:closed]
      end
    end

    def run!
      @assertions = 0
      session_test!
      framing_test!
      cleanup_test!
      dispatch_test!
      trace_test!
      puts "database-native-sessions-test: #{@assertions} assertions passed"
    end

    private

    def assert(condition, label)
      raise Failure, "native session test failed: #{label}" unless condition

      @assertions += 1
    end

    def rejected(label)
      yield
    rescue Failure => error
      assert(error.message.include?(label), "expected #{label}, got #{error.message}")
    else
      raise Failure, "native session test accepted #{label}"
    end

    def with_sessions
      Dir.mktmpdir("revaer-native-session-unit-") do |directory|
        runner = Runner.new
        processes = Processes.new
        tooling = NativeTooling::Prepared.new(debugger_image: "sha256:#{'a' * 64}", musl_source_directory: directory, receipt_path: "unused")
        sessions = NativeSessions.new(runner:, processes:, directory:, container: "revaer-final-proof-1-test", tooling:, probe: "qualified-probe\n")
        yield sessions, runner, processes, directory
      end
    end

    def session_test!
      with_sessions do |sessions, runner, processes, directory|
        result = sessions.execute(command: %w[psql fixture], query: "SELECT 1;", name: "observed", observed: true)
        assert(result.application == CommandRunner::Result.new(stdout: "application\n", stderr: "original diagnostic\n", success: true), "unaltered application payload")
        assert(result.debugger == { stdout_path: "observed-debugger.stdout", stderr_path: "observed-debugger.stderr" }, "debugger paths retained")
        script = File.binread(File.join(directory, "observed-debugger.gdb"))
        assert(script.include?("set may-call-functions off\n"), "target functions cannot be invoked")
        assert(script.include?("directory /observer-source\nattach 42\n"), "qualified source precedes exact backend attach")
        assert(script.include?("file /proc/42/root/usr/local/bin/postgres\n"), "target filesystem used")
        assert(script.scan("qualified-probe").length == 1, "probe inserted once")
        assert(script.index("qualified-probe") < script.index('printf "READY:'), "probe installed before readiness")
        (NativeSessions::CHILD_CALLBACKS + NativeSessions::PARENT_CALLBACKS + %w[plpgsql_call_handler fmgr_sql]).each do |symbol|
          assert(script.scan("break *#{symbol}\n").length == 1, "exact breakpoint #{symbol}")
        end
        create = runner.commands.find { |command| command.take(2) == %w[docker create] }
        assert(create.include?("container:revaer-final-proof-1-test") && create.include?("SYS_PTRACE"), "owned PID namespace attachment")
        assert(create.include?("type=bind,source=#{directory},target=/observer-source,readonly"), "source mount read-only")
        assert(create[create.index("--network") + 1] == "none", "debugger has no network")
        assert(processes.events.include?([:ready, true, false]), "nondefault compiler readiness rejected")
        assert(processes.events.include?([:finish, "observed", "SELECT 1;\n\\q\n"]), "bounded transport owns full input")
        assert((File.stat(File.join(directory, "observed-debugger.gdb")).mode & 0o777) == 0o600, "private script")
        sessions.remove_containers!
        sessions.close_after_owner_cleanup!
        assert(runner.commands.last.take(3) == %w[docker rm -fv] && processes.events.last == [:closed], "containers before clients")
      end
    end

    def framing_test!
      with_sessions do |sessions, runner, processes, directory|
        result = sessions.execute(command: ["psql"], query: "SELECT 1;", name: "plain", observed: false)
        assert(result.debugger.nil? && runner.commands.empty?, "plain arm never creates debugger")
        sessions.execute(command: ["psql"], query: "", name: "selected", observed: true, probe: "selected-probe\n")
        script = File.binread(File.join(directory, "selected-debugger.gdb"))
        assert(script.include?("selected-probe") && !script.include?("qualified-probe"), "explicit observer selection")
        rejected("probe is empty") { sessions.execute(command: ["psql"], query: "", name: "empty", observed: true, probe: "") }
        processes.bad_header = true
        rejected("handshake framing") { sessions.execute(command: ["psql"], query: "", name: "changed", observed: false) }
      end
    end

    def cleanup_test!
      with_sessions do |sessions, runner, _processes, _directory|
        2.times { |index| sessions.execute(command: ["psql"], query: "", name: "case-#{index}", observed: true) }
        runner.fail_cleanup = true
        rejected("debugger cleanup failed") { sessions.remove_containers! }
        assert(runner.commands.count { |command| command.take(3) == %w[docker rm -fv] } == 2, "all owned containers attempted")
      end
      proof = FinalProof.new(nil)
      events = []
      sessions = Object.new
      sessions.define_singleton_method(:remove_containers!) { events << :debuggers; raise Failure, "debugger failure" }
      sessions.define_singleton_method(:close_after_owner_cleanup!) { events << :clients }
      proof.instance_variable_set(:@native_sessions, sessions)
      rejected("proof cleanup failed") { proof.send(:cleanup_native_resources!) { events << :owner } }
      assert(events == %i[debuggers owner clients], "cleanup errors cannot skip owner or client cleanup")
      assert(proof.instance_variable_get(:@failures) == ["disposable native/container cleanup failed"], "cleanup prevents acceptance")
    end

    def dispatch_test!
      runner = Runner.new
      proof = FinalProof.new(nil, runner:)
      proof.send(:result, "SELECT 1;", role: "runtime", database: "test")
      assert(runner.commands.length == 1, "ordinary query uses ordinary runner")
      proof.instance_variable_set(:@native_target, { database: "test", role: "runtime", name: "unit", observed: false })
      calls = []
      sessions = Object.new
      expected = CommandRunner::Result.new(stdout: "native", stderr: "", success: true)
      sessions.define_singleton_method(:execute) { |**options| calls << options; NativeSessions::Capture.new(application: expected, debugger: nil) }
      proof.instance_variable_set(:@native_sessions, sessions)
      assert(proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "test") == expected, "real class method delegates target query")
      assert(calls.length == 1 && runner.commands.length == 1, "native query executes exactly once")
      rejected("target changed") { proof.send(:result, "SAVEPOINT operation;", role: "postgres", database: "test") }
    end

    def trace_test!
      call = { "oid" => 10, "schema" => "public", "name" => "search_result_ingest_v1", "signature" => "search_result_ingest_v1(uuid)",
               "language" => "plpgsql", "source_sha256" => "a" * 64, "config" => nil }
      trigger = { "trigger_oid" => 30, "function_oid" => 20, "function" => "RI_FKey_check_ins", "language" => "internal", "internal" => true, "constraint_type" => "f" }
      query = ->(sql) { JSON.generate(sql.include?("FROM pg_proc") ? [call] : [trigger]) }
      trace = "READY:0\nCALL:plpgsql:10:2\nRI:RI_FKey_check_ins:2\nTRIGGER:RI_FKey_check_ins:20:30:2\nK1:{\"kind\":\"boolean\"}\n[Inferior 1 (process 42) exited normally]\n"
      Dir.mktmpdir("revaer-native-trace-unit-") do |root|
        { "valid" => [trace, ""], "diagnostic" => [trace, "warning\n"], "parent" => [trace + "PARENT_RI:unexpected\n", ""],
          "duplicate" => [trace.sub('{"kind":"boolean"}', '{"kind":"boolean","kind":"assignment"}'), ""],
          "line" => [trace.sub('{"kind":"boolean"}', '{"kind":"boolean","native_line":5}'), ""] }.each do |name, (stdout, stderr)|
          directory = File.join(root, name)
          Dir.mkdir(directory, 0o700)
          paths = { stdout_path: File.join(directory, "debugger.stdout"), stderr_path: File.join(directory, "debugger.stderr") }
          File.write(paths.fetch(:stdout_path), stdout)
          File.write(paths.fetch(:stderr_path), stderr)
          collector = NativeTrace.new(query:, directory:)
          if name == "valid"
            result = collector.collect!(paths)
            assert(result.fetch(:events) == [{ "kind" => "boolean", "native_line" => 5 }], "raw K1 line binding")
            assert(result.fetch(:native).map { |event| event.values_at("kind", "native_line") } == [["call", 2], ["trigger", 4]], "call/trigger order retained")
            rejected("File exists") { collector.collect!(paths) }
          else
            label = { "diagnostic" => "diagnostic", "parent" => "unexpected callback", "duplicate" => "duplicate", "line" => "line identity" }.fetch(name)
            rejected(label) { collector.collect!(paths) }
          end
        end
      end
    end
  end
end

RevaerDatabaseRebaseline::NativeSessionsTest.new.run! if $PROGRAM_NAME == __FILE__
