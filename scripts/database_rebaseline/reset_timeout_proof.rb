# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Test-only observers exercise the actual reset routine without changing it.
  module ResetTimeoutProof
    private

    def reset_observer(body)
      <<~SQL
        CREATE OR REPLACE FUNCTION revaer_system.proof_reset_observer() RETURNS trigger
        LANGUAGE plpgsql SET search_path = pg_catalog AS $proof$
        BEGIN
          RAISE NOTICE 'reset-timeout-inside:%', current_setting('lock_timeout');
          #{body}
          RETURN NULL;
        END;
        $proof$;
      SQL
    end

    def reset_outcome(query, role: @runtime)
      outcome = result("\\set VERBOSITY default\n#{query}", role:)
      File.open(File.join(@contract.output_path, "final-reset-timeout-transcript.txt"), "a") do |file|
        file.write("role=#{role == @runtime ? 'runtime' : 'owner'}\n#{outcome.stdout}#{outcome.stderr}\n")
      end
      raise Failure, "reset timeout proof emitted a PostgreSQL warning" if outcome.stderr.match?(/\bWARNING\b/)

      outcome
    end

    def verify_reset_timeout_scope!
      transcript = File.join(@contract.output_path, "final-reset-timeout-transcript.txt")
      File.write(transcript, "D2 real reset timeout scope observations\n")
      sql(reset_observer(""))
      sql("CREATE TRIGGER proof_reset_timeout BEFORE TRUNCATE ON public.app_profile FOR EACH STATEMENT EXECUTE FUNCTION revaer_system.proof_reset_observer()")
      [@owner, @runtime].each do |role|
        outcome = reset_outcome(<<~SQL, role:)
          SET lock_timeout = '19s';
          BEGIN;
          SET LOCAL statement_timeout = '120s';
          SET LOCAL lock_timeout = '17s';
          SET LOCAL idle_in_transaction_session_timeout = '30s';
          SELECT current_setting('lock_timeout');
          SELECT revaer_config.factory_reset_without_media_defaults_v1();
          SELECT current_setting('statement_timeout') || ',' || current_setting('lock_timeout') || ',' || current_setting('idle_in_transaction_session_timeout');
          ROLLBACK;
          SELECT current_setting('lock_timeout');
        SQL
        check("D2 #{role == @runtime ? 'runtime' : 'owner'} success and transaction rollback restore caller settings",
              outcome.success && outcome.stdout.lines.map(&:strip).reject(&:empty?) == ["17s", "2min,17s,30s", "19s"] &&
              outcome.stderr.lines.grep(/reset-timeout-inside:/).map(&:strip) == ["NOTICE:  reset-timeout-inside:5s"])
      end
      verify_reset_nested_scope!
      verify_reset_error_scope!("division_by_zero", "22012", "PERFORM 1 / 0;")
      verify_reset_cancellation!
      sql(reset_observer(""))
      verify_reset_lock_contention!
    ensure
      sql("DROP TRIGGER IF EXISTS proof_reset_timeout ON public.app_profile; DROP FUNCTION IF EXISTS revaer_system.proof_reset_observer(); DROP FUNCTION IF EXISTS revaer_system.proof_nested_reset();") if @database
    end

    def verify_reset_nested_scope!
      sql(<<~SQL)
        CREATE FUNCTION revaer_system.proof_nested_reset() RETURNS text
        LANGUAGE plpgsql SECURITY DEFINER SET search_path = pg_catalog SET lock_timeout = '9s'
        AS $proof$
        BEGIN
          PERFORM revaer_config.factory_reset_without_media_defaults_v1();
          RETURN current_setting('lock_timeout');
        END;
        $proof$;
        REVOKE ALL ON FUNCTION revaer_system.proof_nested_reset() FROM PUBLIC;
        GRANT EXECUTE ON FUNCTION revaer_system.proof_nested_reset() TO #{identifier(@runtime)};
      SQL
      outcome = reset_outcome(<<~SQL)
        BEGIN;
        SET LOCAL lock_timeout = '17s';
        SELECT revaer_system.proof_nested_reset();
        SELECT current_setting('lock_timeout');
        ROLLBACK;
      SQL
      check("D2 nested reset restores intermediate and outer lock timeouts", outcome.success && outcome.stdout.lines.map(&:strip) == ["9s", "17s"] &&
            outcome.stderr.lines.grep(/reset-timeout-inside:/).map(&:strip) == ["NOTICE:  reset-timeout-inside:5s"])
    end

    def verify_reset_error_scope!(condition, state, body)
      sql(reset_observer(body))
      outcome = reset_outcome(reset_caught_error_sql(condition, state))
      check("D2 #{condition} restores caller timeout", outcome.success && outcome.stdout.strip == "17s" &&
            outcome.stderr.include?("reset-timeout-inside:5s") && outcome.stderr.include?("reset-timeout-caught:#{state}:17s"))
    end

    def reset_caught_error_sql(condition, state)
      <<~SQL
        BEGIN;
        SET LOCAL statement_timeout = '15s';
        SET LOCAL lock_timeout = '17s';
        DO $proof$
        BEGIN
          PERFORM revaer_config.factory_reset_without_media_defaults_v1();
          RAISE EXCEPTION 'reset unexpectedly succeeded';
        EXCEPTION WHEN #{condition} THEN
          IF SQLSTATE <> '#{state}' THEN RAISE; END IF;
          RAISE NOTICE 'reset-timeout-caught:%:%', SQLSTATE, current_setting('lock_timeout');
        END;
        $proof$;
        SELECT current_setting('lock_timeout');
        ROLLBACK;
      SQL
    end

    def verify_reset_cancellation!
      sql(reset_observer("PERFORM pg_sleep(30);"))
      application = "reset-proof-cancel-#{SecureRandom.hex(8)}"
      worker = Thread.new do
        reset_outcome("SET application_name = #{literal(application)};\n#{reset_caught_error_sql('query_canceled', '57014')}")
      end
      worker.report_on_exception = false
      backend = nil
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 10
      loop do
        pid = sql("SELECT pid FROM pg_stat_activity WHERE datname = current_database() AND application_name = #{literal(application)} AND wait_event = 'PgSleep'", role: "postgres")
        unless pid.empty?
          raise Failure, "reset cancellation backend identity is invalid" unless pid.match?(/\A[1-9][0-9]*\z/)

          backend = pid
          break
        end
        raise Failure, "reset cancellation did not reach the observer" if !worker.alive? || Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline

        sleep(0.05)
      end
      check("D2 cancellation delivered to the exact active reset backend", sql("SELECT pg_cancel_backend(#{backend})", role: "postgres") == "t")
      outcome = worker.value
      check("D2 real query cancellation restores caller timeout", outcome.success && outcome.stdout.strip == "17s" &&
            outcome.stderr.include?("reset-timeout-inside:5s") && outcome.stderr.include?("reset-timeout-caught:57014:17s"))
    ensure
      # The proof query also has a 15-second statement timeout, independent of
      # this observer. Join propagates any worker exception instead of hiding it.
      worker.value if worker
    end

    def verify_reset_lock_contention!
      Open3.popen3(*command(@owner, @database)) do |input, output, errors, process|
        input.write("BEGIN; LOCK TABLE public.app_profile IN ACCESS SHARE MODE; SELECT 'reset-lock-ready';\n")
        input.flush
        ready = IO.select([output], nil, nil, 10)
        raise Failure, "reset lock holder did not become ready" unless ready && output.gets&.strip == "reset-lock-ready"

        started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        outcome = reset_outcome(reset_caught_error_sql("lock_not_available", "55P03"))
        elapsed = Process.clock_gettime(Process::CLOCK_MONOTONIC) - started
        check("D2 real lock contention retains five-second bound and restores caller", outcome.success &&
              outcome.stdout.strip == "17s" && outcome.stderr.include?("reset-timeout-caught:55P03:17s") && elapsed >= 4.5 && elapsed < 10)
      ensure
        unless input.closed?
          input.write("ROLLBACK;\n")
          input.close
        end
        diagnostic = errors.read
        raise Failure, "reset lock holder failed cleanup" unless process.value.success? && diagnostic.empty?
      end
    end
  end
end
