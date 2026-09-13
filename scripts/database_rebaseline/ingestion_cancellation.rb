# frozen_string_literal: true

require "open3"

module RevaerDatabaseRebaseline
  # Cancels a real Rust call only after observing its exact owned lock wait.
  module IngestionCancellation
    CANCELLATION_APPLICATION = "revaer-ingestion-cancellation-proof"
    CANCELLATION_CACHE_STATES = %w[cold warm_committed].freeze
    CANCELLATION_SOURCES = %w[
      scripts/database_rebaseline/ingestion_cancellation.rb
      crates/revaer-data/src/indexers/search_results/cancellation_proof_tests.rs
      scripts/tests/database-ingestion-cancellation-test.rb
      scripts/tests/database-ingestion-cancellation-test.sh
    ].freeze

    private

    def verify_ingestion_cancellation!
      directory = File.join(@contract.output_path, "ingestion-cancellation")
      raise Failure, "cancellation evidence directory is a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @cancellation_evidence = Dir.mktmpdir("run-", directory)
      first = @checks.length
      hashes = cancellation_source_hashes
      completed = false
      cases = []
      begin
        endpoint = @runner.run!(["docker", "port", @container, "5432/tcp"]).strip
        match = endpoint.match(/\A127\.0\.0\.1:([1-9][0-9]{0,4})\z/)
        raise Failure, "cancellation proof requires one owned loopback endpoint" unless match && Integer(match[1]) <= 65_535

        %w[reference final].each do |variant|
          CANCELLATION_CACHE_STATES.each do |cache_state|
            cases << cancellation_isolated!(variant, Integer(match[1]), cache_state)
          end
        end
        check("ingestion cancellation source bytes unchanged", hashes == cancellation_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |item| item.fetch(:passed) },
                   d3_complete: false, scope: "server query cancellation during actual Rust ingestion and same-PgPool recovery",
                   limits: "Cold and committed-warm calls at one source-insert lock point and one connection; frozen warm recovery retains D4. Not client-future cancellation, native in-call scope, all helpers or full D3.",
                   postgres_image: @contract.postgres_image, candidate_sha256: @contract.expected_candidate_sha256,
                   final_sha256: @contract.final_sha256, source_sha256: hashes, cases:, checks: }
        File.binwrite(File.join(@cancellation_evidence, "report.json"), JSON.pretty_generate(report) + "\n")
      end
      raise Failure, "ingestion cancellation qualification failed" unless @checks.drop(first).all? { |item| item.fetch(:passed) }
    end

    def cancellation_source_hashes
      pool_source_hashes.merge(CANCELLATION_SOURCES.to_h do |path|
        [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest]
      end)
    end

    def cancellation_isolated!(variant, port, cache_state)
      database = "ingestion_pool_cancel_#{variant}_#{SecureRandom.hex(6)}"
      role = variant == "reference" ? "postgres" : @runtime
      source = variant == "reference" ? "reference_proof" : @database
      prefix = File.join(@cancellation_evidence, "#{variant}-#{cache_state}")
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
        sql("BEGIN;\n#{seed}\nCOMMIT;", role: "postgres", database:)
        evidence = { "cache_state" => cache_state, "before" => ingestion_snapshot(database), "inputs_before" => policy_read_snapshot(database) }
        request = { database_url: "postgresql://#{role}@127.0.0.1:#{port}/#{database}", database:, role:, report_path: "#{prefix}-frames.json" }
        input_path = "#{prefix}-input.json"
        File.open(input_path, File::WRONLY | File::CREAT | File::EXCL, 0o600) do |file|
          file.write(JSON.generate({ connection: request, cache_state: }))
        end
        cancellation_with_probe(input_path, prefix) do |waiter|
          prepared = "#{request.fetch(:report_path)}.prepared.json"
          evidence["preparation"] = cancellation_wait("atomic prepared-session checkpoint", waiter) do
            metadata_json_parse(File.binread(prepared)) if File.file?(prepared)
          end
          cancellation_validate_preparation!(evidence.fetch("preparation"), cache_state, variant, database, role)
          evidence["prepared"] = ingestion_snapshot(database)
          evidence["inputs_prepared"] = policy_read_snapshot(database)
          cancellation_with_lock(database, prefix) do |locker, release|
            evidence["locker"] = locker
            cancellation_publish_start!("#{request.fetch(:report_path)}.start.json", evidence.fetch("preparation").fetch("start_signal"))
            query = cancellation_wait_query(database, role, locker.fetch("pid"))
            File.binwrite("#{prefix}-wait.sql", query)
            evidence["blocked"] = cancellation_wait("owned ingestion lock wait", waiter) do
              raw = sql(query, role: "postgres", database:)
              next nil if raw.empty?

              metadata_json_parse(raw)
            end
            cancellation_validate_wait!(evidence.fetch("blocked"), locker, database, role)
            pid = evidence.fetch("blocked").fetch("pid")
            cancel = "SELECT pg_cancel_backend(pid) FROM pg_stat_activity WHERE pid = #{Integer(pid)} AND datname = #{literal(database)} AND usename = #{literal(role)} AND application_name = #{literal(CANCELLATION_APPLICATION)};"
            File.binwrite("#{prefix}-cancel.sql", cancel)
            evidence["cancel_result"] = sql(cancel, role: "postgres", database:)
            raise Failure, "owned backend did not accept cancellation" unless evidence.fetch("cancel_result") == "t"

            checkpoint = "#{request.fetch(:report_path)}.cancelled.json"
            evidence["checkpoint"] = cancellation_wait("atomic cancellation checkpoint", waiter) do
              metadata_json_parse(File.binread(checkpoint)) if File.file?(checkpoint)
            end
            cancellation_validate_checkpoint!(evidence.fetch("checkpoint"), variant, database, role, pid, cache_state:)
            evidence["cancelled"] = ingestion_snapshot(database)
            evidence["inputs_cancelled"] = policy_read_snapshot(database)
            release.call
            cancellation_wait("Rust recovery completion", nil) { waiter.join(0.05) }
          end
        end
        raise Failure, "Rust cancellation report is missing" unless File.size?(request.fetch(:report_path))

        evidence["frames"] = metadata_json_parse(File.binread(request.fetch(:report_path)))
        evidence["after"] = ingestion_snapshot(database)
        evidence["inputs_after"] = policy_read_snapshot(database)
        File.binwrite("#{prefix}.json", JSON.pretty_generate(evidence) + "\n")
        cancellation_validate!(evidence, variant, database, role, cache_state:)
        { variant:, cache_state:, database:, role:, frames_path: request.fetch(:report_path),
          approved_delta: variant == "reference" && cache_state == "warm_committed" ? "D4 frozen committed-reuse failure" : nil }
      ensure
        File.binwrite("#{prefix}-partial.json", JSON.pretty_generate(evidence) + "\n") if evidence
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def cancellation_publish_start!(path, signal)
      temporary = "#{path}.#{SecureRandom.hex(6)}.tmp"
      begin
        File.open(temporary, File::WRONLY | File::CREAT | File::EXCL, 0o600) do |file|
          file.write(JSON.generate(signal))
          file.flush
          file.fsync
        end
        File.link(temporary, path)
      ensure
        File.unlink(temporary) if File.exist?(temporary)
      end
    end

    def cancellation_wait(label, waiter)
      deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + 180
      loop do
        value = yield
        return value if value
        raise Failure, "Rust cancellation probe ended before #{label}" if waiter && !waiter.alive?
        raise Failure, "timed out waiting for #{label}" if Process.clock_gettime(Process::CLOCK_MONOTONIC) >= deadline

        sleep 0.05
      end
    end

    def cancellation_with_lock(database, prefix)
      transcript = +""
      Open3.popen3(*command("postgres", database)) do |input, output, error, waiter|
        reader = Thread.new { error.read }
        begin
          query = "BEGIN; LOCK TABLE public.canonical_torrent_source IN SHARE MODE; SELECT json_build_object('pid', pg_backend_pid(), 'database', current_database(), 'role', current_user)::text;"
          File.binwrite("#{prefix}-lock.sql", query + "\nROLLBACK;\n")
          input.write(query + "\n\\echo cancellation_lock_ready\n")
          input.flush
          guid_controller_read(output, "cancellation_lock_ready", transcript)
          locker = metadata_json_parse(transcript.delete_suffix("cancellation_lock_ready\n"))
          release = lambda do
            input.write("ROLLBACK;\n\\echo cancellation_lock_released\n")
            input.flush
            guid_controller_read(output, "cancellation_lock_released", transcript)
          end
          yield locker, release
        ensure
          input.close unless input.closed?
          transcript << output.read
          stderr = reader.value
          status = waiter.value
          File.binwrite("#{prefix}-lock.stdout", transcript)
          File.binwrite("#{prefix}-lock.stderr", stderr)
          raise Failure, "cancellation lock controller failed" unless status.success? && stderr.empty?
        end
      end
    end

    def cancellation_with_probe(input_path, prefix)
      env = { "REVAER_INGESTION_CANCELLATION_PROOF" => input_path }
      Open3.popen3(env, "just", "db-init-cancellation-probe", chdir: @contract.root, pgroup: true) do |input, output, error, waiter|
        input.close
        stdout = Thread.new { output.read }
        stderr = Thread.new { error.read }
        forced = false
        begin
          yield waiter
        ensure
          primary_error = $!
          if waiter.alive?
            forced = true
            begin
              Process.kill("TERM", -waiter.pid)
              Process.kill("KILL", -waiter.pid) unless waiter.join(5)
            rescue Errno::ESRCH
              raise Failure, "owned cancellation process disappeared without settlement" unless waiter.join(5)
            end
          end
          status = waiter.value
          raw_error = stderr.value
          File.binwrite("#{prefix}.stdout", stdout.value)
          File.binwrite("#{prefix}.stderr", raw_error)
          File.binwrite("#{prefix}-status.json", JSON.generate({ success: status.success?, exitstatus: status.exitstatus, termsig: status.termsig, forced:, controller_error: primary_error&.message }) + "\n")
          if primary_error.nil?
            raise Failure, "Rust cancellation probe failed or required termination" unless status.success? && !forced
            raise Failure, "Rust cancellation probe emitted a warning" if raw_error.match?(/\bWARN\b|\bwarning:/i)
          end
        end
      end
    end

    def cancellation_wait_query(database, role, locker)
      <<~SQL
        SELECT json_build_object('pid', a.pid, 'database', a.datname, 'role', a.usename,
          'application', a.application_name, 'state', a.state, 'wait_type', a.wait_event_type,
          'wait_event', a.wait_event, 'query', a.query, 'blockers', pg_blocking_pids(a.pid),
          'source_insert_wait', EXISTS (SELECT 1 FROM pg_locks l WHERE l.pid = a.pid
            AND l.relation = 'public.canonical_torrent_source'::regclass
            AND l.mode = 'RowExclusiveLock' AND NOT l.granted),
          'canonical_write_lock', EXISTS (SELECT 1 FROM pg_locks l WHERE l.pid = a.pid
            AND l.relation = 'public.canonical_torrent'::regclass
            AND l.mode = 'RowExclusiveLock' AND l.granted))::text
        FROM pg_stat_activity a WHERE a.datname = #{literal(database)} AND a.usename = #{literal(role)}
          AND a.application_name = #{literal(CANCELLATION_APPLICATION)} AND a.wait_event_type = 'Lock'
          AND #{Integer(locker)} = ANY(pg_blocking_pids(a.pid));
      SQL
    end

    def cancellation_validate_wait!(blocked, locker, database, role)
      raise Failure, "cancellation lock owner changed" unless locker.keys.sort == %w[database pid role] &&
        locker.fetch("pid").is_a?(Integer) && locker.fetch("pid").positive? && locker.values_at("database", "role") == [database, "postgres"]
      raise Failure, "cancellation did not observe its exact owned insertion wait" unless
        blocked.keys.sort == %w[application blockers canonical_write_lock database pid query role source_insert_wait state wait_event wait_type] &&
        blocked.fetch("pid").is_a?(Integer) && blocked.fetch("pid").positive? && blocked.fetch("pid") != locker.fetch("pid") &&
        blocked.values_at("database", "role", "application", "state", "wait_type", "wait_event") ==
          [database, role, CANCELLATION_APPLICATION, "active", "Lock", "relation"] &&
        blocked.fetch("blockers") == [locker.fetch("pid")] && blocked.fetch("source_insert_wait") == true &&
        blocked.fetch("canonical_write_lock") == true && blocked.fetch("query").include?("FROM search_result_ingest(")
    end

    def cancellation_validate_frame!(frame, name, variant, database, role, pid, cold: false)
      raise Failure, "cancellation frame identity changed" unless %w[reference final].include?(variant) &&
        frame.keys.sort == %w[after before name outcome] && frame.fetch("name") == name

      %w[before after].each do |boundary|
        value = frame.fetch(boundary)
        cancellation_validate_session!(value, variant, database, role, pid, cold: cold && boundary == "before")
      end
    end

    def cancellation_validate_session!(value, variant, database, role, pid, cold: false)
      raise Failure, "cancellation backend or restored setting changed" unless
        pid.is_a?(Integer) && pid.positive? && value.keys.sort == IngestionPool::POOL_SESSION_KEYS &&
        value.values_at("pid", "database", "session_role", "current_role", "server_version", "conflict_setting") ==
          [pid, database, role, role, "160014", cold ? nil : "error"] &&
        value.values_at("superuser", "create_role", "bypass_rls") == [variant == "reference"] * 3
    end

    def cancellation_created_row!(result)
      row = result.fetch("row")
      raise Failure, "cancellation successful call did not create distinct identities" unless result.keys.sort == %w[kind row] &&
        result.fetch("kind") == "success" && row.keys.sort == IngestionPool::POOL_ROW_KEYS &&
        row.values_at("canonical", "source").all? { |id| id.match?(IngestionProof::INGESTION_UUID) } &&
        row.values_at("observation_created", "durable_source_created", "canonical_changed") == [true] * 3

      row
    end

    def cancellation_validate_preparation!(preparation, cache_state, variant, database, role)
      raise Failure, "cancellation preparation mode or fields changed" unless CANCELLATION_CACHE_STATES.include?(cache_state) &&
        preparation.keys.sort == %w[frames session start_signal]

      pid = preparation.fetch("session").fetch("pid")
      cold = cache_state == "cold"
      cancellation_validate_session!(preparation.fetch("session"), variant, database, role, pid, cold:)
      raise Failure, "cancellation start signal is not bound to the prepared backend" unless
        preparation.fetch("start_signal") == { "database" => database, "pid" => pid, "cache_state" => cache_state }

      frames = preparation.fetch("frames")
      raise Failure, "cancellation warm-up count changed" unless frames.is_a?(Array) && frames.length == (cold ? 0 : 1)
      return if cold

      frame = frames.first
      cancellation_validate_frame!(frame, "warmup", variant, database, role, pid, cold: true)
      cancellation_created_row!(frame.fetch("outcome"))
      raise Failure, "cancellation preparation lost warm-up scope" unless frame.fetch("after") == preparation.fetch("session")
    end

    def cancellation_validate_checkpoint!(checkpoint, variant, database, role, pid, cache_state: "cold")
      cold = cache_state == "cold"
      raise Failure, "cancellation checkpoint changed" unless checkpoint.is_a?(Array) && checkpoint.length == (cold ? 1 : 2)

      frame = checkpoint.last
      cancellation_validate_frame!(frame, "cancelled", variant, database, role, pid, cold:)
      expected = { "kind" => "database_error", "operation" => "search result ingest", "state" => "57014",
                   "message" => "canceling statement due to user request", "detail" => nil }
      raise Failure, "cancellation outcome was not exact server query cancellation" unless frame.fetch("outcome") == expected
    end

    def cancellation_validate!(evidence, variant, database, role, cache_state: "cold")
      raise Failure, "cancellation cache state changed" unless evidence.fetch("cache_state") == cache_state

      preparation = evidence.fetch("preparation")
      cancellation_validate_preparation!(preparation, cache_state, variant, database, role)
      locker = evidence.fetch("locker")
      blocked = evidence.fetch("blocked")
      cancellation_validate_wait!(blocked, locker, database, role)
      pid = blocked.fetch("pid")
      checkpoint = evidence.fetch("checkpoint")
      cancellation_validate_checkpoint!(checkpoint, variant, database, role, pid, cache_state:)
      frames = evidence.fetch("frames")
      raise Failure, "cancellation evidence lost its preparation or checkpoint" unless frames.is_a?(Array) &&
        frames.length == checkpoint.length + 1 && frames.take(checkpoint.length) == checkpoint &&
        checkpoint.take(checkpoint.length - 1) == preparation.fetch("frames") &&
        checkpoint.last.fetch("before") == preparation.fetch("session") && evidence.fetch("cancel_result") == "t"

      cancellation_validate_frame!(frames.last, "recovery", variant, database, role, pid)
      result = frames.last.fetch("outcome")
      frozen_d4 = variant == "reference" && cache_state == "warm_committed"
      if frozen_d4
        expected = { "kind" => "database_error", "operation" => "search result ingest", "state" => "42P07",
                     "message" => 'relation "tmp_policy_rules" already exists', "detail" => nil }
        raise Failure, "warm cancellation recovery changed the exact frozen D4 failure" unless result == expected
      else
        row = cancellation_created_row!(result)
      end

      %w[before prepared cancelled after].each do |boundary|
        image = evidence.fetch(boundary)
        raise Failure, "cancellation table image incomplete" unless image.keys.sort == IngestionProof::INGESTION_TABLES.sort && image.values.all? { |rows| rows.is_a?(Array) }
      end
      raise Failure, "cancelled ingestion committed writes" unless evidence.fetch("before").values.all?(&:empty?) && evidence.fetch("cancelled") == evidence.fetch("prepared")
      %w[inputs_before inputs_prepared inputs_cancelled inputs_after].each do |boundary|
        image = evidence.fetch(boundary)
        raise Failure, "cancellation read-input image incomplete" unless image.keys.sort == IngestionPolicy::POLICY_READ_TABLES.sort && image.values.all? { |rows| rows.is_a?(Array) }
      end
      raise Failure, "cancellation changed read inputs" unless %w[inputs_prepared inputs_cancelled inputs_after].all? do |boundary|
        evidence.fetch("inputs_before") == evidence.fetch(boundary)
      end

      if cache_state == "cold"
        raise Failure, "cold cancellation was not cold" unless evidence.fetch("prepared") == evidence.fetch("before")
      else
        warm_row = preparation.fetch("frames").first.fetch("outcome").fetch("row")
        cancellation_validate_persistence!(evidence.fetch("prepared"), warm_row, "pool-proof-warmup", "02", 1)
        warm_source = evidence.fetch("prepared").fetch("canonical_torrent_source").first
        raise Failure, "warm-up did not use its distinct exact hash" unless warm_source.fetch("infohash_v1") == "b" * 40
        raise Failure, "warm-up source changed after cancellation/recovery" unless evidence.fetch("after").fetch("canonical_torrent_source").include?(warm_source)
        raise Failure, "warm-up and recovered identities were reused" if row &&
          (row.values_at("canonical", "source") & warm_row.values_at("canonical", "source")).any?
      end

      if frozen_d4
        raise Failure, "frozen D4 recovery committed writes" unless evidence.fetch("after") == evidence.fetch("prepared")
      else
        cancellation_validate_persistence!(evidence.fetch("after"), row, "pool-proof-source", "01", cache_state == "cold" ? 1 : 2)
      end
      check("ingestion cancellation #{variant} #{cache_state} owned wait exact error rollback and same-pool recovery", true)
    rescue KeyError, NoMethodError, TypeError => error
      raise Failure, "malformed cancellation evidence: #{error.class}"
    end

    def cancellation_validate_persistence!(after, row, guid, minute, count)
      canonical = after.fetch("canonical_torrent")
      source = after.fetch("canonical_torrent_source")
      observed = after.fetch("search_request_source_observation")
      selected = source.select { |entry| entry.fetch("canonical_torrent_source_public_id") == row.fetch("source") }
      raise Failure, "cancellation persistence changed" unless canonical.length == count && source.length == count && observed.length == count &&
        canonical.count { |entry| entry.fetch("canonical_torrent_public_id") == row.fetch("canonical") } == 1 && selected.length == 1 &&
        selected.first.fetch("source_guid") == guid && selected.first.fetch("last_seen_at") == "2026-09-10T00:#{minute}:00+00:00"
    end
  end
end
