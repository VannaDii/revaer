# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"

module RevaerDatabaseRebaseline
  class NativePolicyCaptureTest
    def run!
      @assertions = 0
      trace_test!
      empty_trigger_test!
      dispatch_test!
      settings_test!
      source_binding_test!
      cleanup_test!
      puts "database-native-policy-capture-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, label)
      raise Failure, "native policy capture test failed: #{label}" unless value

      @assertions += 1
    end

    def rejected(label)
      yield
    rescue Failure
      @assertions += 1
    else
      raise Failure, "native policy capture accepted #{label}"
    end

    def trace_test!
      call = { "oid" => 10, "schema" => "public", "name" => "policy_text_match_v1", "language" => "plpgsql" }
      trigger = { "trigger_oid" => 30, "function_oid" => 20, "function" => "RI_FKey_check_ins", "language" => "internal", "internal" => true, "constraint_type" => "f" }
      query = ->(sql) { JSON.generate(sql.include?("FROM pg_proc") ? [call] : [trigger]) }
      executor = { "kind" => "executor_end", "query" => "INSERT INTO search_filter_decision", "compiler_setting" => 2, "backend" => 42, "descriptor" => "0x1234" }
      lines = ["READY:0", "CALL:plpgsql:10:2", "REGEX:texticregexeq:2", "RI:RI_FKey_check_ins:2",
        "TRIGGER:RI_FKey_check_ins:20:30:2", "CAST_EXECUTOR:#{JSON.generate(executor)}"]
      valid = lines.join("\n") + "\n"
      variants = {
        "valid" => [valid, ""], "regex-kind" => [valid.sub("texticregexeq", "different_regex"), ""],
        "regex-setting" => [valid.sub("REGEX:texticregexeq:2", "REGEX:texticregexeq:9"), ""],
        "trust-protocol" => [valid + "K1:{}\n", ""], "parent" => [valid + "PARENT_RI:unexpected\n", ""],
        "diagnostic" => [valid, "warning\n"], "probe-error" => [valid + "CAST_EXECUTOR_ERROR:failed\n", ""],
        "duplicate" => [valid.sub('"kind":"executor_end"', '"kind":"executor_end","kind":"executor_end"'), ""],
        "injected-line" => [valid.sub('"backend":42', '"native_line":6,"backend":42'), ""],
        "wrong-kind" => [valid.sub('"kind":"executor_end"', '"kind":"call"'), ""]
      }
      Dir.mktmpdir("revaer-native-policy-capture-") do |root|
        variants.each do |name, (stdout, stderr)|
          directory = File.join(root, name)
          Dir.mkdir(directory, 0o700)
          paths = { stdout_path: File.join(directory, "raw.stdout"), stderr_path: File.join(directory, "raw.stderr") }
          File.binwrite(paths.fetch(:stdout_path), stdout)
          File.binwrite(paths.fetch(:stderr_path), stderr)
          reader = NativeTrace.new(query:, directory:, observations: :policy)
          if name == "valid"
            capture = reader.collect!(paths)
            assert(capture.fetch(:events).map { |row| row.values_at("kind", "native_line") } == [["call", 2], ["regex", 3], ["executor_end", 6]], "exact merged raw order")
            assert(capture.fetch(:events).last == executor.merge("native_line" => 6), "executor payload retained")
            assert(capture.fetch(:native).map { |row| row.fetch("kind") } == %w[call trigger], "separate callback inventory")
            assert((File.stat(File.join(directory, "policy-events.json")).mode & 0o777) == 0o600, "private evidence")
          else
            rejected(name) { reader.collect!(paths) }
          end
        end
        rejected("unknown protocol") { NativeTrace.new(query:, directory: root, observations: :unknown) }
        rejected("policy traces in trust protocol") do
          directory = File.join(root, "trust-reader")
          Dir.mkdir(directory, 0o700)
          NativeTrace.new(query:, directory:).collect!({ stdout_path: File.join(root, "valid/raw.stdout"), stderr_path: File.join(root, "valid/raw.stderr") })
        end
      end
    end

    def dispatch_test!
      proof = FinalProof.new(Contract.new(root: File.expand_path("../..", __dir__)))
      calls = []
      expected = CommandRunner::Result.new(stdout: "application", stderr: "", success: true)
      sessions = Object.new
      sessions.define_singleton_method(:execute) do |**options|
        calls << options
        NativeSessions::Capture.new(application: expected, debugger: nil)
      end
      proof.instance_variable_set(:@native_policy_sessions, sessions)
      target = { database: "owned", role: "runtime", name: "policy-test", observed: false, inlined: false, regex_error: false }
      proof.instance_variable_set(:@native_policy_target, target)
      assert(proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "owned") == expected, "canonical result dispatch")
      assert(calls.length == 1 && !calls.first.fetch(:probe).include?("CastExecutorScope") && !calls.first.fetch(:probe).include?("REGEX:"), "qualified successful-case probe selection")
      proof.instance_variable_set(:@native_policy_target, target.merge(inlined: true))
      proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "owned")
      assert(calls.last.fetch(:probe).scan("cast_executor_scope = CastExecutorScope()").length == 1, "inlined probe installed once")
      proof.instance_variable_set(:@native_policy_target, target.merge(regex_error: true))
      proof.send(:result, "SAVEPOINT operation;", role: "runtime", database: "owned")
      assert(calls.last.fetch(:probe).include?("break *textregexeq") && calls.last.fetch(:probe).include?("break *texticregexeq"), "qualified error-case regex probes retained")
      rejected("retargeted backend") { proof.send(:result, "SAVEPOINT operation;", role: "postgres", database: "owned") }
    end

    def empty_trigger_test!
      calls = [{ "oid" => 10, "language" => "plpgsql", "name" => "policy_text_match_v1", "schema" => "public" }]
      query = lambda do |sql|
        raise Failure, "empty callback path queried trigger catalog" unless sql.include?("FROM pg_proc")

        JSON.generate(calls)
      end
      Dir.mktmpdir("revaer-policy-empty-callback-") do |root|
        paths = { stdout_path: File.join(root, "raw.stdout"), stderr_path: File.join(root, "raw.stderr") }
        File.binwrite(paths.fetch(:stdout_path), "CALL:plpgsql:10:2\nREGEX:texticregexeq:2\n")
        File.binwrite(paths.fetch(:stderr_path), "")
        capture = NativeTrace.new(query:, directory: root, observations: :policy, trigger_expectation: :absent).collect!(paths)
        assert(capture.fetch(:triggers).empty? && capture.fetch(:events).length == 2, "early regex error retains exact no-callback path")
        rejected("K1 cannot use an absent callback expectation") { NativeTrace.new(query:, directory: root, trigger_expectation: :absent) }
        rejected("missing ordinary policy callbacks") do
          directory = File.join(root, "required")
          Dir.mkdir(directory)
          NativeTrace.new(query:, directory:, observations: :policy).collect!(paths)
        end
        File.binwrite(paths.fetch(:stdout_path), "CALL:plpgsql:10:2\nRI:RI_FKey_check_ins:2\nTRIGGER:RI_FKey_check_ins:20:30:2\n")
        rejected("callbacks unexpectedly reached after regex failure") do
          directory = File.join(root, "unexpected")
          Dir.mkdir(directory)
          NativeTrace.new(query:, directory:, observations: :policy, trigger_expectation: :absent).collect!(paths)
        end
      end
    end

    def settings_test!
      proof = FinalProof.new(nil)
      statements = []
      proof.define_singleton_method(:sql) { |query, **options| statements << [query, options]; "" }
      proof.send(:native_policy_observer_settings, 'owned"clone', true)
      proof.send(:native_policy_observer_settings, 'owned"clone', false)
      assert(statements.all? { |query, options| query.lines.length == 7 && options == { role: "postgres", database: "postgres" } }, "all observer settings use owned administrator connection")
      assert(statements.first.first.lines.all? { |line| line.start_with?('ALTER DATABASE "owned""clone" SET ') }, "quoted clone identity")
      assert(statements.last.first.lines.all? { |line| line.start_with?('ALTER DATABASE "owned""clone" RESET ') }, "all settings restored")
    end

    def cleanup_test!
      proof = FinalProof.new(nil)
      events = []
      %i[native_sessions native_policy_sessions].each do |name|
        sessions = Object.new
        sessions.define_singleton_method(:remove_containers!) do
          events << [name, :containers]
          raise Failure, "retained cleanup failure" if name == :native_policy_sessions
        end
        sessions.define_singleton_method(:close_after_owner_cleanup!) { events << [name, :clients] }
        proof.instance_variable_set("@#{name}", sessions)
      end
      rejected("partial cleanup") { proof.send(:cleanup_native_resources!) { events << :owner } }
      assert(events == [[:native_sessions, :containers], [:native_policy_sessions, :containers], :owner,
        [:native_sessions, :clients], [:native_policy_sessions, :clients]], "all containers precede owner and all clients are reaped")
    end

    def source_binding_test!
      proof = FinalProof.new(nil)
      proof.define_singleton_method(:sql) { |_query, **_options| "" }
      source = { "name" => "policy_text_match_v1", "signature" => "policy_text_match_v1(text)", "source" => "BEGIN RETURN true; END", "settings" => nil }
      proof.instance_variable_set(:@ingestion_inventory, { "reference_proof" => { "routines" => [source] } })
      Dir.mktmpdir("revaer-policy-source-binding-") do |directory|
        proof.instance_variable_set(:@correction_evidence, directory)
        proof.send(:correction_observer!, "owned", "reference")
        path = File.join(directory, "observer-reference.sql")
        sql = File.binread(path)
        body = sql.scan(/AS \$\$(.*?)\$\$;/m).first.first
        catalog = [
          { "name" => source.fetch("name"), "schema" => "public", "signature" => source.fetch("signature"), "language" => "plpgsql", "source_sha256" => Digest::SHA256.hexdigest(source.fetch("source")), "config" => nil },
          { "name" => "snapshot", "schema" => "ingestion_observation", "signature" => "ingestion_observation.snapshot()", "language" => "sql", "source_sha256" => Digest::SHA256.hexdigest(body), "config" => ["search_path=pg_catalog"] }
        ]
        assert(proof.send(:native_bind_catalog_sources!, catalog, "reference", directory) == catalog, "canonical helper and snapshot sources bind")
        altered = JSON.parse(JSON.generate(catalog))
        altered.first["source_sha256"] = "b" * 64
        rejected("changed helper body") { proof.send(:native_bind_catalog_sources!, altered, "reference", directory) }
        altered = JSON.parse(JSON.generate(catalog))
        altered.last["config"] = nil
        rejected("changed snapshot settings") { proof.send(:native_bind_catalog_sources!, altered, "reference", directory) }
        changed_body = body.sub("json_build_object", "jsonb_build_object")
        File.binwrite(path, sql.sub(body, changed_body))
        altered.last["source_sha256"] = Digest::SHA256.hexdigest(changed_body)
        altered.last["config"] = catalog.last.fetch("config")
        rejected("self-consistent altered snapshot source") { proof.send(:native_bind_catalog_sources!, altered, "reference", directory) }
      end
    end
  end
end

RevaerDatabaseRebaseline::NativePolicyCaptureTest.new.run! if $PROGRAM_NAME == __FILE__
