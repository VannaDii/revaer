# frozen_string_literal: true

require "open3"
require_relative "ingestion_attributes"

module RevaerDatabaseRebaseline
  # Bounded database-only evidence; does not discharge the D3 sentinel.
  module IngestionRankProof
    RANK_CASES = [
      [19, 20, 19, 0.5, 0.6, 0.5], [20, 19, 20, 0.6, 0.5, 0.6],
      [29, 30, 29, 0.6, 0.7, 0.6], [30, 29, 30, 0.7, 0.6, 0.7],
      [39, 40, 39, 0.7, 0.8, 0.7], [40, 39, 40, 0.8, 0.7, 0.8]
    ].map(&:freeze).freeze

    def verify_ingestion_runtime_rank!
      @contract.validate_output_path!
      directory = File.join(@contract.output_path, "ingestion-runtime-rank")
      raise Failure, "rank evidence directory is a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      previous = @attributes_evidence
      @attributes_evidence = Dir.mktmpdir("run-", directory)
      @runtime_rank_evidence = @attributes_evidence
      first = @checks.length
      hashes = rank_source_hashes
      completed = false
      cases = []
      begin
        RANK_CASES.each do |spec|
          %w[cold helpers-first].each do |mode|
            %w[reference final].each do |variant|
              rank_isolated!(spec, mode, variant)
              cases << { ranks: spec.first(3), mode:, variant:, validated: true,
                         equivalent: false, required_success: variant == "final",
                         reference_failure: variant == "reference" ? "ADR 588 D4 third call 42P07" : nil }
            end
          end
        end
        check("runtime rank source bytes unchanged", hashes == rank_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        attributes_write("report.json", JSON.pretty_generate({ completed:, passed: completed && checks.all? { |c| c.fetch(:passed) },
          d3_complete: false, scope: "same-backend rollback then two commits; independently committed external rank writes",
          limitations: "database v1 entrypoint only; no Rust PgPool qualification or complete D3 claim",
          source_sha256: hashes, postgres_image: @contract.postgres_image,
          candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256, cases:, checks: }) + "\n")
        @attributes_evidence = previous
      end
      raise Failure, "runtime rank checks failed" unless @checks.drop(first).all? { |c| c.fetch(:passed) }
    end

    private

    def rank_source_hashes
      attributes_source_hashes.merge(%w[scripts/database_rebaseline/ingestion_runtime_rank.rb
        scripts/tests/database-ingestion-runtime-rank-test.rb scripts/tests/database-ingestion-runtime-rank-test.sh].to_h do |path|
        [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest]
      end)
    end

    def rank_case(spec, index)
      attributes_case("runtime-rank", rank: spec.fetch(index), confidence: spec.fetch(index + 3))
    end

    def rank_query(test_case, mode, index)
      # Only the first call is rolled back. Keeping the same psql process retains
      # compiled plans and the exact reference scratch-table lifetime.
      attributes_query(calls: [attributes_arguments(test_case)], rollback: index.zero?, helpers: index.zero? && mode == "helpers-first")
    end

    def rank_update_query(rank)
      <<~SQL
        BEGIN;
        UPDATE public.trust_tier SET rank = #{Integer(rank)} WHERE trust_tier_key = 'public';
        SELECT json_build_object('backend', pg_backend_pid()::text, 'transaction', txid_current()::text,
          'rank', rank, 'session', session_user, 'current', current_user)::text
          FROM public.trust_tier WHERE trust_tier_key = 'public';
        COMMIT;
      SQL
    end

    def rank_changed_inputs(current, rank)
      current.to_h do |table, rows|
        next [table, rows] unless table == "trust_tier"

        changed = rows.map { |row| row.fetch("trust_tier_key") == "public" ? row.merge("rank" => rank) : row }
        # policy_read_snapshot orders by jsonb text, not stable row identity.
        # These seeded rows have equal clocks; rank and display_name determine
        # their exact order, including the private/public tie at rank 30.
        [table, changed.sort_by { |row| [row.fetch("rank").to_s, row.fetch("created_at"), row.fetch("display_name")] }]
      end
    end

    def rank_update!(database, rank, name)
      query = rank_update_query(rank)
      attributes_write("#{name}.sql", query)
      raw = result(query, database:, role: "postgres")
      attributes_write("#{name}.stdout", raw.stdout)
      attributes_write("#{name}.stderr", raw.stderr)
      raise Failure, "rank writer failed" unless raw.success && raw.stderr.empty?

      { "stdout" => raw.stdout, "stderr" => raw.stderr, "success" => raw.success, "record" => metadata_json_parse(raw.stdout) }
    end

    def rank_isolated!(spec, mode, variant)
      name = "#{spec.first(3).join('-')}-#{mode}-#{variant}"
      database = "rank_#{SecureRandom.hex(8)}"
      role = variant == "reference" ? "postgres" : @runtime
      source = variant == "reference" ? "reference_proof" : @database
      created = false
      previous = @correction_evidence
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        # Database isolation matches the existing attribute disposable helper.
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
        seed += "\n#{validation_fixture_sql(fixture: {})}\nUPDATE public.trust_tier SET rank = #{spec.first} WHERE trust_tier_key = 'public';"
        clock = policy_setup!(seed, database, File.join(@attributes_evidence, "#{name}-seed"))
        @correction_evidence = File.join(@attributes_evidence, "#{name}-observer")
        FileUtils.mkdir_p(@correction_evidence, mode: 0o700)
        correction_observer!(database, variant)
        inputs = policy_read_snapshot(database)
        fixture = attributes_execute({ calls: [attributes_arguments(rank_case(spec, 0))] }, database, role, "#{name}-fixture")
        evidence = { "fixture_transport" => fixture, "before" => ingestion_snapshot(database), "seed_clock" => clock,
                     "inputs_before" => inputs, "inputs_fixture" => policy_read_snapshot(database) }
        evidence.merge!(rank_reused!(database, role, spec, mode, name))
        evidence["after"] = ingestion_snapshot(database)
        attributes_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        rank_validate!(evidence, spec, mode, variant, role)
        check("runtime rank #{name} exact context committed changes and persisted answers", true)
      ensure
        @correction_evidence = previous
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def rank_reused!(database, role, spec, mode, name)
      transcript = +""
      records = []
      changes = []
      # stdout is drained to explicit per-call barriers; stderr is drained in
      # parallel and retained in full even when semantic validation fails.
      Open3.popen3(*command(role, database)) do |input, output, error, waiter|
        reader = Thread.new { error.read }
        begin
          3.times do |index|
            if index.positive?
              before = policy_read_snapshot(database)
              tables = ingestion_snapshot(database)
              writer = rank_update!(database, spec.fetch(index), "#{name}-writer-#{index}")
              changes << { "before" => before, "after" => policy_read_snapshot(database), "writer" => writer,
                           "tables_before" => tables, "tables_after" => ingestion_snapshot(database) }
            end
            query = rank_query(rank_case(spec, index), mode, index)
            attributes_write("#{name}-call-#{index}.sql", query)
            offset = transcript.length
            marker = "rank_barrier_#{index}"
            input.write(query + "\n\\echo #{marker}\n")
            input.flush
            guid_controller_read(output, marker, transcript)
            records << { "stdout" => transcript[offset..].delete_suffix("#{marker}\n"),
                         "inputs_after" => policy_read_snapshot(database) }
          end
        ensure
          input.close unless input.closed?
          transcript << output.read
          stderr = reader.value
          success = waiter.value.success?
          attributes_write("#{name}-reused.stdout", transcript)
          attributes_write("#{name}-reused.stderr", stderr)
          attributes_write("#{name}-partial.json", JSON.pretty_generate({ calls: records, changes:, success: }) + "\n")
          raise Failure, "rank reused transport failed" unless success
        end
        { "calls" => records, "changes" => changes, "stderr" => stderr }
      end
    end

    def rank_validate!(evidence, spec, mode, variant, role)
      raise Failure, "rank unknown case" unless RANK_CASES.include?(spec) && %w[cold helpers-first].include?(mode) && %w[reference final].include?(variant)

      fixture_raw = evidence.fetch("fixture_transport")
      fixture_parsed = attributes_parse(fixture_raw.fetch("stdout"), fixture_raw.fetch("stderr"), { calls: [{}] }, role)
      fixture = fixture_parsed.fetch("frames").first
      raise Failure, "rank fixture transport changed" unless fixture_raw.slice("context", "frames") == fixture_parsed &&
        fixture_parsed.fetch("context") == { "backend" => fixture.fetch("backend"), "session" => role, "current" => role, "setting" => "error" }
      calls = evidence.fetch("calls")
      raise Failure, "rank missing calls or writes" unless calls.length == 3 && evidence.fetch("changes").length == 2

      frames = calls.each_with_index.map do |raw, index|
        parsed = attributes_parse(raw.fetch("stdout"), index == 2 ? evidence.fetch("stderr") : "",
          { calls: [{}], helpers: index.zero? && mode == "helpers-first" }, role)
        frame = parsed.fetch("frames").first
        expected = { "backend" => frame.fetch("backend"), "session" => role, "current" => role, "setting" => "error" }
        raise Failure, "rank entry context changed" unless parsed.fetch("context") == expected

        frame
      end
      raise Failure, "rank backend role or clock changed" unless validation_context?(frames, role) && validation_context?([fixture], role) &&
        fixture.fetch("backend") != frames.first.fetch("backend") && ([fixture] + frames).map { |f| f.fetch("clock") }.uniq.length == 4
      raise Failure, "rank exact success or D4 changed" unless fixture.fetch("state") == "00000" && validation_states?(frames, { site: nil }, variant, role)
      raise Failure, "rank scratch lifetime changed" unless fixture.values_at("within", "outside") == ["true", (variant == "reference").to_s] && validation_lifetime?(frames, { site: nil }, variant)

      baseline = evidence.fetch("inputs_before")
      check_inputs = evidence.merge("fixture" => fixture, "frames" => frames, "inputs_after" => baseline)
      raise Failure, "rank fixture inputs changed" unless attributes_inputs?(check_inputs, rank_case(spec, 0))
      current = baseline
      calls.each_with_index do |raw, index|
        if index.positive?
          change = evidence.fetch("changes").fetch(index - 1)
          expected = rank_changed_inputs(current, spec.fetch(index))
          writer = change.fetch("writer")
          record = metadata_json_parse(writer.fetch("stdout"))
          raise Failure, "rank writer raw evidence changed" unless writer.fetch("success") == true && writer.fetch("stderr").empty? && record == writer.fetch("record")
          raise Failure, "rank independent writer identity changed" unless record == {
            "backend" => record.fetch("backend"), "transaction" => record.fetch("transaction"), "rank" => spec.fetch(index), "session" => "postgres", "current" => "postgres"
          } && %w[backend transaction].all? { |key| record.fetch(key).match?(/\A[1-9][0-9]*\z/) } && record.fetch("backend") != frames.first.fetch("backend")
          raise Failure, "rank external commit or unrelated context changed" unless change.fetch("before") == current && change.fetch("after") == expected &&
            change.fetch("tables_before") == frames.fetch(index - 1).fetch("tables_finish") && change.fetch("tables_after") == change.fetch("tables_before")

          current = expected
        end
        raise Failure, "rank ingestion read inputs changed" unless raw.fetch("inputs_after") == current
      end
      writers = evidence.fetch("changes").map { |c| c.fetch("writer").fetch("record") }
      raise Failure, "rank writer transactions reused" unless writers.map { |r| r.fetch("transaction") }.uniq.length == 2
      raise Failure, "rank table continuity changed" unless correction_snapshots?({}, attributes_empty, [fixture], evidence.fetch("before")) &&
        correction_snapshots?({ rollback: true }, evidence.fetch("before"), frames, evidence.fetch("after"))
      ids = attributes_identities(fixture.fetch("tables_after"))
      expected = attributes_initial_tables(rank_case(spec, 0), fixture.fetch("clock"), ids)
      raise Failure, "rank initial independent output changed" unless attributes_first_tables?(fixture.fetch("tables_after"), expected) && attributes_result?(fixture, ids, first: true)

      frames.each_with_index do |frame, index|
        next if frame.fetch("state") != "00000"

        raise Failure, "rank persisted confidence or output changed" unless frame.fetch("tables_after") == attributes_repeated_tables(rank_case(spec, index), frame, index) && attributes_result?(frame, ids, first: false)
      end
      true
    end
  end
end
