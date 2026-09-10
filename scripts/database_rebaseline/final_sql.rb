# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Transition-only byte proof. Lifecycle SQL lives exclusively in init.sql.
  class FinalSql
    HEADER = "-- Revaer pre-v1 packaged database baseline.\n"
    ASSEMBLY_HEADER = "-- Revaer pre-v1 init candidate. Assembly-only until ADR 522 cutover.\n"
    MARKER = "\n-- ADR 551 finalization: lifecycle and explicit authored routine privileges.\n"
    SECURITY_START = "-- Generated authored routine security begins.\n"
    SECURITY_END = "-- Generated authored routine security ends.\n"
    GRANTS_START = "        -- Generated authored routine grants begin.\n"
    GRANTS_END = "        -- Generated authored routine grants end.\n"
    TIMEOUTS = %w[statement_timeout lock_timeout idle_in_transaction_session_timeout].freeze
    CONFLICT_SETTING = "    SET \"plpgsql.variable_conflict\" TO 'use_column'\n    AS $_$\n"
    CONFLICT_DIRECTIVE = "    AS $_$\n#variable_conflict use_column\n"
    Routine = Data.define(:identity, :schema, :name, :trigger, :path)

    def initialize(contract)
      @contract = contract
    end

    def legacy(candidate)
      @contract.verify_candidate_source!(candidate)
      unless candidate.start_with?(ASSEMBLY_HEADER)
        raise Failure, "candidate assembly header is missing"
      end
      source = candidate.sub(ASSEMBLY_HEADER, HEADER)
      TIMEOUTS.each do |name|
        reset = "SET #{name} = 0;\n"
        raise Failure, "expected exactly one #{name} dump reset" unless source.scan(reset).length == 1

        source = source.sub(reset, "")
      end
      unless source.scan(CONFLICT_SETTING).length == 1
        raise Failure, "expected exactly one legacy variable-conflict setting"
      end
      source.sub(CONFLICT_SETTING, CONFLICT_DIRECTIVE)
    end

    def routines(candidate)
      @contract.verify_candidate_source!(candidate)
      objects = candidate.scan(/^CREATE (?:FUNCTION|TABLE|TYPE) public\.([a-z_0-9]+)/).flatten
      objects.concat(%w[digest gen_random_bytes unaccent])
      dependency = /(?<![\w.])(?:#{objects.uniq.map { |name| Regexp.escape(name) }.join('|')})\b/i
      offset = 0
      SqlStatements.new(candidate).boundaries.filter_map do |boundary|
        statement = candidate.byteslice(offset, boundary.byte_count - offset)
        offset = boundary.byte_count
        next unless statement.match?(/^CREATE FUNCTION /)

        identity = statement.match(/^-- Name: (.+); Type: FUNCTION; Schema: (public|revaer_config|revaer_runtime); Owner: -$/)
        definition = statement.match(/^CREATE FUNCTION (public|revaer_config|revaer_runtime)\.([a-z_0-9]+)\(.+?\) RETURNS (.+)\n/m)
        # Empty argument lists do not satisfy .+ above.
        definition ||= statement.match(/^CREATE FUNCTION (public|revaer_config|revaer_runtime)\.([a-z_0-9]+)\(\) RETURNS (.+)\n/m)
        raise Failure, "unrecognized authored routine envelope" unless identity && definition
        schema, name = definition.captures
        unless identity[2] == schema && identity[1].start_with?("#{name}(")
          raise Failure, "routine identity does not match its definition"
        end
        body = statement.split(/^    AS \$(?:\w*)\$/, 2).last
        raise Failure, "unrecognized authored routine body" if body == statement

        tokens = body.gsub(/--[^\n]*|\/\*.*?\*\/|'(?:[^']|'')*'/m, " ")
        Routine.new(
          identity: "#{schema}.#{identity[1]}", schema:, name:,
          trigger: statement.match?(/^CREATE FUNCTION .* RETURNS (?:event_)?trigger$/),
          path: tokens.match?(dependency) ? "pg_catalog, public" : "pg_catalog"
        )
      end
    end

    def security(candidate)
      lines = routines(candidate).map do |routine|
        security_mode = routine.trigger ? "" : " SECURITY DEFINER"
        "ALTER FUNCTION #{routine.identity}#{security_mode} SET search_path TO #{routine.path};\n"
      end
      "#{SECURITY_START}#{lines.join}#{SECURITY_END}"
    end

    def grants(candidate)
      lines = routines(candidate).reject(&:trigger).map do |routine|
        "            '#{routine.identity.gsub("'", "''")}'"
      end
      "#{GRANTS_START}#{lines.join(",\n")}\n#{GRANTS_END}"
    end

    def verify!(source = File.binread(@contract.init_path))
      candidate = File.binread(@contract.candidate_path)
      prefix, suffix = source.split(MARKER, 2)
      raise Failure, "final init has unauthorized legacy deltas" unless prefix == legacy(candidate) && suffix
      raise Failure, "final init security delta changed" unless suffix.include?(security(candidate))
      raise Failure, "final init explicit grants changed" unless suffix.include?(grants(candidate))
      unless Digest::SHA256.hexdigest(source) == @contract.final_sha256
        raise Failure, "final init SHA-256 does not match reviewed finalization bytes"
      end
      # Matching exact bytes is the authority; this check also makes unexpected
      # top-level timeout or transaction-control changes independently visible.
      offset = 0
      SqlStatements.new(source).boundaries.each do |boundary|
        statement = source.byteslice(offset, boundary.byte_count - offset)
        offset = boundary.byte_count
        token = statement.gsub(/^--[^\n]*\n/, "").strip
        if token.match?(/\A(?:BEGIN|COMMIT|ROLLBACK|END|VACUUM|CREATE DATABASE|ALTER SYSTEM|DISCARD)\b/i) ||
           token.match?(/\A(?:SET|RESET)\s+(?:LOCAL\s+|SESSION\s+)?(?:#{TIMEOUTS.join('|')}|ALL)\b/i)
          raise Failure, "final init contains forbidden transaction or timeout control"
        end
      end
      source
    rescue Errno::ENOENT
      raise Failure, "final init or frozen candidate evidence is missing"
    end

    def generate!
      candidate = File.binread(@contract.candidate_path)
      _, suffix = File.binread(@contract.init_path).split(MARKER, 2)
      raise Failure, "authored final lifecycle section is missing" unless suffix

      suffix = replace_section(suffix, SECURITY_START, SECURITY_END, security(candidate))
      suffix = replace_section(suffix, GRANTS_START, GRANTS_END, grants(candidate))
      File.binwrite(@contract.init_path, "#{legacy(candidate)}#{MARKER}#{suffix}")
    end

    private

    def replace_section(source, opening, closing, replacement)
      unless source.scan(opening).length == 1 && source.scan(closing).length == 1
        raise Failure, "generated final SQL section markers must be unique"
      end
      source.sub(/#{Regexp.escape(opening)}.*?#{Regexp.escape(closing)}/m, replacement)
    end
  end
end
