#!/usr/bin/env ruby
# frozen_string_literal: true

require_relative "database_rebaseline/support"
require_relative "database_rebaseline/sql_statements"
require_relative "database_rebaseline/contract"
require_relative "database_rebaseline/candidate_builder"

module RevaerDatabaseRebaseline
  class Cli
    def initialize(arguments, output: $stdout)
      @arguments = arguments.dup
      @output = output
    end

    def run
      command = @arguments.shift
      contract = Contract.new
      case command
      when "freeze"
        ensure_argument_count!(0)
        corpus = contract.freeze!
        @output.puts(
          "database-rebaseline: frozen #{corpus.file_count} migrations at #{corpus.sha256}"
        )
      when "generate"
        ensure_argument_count!(0)
        statements = CandidateBuilder.new(contract).generate!
        @output.puts(
          "database-rebaseline: candidate verified at #{contract.candidate_path} " \
          "(#{contract.expected_candidate_sha256}, #{statements.count} statements)"
        )
      when "prefix"
        ensure_argument_count!(0)
        prefix = contract.verify_prefix!
        @output.puts(
          "database-rebaseline: #{contract.relative(contract.init_path)} is a complete " \
          "#{prefix.bytesize}-byte candidate prefix"
        )
      when "changed-lines"
        ensure_argument_count!(3)
        scope, base_ref, head_ref = @arguments
        additions, deletions, total, maximum = ChangedLineGuard.new(contract).verify!(
          scope, base_ref, head_ref
        )
        @output.puts(
          "database-rebaseline: #{scope} diff has #{additions} additions and " \
          "#{deletions} deletions (#{total}/#{maximum})"
        )
      else
        raise Failure, usage
      end
      0
    rescue Failure, SystemCallError => error
      warn "database-rebaseline: #{error.message}"
      1
    end

    private

    def ensure_argument_count!(expected)
      return if @arguments.length == expected

      raise Failure, usage
    end

    def usage
      "usage: database-rebaseline.rb " \
        "{freeze|generate|prefix|changed-lines <stack|init-assembly> <base> <head>}"
    end
  end
end

exit RevaerDatabaseRebaseline::Cli.new(ARGV).run if $PROGRAM_NAME == __FILE__
