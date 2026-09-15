# frozen_string_literal: true

require_relative "database-native-sessions-test"

module RevaerDatabaseRebaseline
  class NativeCancellationCaptureTest
    class Processes < NativeSessionsTest::Processes
      attr_accessor :stderr

      def ready(_process, pattern)
        @events << [:ready, pattern.match?("DETACHED\n"), pattern.match?("READY:0\n")]
      end

      def finish(process, stdin_data: nil)
        @events << [:finish, process.fetch(:name), stdin_data]
        { stdout: "CANCEL_SETTING:42:5012:2\nDETACHED\n", stderr: @stderr || "" }
      end
    end

    def run!
      @assertions = 0
      transport_test!
      clocks_test!
      observe_test!
      cleanup_test!
      puts "database-native-cancellation-capture-test: #{@assertions} assertions passed"
    end

    private

    def assert(value, label)
      raise Failure, "native cancellation test failed: #{label}" unless value

      @assertions += 1
    end

    def rejected(label)
      yield
    rescue Failure
      @assertions += 1
    else
      raise Failure, "native cancellation accepted #{label}"
    end

    def transport_test!
      Dir.mktmpdir("revaer-native-cancel-transport-") do |directory|
        runner = NativeSessionsTest::Runner.new
        processes = Processes.new
        tooling = NativeTooling::Prepared.new(debugger_image: "sha256:#{'a' * 64}", musl_source_directory: directory, receipt_path: "unused")
        sessions = NativeSessions.new(runner:, processes:, directory:, container: "revaer-final-proof-1-test", tooling:, probe: "unused snapshot probe")
        observer = NativeCancellationSnapshot.new(pid: 42, database_oid: 5012, compiler_setting: 2)
        result = sessions.snapshot(name: "owned-snapshot", observer:)
        assert(result == { pid: 42, database_oid: 5012, compiler_setting: 2 }, "exact validated snapshot")
        assert(processes.events[1] == [:ready, true, false], "detach marker, not asynchronous observer readiness")
        assert(processes.events.last == [:finish, "owned-snapshot-debugger", nil], "snapshot client completion required")
        script = File.binread(File.join(directory, "owned-snapshot-debugger.gdb"))
        assert(script == observer.script(musl_source_directory: "/observer-source"), "exact read-only snapshot script")
        create = runner.commands.find { |command| command.take(2) == %w[docker create] }
        assert(create.include?("none") && create.include?("container:revaer-final-proof-1-test"), "network-free owned PID namespace")
        assert(create.include?("type=bind,source=#{directory},target=/observer-source,readonly"), "pinned read-only source mount")
        processes.stderr = "warning: diagnostic\n"
        rejected("debugger warning") { sessions.snapshot(name: "warning-snapshot", observer:) }
        sessions.remove_containers!
        sessions.close_after_owner_cleanup!
        assert(runner.commands.count { |command| command.take(3) == %w[docker rm -fv] } == 2, "both owned observers removed")
      end
    end

    def clock_context(variant, cache_state, directory)
      stages = %w[cancelled]
      stages << "warmup" if cache_state == "warm_committed"
      stages << "recovery" unless [variant, cache_state] == %w[reference warm_committed]
      clocks = (["seed"] + stages).each_with_index.to_h { |stage, i| [stage, "2026-09-14T00:00:0#{i}+00:00"] }
      witnesses = stages.to_h do |stage|
        [stage, { revalidated: true, identity: { "pid" => 42, "database" => "owned", "role" => "runtime",
          "application" => IngestionCancellation::CANCELLATION_APPLICATION,
          "backend_start" => "2026-09-13T00:00:00+00:00", "transaction_clock" => clocks.fetch(stage) } }]
      end
      { variant:, cache_state:, directory:, clocks:, witnesses: }
    end

    def clocks_test!
      raw = { "preparation" => { "session" => { "pid" => 42, "database" => "owned", "current_role" => "runtime" } } }
      %w[reference final].product(%w[cold warm_committed]).each do |variant, cache|
        Dir.mktmpdir("revaer-native-cancel-clock-") do |directory|
          proof = FinalProof.new(nil)
          context = clock_context(variant, cache, directory)
          proof.instance_variable_set(:@native_cancellation_context, context)
          proof.send(:native_cancellation_validate_clocks!, raw)
          assert(File.size?(File.join(directory, "clock-evidence.json")), "#{variant}/#{cache} clock evidence")
          mutations = [->(c) { c[:clocks].delete("seed") }, ->(c) { c[:clocks]["other"] = c[:clocks].fetch("seed") },
            ->(c) { c[:clocks]["seed"] = c[:clocks].fetch("cancelled") }, ->(c) { c[:clocks]["seed"] = "not a clock" },
            ->(c) { c[:witnesses].delete("cancelled") }, ->(c) { c[:witnesses].fetch("cancelled")[:identity]["pid"] = 43 },
            ->(c) { c[:witnesses].fetch("cancelled")[:identity]["database"] = "other" },
            ->(c) { c[:witnesses].fetch("cancelled")[:identity]["role"] = "postgres" },
            ->(c) { c[:witnesses].fetch("cancelled")[:identity]["transaction_clock"] = c[:clocks].fetch("seed") },
            ->(c) { c[:witnesses].fetch("cancelled")[:identity]["backend_start"] = nil },
            ->(c) { c[:witnesses].fetch("cancelled")[:identity]["application"] = "other" },
            ->(c) { c[:witnesses].fetch("cancelled")[:revalidated] = false }]
          mutations.each do |mutate|
            changed = clock_context(variant, cache, directory)
            mutate.call(changed)
            proof.instance_variable_set(:@native_cancellation_context, changed)
            rejected("clock provenance drift") { proof.send(:native_cancellation_validate_clocks!, raw) }
          end
        end
      end
    end

    def observe_test!
      %w[valid prepared-pid before-role after-identity after-wait setting].each do |mutation|
        Dir.mktmpdir("revaer-native-cancel-observe-") do |directory|
          proof = FinalProof.new(nil)
          context = { variant: "reference", cache_state: "cold", name: "reference-cold-observed", directory: }
          proof.instance_variable_set(:@native_cancellation_context, context)
          prepared = { "session" => { "pid" => mutation == "prepared-pid" ? 43 : 42 } }
          File.binwrite(File.join(directory, "reference-cold-frames.json.prepared.json"), JSON.generate(prepared))
          locker = { "pid" => 23, "database" => "owned", "role" => "postgres" }
          blocked = { "pid" => 42, "database" => "owned", "role" => "postgres", "application" => IngestionCancellation::CANCELLATION_APPLICATION,
            "state" => "active", "wait_type" => "Lock", "wait_event" => "relation", "blockers" => [23],
            "source_insert_wait" => true, "canonical_write_lock" => true, "query" => "FROM search_result_ingest(" }
          identity = { "pid" => 42, "database_oid" => 5012, "database" => "owned", "role" => "postgres",
            "application" => IngestionCancellation::CANCELLATION_APPLICATION, "backend_start" => "2026-09-14T00:00:00+00:00" }
          before = mutation == "before-role" ? identity.merge("role" => "other") : identity
          after = mutation == "after-identity" ? identity.merge("backend_start" => "changed") : identity
          identities = [before, after]
          proof.define_singleton_method(:sql) do |query, **_options|
            value = if query.include?("JOIN pg_database")
                      identities.shift
                    else
                      mutation == "after-wait" ? blocked.merge("pid" => 43) : blocked
                    end
            JSON.generate(value)
          end
          sessions = Object.new
          sessions.define_singleton_method(:snapshot) do |name:, observer:|
            raise Failure, "wrong snapshot name" unless name == "reference-cold-observed"

            observer.validate!(stdout: "CANCEL_SETTING:42:5012:#{mutation == 'setting' ? 0 : 2}\nDETACHED\n", stderr: "")
          end
          proof.instance_variable_set(:@native_cancellation_sessions, sessions)
          if mutation == "valid"
            proof.send(:native_cancellation_observe!, blocked, locker, "owned", "postgres")
            assert(context.fetch(:snapshot).fetch(:detached_and_owned_wait_revalidated), "same owned wait survives read-only detach")
          else
            rejected(mutation) { proof.send(:native_cancellation_observe!, blocked, locker, "owned", "postgres") }
            assert(!context.key?(:snapshot), "failed observation cannot produce acceptance")
          end
        end
      end
    end

    def cleanup_test!
      proof = FinalProof.new(nil)
      events = []
      sessions = Object.new
      sessions.define_singleton_method(:remove_containers!) { events << :debuggers }
      sessions.define_singleton_method(:close_after_owner_cleanup!) { events << :clients }
      proof.instance_variable_set(:@native_cancellation_sessions, sessions)
      proof.define_singleton_method(:cleanup_native_tooling!) { events << :tooling }
      Dir.mktmpdir("revaer-native-cancel-cleanup-") do |directory|
        proof.instance_variable_set(:@native_cancellation_evidence, directory)
        proof.send(:cleanup_native_resources!) { events << :database }
        assert(events == %i[debuggers database clients tooling], "owned cleanup order")
        assert(JSON.parse(File.binread(File.join(directory, "cleanup.json"))) == { "passed" => true, "failures" => [] }, "separate cleanup receipt")
      end
    end
  end
end

RevaerDatabaseRebaseline::NativeCancellationCaptureTest.new.run! if $PROGRAM_NAME == __FILE__
