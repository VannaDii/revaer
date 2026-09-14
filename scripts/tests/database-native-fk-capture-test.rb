# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"

module RevaerDatabaseRebaseline
  class NativeFkCaptureTest
    def run!
      @assertions = 0
      trace_test!
      dispatch_test!
      fixture_test!
      cleanup_test!
      puts "database-native-fk-capture-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, label)
      raise Failure, "native FK capture test failed: #{label}" unless value

      @assertions += 1
    end

    def rejected(label)
      yield
    rescue Failure
      @assertions += 1
    else
      raise Failure, "native FK capture accepted #{label}"
    end

    def trace_test!
      call = { "oid" => 10, "schema" => "public", "name" => "search_result_ingest_v1", "language" => "plpgsql" }
      trigger = { "trigger_oid" => 30, "function_oid" => 20, "function" => "RI_FKey_check_ins", "language" => "internal", "internal" => true, "constraint_type" => "f" }
      query = ->(sql) { JSON.generate(sql.include?("FROM pg_proc") ? [call] : [trigger]) }
      valid = "READY:0\nCALL:plpgsql:10:2\nRI:RI_FKey_check_ins:2\nTRIGGER:RI_FKey_check_ins:20:30:2\n"
      variants = { "valid" => [valid, ""], "stderr" => [valid, "warning\n"],
        "missing-callback" => ["CALL:plpgsql:10:2\n", ""],
        "missing-ri" => [valid.sub("RI:RI_FKey_check_ins:2\n", ""), ""],
        "detached-ri" => [valid.sub("RI:RI_FKey_check_ins:2\n", "RI:RI_FKey_check_ins:2\ninterposed output\n"), ""],
        "wrong-function" => [valid.sub("TRIGGER:RI_FKey_check_ins:20", "TRIGGER:RI_FKey_check_ins:21"), ""],
        "unknown-call" => [valid.sub("CALL:plpgsql", "CALL:unknown"), ""],
        "unknown-trigger" => [valid.sub("TRIGGER:RI_FKey_check_ins", "TRIGGER:unknown"), ""] }
      %w[PARENT_RI: K1: REGEX: CAST_EXECUTOR: CAST_EXECUTOR_ERROR:].each { |marker| variants[marker] = [valid + "#{marker}unexpected\n", ""] }
      Dir.mktmpdir("revaer-native-fk-capture-") do |root|
        variants.each_with_index do |(name, (stdout, stderr)), index|
          directory = File.join(root, index.to_s)
          Dir.mkdir(directory, 0o700)
          paths = { stdout_path: File.join(directory, "raw.stdout"), stderr_path: File.join(directory, "raw.stderr") }
          File.binwrite(paths.fetch(:stdout_path), stdout)
          File.binwrite(paths.fetch(:stderr_path), stderr)
          reader = NativeTrace.new(query:, directory:, observations: :fk)
          if name == "valid"
            trace = reader.collect!(paths)
            assert(trace.fetch(:events) == trace.fetch(:native), "FK uses the complete bound call/callback stream")
            assert(trace.fetch(:events).map { |row| row.values_at("kind", "native_line") } == [["call", 2], ["trigger", 4]], "raw line order retained")
            assert(JSON.parse(File.binread(File.join(directory, "fk-events.json"))) == trace.fetch(:events), "exact FK evidence persisted")
            assert((File.stat(File.join(directory, "fk-events.json")).mode & 0o777) == 0o600, "private trace evidence")
          else
            rejected(name) { reader.collect!(paths) }
          end
        end
        rejected("absent FK callback expectation") { NativeTrace.new(query:, directory: root, observations: :fk, trigger_expectation: :absent) }
      end
    end

    def dispatch_test!
      expected = CommandRunner::Result.new(stdout: "application", stderr: "", success: true)
      sessions = Object.new
      calls = []
      sessions.define_singleton_method(:execute) do |**options|
        calls << options
        NativeSessions::Capture.new(application: expected, debugger: nil)
      end
      inputs = IngestionPolicy::POLICY_READ_TABLES.to_h { |name| [name, []] }
      Dir.mktmpdir("revaer-native-fk-dispatch-") do |directory|
        proof = FinalProof.new(nil)
        proof.instance_variable_set(:@native_fk_sessions, sessions)
        context = { name: "owned-case", database: "owned", role: "runtime", observed: false, directory:, calls: 0 }
        proof.instance_variable_set(:@native_fk_context, context)
        proof.define_singleton_method(:policy_read_snapshot) { |_database| inputs }
        rejected("retargeted role") { proof.send(:result, "SAVEPOINT operation;", role: "postgres", database: "owned") }
        rejected("retargeted database") { proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "other") }
        assert(calls.empty? && context.fetch(:calls).zero?, "retargeting fails before capture")
        assert(proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "owned") == expected, "canonical native FK dispatch")
        assert(calls.length == 1 && !calls.first.fetch(:observed), "plain arm remains unobserved")
        retained = JSON.parse(File.binread(File.join(directory, "read-inputs.json")))
        assert(retained == { "before" => inputs, "after" => inputs }, "complete read-input pairing retained")
        rejected("second operation transport") { proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "owned") }

        changed = File.join(directory, "changed")
        Dir.mkdir(changed)
        proof.instance_variable_set(:@native_fk_context, context.merge(directory: changed, calls: 0))
        snapshots = [inputs, inputs.merge(inputs.keys.first => ["changed"])]
        proof.define_singleton_method(:policy_read_snapshot) { |_database| snapshots.shift }
        rejected("read-input mutation") { proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "owned") }
        rejected("missing canonical case") { proof.send(:native_fk_case!, [], "name") }
        rejected("duplicate canonical case") { proof.send(:native_fk_case!, [{ name: "name" }] * 2, "name") }
        entry = { name: "name" }
        assert(proof.send(:native_fk_case!, [entry], "name") == entry, "exact canonical selector")
      end
    end

    def fixture_test!
      base = Class.new do
        attr_reader :seen

        def compilation_fixture(*)
          @seen = @native_fk_fixture
          raise Failure, "fixture failed"
        end

        def wrapper_execute(*)
          @seen = @native_fk_fixture
          :executed
        end
      end
      proof = Class.new(base) { include NativeFkProof }.new
      rejected("fixture failure") { proof.send(:compilation_fixture, :test) }
      assert(proof.seen && proof.instance_variable_get(:@native_fk_fixture).nil?, "failed compilation fixture restores capture state")
      assert(proof.send(:wrapper_execute, {}, "owned", "runtime", "case-fixture") == :executed && proof.seen, "wrapper fixture excluded")
      assert(proof.instance_variable_get(:@native_fk_fixture).nil?, "wrapper fixture restores capture state")
      proof.send(:wrapper_execute, {}, "owned", "runtime", "case")
      assert(proof.seen.nil?, "actual wrapper operation remains capturable")
    end

    def cleanup_test!
      proof = FinalProof.new(nil)
      events = []
      sessions = Object.new
      sessions.define_singleton_method(:remove_containers!) { events << :debuggers }
      sessions.define_singleton_method(:close_after_owner_cleanup!) { events << :clients }
      proof.instance_variable_set(:@native_fk_sessions, sessions)
      proof.define_singleton_method(:cleanup_native_tooling!) { events << :tooling }
      Dir.mktmpdir("revaer-native-fk-cleanup-") do |directory|
        proof.instance_variable_set(:@native_fk_evidence, directory)
        proof.send(:cleanup_native_resources!) { events << :database }
        assert(events == %i[debuggers database clients tooling], "owned cleanup order")
        assert(JSON.parse(File.binread(File.join(directory, "cleanup.json"))) == { "passed" => true, "failures" => [] }, "separate cleanup receipt")
      end
    end
  end
end

RevaerDatabaseRebaseline::NativeFkCaptureTest.new.run! if $PROGRAM_NAME == __FILE__
