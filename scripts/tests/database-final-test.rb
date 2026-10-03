# frozen_string_literal: true

require "tmpdir"
require_relative "../database-rebaseline"

module DatabaseFinalTest
  module_function

  def run
    contract = RevaerDatabaseRebaseline::Contract.new
    if contract.transition_phase == "feature-development"
      development_tests(contract)
      return
    end

    final = File.binread(contract.init_path)
    legacy = final.split(RevaerDatabaseRebaseline::FinalSql::MARKER, 2).first
    candidate = legacy.sub(RevaerDatabaseRebaseline::FinalSql::HEADER, RevaerDatabaseRebaseline::FinalSql::ASSEMBLY_HEADER)
    resets = RevaerDatabaseRebaseline::FinalSql::TIMEOUTS.map { |name| "SET #{name} = 0;\n" }.join
    candidate = candidate.sub("SET client_encoding", "#{resets}SET client_encoding")
    candidate = candidate.sub(RevaerDatabaseRebaseline::FinalSql::CONFLICT_DIRECTIVE, RevaerDatabaseRebaseline::FinalSql::CONFLICT_SETTING)
    candidate = candidate.sub(RevaerDatabaseRebaseline::FinalSql::RESET_SCOPED_HEADER, RevaerDatabaseRebaseline::FinalSql::RESET_HEADER)
    reset_start = "    base_rate_limit_message CONSTANT text := 'Failed to seed rate limit policies';\n    errcode CONSTANT text := 'P0001';\n    rec RECORD;\nBEGIN\n"
    candidate = candidate.sub(reset_start, reset_start + RevaerDatabaseRebaseline::FinalSql::RESET_LOCAL_CALL)
    candidate = candidate.sub("CREATE TEMP TABLE tmp_policy_rules ON COMMIT DROP AS", "CREATE TEMP TABLE tmp_policy_rules AS")
    %w[text int].each do |type|
      candidate = candidate.gsub("ON CONFLICT (canonical_torrent_id, id_type, id_value_#{type})\n        WHERE id_value_#{type} IS NOT NULL\n        DO UPDATE SET",
                                 "ON CONFLICT (canonical_torrent_id, id_type, id_value_#{type})\n        DO UPDATE SET")
    end
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
      count += ingestion_delta_tests(validator, candidate, final)
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

  def development_tests(contract)
    contract.freeze!
    validator = RevaerDatabaseRebaseline::FinalSql.new(contract)
    source = File.binread(contract.init_path)
    validator.verify_development!(source)
    count = 1
    forbidden = %w[BEGIN COMMIT ROLLBACK END VACUUM].map { |token| "#{token};" }
    forbidden.concat(["CREATE DATABASE unapproved;", "ALTER SYSTEM SET work_mem = '4MB';", "DISCARD ALL;"])
    %w[SET RESET].each do |command|
      (RevaerDatabaseRebaseline::FinalSql::TIMEOUTS + ["ALL"]).each do |name|
        suffix = command == "SET" && name != "ALL" ? " = 0" : ""
        ["", "LOCAL ", "SESSION "].each do |scope|
          forbidden << "#{command} #{scope}#{name}#{suffix};"
        end
      end
    end
    mutations = forbidden.map { |statement| source + "\n-- negative control\n#{statement}\n" }
    mutations.concat([source.sub(RevaerDatabaseRebaseline::FinalSql::HEADER, "-- unapproved header\n"),
                      RevaerDatabaseRebaseline::FinalSql::HEADER,
                      source + "SELECT 'unterminated;\n"])
    mutations.each_with_index do |mutated, index|
      begin
        validator.verify_development!(mutated)
      rescue RevaerDatabaseRebaseline::Failure
        count += 1
      else
        raise "invalid feature init mutation #{index} passed"
      end
    end
    puts "database-final-test: #{count} current-init assertions passed; historical pins unchanged"
  end

  def ingestion_delta_tests(validator, candidate, final)
    identity = "CREATE FUNCTION public.search_result_ingest_v1("
    start = final.index(identity)
    finish = final.index("\n$_$;", start)
    raise "missing ingestion envelope" unless start && finish

    body = final[start...finish]
    mutations = [
      final.sub("tmp_policy_rules ON COMMIT DROP AS", "tmp_policy_rules AS"),
      final.sub("tmp_policy_rules ON COMMIT DROP AS", "tmp_policy_rules ON COMMIT DELETE ROWS AS"),
      final.sub("tmp_policy_rules ON COMMIT DROP AS", "IF NOT EXISTS tmp_policy_rules ON COMMIT DROP AS"),
      final.sub("tmp_policy_rules ON COMMIT DROP AS", "tmp_policy_rules ON COMMIT DROP AS SELECT 1; CREATE TEMP TABLE unapproved AS"),
      final.sub("ON CONFLICT (canonical_torrent_id, id_type, id_value_text)\n", "ON CONFLICT (canonical_torrent_id, id_type, id_value_text)\n        WHERE id_value_text IS NOT NULL\n"),
      final.sub("ON CONFLICT (canonical_torrent_id, id_type, id_value_int)\n", "ON CONFLICT (canonical_torrent_id, id_type, id_value_int)\n        WHERE id_value_int IS NOT NULL\n")
    ]
    %w[imdb tmdb tvdb].each do |id|
      type = id == "imdb" ? "text" : "int"
      branch = body[/    IF #{id}_id_value IS NOT NULL THEN\n        INSERT INTO canonical_external_id .*?    END IF;/m]
      raise "missing #{id} ingestion branch" unless branch

      predicate = "        WHERE id_value_#{type} IS NOT NULL\n"
      ["", predicate.sub("IS NOT NULL", "IS NULL"), predicate * 2].each do |replacement|
        mutations << final.sub(branch, branch.sub(predicate, replacement))
      end
    end
    mutations.each_with_index do |mutated, index|
      raise "ingestion mutation #{index} did not change the fixture" if mutated == final

      begin
        validator.verify!(mutated)
      rescue RevaerDatabaseRebaseline::Failure => error
        raise "ingestion mutation escaped exact legacy guard" unless error.message == "final init has unauthorized legacy deltas"
      else
        raise "unauthorized ingestion mutation #{index} passed"
      end
    end
    # A changed pin cannot authorize a widened legacy delta; regenerate exact bytes.
    generated = validator.legacy(candidate)
    raise "generator changed SQL outside approved final prefix" unless final.start_with?(generated + RevaerDatabaseRebaseline::FinalSql::MARKER)

    start = candidate.index(identity)
    frozen_body = candidate[start...candidate.index("\n$_$;", start)]
    RevaerDatabaseRebaseline::FinalSql::INGESTION_DELTAS.each_key do |original|
      ["", original * 2].each do |replacement|
        begin
          validator.approved_ingestion_body(frozen_body.sub(original, replacement))
        rescue RevaerDatabaseRebaseline::Failure => error
          raise "wrong generator failure" unless error.message.include?("does not match exactly")
        else
          raise "missing or duplicated approved delta passed generation"
        end
      end
    end
    mutations.length + 7
  end
end

DatabaseFinalTest.run
