# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # Separate fixture and tested connections establish existing-data, cold-backend evidence.
  module IngestionExisting
    EXISTING_CLOCK_COLUMNS = {
      "canonical_torrent" => %w[created_at updated_at],
      "canonical_torrent_source" => %w[created_at updated_at],
      "canonical_torrent_source_context_score" => %w[computed_at],
      "canonical_torrent_best_source_context" => %w[computed_at],
      "search_request_source_observation_attr" => %w[created_at],
      "canonical_size_rollup" => %w[updated_at],
      "search_request_canonical" => %w[first_seen_at],
      "search_page" => %w[sealed_at],
      "search_filter_decision" => %w[decided_at],
      "source_metadata_conflict_audit_log" => %w[occurred_at]
    }.freeze
    EXISTING_STAGES = %w[fixture tested].freeze
    EXISTING_IDENTITIES = %w[canonical_torrent canonical_torrent_source].freeze

    private

    def ingestion_existing_attrs(tracker: "proof-tracker", category: 2000)
      {
        attr_keys_input: "ARRAY['tracker_name','tracker_category']::public.observation_attr_key[]",
        attr_types_input: "ARRAY['text','int']::public.attr_value_type[]",
        attr_value_text_input: "ARRAY[#{literal(tracker)},NULL]::varchar[]",
        attr_value_int_input: "ARRAY[NULL,#{category}]::integer[]",
        attr_value_bigint_input: "ARRAY[NULL,NULL]::bigint[]",
        attr_value_numeric_input: "ARRAY[NULL,NULL]::numeric[]",
        attr_value_bool_input: "ARRAY[NULL,NULL]::boolean[]",
        attr_value_uuid_input: "ARRAY[NULL,NULL]::uuid[]"
      }
    end

    def ingestion_existing_imdb
      ingestion_existing_attrs.merge(
        attr_keys_input: "ARRAY['tracker_name','imdb_id']::public.observation_attr_key[]",
        attr_types_input: "ARRAY['text','text']::public.attr_value_type[]",
        attr_value_text_input: "ARRAY['proof-tracker','tt1234567']::varchar[]",
        attr_value_int_input: "ARRAY[NULL,NULL]::integer[]"
      )
    end

    def ingestion_existing_cases
      later = { observed_at_input: "'2026-09-10T00:01:00Z'::timestamptz", seeders_input: "17" }
      [
        ["existing-refresh", {}, later.merge(title_raw_input: "'Updated proof title'::varchar")],
        ["existing-older", {}, later.merge(observed_at_input: "'2026-09-09T23:59:00Z'::timestamptz")],
        ["existing-hash-reuse", { source_guid_input: "NULL::varchar" }, later.merge(source_guid_input: "NULL::varchar")],
        ["existing-hash-conflict", {}, later.merge(infohash_v1_input: "repeat('b',40)::char(40)")],
        ["existing-attrs-reuse", ingestion_existing_attrs, later.merge(ingestion_existing_attrs)],
        ["existing-attrs-conflict", ingestion_existing_attrs, later.merge(ingestion_existing_attrs(tracker: "changed-tracker", category: 3000))],
        ["existing-external-id", ingestion_existing_attrs, later.merge(ingestion_existing_imdb)],
        ["existing-attrs-rollback", ingestion_existing_attrs, later.merge(attr_keys_input: "ARRAY['imdb_id']::public.observation_attr_key[]")]
      ]
    end

    def verify_existing_ingestion_parity!
      ingestion_existing_cases.each do |name, fixture, tested|
        reference = ingestion_existing_isolated(name, fixture, tested, source: "reference_proof", role: "postgres", variant: "reference")
        final = ingestion_existing_isolated(name, fixture, tested, source: @database, role: @runtime, variant: "final")
        equivalent = reference == final
        approved_delta = ingestion_approved_delta(name, reference, final)
        admissible = equivalent || !approved_delta.nil?
        accepted = admissible && ingestion_existing_outcome?(name, final)
        expected = name == "existing-attrs-rollback" ? ["P0001"] : ["00000"]
        @ingestion_results << { name:, expected:, reference:, final:, equivalent:, approved_delta:, accepted:, existing_data_cold_backend: true }
        check("ingestion #{name} parity or exact approved correction", admissible)
        check("ingestion #{name} required outcome", accepted)
      end
    end

    def ingestion_existing_isolated(name, fixture, tested, source:, role:, variant:)
      database = "ingestion_#{variant}_proof"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        sql(File.binread(File.expand_path("../tests/database-ingestion-proof-seed.sql", __dir__)), role: "postgres", database:)
        stages = {}
        EXISTING_STAGES.zip([fixture, tested]).each do |stage, changes|
          before = ingestion_snapshot(database)
          query = "SELECT 'backend:' || pg_backend_pid();\n#{ingestion_session(changes)}"
          prefix = File.join(@ingestion_evidence, "#{name}-#{variant}-#{stage}")
          File.binwrite("#{prefix}.sql", query)
          outcome = result(query, role:, database:)
          File.binwrite("#{prefix}.stdout", outcome.stdout)
          File.binwrite("#{prefix}.stderr", outcome.stderr)
          raise Failure, "existing ingestion transport failed" unless outcome.success

          value = ingestion_existing_parse(outcome.stdout, outcome.stderr, role:)
          stages[stage] = value.merge("before" => before, "after" => ingestion_snapshot(database))
          File.binwrite("#{prefix}.json", JSON.pretty_generate(stages.fetch(stage)) + "\n")
          if stage == "fixture" && value.fetch("states") != ["00000"]
            raise Failure, "existing ingestion fixture must commit real successful ingestion"
          end
        end
        ingestion_existing_comparable(stages, role:)
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def ingestion_existing_parse(stdout, stderr, role:)
      lines = stdout.lines
      backend = lines.shift&.match(/\Abackend:([1-9][0-9]*)\n\z/)
      raise Failure, "existing ingestion backend evidence missing" unless backend

      parsed = ingestion_parse(lines.join, stderr, role:)
      roles = lines.grep(/\Arole:/).map { |line| JSON.parse(line.delete_prefix("role:")) }
      expected = { "session" => role, "current" => role, "superuser" => role == "postgres",
                   "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }
      raise Failure, "existing ingestion exact role contract changed" unless roles == [expected]

      parsed.merge("backend" => backend[1].to_i, "role" => roles.fetch(0))
    end

    def ingestion_existing_comparable(stages, role:)
      unless stages.keys == EXISTING_STAGES && stages.values.map { |stage| stage.fetch("backend") }.uniq.length == 2
        raise Failure, "existing ingestion requires distinct fixture and tested backends"
      end
      unless stages.fetch("fixture").fetch("after") == stages.fetch("tested").fetch("before")
        raise Failure, "existing ingestion fixture continuity changed"
      end
      clocks = {}
      stages.each do |name, stage|
        values = stage.fetch("clocks")
        unless values.length == 1 && values.first.is_a?(String) && values.first.match?(/\A\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?\+00:00\z/) && !clocks.key?(values.first)
          raise Failure, "existing ingestion transaction provenance missing or ambiguous"
        end
        clocks[values.first] = "<#{name}-transaction-time>"
      end
      identities = ingestion_existing_identities(stages)
      stages.to_h do |name, stage|
        value = stage.dup
        value["backend"] = "<#{name}-backend>"
        expected_role = { "session" => role, "current" => role, "superuser" => role == "postgres",
                          "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }
        raise Failure, "existing ingestion exact role contract changed" unless value.fetch("role") == expected_role

        value["role"] = "<validated-direct-variant-role>"
        value["clocks"] = stage.fetch("clocks").map { |clock| clocks.fetch(clock) }
        value["results"] = stage.fetch("results").map do |row|
          result = row.dup
          EXISTING_IDENTITIES.each do |table|
            key = "#{table}_public_id"
            uuid = row.fetch(key)
            unless stage.fetch("after").fetch(table).any? { |stored| stored.fetch(key) == uuid }
              raise Failure, "existing ingestion result has no committed stage row"
            end
            result[key] = identities.fetch(uuid)
          end
          result
        end
        %w[before after].each do |image|
          value[image] = stage.fetch(image).to_h do |table, rows|
            [table, rows.map { |row| ingestion_existing_row(table, row, identities, clocks) }]
          end
        end
        [name, value]
      end
    end

    def ingestion_existing_identities(stages)
      identities = {}
      labels = {}
      stages.each_value do |stage|
        %w[before after].each do |image|
          tables = stage.fetch(image)
          raise Failure, "existing ingestion table inventory incomplete" unless tables.keys.sort == IngestionProof::INGESTION_TABLES.sort

          EXISTING_IDENTITIES.each do |table|
            seen = []
            tables.fetch(table).each do |row|
              uuid = row.fetch("#{table}_public_id")
              label = "<#{table}:#{row.fetch("#{table}_id")}>"
              unless uuid.match?(IngestionProof::INGESTION_UUID) && !seen.include?(label) &&
                     (!identities.key?(uuid) || identities.fetch(uuid) == label) &&
                     (!labels.key?(label) || labels.fetch(label) == uuid)
                raise Failure, "existing ingestion identity is invalid, duplicated or changed"
              end
              seen << label
              identities[uuid] = label
              labels[label] = uuid
            end
          end
        end
      end
      identities
    end

    def ingestion_existing_row(table, row, identities, clocks)
      row.to_h do |column, item|
        value = if EXISTING_IDENTITIES.include?(table) && column == "#{table}_public_id"
                  identities.fetch(item)
                elsif EXISTING_CLOCK_COLUMNS.fetch(table, []).include?(column) && clocks.key?(item)
                  clocks.fetch(item)
                else
                  item
                end
        [column, value]
      end
    end

    def ingestion_existing_outcome?(name, stages)
      fixture = stages.fetch("fixture")
      tested = stages.fetch("tested")
      flags = %w[observation_created durable_source_created canonical_changed]
      return false unless fixture.fetch("states") == ["00000"] && fixture.fetch("results").length == 1 &&
                          fixture.fetch("results").first.values_at(*flags) == [true, true, true] &&
                          fixture.fetch("before").values.all?(&:empty?)

      if name == "existing-attrs-rollback"
        return tested.fetch("states") == ["P0001"] && tested.fetch("details") == ["attr_length_mismatch"] &&
               tested.fetch("results").empty? && tested.fetch("before") == tested.fetch("after")
      end
      return false unless tested.fetch("states") == ["00000"] && tested.fetch("results").length == 1

      first = fixture.fetch("results").first
      second = tested.fetch("results").first
      conflict = name == "existing-hash-conflict"
      return false unless second.values_at(*flags) == [false, false, conflict] &&
                          first.fetch("canonical_torrent_source_public_id") == second.fetch("canonical_torrent_source_public_id") &&
                          (first.fetch("canonical_torrent_public_id") != second.fetch("canonical_torrent_public_id")) == conflict

      before = tested.fetch("before")
      after = tested.fetch("after")
      source = after.fetch("canonical_torrent_source")
      return false unless source.length == 1 && source.first.fetch("infohash_v1") == "a" * 40
      return false unless source.first.fetch("last_seen_seeders") == (name == "existing-older" ? 5 : 17)
      return false unless after.fetch("search_request_source_observation").length == 1

      ingestion_existing_case_outcome?(name, before, after)
    end

    def ingestion_existing_case_outcome?(name, before, after)
      conflicts = after.fetch("source_metadata_conflict")
      case name
      when "existing-refresh"
        after.fetch("canonical_torrent").first.fetch("title_display") == "Updated proof title" && conflicts.empty?
      when "existing-older"
        before.fetch("canonical_torrent_source").first.fetch("last_seen_at") == after.fetch("canonical_torrent_source").first.fetch("last_seen_at") && conflicts.empty?
      when "existing-hash-reuse"
        after.fetch("canonical_torrent_source").first.fetch("source_guid").nil? && conflicts.empty?
      when "existing-hash-conflict"
        conflicts.any? { |row| row.values_at("conflict_type", "existing_value", "incoming_value") == ["hash", "a" * 40, "b" * 40] } &&
          after.fetch("canonical_torrent").length == 2 && ingestion_existing_conflict_links?(after)
      when "existing-attrs-reuse", "existing-attrs-conflict"
        changed = name == "existing-attrs-conflict"
        durable = after.fetch("canonical_torrent_source_attr")
        observation = after.fetch("search_request_source_observation_attr")
        expected = { "tracker_category" => [nil, changed ? 3000 : 2000], "tracker_name" => [changed ? "changed-tracker" : "proof-tracker", nil] }
        original = { "tracker_category" => [nil, 2000], "tracker_name" => ["proof-tracker", nil] }
        expected_conflicts = [["tracker_category", "2000", "3000"], ["tracker_name", "proof-tracker", "changed-tracker"]]
        before.fetch("canonical_torrent_source_attr") == durable && durable.length == 2 &&
          durable.to_h { |row| [row.fetch("attr_key"), row.values_at("value_text", "value_int")] } == original &&
          observation.to_h { |row| [row.fetch("attr_key"), row.values_at("value_text", "value_int")] } == expected &&
          (changed ? conflicts.map { |row| row.values_at("conflict_type", "existing_value", "incoming_value") }.sort == expected_conflicts && ingestion_existing_conflict_links?(after) : conflicts.empty?)
      when "existing-external-id"
        after.fetch("canonical_torrent").first.fetch("imdb_id") == "tt1234567" &&
          after.fetch("canonical_external_id").any? { |row| row.values_at("id_type", "id_value_text") == %w[imdb tt1234567] }
      else
        raise Failure, "unknown existing ingestion outcome"
      end
    end

    def ingestion_existing_conflict_links?(tables)
      conflicts = tables.fetch("source_metadata_conflict")
      logs = tables.fetch("source_metadata_conflict_audit_log")
      health = tables.fetch("indexer_health_event")
      conflicts.all? do |conflict|
        logs.any? { |row| row.fetch("conflict_id") == conflict.fetch("source_metadata_conflict_id") && row.fetch("action") == "created" } &&
          health.any? { |row| row.fetch("event_type") == "identity_conflict" && row.fetch("detail") == conflict.fetch("conflict_type") }
      end && logs.length == conflicts.length && health.length == conflicts.length
    end
  end
end
