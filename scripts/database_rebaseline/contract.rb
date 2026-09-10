# frozen_string_literal: true

require "digest"

module RevaerDatabaseRebaseline
  Corpus = Data.define(:file_count, :last_file, :sha256)

  class Contract
    SHA256_PATTERN = /\A[0-9a-f]{64}\z/
    MIGRATION_PATTERN = /\A\d{4}_[a-z0-9_]+\.sql\z/
    IMAGE_PATTERN = /\Adocker\.io\/library\/postgres@sha256:[0-9a-f]{64}\z/
    VERSION_PATTERN = /\A\d+\.\d+\z/

    attr_reader :root, :config_path, :build_inputs_path, :migrations_path,
                :init_path, :output_path

    def initialize(root: ENV.fetch("REVAER_REBASELINE_ROOT", File.expand_path("../..", __dir__)))
      @root = File.expand_path(root)
      @config_path = File.join(@root, "config/database-rebaseline.env")
      @build_inputs_path = File.join(@root, ".github/build-inputs.env")
      @migrations_path = File.join(@root, "crates/revaer-data/migrations")
      @init_path = File.join(@root, "crates/revaer-data/init.sql")
      @output_path = File.join(@root, "target/database-rebaseline")
      @config = EnvFile.load(config_path)
      @build_inputs = EnvFile.load(build_inputs_path)
    end

    def freeze!
      validate_tool_contract!
      expected_count = positive_integer("MIGRATION_FILE_COUNT")
      expected_last = required_config("MIGRATION_LAST_FILE")
      expected_digest = sha256_config("MIGRATION_CORPUS_SHA256")
      actual = corpus

      failures = []
      failures << "migration file count is #{actual.file_count}, expected #{expected_count}" if actual.file_count != expected_count
      failures << "last migration is #{actual.last_file}, expected #{expected_last}" if actual.last_file != expected_last
      failures << "migration corpus SHA-256 is #{actual.sha256}, expected #{expected_digest}" if actual.sha256 != expected_digest
      unless failures.empty?
        raise Failure, "frozen migration corpus drifted:\n- #{failures.join("\n- ")}"
      end

      phase = transition_phase
      if phase == "freeze" && File.symlink?(init_path)
        raise Failure, "#{relative(init_path)} must remain absent during the freeze phase"
      end
      if phase == "freeze" && File.exist?(init_path)
        raise Failure, "#{relative(init_path)} must remain absent during the freeze phase"
      end
      if %w[assembly finalization].include?(phase) && !regular_file?(init_path)
        raise Failure, "#{relative(init_path)} is required during the #{phase} phase"
      end
      if phase == "finalization" && Digest::SHA256.file(init_path).hexdigest != final_sha256
        raise Failure, "final init SHA-256 does not match reviewed finalization bytes"
      end

      sha256_config("CANDIDATE_SHA256")
      positive_integer("CANDIDATE_STATEMENT_COUNT")
      positive_integer("STACK_CHANGED_LINE_MAX")
      positive_integer("INIT_ASSEMBLY_CHANGED_LINE_MAX")
      actual
    end

    def corpus
      entries = Dir.children(migrations_path).sort
      raise Failure, "frozen migration directory is empty" if entries.empty?

      digest = Digest::SHA256.new
      entries.each do |entry|
        unless entry.match?(MIGRATION_PATTERN)
          raise Failure, "unexpected migration entry: #{relative(File.join(migrations_path, entry))}"
        end
        path = File.join(migrations_path, entry)
        stat = File.lstat(path)
        raise Failure, "migration entry is not a regular file: #{relative(path)}" unless stat.file?

        relative_path = relative(path)
        digest << relative_path << "\0" << File.binread(path) << "\0"
      rescue Errno::ENOENT
        raise Failure, "migration entry disappeared during validation: #{entry}"
      end

      Corpus.new(
        file_count: entries.length,
        last_file: relative(File.join(migrations_path, entries.last)),
        sha256: digest.hexdigest
      )
    rescue Errno::ENOENT
      raise Failure, "frozen migration directory is missing: #{relative(migrations_path)}"
    end

    def transition_phase
      phase = required_config("TRANSITION_PHASE")
      return phase if %w[freeze assembly finalization].include?(phase)

      raise Failure, "unsupported or not-yet-implemented database transition phase: #{phase}"
    end

    def postgres_image
      image = required_build_input("POSTGRES_REBASELINE_IMAGE")
      return image if image.match?(IMAGE_PATTERN)

      raise Failure, "POSTGRES_REBASELINE_IMAGE must be a fully qualified sha256 digest"
    end

    def postgres_version
      version = required_build_input("POSTGRES_REBASELINE_VERSION")
      return version if version.match?(VERSION_PATTERN)

      raise Failure, "POSTGRES_REBASELINE_VERSION must be an exact major.minor version"
    end

    def expected_candidate_sha256
      sha256_config("CANDIDATE_SHA256")
    end

    def expected_statement_count
      positive_integer("CANDIDATE_STATEMENT_COUNT")
    end

    def final_sha256
      sha256_config("FINAL_INIT_SHA256")
    end

    def changed_line_limit(scope)
      key = case scope
            when "stack" then "STACK_CHANGED_LINE_MAX"
            when "init-assembly" then "INIT_ASSEMBLY_CHANGED_LINE_MAX"
            else raise Failure, "unknown changed-line scope: #{scope}"
            end
      positive_integer(key)
    end

    def candidate_path
      File.join(output_path, "init-candidate.sql")
    end

    def statement_map_path
      File.join(output_path, "statement-boundaries.tsv")
    end

    def evidence_path
      File.join(output_path, "evidence.env")
    end

    def validate_output_path!
      expected = File.join(root, "target/database-rebaseline")
      unless output_path == expected
        raise Failure, "rebaseline output must remain confined to #{relative(expected)}"
      end
      target_path = File.dirname(output_path)
      if File.symlink?(target_path) || File.symlink?(output_path)
        raise Failure, "rebaseline output path must not contain a symlink"
      end
    end

    def verify_candidate!(path = candidate_path)
      source = File.binread(path)
      verify_candidate_source!(source)
    rescue Errno::ENOENT
      raise Failure, "candidate is missing: #{relative(path)}"
    end

    def verify_candidate_source!(source)
      actual_sha = Digest::SHA256.hexdigest(source)
      statements = SqlStatements.new(source)
      failures = []
      failures << "candidate SHA-256 is #{actual_sha}, expected #{expected_candidate_sha256}" if actual_sha != expected_candidate_sha256
      if statements.count != expected_statement_count
        failures << "candidate statement count is #{statements.count}, expected #{expected_statement_count}"
      end
      raise Failure, failures.join("; ") unless failures.empty?

      statements
    end

    def verify_prefix!(candidate = candidate_path, prefix = init_path)
      statements = verify_candidate!(candidate)
      candidate_bytes = File.binread(candidate)
      prefix_bytes = File.binread(prefix)
      unless candidate_bytes.start_with?(prefix_bytes)
        raise Failure, "#{relative(prefix)} is not an exact prefix of the pinned candidate"
      end
      unless statements.complete_prefix?(prefix_bytes.bytesize)
        raise Failure, "#{relative(prefix)} does not end at a complete SQL statement boundary"
      end
      prefix_bytes
    rescue Errno::ENOENT => error
      raise Failure, "candidate prefix input is missing: #{error.path}"
    end

    def relative(path)
      expanded = File.expand_path(path)
      root_prefix = "#{root}/"
      expanded.start_with?(root_prefix) ? expanded.delete_prefix(root_prefix) : expanded
    end

    private

    def validate_tool_contract!
      postgres_image
      postgres_version
    end

    def required_config(key)
      required(@config, key, config_path)
    end

    def required_build_input(key)
      required(@build_inputs, key, build_inputs_path)
    end

    def required(values, key, path)
      value = values[key]
      return value if value && !value.empty?

      raise Failure, "#{key} is missing from #{relative(path)}"
    end

    def sha256_config(key)
      value = required_config(key)
      return value if value.match?(SHA256_PATTERN)

      raise Failure, "#{key} must be a lowercase SHA-256 digest"
    end

    def positive_integer(key)
      raw_value = required_config(key)
      value = Integer(raw_value, 10)
      return value if value.positive?

      raise Failure, "#{key} must be positive"
    rescue ArgumentError
      raise Failure, "#{key} must be a base-10 integer"
    end

    def regular_file?(path)
      File.lstat(path).file?
    rescue Errno::ENOENT
      false
    end
  end

  class ChangedLineGuard
    def initialize(contract, runner: CommandRunner.new)
      @contract = contract
      @runner = runner
    end

    def verify!(scope, base_ref, head_ref)
      base_sha = resolve_ref!(base_ref)
      head_sha = resolve_ref!(head_ref)
      output = @runner.run!(
        ["git", "diff", "--no-renames", "--numstat", base_sha, head_sha, "--"],
        chdir: @contract.root
      )
      additions, deletions = totals(output)
      changed_lines = additions + deletions
      maximum = @contract.changed_line_limit(scope)
      if changed_lines > maximum
        raise Failure, "#{scope} diff has #{changed_lines} changed lines; maximum is #{maximum}"
      end

      [additions, deletions, changed_lines, maximum]
    end

    private

    def resolve_ref!(ref)
      raise Failure, "Git diff reference must not be empty" if ref.empty?

      result = @runner.capture(
        ["git", "rev-parse", "--verify", "--quiet", "--end-of-options", "#{ref}^{commit}"],
        chdir: @contract.root
      )
      raise Failure, "Git diff reference does not resolve to a commit: #{ref}" unless result.success

      sha = result.stdout.strip
      return sha if sha.match?(/\A[0-9a-f]{40,64}\z/)

      raise Failure, "Git diff reference resolved to an invalid object ID: #{ref}"
    end

    def totals(output)
      output.lines(chomp: true).reduce([0, 0]) do |(additions, deletions), line|
        added, removed, path = line.split("\t", 3)
        unless added&.match?(/\A\d+\z/) && removed&.match?(/\A\d+\z/) && path && !path.empty?
          raise Failure, "Git reported a binary or uncountable changed-line entry: #{line}"
        end
        [additions + Integer(added, 10), deletions + Integer(removed, 10)]
      end
    end
  end
end
