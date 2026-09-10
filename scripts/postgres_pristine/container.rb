# frozen_string_literal: true

require "json"
require "open3"
require "securerandom"
require "tempfile"

module RevaerPostgresPristine
  class Failure < StandardError; end

  # Owns only a newly created, network-isolated fixture container, never an operator database.
  class Container
    attr_reader :image, :version, :identity

    def initialize(root)
      inputs = File.readlines(File.join(root, ".github/build-inputs.env"), chomp: true)
      @image = input(inputs, "POSTGRES_REBASELINE_IMAGE")
      @version = input(inputs, "POSTGRES_REBASELINE_VERSION")
      unless @version == "16.14" && @image.match?(%r{\Adocker.io/library/postgres@sha256:[a-f0-9]{64}\z})
        raise Failure, "pristine PostgreSQL input is not the approved digest-qualified version"
      end
      @name = "revaer-pristine-#{Process.pid}-#{SecureRandom.hex(6)}"
    end

    def with_running
      primary_error = nil
      cleanup_error = nil
      begin
        create!
        command!(%w[docker start] + [@id], "container start")
        ready!
        verify_tools!
        yield self
      rescue StandardError => error
        primary_error = error
      ensure
        if @id
          begin
            command!(%w[docker rm --force --volumes] + [@id], "container cleanup")
          rescue StandardError => error
            cleanup_error = error
          end
        end
      end
      raise Failure, "pristine operation and container cleanup both failed" if primary_error && cleanup_error
      raise primary_error if primary_error
      raise cleanup_error if cleanup_error
    end

    def provision(suffix)
      raise Failure, "invalid fixture identity" unless suffix.match?(/\A[a-z0-9_]+\z/)

      owner = "pristine_owner_#{suffix}"
      database = "pristine_database_#{suffix}"
      sql!("CREATE ROLE #{owner} LOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE NOREPLICATION NOBYPASSRLS;" \
           "CREATE DATABASE #{database} OWNER #{owner} TEMPLATE template0 ENCODING 'UTF8' LC_COLLATE 'C' LC_CTYPE 'C';")
      [database, owner]
    end

    def sql!(sql, database: "postgres", user: "postgres")
      command!(["docker", "exec", "-i", @id, "psql", "--no-psqlrc", "--quiet", "--tuples-only",
                "--no-align", "--set", "ON_ERROR_STOP=1", "--set", "VERBOSITY=sqlstate",
                "-U", user, "-d", database], "catalog SQL", stdin_data: sql)
    end

    def command!(arguments, stage, stdin_data: nil)
      output, error, status = Open3.capture3(*arguments, stdin_data: stdin_data.to_s)
      unless status.success?
        sqlstate = error[/ERROR:\s+([A-Z0-9]{5})(?:\s|$)/, 1]
        line = error[/psql:<stdin>:(\d+):/, 1]
        raise Failure, "pristine #{stage} failed#{sqlstate ? " SQLSTATE=#{sqlstate}" : ''}#{line ? " line=#{line}" : ''}; emitted_records=#{output.lines.count}"
      end

      output
    end

    private

    def input(lines, key)
      entries = lines.select { |line| line.start_with?("#{key}=") }
      raise Failure, "missing or duplicate PostgreSQL build input" unless entries.length == 1

      entries.first.split("=", 2).last
    end

    def create!
      Tempfile.create("revaer-pristine-env") do |file|
        file.chmod(0o600)
        file.write("POSTGRES_PASSWORD=#{SecureRandom.hex(32)}\n")
        file.flush
        @id = command!(["docker", "create", "--name", @name, "--network", "none", "--shm-size", "1g",
                       "--env-file", file.path, "-e", "POSTGRES_INITDB_ARGS=--locale=C --encoding=UTF8 --data-checksums",
                       "-e", "TZ=UTC", image, "postgres", "-c", "timezone=UTC", "-c", "wal_level=logical"],
                      "container create").strip
      end
      raise Failure, "invalid created container identity" unless @id.match?(/\A[a-f0-9]{64}\z/)

      @identity = JSON.parse(command!(["docker", "image", "inspect", image], "image identity")).fetch(0)
        .slice("Id", "RepoDigests", "Architecture", "Os")
    end

    def ready!
      120.times do
        _output, _error, status = Open3.capture3("docker", "exec", @id, "pg_isready", "-h", "127.0.0.1", "-U", "postgres")
        return if status.success?

        sleep(0.25)
      end
      raise Failure, "pristine container readiness deadline exceeded"
    end

    def verify_tools!
      { "psql" => "psql (PostgreSQL) #{version}", "postgres" => "postgres (PostgreSQL) #{version}" }.each do |tool, expected|
        actual = command!(["docker", "exec", @id, tool, "--version"], "tool identity").strip
        raise Failure, "pristine PostgreSQL tool version mismatch" unless actual == expected
      end
    end
  end
end
