# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Both arms observe the same owned waits without changing the Rust transactions.
  module NativeCancellationClocks
    private

    def cancellation_seed!(database, seed)
      return super unless @native_cancellation_context

      clocks = @native_cancellation_context.fetch(:clocks)
      raise Failure, "duplicate cancellation seed clock" if clocks.key?("seed")

      query = "BEGIN; SELECT to_json(transaction_timestamp());\n#{seed}\nCOMMIT;"
      native_write("seed.sql", query, directory: @native_cancellation_context.fetch(:directory))
      clocks["seed"] = metadata_json_parse(sql(query, role: "postgres", database:))
    end

    def cancellation_with_probe(input_path, prefix)
      return super unless @native_cancellation_context

      context = @native_cancellation_context
      request = metadata_json_parse(File.binread(input_path))
      connection = request.fetch("connection")
      context[:database] = connection.fetch("database")
      context[:role] = connection.fetch("role")
      if request.fetch("cache_state") == "warm_committed"
        original = IngestionCancellation.instance_method(:cancellation_with_lock).bind(self)
        original.call(context.fetch(:database), "#{prefix}-warmup-clock") do |locker, release|
          super(input_path, prefix) do |waiter|
            context[:waiter] = waiter
            native_cancellation_wait_clock!("warmup", locker)
            release.call
            yield waiter
          ensure
            context[:waiter] = nil
          end
        end
      else
        super(input_path, prefix) do |waiter|
          context[:waiter] = waiter
          yield waiter
        ensure
          context[:waiter] = nil
        end
      end
    end

    def cancellation_with_lock(database, prefix)
      return super unless @native_cancellation_context

      super do |locker, release|
        release_with_clock = lambda do
          context = @native_cancellation_context
          unless context.values_at(:variant, :cache_state) == %w[reference warm_committed]
            native_cancellation_wait_clock!("recovery", locker)
          end
          release.call
        end
        yield locker, release_with_clock
      end
    end

    def native_cancellation_wait_clock!(stage, locker)
      context = @native_cancellation_context
      database, role = context.values_at(:database, :role)
      query = cancellation_wait_query(database, role, locker.fetch("pid"))
      blocked = cancellation_wait("#{stage} transaction clock at owned source lock", context.fetch(:waiter)) do
        raw = sql(query, role: "postgres", database:)
        metadata_json_parse(raw) unless raw.empty?
      end
      native_cancellation_record_clock!(stage, blocked, locker, database, role)
    end

    def native_cancellation_record_clock!(stage, blocked, locker, database, role)
      original = IngestionCancellation.instance_method(:cancellation_validate_wait!).bind(self)
      original.call(blocked, locker, database, role)
      context = @native_cancellation_context
      raise Failure, "duplicate cancellation #{stage} clock" if context.fetch(:clocks).key?(stage)

      pid = blocked.fetch("pid")
      query = "SELECT json_build_object('pid', a.pid, 'database', a.datname, 'role', a.usename, 'application', a.application_name, 'backend_start', a.backend_start, 'transaction_clock', a.xact_start)::text FROM pg_stat_activity a WHERE a.pid=#{Integer(pid)} AND a.datname=#{literal(database)} AND a.usename=#{literal(role)} AND a.application_name=#{literal(IngestionCancellation::CANCELLATION_APPLICATION)};"
      native_write("#{stage}-clock.sql", query, directory: context.fetch(:directory))
      clock = metadata_json_parse(sql(query, role: "postgres", database:))
      repeated = metadata_json_parse(sql(query, role: "postgres", database:))
      after = metadata_json_parse(sql(cancellation_wait_query(database, role, locker.fetch("pid")), role: "postgres", database:))
      native_policy_record(context.fetch(:directory), "#{stage}-clock-raw.json", { clock:, repeated:, before: blocked, after:, locker: })
      original.call(after, locker, database, role)
      raise Failure, "cancellation transaction identity or wait changed during clock observation" unless
        clock == repeated && blocked == after && clock.keys.sort == %w[application backend_start database pid role transaction_clock] &&
        clock.values_at("pid", "database", "role", "application") == [pid, database, role, IngestionCancellation::CANCELLATION_APPLICATION]

      context.fetch(:clocks)[stage] = clock.fetch("transaction_clock")
      context.fetch(:witnesses)[stage] = { identity: clock, wait: blocked, locker:, revalidated: true }
    end

    def native_cancellation_validate_clocks!(raw)
      context = @native_cancellation_context
      stages = %w[cancelled]
      stages << "warmup" if context.fetch(:cache_state) == "warm_committed"
      stages << "recovery" unless context.values_at(:variant, :cache_state) == %w[reference warm_committed]
      clocks, witnesses = context.values_at(:clocks, :witnesses)
      raise Failure, "cancellation transaction clock matrix changed" unless
        clocks.keys.sort == (["seed"] + stages).sort && witnesses.keys.sort == stages.sort &&
        clocks.values.uniq.length == clocks.length && clocks.values.all? do |clock|
          clock.is_a?(String) && clock.match?(/\A\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?\+00:00\z/)
        end

      prepared = raw.fetch("preparation").fetch("session")
      identities = witnesses.values.map { |witness| witness.fetch(:identity) }
      raise Failure, "cancellation clocks do not belong to the prepared Rust backend" unless witnesses.all? do |stage, witness|
        identity = witness.fetch(:identity)
        witness.fetch(:revalidated) == true && identity.fetch("transaction_clock") == clocks.fetch(stage) &&
          identity.fetch("application") == IngestionCancellation::CANCELLATION_APPLICATION &&
          identity.fetch("backend_start").is_a?(String) && !identity.fetch("backend_start").empty? &&
          identity.values_at("pid", "database", "role") == prepared.values_at("pid", "database", "current_role")
      end && identities.map { |identity| identity.fetch("backend_start") }.uniq.length == 1

      native_policy_record(context.fetch(:directory), "clock-evidence.json", { clocks:, witnesses: })
    end
  end
end
