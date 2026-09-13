# frozen_string_literal: true

require "open3"

module RevaerDatabaseRebaseline
  # Cancels a real Rust call only after observing its exact owned lock wait.
  module IngestionCancellation
    CANCELLATION_APPLICATION = "revaer-ingestion-cancellation-proof"
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
          cases << cancellation_isolated!(variant, Integer(match[1]))
        end
        check("ingestion cancellation source bytes unchanged", hashes == cancellation_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |item| item.fetch(:passed) },
                   d3_complete: false, scope: "server query cancellation during actual Rust ingestion and same-PgPool recovery",
                   limits: "One source-insert lock point and one connection. Not client-future cancellation, all helpers, native closure or full D3.",
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

    def cancellation_isolated!(variant, port)
      database = "ingestion_pool_cancel_#{variant}_#{SecureRandom.hex(6)}"
      role = variant == "reference" ? "postgres" : @runtime
      source = variant == "reference" ? "reference_proof" : @database
      prefix = File.join(@cancellation_evidence, variant)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
        sql("BEGIN;\n#{seed}\nCOMMIT;", role: "postgres", database:)
        evidence = { "before" => ingestion_snapshot(database), "inputs_before" => policy_read_snapshot(database) }
        request = { database_url: "postgresql://#{role}@127.0.0.1:#{port}/#{database}", database:, role:, report_path: "#{prefix}-frames.json" }
        input_path = "#{prefix}-input.json"
        File.open(input_path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(JSON.generate(request)) }
        cancellation_with_lock(database, prefix) do |locker, release|
          evidence["locker"] = locker
          cancellation_with_probe(input_path, prefix) do |waiter|
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
            cancellation_validate_checkpoint!(evidence.fetch("checkpoint"), variant, database, role, pid)
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
        cancellation_validate!(evidence, variant, database, role)
        { variant:, database:, role:, frames_path: request.fetch(:report_path) }
      ensure
        File.binwrite("#{prefix}-partial.json", JSON.pretty_generate(evidence) + "\n") if evidence
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
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
        setting = cold && boundary == "before" ? nil : "error"
        raise Failure, "cancellation backend or restored setting changed" unless value.keys.sort == IngestionPool::POOL_SESSION_KEYS &&
          value.values_at("pid", "database", "session_role", "current_role", "server_version", "conflict_setting") ==
            [pid, database, role, role, "160014", setting] &&
          value.values_at("superuser", "create_role", "bypass_rls") == [variant == "reference"] * 3
      end
    end

    def cancellation_validate_checkpoint!(checkpoint, variant, database, role, pid)
      raise Failure, "cancellation checkpoint changed" unless checkpoint.is_a?(Array) && checkpoint.length == 1

      frame = checkpoint.first
      cancellation_validate_frame!(frame, "cancelled", variant, database, role, pid, cold: true)
      expected = { "kind" => "database_error", "operation" => "search result ingest", "state" => "57014",
                   "message" => "canceling statement due to user request", "detail" => nil }
      raise Failure, "cancellation outcome was not exact server query cancellation" unless frame.fetch("outcome") == expected
    end

    def cancellation_validate!(evidence, variant, database, role)
      locker = evidence.fetch("locker")
      blocked = evidence.fetch("blocked")
      cancellation_validate_wait!(blocked, locker, database, role)
      pid = blocked.fetch("pid")
      checkpoint = evidence.fetch("checkpoint")
      cancellation_validate_checkpoint!(checkpoint, variant, database, role, pid)
      frames = evidence.fetch("frames")
      raise Failure, "cancellation evidence lost its checkpoint" unless frames.is_a?(Array) && frames.length == 2 &&
        frames.first == checkpoint.first && evidence.fetch("cancel_result") == "t"

      cancellation_validate_frame!(frames.last, "recovery", variant, database, role, pid)
      result = frames.last.fetch("outcome")
      row = result.fetch("row")
      raise Failure, "cancellation recovery did not create new stable identities" unless result.keys.sort == %w[kind row] &&
        result.fetch("kind") == "success" && row.keys.sort == IngestionPool::POOL_ROW_KEYS &&
        row.values_at("canonical", "source").all? { |id| id.match?(IngestionProof::INGESTION_UUID) } &&
        row.values_at("observation_created", "durable_source_created", "canonical_changed") == [true] * 3

      %w[before cancelled after].each do |boundary|
        image = evidence.fetch(boundary)
        raise Failure, "cancellation table image incomplete" unless image.keys.sort == IngestionProof::INGESTION_TABLES.sort && image.values.all? { |rows| rows.is_a?(Array) }
      end
      raise Failure, "cancelled ingestion committed writes" unless evidence.fetch("before").values.all?(&:empty?) && evidence.fetch("cancelled") == evidence.fetch("before")
      %w[inputs_before inputs_cancelled inputs_after].each do |boundary|
        image = evidence.fetch(boundary)
        raise Failure, "cancellation read-input image incomplete" unless image.keys.sort == IngestionPolicy::POLICY_READ_TABLES.sort && image.values.all? { |rows| rows.is_a?(Array) }
      end
      raise Failure, "cancellation changed read inputs" unless evidence.fetch("inputs_before") == evidence.fetch("inputs_cancelled") && evidence.fetch("inputs_before") == evidence.fetch("inputs_after")

      after = evidence.fetch("after")
      canonical = after.fetch("canonical_torrent")
      source = after.fetch("canonical_torrent_source")
      observed = after.fetch("search_request_source_observation")
      raise Failure, "cancellation recovery persistence changed" unless canonical.length == 1 && source.length == 1 && observed.length == 1 &&
        canonical.first.fetch("canonical_torrent_public_id") == row.fetch("canonical") &&
        source.first.fetch("canonical_torrent_source_public_id") == row.fetch("source") &&
        source.first.fetch("source_guid") == "pool-proof-source" && source.first.fetch("last_seen_at") == "2026-09-10T00:01:00+00:00"

      check("ingestion cancellation #{variant} owned wait exact error rollback and same-pool recovery", true)
    rescue KeyError, NoMethodError, TypeError => error
      raise Failure, "malformed cancellation evidence: #{error.class}"
    end
  end
end
