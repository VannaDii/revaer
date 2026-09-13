# frozen_string_literal: true

require "tmpdir"

module RevaerDatabaseRebaseline
  # Uses the real Rust wrapper with explicit, disposable inputs; never bootstraps the app.
  module IngestionPool
    POOL_STEPS = %w[cold warm-committed invalid-request after-error].freeze
    POOL_SESSION_KEYS = %w[pid database session_role current_role superuser create_role bypass_rls server_version conflict_setting].sort.freeze
    POOL_ROW_KEYS = %w[canonical source observation_created durable_source_created canonical_changed].sort.freeze
    POOL_SOURCE_FILES = %w[
      crates/revaer-data/src/indexers/search_results.rs
      crates/revaer-data/src/indexers/search_results/pool_proof_tests.rs
      crates/revaer-data/src/error.rs crates/revaer-data/Cargo.toml
      Cargo.toml Cargo.lock rust-toolchain.toml just/database.just
      scripts/tests/database-ingestion-pool-test.rb scripts/tests/database-ingestion-pool-test.sh
      scripts/tests/database-ingestion-proof-live.rb
    ].freeze

    private

    def verify_ingestion_pool!
      directory = File.join(@contract.output_path, "ingestion-pool")
      raise Failure, "pool evidence directory must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @pool_evidence = Dir.mktmpdir("run-", directory)
      first = @checks.length
      hashes = pool_source_hashes
      cases = []
      completed = false
      begin
        endpoint = @runner.run!(["docker", "port", @container, "5432/tcp"]).strip
        match = endpoint.match(/\A127\.0\.0\.1:([1-9][0-9]{0,4})\z/)
        raise Failure, "pool proof requires one owned loopback endpoint" unless match && Integer(match[1]) <= 65_535

        @runner.run!(["just", "--command", "rustc", "--version", "--verbose"], chdir: @contract.root).then do |identity|
          File.binwrite(File.join(@pool_evidence, "rust-toolchain.txt"), identity)
        end
        { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
          cases << pool_isolated(variant, source, role, Integer(match[1]))
        end
        check("pool wrapper source bytes unchanged during live pair", hashes == pool_source_hashes)
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |item| item.fetch(:passed) }, d3_complete: false,
                   scope: "actual Rust search_result_ingest wrapper, one-connection PgPool, cold/committed/error recovery",
                   limits: "Not all pool sizes, cancellation, helper/native closure, in-call observations, full D3 or installed-service proof.",
                   postgres_image: @contract.postgres_image, candidate_sha256: @contract.expected_candidate_sha256,
                   final_sha256: @contract.final_sha256, source_sha256: hashes, cases:, checks: }
        File.binwrite(File.join(@pool_evidence, "report.json"), JSON.pretty_generate(report) + "\n")
      end
      raise Failure, "Rust pool qualification failed; retained evidence" unless @checks.drop(first).all? { |item| item.fetch(:passed) }
    end

    def pool_source_hashes
      wrapper_source_hashes.merge(POOL_SOURCE_FILES.to_h do |path|
        [path, Digest::SHA256.file(File.join(@contract.root, path)).hexdigest]
      end)
    end

    def pool_isolated(variant, source, role, port)
      database = "ingestion_pool_#{variant}_#{SecureRandom.hex(6)}"
      prefix = File.join(@pool_evidence, variant)
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql"))
        sql("BEGIN;\n#{seed}\nCOMMIT;", role: "postgres", database:)
        before = ingestion_snapshot(database)
        input = { database_url: "postgresql://#{role}@127.0.0.1:#{port}/#{database}", database:, role:, report_path: "#{prefix}-frames.json" }
        input_path = "#{prefix}-input.json"
        File.open(input_path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(JSON.generate(input)) }
        result = @runner.capture(["just", "db-init-pool-probe"], env: { "REVAER_INGESTION_POOL_PROOF" => input_path }, chdir: @contract.root)
        File.binwrite("#{prefix}.stdout", result.stdout)
        File.binwrite("#{prefix}.stderr", result.stderr)
        raise Failure, "Rust pool probe did not complete" unless result.success && File.size?(input.fetch(:report_path))
        raise Failure, "Rust pool probe emitted a warning" if result.stderr.match?(/\bWARN\b|\bwarning:/i)

        frames = metadata_json_parse(File.binread(input.fetch(:report_path)))
        after = ingestion_snapshot(database)
        evidence = { "frames" => frames, "before" => before, "after" => after }
        File.binwrite("#{prefix}.json", JSON.pretty_generate(evidence) + "\n")
        pool_validate!(evidence, variant, database, role)
        { variant:, database:, role:, frames_path: input.fetch(:report_path) }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def pool_validate!(evidence, variant, database, role)
      raise Failure, "pool comparison variant changed" unless %w[reference final].include?(variant)
      %w[before after].each do |boundary|
        tables = evidence.fetch(boundary)
        raise Failure, "pool table evidence is incomplete" unless tables.keys.sort == IngestionProof::INGESTION_TABLES.sort && tables.values.all? { |rows| rows.is_a?(Array) }
      end
      frames = evidence.fetch("frames")
      raise Failure, "pool probe step matrix changed" unless frames.is_a?(Array) && frames.map { |frame| frame.fetch("name") } == POOL_STEPS &&
        frames.all? { |frame| frame.keys.sort == %w[after before name outcome] }

      first_pid = frames.first.fetch("before").fetch("pid")
      sessions = frames.flat_map { |frame| [frame.fetch("before"), frame.fetch("after")] }
      valid_sessions = sessions.all? do |session|
        session.keys.sort == POOL_SESSION_KEYS && first_pid.is_a?(Integer) && first_pid.positive? &&
          session.values_at("pid", "database", "session_role", "current_role", "server_version") == [first_pid, database, role, role, "160014"] &&
          session.values_at("superuser", "create_role", "bypass_rls") == [variant == "reference"] * 3
      end
      check("pool #{variant} reuses one direct-role backend at every boundary", valid_sessions)
      check("pool #{variant} preserves cold and restored compiler settings",
            sessions.first.fetch("conflict_setting").nil? && sessions.drop(1).all? { |session| session.fetch("conflict_setting") == "error" })
      expected = variant == "reference" ? %w[00000 42P07 P0001 42P07] : %w[00000 00000 P0001 00000]
      identity = frames.first.fetch("outcome").fetch("row").values_at("canonical", "source")
      frames.zip(expected).each_with_index do |(frame, state), index|
        value = frame.fetch("outcome")
        if state == "00000"
          row = value.fetch("row")
          valid = value.keys.sort == %w[kind row] && value.fetch("kind") == "success" && row.keys.sort == POOL_ROW_KEYS &&
            identity.all? { |id| id.match?(IngestionProof::INGESTION_UUID) } && row.values_at("canonical", "source") == identity &&
            row.values_at("observation_created", "durable_source_created", "canonical_changed") == [index.zero?] * 3
        else
          message = state == "42P07" ? 'relation "tmp_policy_rules" already exists' : "Failed to ingest search result"
          detail = state == "42P07" ? nil : "search_request_not_found"
          valid = value.keys.sort == %w[detail kind message operation state] &&
            value.values_at("kind", "operation", "state", "message", "detail") == ["database_error", "search result ingest", state, message, detail]
        end
        check("pool #{variant} #{frame.fetch('name')} exact typed outcome", valid)
      end
      check("pool #{variant} starts without application writes", evidence.fetch("before").values.all?(&:empty?))
      after = evidence.fetch("after")
      canonical = after.fetch("canonical_torrent")
      source = after.fetch("canonical_torrent_source")
      observed = after.fetch("search_request_source_observation")
      check("pool #{variant} retains only its ingested identities",
            canonical.length == 1 && source.length == 1 && observed.length == 1 &&
            canonical.first.fetch("canonical_torrent_public_id") == identity.first &&
            source.first.fetch("canonical_torrent_source_public_id") == identity.last &&
            source.first.fetch("source_guid") == "pool-proof-source")
      minute = variant == "reference" ? "00" : "03"
      check("pool #{variant} committed last-seen matches successful calls only",
            source.first.fetch("last_seen_at") == "2026-09-10T00:#{minute}:00+00:00")
    rescue KeyError, NoMethodError, TypeError => error
      raise Failure, "malformed Rust pool evidence: #{error.class}"
    end
  end
end
