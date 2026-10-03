# frozen_string_literal: true

require "digest"
require "fileutils"
require "open3"
require "stringio"
require "tmpdir"

require_relative "../database_rebaseline/support"
require_relative "../database_rebaseline/sql_statements"
require_relative "../database_rebaseline/contract"
require_relative "../database_rebaseline/candidate_builder"

module DatabaseRebaselineTest
  IMAGE = "docker.io/library/postgres@sha256:#{'a' * 64}"
  VERSION = "16.14"

  class Assertions
    attr_reader :count

    def initialize
      @count = 0
    end

    def equal(expected, actual, label)
      @count += 1
      return if expected == actual

      raise "#{label}: expected #{expected.inspect}, got #{actual.inspect}"
    end

    def truthy(value, label)
      @count += 1
      raise "#{label}: expected a truthy value" unless value
    end

    def failure(pattern, label)
      @count += 1
      yield
    rescue RevaerDatabaseRebaseline::Failure => error
      return if error.message.match?(pattern)

      raise "#{label}: wrong failure #{error.message.inspect}"
    else
      raise "#{label}: expected failure"
    end
  end

  class Fixture
    attr_reader :root

    def initialize
      @root = Dir.mktmpdir("revaer-database-rebaseline-test.")
      FileUtils.mkdir_p(File.join(root, ".github"))
      FileUtils.mkdir_p(File.join(root, "config"))
      FileUtils.mkdir_p(File.join(root, "crates/revaer-data/migrations"))
      File.write(File.join(root, ".github/build-inputs.env"), build_inputs)
      File.write(migration_path("0001_initial.sql"), "CREATE TABLE item (id bigint PRIMARY KEY);\n")
      write_config
    end

    def close
      FileUtils.remove_entry(root)
    end

    def contract
      RevaerDatabaseRebaseline::Contract.new(root:)
    end

    def migration_path(name)
      File.join(root, "crates/revaer-data/migrations", name)
    end

    def init_path
      File.join(root, "crates/revaer-data/init.sql")
    end

    def config_path
      File.join(root, "config/database-rebaseline.env")
    end

    def write_config(
      phase: "freeze",
      candidate_sha: "0" * 64,
      statement_count: 1,
      stack_limit: 9,
      assembly_limit: 5,
      corpus_override: nil
    )
      corpus = corpus_override || calculate_corpus
      contents = <<~CONFIG
        TRANSITION_PHASE=#{phase}
        MIGRATION_FILE_COUNT=#{corpus.file_count}
        MIGRATION_LAST_FILE=#{corpus.last_file}
        MIGRATION_CORPUS_SHA256=#{corpus.sha256}
        CANDIDATE_SHA256=#{candidate_sha}
        CANDIDATE_STATEMENT_COUNT=#{statement_count}
        STACK_CHANGED_LINE_MAX=#{stack_limit}
        INIT_ASSEMBLY_CHANGED_LINE_MAX=#{assembly_limit}
      CONFIG
      File.write(config_path, contents)
    end

    def write_build_inputs(image: IMAGE, version: VERSION)
      File.write(
        File.join(root, ".github/build-inputs.env"),
        "POSTGRES_REBASELINE_IMAGE=#{image}\nPOSTGRES_REBASELINE_VERSION=#{version}\n"
      )
    end

    def pin_candidate(candidate, phase: "freeze")
      parser = RevaerDatabaseRebaseline::SqlStatements.new(candidate)
      write_config(
        phase:,
        candidate_sha: Digest::SHA256.hexdigest(candidate),
        statement_count: parser.count
      )
      FileUtils.mkdir_p(File.dirname(contract.candidate_path))
      File.binwrite(contract.candidate_path, candidate)
      parser
    end

    def git(*arguments)
      output, error, status = Open3.capture3("git", "-C", root, *arguments)
      raise "git #{arguments.join(' ')} failed: #{error}" unless status.success?

      output.strip
    end

    private

    def build_inputs
      "POSTGRES_REBASELINE_IMAGE=#{IMAGE}\nPOSTGRES_REBASELINE_VERSION=#{VERSION}\n"
    end

    def calculate_corpus
      digest = Digest::SHA256.new
      paths = Dir.children(File.join(root, "crates/revaer-data/migrations")).sort
      paths.each do |entry|
        relative = "crates/revaer-data/migrations/#{entry}"
        digest << relative << "\0" << File.binread(File.join(root, relative)) << "\0"
      end
      RevaerDatabaseRebaseline::Corpus.new(
        file_count: paths.length,
        last_file: "crates/revaer-data/migrations/#{paths.last}",
        sha256: digest.hexdigest
      )
    end
  end

  class FakeSleeper
    def self.sleep(_duration); end
  end

  class FakeRunner
    attr_reader :applied_sql, :commands

    def initialize(
      source_dump:,
      target_dump: source_dump,
      verify_dump: target_dump,
      versions: expected_versions,
      apply_failure: false,
      cleanup_failure: false
    )
      @source_dump = source_dump
      @target_dump = target_dump
      @verify_dump = verify_dump
      @versions = versions
      @apply_failure = apply_failure
      @cleanup_failure = cleanup_failure
      @dump_count = 0
      @applied_sql = []
      @commands = []
    end

    def capture(command, **_options)
      @commands << command
      if command.include?("SELECT 1") || command[0, 3] == ["docker", "rm", "-fv"]
        return result("1\n", "", true) if command.include?("SELECT 1")

        return result("", "", true)
      end
      result(run!(command), "", true)
    rescue RevaerDatabaseRebaseline::Failure => error
      result("", error.message, false)
    end

    def run!(command, stdin_data: nil, **_options)
      @commands << command
      return "sqlx-cli 0.8.6\n" if command == ["sqlx", "--version"]
      return @versions.join("\n") + "\n" if command[0, 3] == ["docker", "run", "--rm"]
      return "container-id\n" if command[0, 3] == ["docker", "run", "-d"]
      return "127.0.0.1:55432\n" if command[0, 2] == ["docker", "port"]
      return "" if command[0, 3] == ["sqlx", "migrate", "run"]
      return "" if command.include?("createdb")

      if command.include?("pg_dump")
        @dump_count += 1
        return @source_dump if @dump_count == 1
        return @target_dump if @dump_count == 2

        return @verify_dump
      end
      if command.include?("psql")
        raise RevaerDatabaseRebaseline::Failure, "fresh apply failed" if @apply_failure

        @applied_sql << stdin_data
        return ""
      end
      if command[0, 3] == ["docker", "rm", "-fv"]
        if @cleanup_failure
          raise RevaerDatabaseRebaseline::Failure, "container cleanup failed"
        end

        return ""
      end

      raise RevaerDatabaseRebaseline::Failure, "unexpected fake command: #{command.join(' ')}"
    end

    def self.expected_versions
      [
        "pg_dump (PostgreSQL) #{VERSION}",
        "psql (PostgreSQL) #{VERSION}",
        "postgres (PostgreSQL) #{VERSION}"
      ]
    end

    private

    def expected_versions
      self.class.expected_versions
    end

    def result(stdout, stderr, success)
      RevaerDatabaseRebaseline::CommandRunner::Result.new(stdout:, stderr:, success:)
    end
  end

  module_function

  def with_fixture
    fixture = Fixture.new
    yield fixture
  ensure
    fixture&.close
  end

  def dump_fixture(token: "abc", table: "item")
    <<~DUMP
      --
      -- PostgreSQL database dump
      --

      \\restrict #{token}

      -- Dumped from database version #{VERSION}
      -- Dumped by pg_dump version #{VERSION}

      CREATE TABLE public.#{table} (id bigint);

      CREATE FUNCTION revaer_config.factory_reset() RETURNS void
          LANGUAGE sql
          AS $$ SELECT NULL; $$;

      \\unrestrict #{token}
    DUMP
  end

  def test_freeze_guards(assertions)
    with_fixture do |fixture|
      assertions.equal(1, fixture.contract.freeze!.file_count, "frozen corpus passes")
      pinned = fixture.contract.corpus

      File.write(fixture.migration_path("0002_new.sql"), "SELECT 1;\n")
      assertions.failure(/frozen migration corpus drifted/, "new migration rejected") do
        fixture.contract.freeze!
      end
      FileUtils.rm_f(fixture.migration_path("0002_new.sql"))

      File.write(fixture.migration_path("0001_initial.sql"), "SELECT 2;\n")
      assertions.failure(/SHA-256/, "modified migration rejected") { fixture.contract.freeze! }
      File.write(fixture.migration_path("0001_initial.sql"), "CREATE TABLE item (id bigint PRIMARY KEY);\n")

      FileUtils.rm_f(fixture.migration_path("0001_initial.sql"))
      assertions.failure(/empty/, "removed migration rejected") { fixture.contract.freeze! }
      File.write(fixture.migration_path("0001_initial.sql"), "CREATE TABLE item (id bigint PRIMARY KEY);\n")
      fixture.write_config(corpus_override: pinned)

      File.write(fixture.migration_path("README.md"), "not a migration\n")
      assertions.failure(/unexpected migration entry/, "nonnumeric entry rejected") do
        fixture.contract.freeze!
      end
      FileUtils.rm_f(fixture.migration_path("README.md"))

      FileUtils.rm_f(fixture.migration_path("0001_initial.sql"))
      File.symlink("missing.sql", fixture.migration_path("0001_initial.sql"))
      assertions.failure(/not a regular file/, "migration symlink rejected") do
        fixture.contract.freeze!
      end
      FileUtils.rm_f(fixture.migration_path("0001_initial.sql"))
      File.write(fixture.migration_path("0001_initial.sql"), "CREATE TABLE item (id bigint PRIMARY KEY);\n")

      File.write(fixture.init_path, "SELECT 1;\n")
      assertions.failure(/must remain absent/, "freeze rejects init.sql") { fixture.contract.freeze! }
      FileUtils.rm_f(fixture.init_path)
      File.symlink("missing-init.sql", fixture.init_path)
      assertions.failure(/must remain absent/, "freeze rejects dangling init symlink") do
        fixture.contract.freeze!
      end
      FileUtils.rm_f(fixture.init_path)
      File.write(fixture.init_path, "SELECT 1;\n")
      fixture.write_config(phase: "assembly")
      assertions.equal("assembly", fixture.contract.transition_phase, "assembly phase accepted")
      FileUtils.rm_f(fixture.init_path)
      assertions.failure(/required during the assembly/, "assembly requires init.sql") do
        fixture.contract.freeze!
      end
      fixture.write_config(phase: "cutover")
      assertions.failure(/not-yet-implemented/, "unimplemented phase rejected") do
        fixture.contract.freeze!
      end
    end
  end

  def test_feature_development_guards(assertions)
    with_fixture do |fixture|
      fixture.write_config(phase: "feature-development")
      assertions.failure(/required during the feature-development/, "feature init must exist") do
        fixture.contract.freeze!
      end
      File.write(fixture.init_path, "SELECT 1;\n")
      assertions.equal(1, fixture.contract.freeze!.file_count, "approved feature init accepted")
      File.write(fixture.init_path, "SELECT 2;\n")
      assertions.equal(1, fixture.contract.freeze!.file_count, "feature changes do not require parity")
      File.write(fixture.init_path, "-- no statements\n")
      assertions.failure(/no complete statements/, "empty feature init rejected") do
        fixture.contract.freeze!
      end
      FileUtils.rm_f(fixture.init_path)
      File.symlink(fixture.migration_path("0001_initial.sql"), fixture.init_path)
      assertions.failure(/required during the feature-development/, "feature init symlink rejected") do
        fixture.contract.freeze!
      end
      FileUtils.rm_f(fixture.init_path)
      File.write(fixture.init_path, "SELECT 1;\n")
      File.write(fixture.migration_path("0002_new.sql"), "SELECT 1;\n")
      assertions.failure(/frozen migration corpus drifted/, "feature work cannot add migrations") do
        fixture.contract.freeze!
      end
    end
  end

  def test_tool_contract_guards(assertions)
    with_fixture do |fixture|
      fixture.write_build_inputs(image: "postgres:16-alpine")
      assertions.failure(/fully qualified sha256/, "floating image rejected") do
        fixture.contract.freeze!
      end
      fixture.write_build_inputs(version: "16")
      assertions.failure(/exact major.minor/, "partial PostgreSQL version rejected") do
        fixture.contract.freeze!
      end
      File.write(fixture.config_path, "TRANSITION_PHASE=freeze\nTRANSITION_PHASE=freeze\n")
      assertions.failure(/duplicate environment key/, "duplicate config key rejected") do
        fixture.contract
      end

      fixture.write_build_inputs
      fixture.write_config
      FileUtils.mkdir_p(File.join(fixture.root, "outside-target"))
      File.symlink(File.join(fixture.root, "outside-target"), File.join(fixture.root, "target"))
      assertions.failure(/must not contain a symlink/, "candidate output symlink rejected") do
        fixture.contract.validate_output_path!
      end
    end
  end

  def test_statement_and_prefix_guards(assertions)
    source = <<~SQL
      -- opening ; comment
      SELECT ';' AS value;
      /* outer ; /* inner ; */ done */
      CREATE FUNCTION f() RETURNS void AS $body$
      BEGIN
        PERFORM 1;
      END;
      $body$ LANGUAGE plpgsql;
      SELECT "semi;colon";
    SQL
    parser = RevaerDatabaseRebaseline::SqlStatements.new(source)
    assertions.equal(3, parser.count, "top-level statement count")
    assertions.truthy(parser.complete_prefix?(source.bytesize), "full source is a boundary")
    assertions.failure(/inside dollar quote/, "unterminated dollar quote rejected") do
      RevaerDatabaseRebaseline::SqlStatements.new("SELECT $tag$unterminated;\n")
    end
    assertions.failure(/no complete statements/, "statement-free SQL rejected") do
      RevaerDatabaseRebaseline::SqlStatements.new("-- only a comment\n")
    end

    with_fixture do |fixture|
      assertions.failure(
        /\Acandidate is missing: target\/database-rebaseline\/init-candidate\.sql\z/,
        "missing candidate reports the bounded operational diagnostic"
      ) do
        fixture.contract.verify_candidate!
      end
      assertions.failure(
        /\Acandidate is missing: target\/database-rebaseline\/alternate\.sql\z/,
        "missing explicit candidate reports the supplied relative path"
      ) do
        fixture.contract.verify_candidate!(
          File.join(fixture.contract.output_path, "alternate.sql")
        )
      end
      candidate = "-- candidate\nSELECT 1;\nSELECT $$a;b$$;\n"
      fixture.pin_candidate(candidate)
      File.binwrite(fixture.init_path, "-- candidate\nSELECT 1;\n")
      assertions.equal(23, fixture.contract.verify_prefix!.bytesize, "complete prefix accepted")

      File.binwrite(fixture.init_path, "-- candidate\nSELECT")
      assertions.failure(/statement boundary/, "partial statement rejected") do
        fixture.contract.verify_prefix!
      end
      File.binwrite(fixture.init_path, "-- divergent\nSELECT 1;\n")
      assertions.failure(/not an exact prefix/, "divergent prefix rejected") do
        fixture.contract.verify_prefix!
      end
      File.binwrite(fixture.contract.candidate_path, "SELECT 9;\n")
      assertions.failure(/candidate SHA-256/, "candidate digest drift rejected") do
        fixture.contract.verify_candidate!
      end
    end
  end

  def test_candidate_builder_guards(assertions)
    with_fixture do |fixture|
      runner = FakeRunner.new(source_dump: dump_fixture)
      builder = RevaerDatabaseRebaseline::CandidateBuilder.new(
        fixture.contract, runner:, sleeper: FakeSleeper
      )
      assertions.failure(/candidate SHA-256/, "unpinned generated candidate rejected") do
        builder.generate!
      end

      assertions.truthy(
        !File.exist?(fixture.contract.candidate_path) &&
          !File.exist?(fixture.contract.evidence_path),
        "unpinned candidate leaves no artifacts"
      )
      candidate = runner.applied_sql.last
      parser = RevaerDatabaseRebaseline::SqlStatements.new(candidate)
      fixture.write_config(
        candidate_sha: Digest::SHA256.hexdigest(candidate),
        statement_count: parser.count
      )
      runner = FakeRunner.new(source_dump: dump_fixture)
      statements = RevaerDatabaseRebaseline::CandidateBuilder.new(
        fixture.contract, runner:, sleeper: FakeSleeper
      ).generate!
      assertions.equal(parser.count, statements.count, "pinned candidate accepted")
      assertions.equal(2, runner.applied_sql.length, "normalization and candidate databases applied")
      cleanup_commands = runner.commands.select { |command| command[0, 2] == ["docker", "rm"] }
      assertions.equal(1, cleanup_commands.length, "owned database removed exactly once")
      assertions.equal("-fv", cleanup_commands.first[2], "owned anonymous volume removed with database")
      assertions.truthy(
        runner.applied_sql.first.include?("SELECT revaer_config.factory_reset();"),
        "existing seed contract retained"
      )
      evidence = File.read(fixture.contract.evidence_path)
      assertions.truthy(evidence.include?("fresh_candidate_apply=passed"), "fresh apply evidence")
      assertions.truthy(evidence.include?("schema_redump_match=passed"), "schema redump evidence")

      mismatch = FakeRunner.new(
        source_dump: dump_fixture,
        verify_dump: dump_fixture(table: "different_item")
      )
      assertions.failure(/does not reproduce/, "schema redump mismatch rejected") do
        RevaerDatabaseRebaseline::CandidateBuilder.new(
          fixture.contract, runner: mismatch, sleeper: FakeSleeper
        ).generate!
      end

      apply_failure = FakeRunner.new(source_dump: dump_fixture, apply_failure: true)
      assertions.failure(/fresh apply failed/, "fresh database apply failure rejected") do
        RevaerDatabaseRebaseline::CandidateBuilder.new(
          fixture.contract, runner: apply_failure, sleeper: FakeSleeper
        ).generate!
      end

      bad_envelope = FakeRunner.new(source_dump: dump_fixture.sub("\\unrestrict abc\n", ""))
      assertions.failure(/restriction envelope changed/, "pg_dump envelope drift rejected") do
        RevaerDatabaseRebaseline::CandidateBuilder.new(
          fixture.contract, runner: bad_envelope, sleeper: FakeSleeper
        ).generate!
      end

      bad_versions = FakeRunner.new(
        source_dump: dump_fixture,
        versions: ["pg_dump (PostgreSQL) 15.0"]
      )
      assertions.failure(/tool versions do not match/, "runtime tool version drift rejected") do
        RevaerDatabaseRebaseline::CandidateBuilder.new(
          fixture.contract, runner: bad_versions, sleeper: FakeSleeper
        ).generate!
      end

      cleanup_failure = FakeRunner.new(source_dump: dump_fixture, cleanup_failure: true)
      assertions.failure(/container cleanup failed/, "container cleanup failure rejected") do
        RevaerDatabaseRebaseline::CandidateBuilder.new(
          fixture.contract, runner: cleanup_failure, sleeper: FakeSleeper
        ).generate!
      end

      combined_failure = FakeRunner.new(
        source_dump: dump_fixture,
        apply_failure: true,
        cleanup_failure: true
      )
      assertions.failure(
        /fresh apply failed; disposable container cleanup also failed: container cleanup failed/,
        "primary and cleanup failures retained"
      ) do
        RevaerDatabaseRebaseline::CandidateBuilder.new(
          fixture.contract, runner: combined_failure, sleeper: FakeSleeper
        ).generate!
      end
    end
  end

  def test_changed_line_guards(assertions)
    with_fixture do |fixture|
      fixture.git("init", "-q")
      fixture.git("config", "user.name", "Revaer Test")
      fixture.git("config", "user.email", "revaer-test@example.invalid")
      File.write(File.join(fixture.root, "tracked.txt"), "one\ntwo\nthree\nfour\n")
      fixture.git("add", ".")
      fixture.git("commit", "-q", "-m", "base")
      base = fixture.git("rev-parse", "HEAD")

      File.write(File.join(fixture.root, "small.txt"), "one\ntwo\nthree\n")
      fixture.git("add", "small.txt")
      fixture.git("commit", "-q", "-m", "small")
      small = fixture.git("rev-parse", "HEAD")
      result = RevaerDatabaseRebaseline::ChangedLineGuard.new(fixture.contract).verify!(
        "init-assembly", base, small
      )
      assertions.equal([3, 0, 3, 5], result, "bounded text diff accepted")

      fixture.git("mv", "tracked.txt", "renamed.txt")
      fixture.git("commit", "-q", "-m", "rename")
      renamed = fixture.git("rev-parse", "HEAD")
      assertions.failure(/8 changed lines/, "rename cannot evade line accounting") do
        RevaerDatabaseRebaseline::ChangedLineGuard.new(fixture.contract).verify!(
          "init-assembly", small, renamed
        )
      end

      File.binwrite(File.join(fixture.root, "binary.bin"), "\x00\x01\x02".b)
      fixture.git("add", "binary.bin")
      fixture.git("commit", "-q", "-m", "binary")
      binary = fixture.git("rev-parse", "HEAD")
      assertions.failure(/binary or uncountable/, "binary diff rejected") do
        RevaerDatabaseRebaseline::ChangedLineGuard.new(fixture.contract).verify!(
          "stack", renamed, binary
        )
      end
      assertions.failure(/does not resolve/, "invalid diff ref rejected") do
        RevaerDatabaseRebaseline::ChangedLineGuard.new(fixture.contract).verify!(
          "stack", "missing-ref", binary
        )
      end
      assertions.failure(/unknown changed-line scope/, "unknown line scope rejected") do
        fixture.contract.changed_line_limit("other")
      end
    end
  end

  def run
    assertions = Assertions.new
    test_freeze_guards(assertions)
    test_feature_development_guards(assertions)
    test_tool_contract_guards(assertions)
    test_statement_and_prefix_guards(assertions)
    test_candidate_builder_guards(assertions)
    test_changed_line_guards(assertions)
    puts "database-rebaseline-test: #{assertions.count} focused assertions passed"
  end
end

DatabaseRebaselineTest.run
