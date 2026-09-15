# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"

module RevaerDatabaseRebaseline
  class NativeSettingCaptureTest
    def run!
      @assertions = 0
      dispatch!
      protocol!
      cleanup!
      puts "database-native-setting-capture-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, label)
      raise Failure, "native settings capture: #{label}" unless value

      @assertions += 1
    end

    def rejected(label)
      yield
    rescue Failure
      @assertions += 1
    else
      raise Failure, "native settings capture accepted #{label}"
    end

    def dispatch!
      expected = CommandRunner::Result.new(stdout: "application", stderr: "", success: true)
      sessions = Object.new
      calls = []
      sessions.define_singleton_method(:execute) do |**options|
        calls << options
        NativeSessions::Capture.new(application: expected, debugger: nil)
      end
      proof = FinalProof.new(nil)
      context = { name: "owned-settings", database: "owned", role: "runtime", observed: false, calls: 0 }
      proof.instance_variable_set(:@native_setting_sessions, sessions)
      proof.instance_variable_set(:@native_setting_context, context)
      rejected("retargeted database") { proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "other") }
      rejected("retargeted role") { proof.send(:result, "SAVEPOINT operation;", role: "postgres", database: "owned") }
      assert(calls.empty? && context.fetch(:calls).zero?, "retargeting fails before execution")
      assert(proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "owned") == expected, "canonical transport dispatch")
      assert(calls.length == 1 && !calls.first.fetch(:observed), "plain arm has no debugger")
      rejected("second application capture") { proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "owned") }
    end

    def protocol!
      call = { "oid" => 10, "schema" => "public", "name" => "search_result_ingest_v1", "language" => "plpgsql" }
      trigger = { "trigger_oid" => 30, "function_oid" => 20, "function" => "RI_FKey_check_ins", "language" => "internal", "internal" => true, "constraint_type" => "f" }
      query = ->(statement) { JSON.generate(statement.include?("FROM pg_proc") ? [call] : [trigger]) }
      valid = "READY:0\nCALL:plpgsql:10:2\nRI:RI_FKey_check_ins:2\nTRIGGER:RI_FKey_check_ins:20:30:2\n"
      Dir.mktmpdir("revaer-native-settings-capture-") do |root|
        cases = [[valid, "", :present, true], [valid, "warning\n", :present, false], ["CALL:plpgsql:10:2\n", "", :present, false]]
        %w[PARENT_RI: K1: REGEX: CAST_EXECUTOR: CAST_EXECUTOR_ERROR:].each { |marker| cases << [valid + "#{marker}unexpected\n", "", :present, false] }
        cases.each_with_index do |(stdout, stderr, expectation, passes), index|
          directory = File.join(root, index.to_s)
          Dir.mkdir(directory, 0o700)
          paths = { stdout_path: File.join(directory, "raw.stdout"), stderr_path: File.join(directory, "raw.stderr") }
          File.binwrite(paths.fetch(:stdout_path), stdout)
          File.binwrite(paths.fetch(:stderr_path), stderr)
          trace = NativeTrace.new(query:, directory:, observations: :settings, trigger_expectation: expectation)
          if passes
            result = trace.collect!(paths)
            assert(result.fetch(:events) == result.fetch(:native), "complete native stream retained")
            path = File.join(directory, "settings-events.json")
            assert(JSON.parse(File.binread(path)) == result.fetch(:events), "settings evidence persisted")
            assert(File.stat(path).mode & 0o777 == 0o600, "private evidence")
          else
            rejected("protocol #{index}") { trace.collect!(paths) }
          end
        end
        %i[fk trust_rank settings].each do |protocol|
          rejected("existing protocol absent triggers") { NativeTrace.new(query:, directory: root, observations: protocol, trigger_expectation: :absent) }
        end
      end
    end

    def cleanup!
      events = []
      sessions = Object.new
      sessions.define_singleton_method(:remove_containers!) { events << :debuggers }
      sessions.define_singleton_method(:close_after_owner_cleanup!) { events << :clients }
      proof = FinalProof.new(nil)
      proof.instance_variable_set(:@native_setting_sessions, sessions)
      proof.define_singleton_method(:cleanup_native_tooling!) { events << :tooling }
      Dir.mktmpdir("revaer-native-settings-cleanup-") do |directory|
        proof.instance_variable_set(:@native_setting_evidence, directory)
        proof.send(:cleanup_native_resources!) { events << :database }
        assert(events == %i[debuggers database clients tooling], "owned cleanup ordering")
        assert(JSON.parse(File.binread(File.join(directory, "cleanup.json"))) == { "passed" => true, "failures" => [] }, "independent cleanup receipt")
      end
    end
  end
end

RevaerDatabaseRebaseline::NativeSettingCaptureTest.new.run! if $PROGRAM_NAME == __FILE__
