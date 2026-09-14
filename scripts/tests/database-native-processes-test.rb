# frozen_string_literal: true

require "tmpdir"
require "rbconfig"
require_relative "../database_rebaseline/native_processes"

module RevaerDatabaseRebaseline
  class NativeProcessesTest
    def run!
      @assertions = 0
      input_tests!
      retained_stream_tests!
      backpressure_test!
      partial_ready_test!
      early_exit_test!
      failed_exit_test!
      deadline_test!
      handshake_failure_test!
      readiness_bound_test!
      cleanup_failure_test!
      ownership_test!
      puts "database-native-processes-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, message)
      raise Failure, "native process test failed: #{message}" unless value
      @assertions += 1
    end

    def rejected(message)
      yield
    rescue Failure => error
      assert(error.message.include?(message), "unexpected rejection: #{error.message}")
    else
      raise Failure, "native process test accepted #{message}"
    end

    def command(body)
      [RbConfig.ruby, "--disable-gems", "-e", "STDOUT.sync = true; STDERR.sync = true; #{body}"]
    end

    def with_group(**limits)
      Dir.mktmpdir("revaer-native-process-unit-") do |directory|
        group = NativeProcesses.new(directory:, **limits)
        begin
          yield group, directory
        ensure
          group.close_after_owner_cleanup!
        end
      end
    end

    def input_tests!
      Dir.mktmpdir("revaer-native-input-unit-") do |directory|
        [0, -1, Float::INFINITY, Float::NAN, nil, "30"].each do |limit|
          %i[readiness_seconds completion_seconds cleanup_seconds].each do |key|
            rejected("positive and finite") { NativeProcesses.new(directory:, **{ key => limit }) }
          end
        end
        File.chmod(0o755, directory)
        rejected("private and owned") { NativeProcesses.new(directory:) }
        File.chmod(0o700, directory)
        File.symlink(directory, File.join(directory, "alias"))
        rejected("private and owned") { NativeProcesses.new(directory: File.join(directory, "alias")) }
        rejected("unavailable") { NativeProcesses.new(directory: File.join(directory, "missing")) }
      end
      with_group do |group, directory|
        [nil, "", "../escape", "with_space", "a/b"].each do |name|
          rejected("label is invalid") { group.launch(command(""), name) }
        end
        [nil, [], [""], [nil], ["ruby\0"]].each do |args|
          rejected("command is invalid") { group.launch(args, "valid") }
        end
        assert(Dir.empty?(directory), "invalid input created no evidence")
        rejected("launch failed") { group.launch([File.join(directory, "absent-program")], "absent") }
        assert(Dir.children(directory).sort == %w[absent.stderr absent.stdout], "failed launch evidence retained")
      end
    end

    def retained_stream_tests!
      with_group do |group, directory|
        process = group.launch(command('STDIN.gets; STDOUT.write("READY:0\nprefetched\n"); STDERR.write("diagnostic\n"); STDOUT.write(STDIN.read)'), "streams")
        assert(group.handshake(process, "begin\n", /\AREADY:0\n\z/) == "READY:0\n", "exact ready frame")
        rejected("already consumed") { group.ready(process, /READY/) }
        rejected("bounded and unique") { group.handshake(process, "begin", /READY/) }
        result = group.finish(process, stdin_data: "payload\n")
        assert(result == { stdout: "READY:0\nprefetched\npayload\n", stderr: "diagnostic\n" }, "prefetched bytes and both streams retained")
        %w[stdout stderr].each do |suffix|
          path = File.join(directory, "streams.#{suffix}")
          assert((File.stat(path).mode & 0o777) == 0o600, "private evidence file")
          assert(File.binread(path) == result.fetch(suffix.to_sym), "retained output matches result")
        end
        rejected("already completed") { group.finish(process) }
        rejected("launch failed") { group.launch(command(""), "streams") }
        assert(File.binread(process.fetch(:stdout_path)) == result.fetch(:stdout), "duplicate name did not overwrite")
        group.close_after_owner_cleanup!
      end
    end

    def backpressure_test!
      with_group do |group, _directory|
        process = group.launch(command('puts "READY:0"; STDOUT.write("x" * 262144); bytes = STDIN.read; STDERR.write(bytes); puts bytes.bytesize'), "pressure")
        group.ready(process, /\AREADY:0\n\z/)
        result = group.finish(process, stdin_data: "y" * 262144)
        assert(result.fetch(:stdout) == "READY:0\n" + "x" * 262144 + "262144\n", "large output did not block input")
        assert(result.fetch(:stderr) == "y" * 262144, "large input and stderr retained")
      end
    end

    def partial_ready_test!
      with_group(readiness_seconds: 0.2) do |group, _directory|
        process = group.launch(command('print "partial"; STDIN.read'), "partial")
        started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        rejected("readiness timed out") { group.ready(process, /\AREADY:0\n\z/) }
        elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
        assert(elapsed < 2, "partial line cannot bypass deadline")
        assert(File.binread(process.fetch(:stdout_path)) == "partial", "partial readiness bytes retained")
      end
    end

    def early_exit_test!
      with_group do |group, _directory|
        process = group.launch(command('warn "early exit"'), "early")
        rejected("exited before readiness") { group.ready(process, /\AREADY:0\n\z/) }
        group.close_after_owner_cleanup!
        assert(File.binread(process.fetch(:stderr_path)) == "early exit\n", "early diagnostic retained")
      end
    end

    def failed_exit_test!
      with_group do |group, _directory|
        process = group.launch(command('puts "READY:0"; STDIN.read; warn "fixture failure"; exit 7'), "failure")
        group.ready(process, /\AREADY:0\n\z/)
        rejected("returned failure") { group.finish(process, stdin_data: "end") }
        assert(process.fetch(:waiter).value.exitstatus == 7, "actual failed exit retained")
        assert(File.binread(process.fetch(:stderr_path)) == "fixture failure\n", "failed diagnostic retained")
      end
    end

    def deadline_test!
      with_group(completion_seconds: 0.1) do |group, _directory|
        process = group.launch(command('puts "READY:0"; STDIN.read; sleep 5'), "deadline")
        group.ready(process, /\AREADY:0\n\z/)
        begin
          rejected("did not finish") { group.finish(process, stdin_data: "end") }
          assert(process.fetch(:waiter).alive?, "timeout is not misreported as termination")
          rejected("completion already started") { group.finish(process, stdin_data: "retry") }
        ensure
          Process.kill("TERM", process.fetch(:waiter).pid) if process.fetch(:waiter).alive?
          assert(process.fetch(:waiter).join(2), "owned fixture terminated before transport cleanup")
        end
      end
    end

    def handshake_failure_test!
      with_group do |group, _directory|
        process = group.launch(command('puts STDIN.gets.inspect; STDIN.read'), "handshake")
        rejected("pattern is invalid") { group.handshake(process, "invalid\n", "READY") }
        assert(group.handshake(process, "valid\n", /\A"valid\\n"\n\z/) == "\"valid\\n\"\n", "invalid pattern sent no bytes")
        group.finish(process)
      end
      with_group do |group, _directory|
        process = group.launch(command('STDIN.read'), "closed-handshake")
        process.fetch(:input).close
        rejected("handshake transport failed") { group.handshake(process, "begin\n", /READY/) }
      end
    end

    def readiness_bound_test!
      with_group do |group, _directory|
        process = group.launch(command('STDOUT.write("x" * 1048577); STDIN.read'), "prelude")
        rejected("prelude exceeds bound") { group.ready(process, /READY/) }
        assert(File.size(process.fetch(:stdout_path)) > NativeProcesses::READY_BYTES, "oversized readiness bytes retained")
      end
    end

    def ownership_test!
      with_group do |group, _directory|
        rejected("not owned") { group.ready({}, /READY/) }
        rejected("not owned") { group.handshake({}, "", /READY/) }
        rejected("not owned") { group.finish({}) }
        process = group.launch(command('STDIN.gets; puts "READY:0"; STDIN.read'), "guards")
        rejected("not established") { group.finish(process) }
        rejected("pattern is invalid") { group.ready(process, "READY") }
        rejected("bounded and unique") { group.handshake(process, "x" * 1025, /READY/) }
        group.handshake(process, "start\n", /\AREADY:0\n\z/)
        rejected("payload must be a string") { group.finish(process, stdin_data: 1) }
        process.fetch(:input).close
        rejected("input closed") { group.finish(process, stdin_data: "payload") }
        group.finish(process)
      end
    end

    def cleanup_failure_test!
      Dir.mktmpdir("revaer-native-cleanup-unit-") do |directory|
        group = NativeProcesses.new(directory:)
        first = group.launch(command('puts "READY:0"; STDIN.read; warn "write failure"'), "failed-stream")
        second = group.launch(command('STDIN.read; puts "later client completed"'), "later-client")
        begin
          group.ready(first, /\AREADY:0\n\z/)
          first.fetch(:stderr_file).close
        ensure
          rejected("native client cleanup failed: native stream failed: error_reader:") { group.close_after_owner_cleanup! }
        end
        assert(!first.fetch(:error_reader).alive? && !second.fetch(:output_reader).alive?, "failed stream and later reader were joined")
        assert(!first.fetch(:waiter).alive? && !second.fetch(:waiter).alive?, "both owned clients were reaped")
        assert(File.binread(second.fetch(:stdout_path)) == "later client completed\n", "cleanup continued after the first client failed")
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    raise RevaerDatabaseRebaseline::Failure, "no arguments accepted" unless ARGV.empty?
    RevaerDatabaseRebaseline::NativeProcessesTest.new.run!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn error.message
    exit 1
  end
end
