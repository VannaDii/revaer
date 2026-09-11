# frozen_string_literal: true

module RevaerDatabaseRebaseline
  module IngestionCorrectionsTest
    private

    def correction_test_frames
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      tables = empty.merge(sample_value.fetch("after"))
      tables.fetch("canonical_torrent").first["imdb_id"] = "tt1234567"
      tables["canonical_external_id"] = [{
        "canonical_external_id_id" => 1, "canonical_torrent_id" => 1,
        "source_canonical_torrent_source_id" => 2, "id_type" => "imdb", "id_value_text" => "tt1234567",
        "first_seen_at" => "2026-09-10T00:00:00+00:00", "last_seen_at" => "2026-09-10T00:00:00+00:00"
      }]
      result = sample_value.fetch("results").first.merge("observation_created" => true, "durable_source_created" => true)
      first = {
        "backend" => "101", "role" => { "session" => @runtime, "current" => @runtime, "superuser" => false, "create_role" => false, "bypass_rls" => false },
        "clock" => "2026-09-10T00:00:00+00:00", "before" => "error", "after" => "error", "state" => "00000", "result" => result,
        "within" => "true", "outside" => "false", "tables_before" => empty, "tables_after" => tables, "tables_finish" => tables
      }
      second = Marshal.load(Marshal.dump(first))
      second["tables_before"] = Marshal.load(Marshal.dump(tables))
      second["clock"] = "2026-09-10T00:01:00+00:00"
      second.fetch("tables_after").fetch("canonical_external_id").first["last_seen_at"] = second.fetch("clock")
      second.fetch("result").merge!("observation_created" => false, "durable_source_created" => false, "canonical_changed" => false)
      [first, second]
    end

    def correction_test_stdout(frames)
      frames.map do |frame|
        IngestionCorrections::CORRECTION_RECORDS.map do |key|
          item = frame.fetch(key)
          encoded = %w[role clock tables_before tables_after tables_finish].include?(key) ? JSON.generate(item) : item
          result = key == "state" && frame.key?("result") ? "#{JSON.generate(frame.fetch('result'))}\n" : ""
          "#{result}#{key}:#{key == 'state' ? ' ' : ''}#{encoded}\n"
        end.join
      end.join
    end

    def correction_tests!
      cases = correction_cases
      assert(cases.length == 13, "D4/D5 focused case inventory changed")
      %w[imdb tmdb tvdb].each do |id|
        entry = cases.find { |item| item.fetch(:name) == "#{id}-upsert" }
        assert(correction_states(entry, "reference") == %w[42P10 42P10], "retain original #{id} conflict inference failure")
        assert(correction_states(entry, "final") == %w[00000 00000], "require both #{id} application successes")
      end
      warm = cases.first
      assert(correction_states(warm, "reference") == %w[00000 42P07], "retain original warm failure")
      same = cases.find { |item| item[:same_transaction] }
      assert(correction_states(same, "final") == %w[00000 42P07], "same-transaction repetition is not approved")
      query = correction_session(warm)
      assert(query.scan("BEGIN;").length == 2 && query.scan("COMMIT;").length == 2, "committed reuse must use two transactions")
      assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "D4 regression must not repair the tested backend")
      frames = correction_test_frames
      stdout = correction_test_stdout(frames)
      assert(correction_parse(stdout, "", warm) == frames, "retain exact correction records")
      rejected("missing or reordered") { correction_parse(stdout.sub("before:error\n", ""), "", warm) }
      rejected("missing or reordered") { correction_parse(stdout.sub("before:error\n", "before:error\nbefore:error\n"), "", warm) }
      rejected("unexpected stdout") { correction_parse(stdout + "extra\n", "", warm) }
      rejected("unexpected diagnostic") { correction_parse(stdout, "WARNING: unexpected\n", warm) }
      rejected("unexpected diagnostic") { correction_parse(stdout, sample_stderr, warm) }
      rejected("result framing") { correction_parse(stdout.sub("state: 00000", "state: P0001"), "", warm) }
      rejected("invalid JSON") { correction_parse(stdout.sub('role:{', 'role:broken{'), "", warm) }
      rejected("helper-first") { correction_parse(stdout, "", warm.merge(helpers: true)) }
      helpers = "helpers:#{JSON.generate(ingestion_helper_expectations)}\n"
      assert(correction_parse(helpers + stdout, "", warm.merge(helpers: true)) == frames, "verify helper-first known answers")
      rejected("missing or reordered") { correction_parse(helpers + helpers + stdout, "", warm.merge(helpers: true)) }
      diagnostic = correction_diagnostics(sample_stderr, wrapper: false)
      assert(diagnostic.first.values_at("state", "detail") == %w[P0001 search_request_missing], "retain bounded failure details")
      ["extra\n", "WARNING: unexpected\n", "NOTICE: unexpected\n", "DETAIL: duplicate\n"].each do |extra|
        rejected("unexpected diagnostic") { correction_diagnostics(sample_stderr + extra, wrapper: false) }
        rejected("unexpected diagnostic") { correction_diagnostics(extra + sample_stderr, wrapper: false) }
      end
      rejected("unexpected diagnostic") { correction_diagnostics(sample_stderr, wrapper: true) }
      before = frames.first.fetch("tables_before")
      after = frames.last.fetch("tables_finish")
      assert(correction_snapshots?(warm, before, frames, after), "valid per-call snapshots must pass")
      correction_rollback_tests!(warm, frames, before, after)
      correction_result_tests!(frames, after)
    end

    def correction_rollback_tests!(warm, frames, before, after)
      failed = Marshal.load(Marshal.dump(frames))
      failed.last["state"] = "P0001"
      assert(!correction_snapshots?(warm, before, failed, after), "failed upsert must preserve prior state")
      frames.first.fetch("tables_before").each_key do |table|
        changed = Marshal.load(Marshal.dump(frames))
        changed.last.fetch("tables_before").fetch(table) << { "unexpected" => true }
        assert(!correction_snapshots?(warm, before, changed, after), "#{table} continuity loss must fail")
      end
      changed = Marshal.load(Marshal.dump(frames))
      changed.last.fetch("tables_after").delete("canonical_size_sample")
      assert(!correction_snapshots?(warm, before, changed, after), "missing table evidence must fail")
      rollback = warm.merge(rollback: true)
      assert(!correction_snapshots?(rollback, before, frames, after), "actual committed first call must fail rollback proof")
      assert(correction_snapshots?(rollback.merge(commit_control: true), before, frames, after), "negative control must prove actual commit")
      rolled_back = Marshal.load(Marshal.dump(frames.first))
      rolled_back["tables_finish"] = before
      assert(correction_snapshots?(rollback, before, [rolled_back], before), "observed first rollback must restore all tables")
    end

    def correction_result_tests!(frames, after)
      test_case = { name: "imdb-upsert", id: "imdb" }
      assert(correction_results?(test_case, frames, after), "exact upsert must retain first seen and source links")
      %w[first_seen_at last_seen_at id_value_text id_type canonical_torrent_id source_canonical_torrent_source_id].each do |column|
        changed = Marshal.load(Marshal.dump(frames))
        changed.last.fetch("tables_after").fetch("canonical_external_id").first[column] = "changed"
        assert(!correction_results?(test_case, changed, after), "changed #{column} must fail upsert proof")
      end
      %w[observation_created durable_source_created canonical_changed].each do |flag|
        changed = Marshal.load(Marshal.dump(frames))
        changed.last.fetch("result")[flag] = true
        assert(!correction_results?(test_case, changed, after), "changed #{flag} must fail application proof")
      end
      changed = Marshal.load(Marshal.dump(frames))
      changed.last.fetch("result")["canonical_torrent_public_id"] = "58800000-0000-4000-8000-000000000009"
      assert(!correction_results?(test_case, changed, after), "changed returned identity must fail")
      changed = Marshal.load(Marshal.dump(frames))
      changed.last.fetch("result")["unexpected"] = true
      assert(!correction_results?(test_case, changed, after), "extra application fields must fail")
      assert(!correction_results?(test_case, frames.take(1), after), "one successful call cannot prove upsert")
    end
  end
end
