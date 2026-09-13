# frozen_string_literal: true

require_relative "database-ingestion-attributes-test"
require_relative "../database_rebaseline/ingestion_runtime_rank"

module RevaerDatabaseRebaseline
  class IngestionRuntimeRankTest < IngestionAttributesTest
    include IngestionRankProof

    def run_tests!
      @assertions = 0
      @runtime = "rank_unit_runtime"
      source = File.binread(File.join(@contract.root, "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"))
      body = source.split("CREATE OR REPLACE FUNCTION search_result_ingest_v1(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
      @ingestion_inventory = { "reference_proof" => { "routines" => [{ "name" => "search_result_ingest_v1", "signature" => "search_result_ingest_v1(uuid)", "source" => body }] } }
      RANK_CASES.each do |spec|
        %w[cold helpers-first].each do |mode|
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = rank_test_evidence(spec, mode, variant)
            assert(rank_validate!(evidence, spec, mode, variant, role), "rank #{spec.first(3)} #{mode} #{variant}")
          end
        end
      end
      rank_mutations!
      rank_writer_duplicate_tests!
      3.times do |index|
        query = rank_query(rank_case(RANK_CASES.first, index), "helpers-first", index)
        assert(query.scan("FROM public.search_result_ingest_v1(").length == 1, "one real ingestion per barrier")
        assert(query.include?(index.zero? ? "ROLLBACK;" : "COMMIT;"), "transaction boundary")
        assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect|UPDATE public.trust_tier/), "no backend repair or caller rank update")
        assert(query.include?("helpers:") == index.zero?, "helpers precede initial tested call only")
      end
      query = rank_update_query(20)
      assert(query.index("BEGIN;") < query.index("UPDATE public.trust_tier") && query.end_with?("COMMIT;\n"), "writer independently commits")
      assert(query.include?("WHERE trust_tier_key = 'public'"), "only exact rank input is written")
      puts "database-ingestion-runtime-rank-test: #{@assertions} assertions passed"
    end

    private

    def rank_writer_duplicate_tests!
      original_runner = @runner
      original_evidence = @attributes_evidence
      spec = RANK_CASES.first
      original = rank_test_evidence(spec, "cold", "final")
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      Dir.mktmpdir("rank-writer-unit-", @contract.output_path) do |directory|
        @attributes_evidence = directory
        original.fetch("changes").first.fetch("writer").fetch("record").each do |key, value|
          changed = copy(original)
          writer = changed.fetch("changes").first.fetch("writer")
          writer["stdout"] = writer.fetch("stdout").sub("{", "{#{JSON.generate(key)}:#{JSON.generate(value)},")
          rejected("invalid or duplicate metadata JSON evidence") { rank_validate!(changed, spec, "cold", "final", @runtime) }

          response = CommandRunner::Result.new(stdout: writer.fetch("stdout"), stderr: "", success: true)
          fake = Object.new
          fake.define_singleton_method(:capture) { |*_arguments, **_options| response }
          @runner = fake
          rejected("invalid or duplicate metadata JSON evidence") { rank_update!("rank_unit", 20, "duplicate-#{key}") }
        end
      end
    ensure
      @runner = original_runner
      @attributes_evidence = original_evidence
    end

    def rank_test_evidence(spec, mode, variant)
      base = attributes_test_evidence(rank_case(spec, 0), mode, variant)
      frames = base.fetch("frames")
      previous = base.fetch("before")
      current = base.fetch("inputs_before")
      names = { "public" => "Public", "semi_private" => "Semi-Private", "private" => "Private", "invite_only" => "Invite Only" }
      current.fetch("trust_tier").each { |row| row["display_name"] = names.fetch(row.fetch("trust_tier_key")) }
      current.fetch("trust_tier").sort_by! { |row| [row.fetch("rank").to_s, row.fetch("display_name")] }
      changes = []
      calls = frames.each_with_index.map do |frame, index|
        if index.positive?
          changed = copy(current)
          changed.fetch("trust_tier").find { |r| r.fetch("trust_tier_key") == "public" }["rank"] = spec.fetch(index)
          changed.fetch("trust_tier").sort_by! { |row| [row.fetch("rank").to_s, row.fetch("display_name")] }
          record = { "backend" => "#{200 + index}", "transaction" => "#{300 + index}", "rank" => spec.fetch(index), "session" => "postgres", "current" => "postgres" }
          changes << { "before" => current, "after" => changed, "tables_before" => previous, "tables_after" => previous,
                       "writer" => { "record" => record, "stdout" => JSON.generate(record) + "\n", "stderr" => "", "success" => true } }
          current = changed
        end
        frame["tables_before"] = previous
        frame["tables_after"] = frame.fetch("state") == "00000" ? attributes_repeated_tables(rank_case(spec, index), frame, index) : previous
        frame["tables_finish"] = index.zero? ? previous : frame.fetch("tables_after")
        previous = frame.fetch("tables_finish")
        { "stdout" => attributes_test_transport([frame], index.zero? ? mode : "cold").fetch("stdout"), "inputs_after" => current }
      end
      base.reject { |key, _value| key == "frames" }.merge("calls" => calls, "changes" => changes, "after" => previous)
    end

    def rank_mutations!
      spec = RANK_CASES.first
      original = rank_test_evidence(spec, "cold", "final")
      mutations = [
        ["missing calls", ->(e) { e.fetch("calls").pop }],
        ["missing calls", ->(e) { e.fetch("changes").pop }],
        ["raw evidence", ->(e) { e.fetch("changes").first.fetch("writer")["success"] = false }],
        ["independent writer", ->(e) {
          w = e.fetch("changes").first.fetch("writer"); w.fetch("record")["backend"] = "101"; w["stdout"] = JSON.generate(w.fetch("record"))
        }],
        ["external commit", ->(e) { e.fetch("changes").first.fetch("after").fetch("trust_tier").first["rank"] = 19 }],
        ["external commit", ->(e) { e.fetch("changes").first.fetch("after").fetch("policy_rule") << { "unrelated" => true } }],
        ["fixture inputs", ->(e) { e.fetch("calls").first.fetch("inputs_after").fetch("media_domain").clear }],
        ["backend", ->(e) { e.fetch("calls").last["stdout"].gsub!("101", "102") }],
        ["persisted confidence", ->(e) {
          e.fetch("calls").first["stdout"].sub!(/^tables_after:.*$/) { |line| line.gsub('"confidence":0.5', '"confidence":0.4') }
        }],
        ["exact success", ->(e) { e.fetch("calls").last["stdout"].sub!('"canonical_changed":false', '"canonical_changed":true') }]
      ]
      mutations.each do |message, mutate|
        changed = copy(original)
        mutate.call(changed)
        # The changed result boolean remains a valid success; its independent
        # output assertion, rather than SQLSTATE framing, must reject it.
        message = "persisted confidence" if message == "exact success"
        rejected(message) { rank_validate!(changed, spec, "cold", "final", @runtime) }
      end
      IngestionProof::INGESTION_TABLES.each do |table|
        changed = copy(original)
        raw = changed.fetch("calls").last
        parsed = attributes_parse(raw.fetch("stdout"), "", { calls: [{}] }, @runtime)
        frame = parsed.fetch("frames").first
        frame.fetch("tables_after").fetch(table) << { "unintended" => true }
        frame["tables_finish"] = frame.fetch("tables_after")
        changed["after"] = frame.fetch("tables_after")
        raw["stdout"] = attributes_test_transport([frame], "cold").fetch("stdout")
        rejected("persisted confidence") { rank_validate!(changed, spec, "cold", "final", @runtime) }
      end
      reference = rank_test_evidence(spec, "helpers-first", "reference")
      reference["stderr"] = reference.fetch("stderr").sub("createas.c:406", "createas.c:407")
      rejected("exact success") { rank_validate!(reference, spec, "helpers-first", "reference", "postgres") }
      reference["stderr"] = ""
      rejected("diagnostic") { rank_validate!(reference, spec, "helpers-first", "reference", "postgres") }
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    valid = ARGV.empty? || ARGV == ["--unit"]
    raise RevaerDatabaseRebaseline::Failure, "usage: database-ingestion-runtime-rank-test.rb [--unit]" unless valid

    contract = RevaerDatabaseRebaseline::Contract.new(root: File.expand_path("../..", __dir__))
    proof = RevaerDatabaseRebaseline::IngestionRuntimeRankTest.new(contract)
    proof.run_tests!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-runtime-rank: #{error.message}"
    exit 1
  end
end
