# frozen_string_literal: true

require "fileutils"
require "tmpdir"
require_relative "../database_rebaseline/native_fk_expectations"
require_relative "../database_rebaseline/final_sql"

module RevaerDatabaseRebaseline
  class NativeFkExpectationsTest
    ROOT = File.expand_path("../..", __dir__)
    MIGRATION = "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"
    WRAPPER = "crates/revaer-data/migrations/0120_search_result_ingest_seed_best_source_context.sql"
    SOURCES = [MIGRATION, WRAPPER,
      *%w[final_sql ingestion_compilation ingestion_corrections ingestion_existing ingestion_wrapper ingestion_proof].map { |name| "scripts/database_rebaseline/#{name}.rb" },
      *%w[proof corrections].map { |name| "scripts/tests/database-ingestion-#{name}-seed.sql" }].freeze
    NAMES = %w[search_result_ingest_v1 log_source_metadata_conflict_v1 search_result_ingest].freeze

    def initialize
      @assertions = 0
    end

    def run
      success
      inventory_mutations
      source_mutations
      puts "native FK expectations: #{@assertions} assertions passed"
    end

    private

    def assert(value, message)
      raise Failure, message unless value

      @assertions += 1
    end

    def rejects(message)
      yield
    rescue Failure => error
      assert(error.message.include?(message), "unexpected failure: #{error.message}")
    else
      raise Failure, "mutation accepted: #{message}"
    end

    def frozen_mutation
      yield
    rescue FrozenError
      @assertions += 1
    else
      raise Failure, "expected data was mutable"
    end

    # Test fixtures come from migration declarations, never callback captures.
    def inventory
      reference = NAMES.map do |name|
        path = name == "search_result_ingest" ? WRAPPER : MIGRATION
        definition = File.read(File.join(ROOT, path)).split("CREATE OR REPLACE FUNCTION #{name}(", 2).fetch(1)
        arguments = definition.split(")\nRETURNS", 2).first
        types = arguments.lines.map(&:strip).reject(&:empty?).map do |line|
          line.delete_suffix(",").split(" ", 2).last.downcase.gsub(/\([^)]*\)/, "")
            .sub(/\Avarchar/, "character varying").sub(/\Achar\b/, "character").sub(/\Atimestamptz\b/, "timestamp with time zone")
        end
        { "name" => name, "signature" => "#{name}(#{types.join(',')})",
          "source" => definition.split("AS $$", 2).fetch(1).split("$$;", 2).first,
          "settings" => name == "search_result_ingest_v1" ? ["plpgsql.variable_conflict=use_column"] : nil }
      end
      final = reference.map do |routine|
        source = routine.fetch("source")
        if routine.fetch("name") == "search_result_ingest_v1"
          source = "\n#variable_conflict use_column\n#{FinalSql.new(nil).approved_ingestion_body(source).delete_prefix("\n")}"
        end
        routine.merge("source" => source, "settings" => ["search_path=pg_catalog, public"])
      end
      { "reference_proof" => { "routines" => reference }, "final_proof" => { "routines" => final } }
    end

    def provider(input = inventory, root: ROOT)
      NativeFkExpectations.new(root:, inventory: input)
    end

    def success
      expected = provider
      # Independently calculated from only the retained fields in the two oracles.
      assert(Digest::SHA256.hexdigest(JSON.generate(expected.compilation)) ==
        "258811b8d22d241fbbe78ea1904ed1113855c49f00a13b05353ac76c848d0360", "compilation oracle projection changed")
      assert(Digest::SHA256.hexdigest(JSON.generate(expected.remaining)) ==
        "8c56775af3fb490b842a9a9e6c377ef70a266dc94754d4ebf0619118fe29a864", "remaining oracle projection changed")
      assert(expected.compilation.fetch("operations").map { |row| row.fetch("exact_total") } == [19, 5, 5, 19], "compilation totals changed")
      expected.compilation.fetch("operations").each do |operation|
        total = operation.fetch("multisets").sum do |name, multiplier|
          expected.compilation.fetch("multisets").fetch(name).sum { |row| row.fetch("count_each") * row.fetch("constraints").length * multiplier }
        end
        assert(total == operation.fetch("exact_total"), "compilation multiset total differs")
      end
      cases = expected.remaining.fetch("cases")
      totals = cases.flat_map do |entry|
        entry.fetch("variants").flat_map do |_variant, data|
          data.fetch("operations").map { |operation| operation.fetch("total_callback_entries") }
        end
      end
      assert(totals == [9, 9, 18, 1, 21, 21, 21, 21], "remaining totals changed")
      assert(expected.target_constraints == %w[
        canonical_external_id_canonical_torrent_id_fkey
        canonical_external_id_source_canonical_torrent_source_id_fkey
        canonical_torrent_source_attr_canonical_torrent_source_id_fkey
        canonical_torrent_best_source_context_canonical_torrent_id_fkey
        canonical_torrent_best_source_canonical_torrent_source_id_fkey1
      ], "target constraint bindings changed")
      cases.each do |entry|
        entry.fetch("variants").each do |variant, data|
          data.fetch("operations").each do |operation|
            rows = operation.fetch("multiset")
            assert(rows.sum { |row| row.fetch("count") } == operation.fetch("total_callback_entries"), "remaining multiset total differs")
            state = entry.fetch("name") == "imdb-upsert" && variant == "reference" ? "42P10" : "00000"
            assert(operation.fetch("expected_sqlstate") == state, "SQLSTATE changed")
            operation.fetch("scope_totals").each do |scope, count|
              assert(rows.select { |row| row.fetch("scope") == scope }.sum { |row| row.fetch("count") } == count, "ownership total changed")
            end
          end
        end
      end
      frozen_mutation { expected.compilation.fetch("operations").first["exact_total"] = 0 }
      frozen_mutation { expected.remaining.fetch("cases").first.fetch("variants").fetch("reference").fetch("operations").first.fetch("multiset").first["count"] = 0 }
      frozen_mutation { expected.target_constraints.first.replace("substitute") }
      input = inventory.merge("source_commit" => "not-the-historical-revision", "operations" => [], "cases" => [], "target_constraints" => [])
      assert(provider(input).remaining == expected.remaining, "arbitrary claimed expectations affected predictions")
      input.fetch("final_proof").fetch("routines").first["source"] = "changed after construction"
      assert(expected.remaining == provider.remaining, "input alias changed retained expectations")
    end

    def inventory_mutations
      rejects("inventory must be an object") { provider(nil) }
      %w[reference_proof final_proof].each do |variant|
        input = inventory
        input.delete(variant)
        rejects("inventory missing or malformed") { provider(input) }
        [nil, {}, [nil], [{ "name" => 1 }]].each do |bad|
          input = inventory
          input.fetch(variant)["routines"] = bad
          rejects("inventory missing or malformed") { provider(input) }
        end
        NAMES.each do |name|
          input = inventory
          input.fetch(variant).fetch("routines").reject! { |row| row.fetch("name") == name }
          rejects("routine missing or duplicated") { provider(input) }
          input = inventory
          routines = input.fetch(variant).fetch("routines")
          routines << routines.find { |row| row.fetch("name") == name }.dup
          rejects("routine missing or duplicated") { provider(input) }
          %w[signature source settings].each do |field|
            [nil, "substitute"].each do |replacement|
              input = inventory
              routine = input.fetch(variant).fetch("routines").find { |row| row.fetch("name") == name }
              replacement.nil? ? routine.delete(field) : routine[field] = replacement
              rejects("routine binding changed") { provider(input) }
            end
          end
          input = inventory
          routine = input.fetch(variant).fetch("routines").find { |row| row.fetch("name") == name }
          routine["source"] += "\n-- body drift"
          rejects("routine binding changed") { provider(input) }
        end
        input = inventory
        input.fetch(variant).fetch("routines").first["settings"] = ["plpgsql.variable_conflict=error"]
        rejects("routine binding changed") { provider(input) }
      end
      input = inventory
      input.fetch("final_proof").fetch("routines").first["source"] = input.fetch("reference_proof").fetch("routines").first.fetch("source")
      rejects("routine binding changed") { provider(input) }
    end

    def source_mutations
      Dir.mktmpdir("native-fk-expectations-test-", ROOT) do |root|
        SOURCES.each do |path|
          destination = File.join(root, path)
          FileUtils.mkdir_p(File.dirname(destination))
          FileUtils.cp(File.join(ROOT, path), destination)
        end
        assert(provider(root:).compilation == provider.compilation, "source binding incorrectly requires historical Git revision")
        SOURCES.each do |path|
          destination = File.join(root, path)
          bytes = File.binread(destination)
          File.binwrite(destination, bytes + "\n")
          rejects("source definition changed: #{path}") { provider(root:) }
          File.delete(destination)
          rejects("required source unreadable: #{path}") { provider(root:) }
          File.binwrite(destination, bytes)
        end
      end
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    RevaerDatabaseRebaseline::NativeFkExpectationsTest.new.run
  rescue RevaerDatabaseRebaseline::Failure => error
    warn error.message
    exit 1
  end
end
