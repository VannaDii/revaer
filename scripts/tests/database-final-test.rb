# frozen_string_literal: true

require "tmpdir"
require_relative "../database-rebaseline"

module DatabaseFinalTest
  module_function

  def run
    contract = RevaerDatabaseRebaseline::Contract.new
    final = File.binread(contract.init_path)
    legacy = final.split(RevaerDatabaseRebaseline::FinalSql::MARKER, 2).first
    candidate = legacy.sub(RevaerDatabaseRebaseline::FinalSql::HEADER, RevaerDatabaseRebaseline::FinalSql::ASSEMBLY_HEADER)
    resets = RevaerDatabaseRebaseline::FinalSql::TIMEOUTS.map { |name| "SET #{name} = 0;\n" }.join
    candidate = candidate.sub("SET client_encoding", "#{resets}SET client_encoding")
    candidate = candidate.sub(RevaerDatabaseRebaseline::FinalSql::CONFLICT_DIRECTIVE, RevaerDatabaseRebaseline::FinalSql::CONFLICT_SETTING)
    candidate = candidate.sub(RevaerDatabaseRebaseline::FinalSql::RESET_SCOPED_HEADER, RevaerDatabaseRebaseline::FinalSql::RESET_HEADER)
    reset_start = "    base_rate_limit_message CONSTANT text := 'Failed to seed rate limit policies';\n    errcode CONSTANT text := 'P0001';\n    rec RECORD;\nBEGIN\n"
    candidate = candidate.sub(reset_start, reset_start + RevaerDatabaseRebaseline::FinalSql::RESET_LOCAL_CALL)
    contract.verify_candidate_source!(candidate)
    count = 1
    Dir.mktmpdir("revaer-final-policy.") do |directory|
      candidate_path = File.join(directory, "candidate.sql")
      File.binwrite(candidate_path, candidate)
      proxy = contract.dup
      proxy.define_singleton_method(:candidate_path) { candidate_path }
      validator = RevaerDatabaseRebaseline::FinalSql.new(proxy)
      validator.verify!(final)
      count += 1
      mutations = [
        final.sub("CREATE TABLE public.app_user", "CREATE TABLE public.unapproved_user"),
        final.sub("-- Revaer pre-v1 packaged database baseline.", "-- unapproved header"),
        final.sub("SET client_encoding", "SET lock_timeout = 0;\nSET client_encoding"),
        final.sub("#variable_conflict use_column\nDECLARE\n    base_message CONSTANT text := 'Failed to ingest search result'", "#variable_conflict use_variable\nDECLARE\n    base_message CONSTANT text := 'Failed to ingest search result'"),
        final.sub("    SET lock_timeout TO '5s'\n", ""),
        final.sub("    SET lock_timeout TO '5s'\n", "    SET lock_timeout TO '6s'\n"),
        final.sub(reset_start, reset_start + RevaerDatabaseRebaseline::FinalSql::RESET_LOCAL_CALL),
        final.sub("SECURITY DEFINER SET search_path TO pg_catalog", "SECURITY INVOKER SET search_path TO pg_catalog"),
        final.sub("SECURITY DEFINER SET search_path TO pg_catalog", "SECURITY DEFINER SET search_path TO pg_catalog, pg_temp"),
        final.sub("'public.app_user_create(character varying, character varying)'", "'public.revaer_touch_updated_at()'"),
        final.sub("'public.app_user_create(character varying, character varying)'", "'public.digest(text, text)'"),
        final.sub("GRANT CONNECT ON DATABASE", "GRANT ALL ON DATABASE"),
        final.sub("CONSTRAINT database_baseline_contract_v1 CHECK (contract_version = 1)", "CONSTRAINT database_baseline_contract_v1 CHECK (contract_version > 0)"),
        final + "COMMIT;\n",
        final + "RESET ALL;\n"
      ]
      mutations.each_with_index do |mutated, index|
        raise "mutation #{index} did not change the fixture" if mutated == final

        begin
          validator.verify!(mutated)
        rescue RevaerDatabaseRebaseline::Failure
          count += 1
        else
          raise "unauthorized finalization mutation #{index} passed"
        end
      end
      File.binwrite(candidate_path, candidate + "SELECT 1;\n")
      begin
        validator.verify!(final)
      rescue RevaerDatabaseRebaseline::Failure
        count += 1
      else
        raise "unfrozen candidate passed finalization"
      end
    end
    puts "database-final-test: #{count} exact-delta assertions passed"
  end
end

DatabaseFinalTest.run
