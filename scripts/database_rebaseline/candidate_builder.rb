# frozen_string_literal: true

require "digest"
require "fileutils"
require "securerandom"

module RevaerDatabaseRebaseline
  class CandidateBuilder
    DATABASE_USER = "revaer_rebaseline"
    SOURCE_DATABASE = "revaer_source"
    NORMALIZATION_DATABASE = "revaer_normalization"
    CANDIDATE_DATABASE = "revaer_candidate"
    PREFIX_DATABASE = "revaer_prefix"
    SQLX_VERSION = "0.8.6"
    RESTRICT_PATTERN = /\A\\(?:un)?restrict [A-Za-z0-9]+\s*\z/
    FACTORY_RESET_PATTERN = /CREATE (?:OR REPLACE )?FUNCTION revaer_config\.factory_reset\(\)/
    TRANSIENT_DATABASE_PATTERN =
      /connection refused|unexpected response from SSLRequest|got 0 bytes at EOF|database system is starting up/i

    def initialize(contract, runner: CommandRunner.new, sleeper: Kernel)
      @contract = contract
      @runner = runner
      @sleeper = sleeper
    end

    def generate!
      corpus = @contract.freeze!
      verify_local_tools!
      prepare_output!
      password = SecureRandom.hex(24)
      container = "revaer-rebaseline-#{Process.pid}-#{SecureRandom.hex(6)}"
      container_started = false
      primary_error = nil
      cleanup_error = nil
      result = nil

      begin
        start_container!(container, password)
        container_started = true
        wait_until_ready!(container)
        port = published_port(container)
        migrate_source!(port, password)
        source_schema = normalized_schema_dump(container, SOURCE_DATABASE)
        normalization_candidate = build_candidate(source_schema, corpus)
        create_database!(container, NORMALIZATION_DATABASE)
        apply_sql!(container, NORMALIZATION_DATABASE, normalization_candidate)
        normalized_schema = normalized_schema_dump(container, NORMALIZATION_DATABASE)
        candidate = build_candidate(normalized_schema, corpus)
        create_database!(container, CANDIDATE_DATABASE)
        apply_sql!(container, CANDIDATE_DATABASE, candidate)
        verified_schema = normalized_schema_dump(container, CANDIDATE_DATABASE)
        unless verified_schema == normalized_schema
          raise Failure, schema_mismatch_message(normalized_schema, verified_schema)
        end

        statements = @contract.verify_candidate_source!(candidate)
        write_artifacts!(candidate, corpus, source_schema, normalized_schema, statements)
        verify_optional_prefix!(container)
        result = statements
      rescue StandardError => error
        primary_error = error
      ensure
        if container_started
          begin
            remove_container!(container)
          rescue StandardError => error
            cleanup_error = error
          end
        end
      end

      if primary_error && cleanup_error
        raise Failure,
              "#{primary_error.message}; disposable container cleanup also failed: " \
              "#{cleanup_error.message}"
      end
      raise primary_error if primary_error
      raise cleanup_error if cleanup_error

      result
    end

    private

    def verify_local_tools!
      sqlx_version = @runner.run!(["sqlx", "--version"]).strip
      unless sqlx_version == "sqlx-cli #{SQLX_VERSION}"
        raise Failure, "sqlx version is #{sqlx_version.inspect}, expected sqlx-cli #{SQLX_VERSION}"
      end

      versions = @runner.run!(
        [
          "docker", "run", "--rm", @contract.postgres_image,
          "sh", "-eu", "-c", "pg_dump --version; psql --version; postgres --version"
        ]
      ).lines(chomp: true)
      expected = [
        "pg_dump (PostgreSQL) #{@contract.postgres_version}",
        "psql (PostgreSQL) #{@contract.postgres_version}",
        "postgres (PostgreSQL) #{@contract.postgres_version}"
      ]
      raise Failure, "pinned PostgreSQL tool versions do not match #{expected.join(', ')}" unless versions == expected
    end

    def prepare_output!
      @contract.validate_output_path!
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      File.chmod(0o700, @contract.output_path)
      [
        @contract.candidate_path,
        @contract.statement_map_path,
        @contract.evidence_path
      ].each { |path| FileUtils.rm_f(path) }
    end

    def start_container!(container, password)
      @runner.run!(
        [
          "docker", "run", "-d", "--name", container,
          "--shm-size", "1g", "-p", "127.0.0.1::5432",
          "-e", "POSTGRES_USER=#{DATABASE_USER}",
          "-e", "POSTGRES_PASSWORD=#{password}",
          "-e", "POSTGRES_DB=#{SOURCE_DATABASE}",
          "-e", "POSTGRES_INITDB_ARGS=--locale=C --encoding=UTF8 --data-checksums",
          "-e", "TZ=UTC", @contract.postgres_image
        ],
        redactions: [password]
      )
    end

    def wait_until_ready!(container)
      120.times do
        result = @runner.capture(
          [
            "docker", "exec", container, "psql", "--no-psqlrc", "--tuples-only", "--no-align",
            "-U", DATABASE_USER, "-d", SOURCE_DATABASE, "-c", "SELECT 1"
          ]
        )
        return if result.success && result.stdout.strip == "1"

        @sleeper.sleep(0.25)
      end
      raise Failure, "pinned PostgreSQL container did not become ready"
    end

    def published_port(container)
      output = @runner.run!(["docker", "port", container, "5432/tcp"]).strip
      match = output.match(/\A127\.0\.0\.1:(\d+)\z/)
      raise Failure, "PostgreSQL container reported an invalid host port" unless match

      match[1]
    end

    def migrate_source!(port, password)
      database_url = \
        "postgres://#{DATABASE_USER}:#{password}@127.0.0.1:#{port}/#{SOURCE_DATABASE}?sslmode=disable"
      command = ["sqlx", "migrate", "run", "--source", @contract.migrations_path]
      environment = { "DATABASE_URL" => database_url }
      120.times do |attempt|
        result = @runner.capture(command, env: environment, chdir: @contract.root)
        return if result.success

        break unless result.stderr.match?(TRANSIENT_DATABASE_PATTERN) && attempt < 119

        @sleeper.sleep(0.25)
      end
      @runner.run!(
        command,
        env: environment,
        chdir: @contract.root,
        redactions: [password, database_url]
      )
    end

    def schema_dump(container, database)
      @runner.run!(
        [
          "docker", "exec", container, "pg_dump",
          "-U", DATABASE_USER, "-d", database,
          "--schema-only", "--no-owner", "--no-tablespaces",
          "--exclude-table=public._sqlx_migrations"
        ]
      )
    end

    def normalized_schema_dump(container, database)
      dump = schema_dump(container, database)
      restriction_lines = dump.lines.count { |line| line.match?(RESTRICT_PATTERN) }
      unless restriction_lines == 2
        raise Failure, "pg_dump restriction envelope changed: expected 2 lines, found #{restriction_lines}"
      end

      normalized = dump.lines.reject { |line| line.match?(RESTRICT_PATTERN) }.join
      if normalized.match?(/^-- Name: _sqlx_migrations; Type:/) ||
         normalized.match?(/^CREATE TABLE public\._sqlx_migrations\b/)
        raise Failure, "schema dump contains the SQLx migration metadata table"
      end
      raise Failure, "schema dump contains source database identity" if normalized.include?(SOURCE_DATABASE)
      raise Failure, "schema dump contains source role identity" if normalized.include?(DATABASE_USER)
      unless normalized.include?("-- Dumped from database version #{@contract.postgres_version}") &&
             normalized.include?("-- Dumped by pg_dump version #{@contract.postgres_version}")
        raise Failure, "schema dump does not carry the pinned PostgreSQL version"
      end
      unless normalized.match?(FACTORY_RESET_PATTERN)
        raise Failure, "schema dump does not contain revaer_config.factory_reset()"
      end
      normalized
    end

    def build_candidate(schema, corpus)
      header = <<~HEADER
        -- Revaer pre-v1 init candidate. Assembly-only until ADR 522 cutover.
        -- Frozen migration corpus SHA-256: #{corpus.sha256}
        -- Generated with PostgreSQL #{@contract.postgres_version} from #{@contract.postgres_image}

      HEADER
      seed = <<~SEED

        -- Initialize the repository's existing canonical default state.
        SET search_path = public, revaer_config, revaer_runtime;
        SELECT revaer_config.factory_reset();
        RESET search_path;
      SEED
      "#{header}#{schema.rstrip}\n#{seed}"
    end

    def schema_mismatch_message(source_schema, target_schema)
      source_lines = source_schema.lines
      target_lines = target_schema.lines
      differing_index = [source_lines.length, target_lines.length].max.times.find do |index|
        source_lines[index] != target_lines[index]
      end
      line_number = differing_index ? differing_index + 1 : 0
      source_digest = Digest::SHA256.hexdigest(source_schema)
      target_digest = Digest::SHA256.hexdigest(target_schema)
      source_line = differing_index ? source_lines[differing_index].to_s.strip : ""
      target_line = differing_index ? target_lines[differing_index].to_s.strip : ""
      "fresh candidate schema does not reproduce the migrated source schema " \
        "(first difference line #{line_number}, source #{source_digest}, target #{target_digest}, " \
        "source line #{source_line.inspect}, target line #{target_line.inspect})"
    end

    def create_database!(container, database)
      @runner.run!(["docker", "exec", container, "createdb", "-U", DATABASE_USER, database])
    end

    def apply_sql!(container, database, sql)
      @runner.run!(
        [
          "docker", "exec", "-i", container, "psql", "--no-psqlrc",
          "--set", "ON_ERROR_STOP=1", "--single-transaction",
          "-U", DATABASE_USER, "-d", database
        ],
        stdin_data: sql
      )
    end

    def write_artifacts!(candidate, corpus, source_schema, normalized_schema, statements)
      candidate_sha = Digest::SHA256.hexdigest(candidate)
      source_schema_sha = Digest::SHA256.hexdigest(source_schema)
      normalized_schema_sha = Digest::SHA256.hexdigest(normalized_schema)
      write_private(@contract.candidate_path, candidate)
      statement_map = statements.boundaries.map do |boundary|
        [boundary.ordinal, boundary.byte_count, boundary.line_number].join("\t")
      end.join("\n")
      write_private(
        @contract.statement_map_path,
        "ordinal\tbyte_count\tline_number\n#{statement_map}\n"
      )
      evidence = <<~EVIDENCE
        candidate_path=#{@contract.relative(@contract.candidate_path)}
        candidate_sha256=#{candidate_sha}
        candidate_statement_count=#{statements.count}
        migration_file_count=#{corpus.file_count}
        migration_corpus_sha256=#{corpus.sha256}
        postgres_image=#{@contract.postgres_image}
        postgres_version=#{@contract.postgres_version}
        migrated_schema_sha256=#{source_schema_sha}
        normalized_schema_sha256=#{normalized_schema_sha}
        normalization_apply=passed
        fresh_candidate_apply=passed
        schema_redump_match=passed
      EVIDENCE
      write_private(@contract.evidence_path, evidence)
    end

    def write_private(path, contents)
      File.binwrite(path, contents)
      File.chmod(0o600, path)
    end

    def verify_optional_prefix!(container)
      return unless @contract.transition_phase == "assembly"

      prefix = @contract.verify_prefix!
      create_database!(container, PREFIX_DATABASE)
      apply_sql!(container, PREFIX_DATABASE, prefix)
    end

    def remove_container!(container)
      @runner.run!(["docker", "rm", "-f", container])
    end
  end
end
