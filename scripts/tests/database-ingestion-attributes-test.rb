# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"
require_relative "../database_rebaseline/ingestion_attributes"

module RevaerDatabaseRebaseline
  class IngestionAttributesTest < FinalProof
    include IngestionAttributes

    def run_tests!
      @assertions = 0
      @runtime = "attributes_unit_runtime"
      source = File.binread(File.join(@contract.root, "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"))
      body = source.split("CREATE OR REPLACE FUNCTION search_result_ingest_v1(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
      @ingestion_inventory = { "reference_proof" => { "routines" => [{ "name" => "search_result_ingest_v1", "signature" => "search_result_ingest_v1(uuid)", "source" => body }] } }
      attributes_case_tests!
      attributes_trust_tests!(source)
      attributes_trust_producer_tests!
      attributes_mutation_tests!
      attributes_transport_tests!
      attributes_bytes_tests!
      attributes_cleanup_tests!
      puts "database-ingestion-attributes-test: #{@assertions} assertions passed"
    end

    # Uses the exact FinalProof bootstrap and correction helpers in one uniquely
    # owned, network-isolated server. This is not the canonical full proof.
    def run_live!(candidate_path)
      @contract.freeze!
      @candidate = File.binread(candidate_path)
      @contract.verify_candidate_source!(@candidate)
      @contract.validate_output_path!
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      raise Failure, "attribute candidate path must be owned" unless File.expand_path(candidate_path) == @contract.candidate_path && !File.symlink?(candidate_path)

      final = FinalSql.new(@contract).verify!
      @routines = FinalSql.new(@contract).routines(@candidate)
      @container = "revaer-attributes-proof-#{Process.pid}-#{SecureRandom.hex(8)}"
      @attributes_volume = "#{@container}-data"
      @attributes_owned_container = false
      @attributes_owned_volume = false
      record = { completed: false, passed: false, d3_complete: false, cleanup: false,
                 source_commit: @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip,
                 source_status: @runner.run!(["git", "status", "--porcelain=v1"], chdir: @contract.root),
                 source_sha256: attributes_source_hashes, postgres_image: @contract.postgres_image,
                 candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
                 container: @container, volume: @attributes_volume }
      begin
        attributes_start!
        wait_ready!
        record[:image] = JSON.parse(@runner.run!(["docker", "image", "inspect", @contract.postgres_image]))
        provision!
        check("attributes pinned server identity", sql("SELECT current_setting('server_version_num')", role: "postgres") == "160014")
        versions = %w[psql pg_dump postgres].to_h do |program|
          [program, @runner.run!(["docker", "exec", @container, program, "--version"]).strip]
        end
        record[:versions] = versions
        check("attributes pinned client and server binaries", versions.all? { |program, version| version == "#{program} (PostgreSQL) #{@contract.postgres_version}" })
        raise Failure, "pinned PostgreSQL identity mismatch" unless @failures.empty?

        apply!("reference_proof", @candidate, role: "postgres")
        apply!(@database, final)
        seal!
        @ingestion_evidence = File.join(@contract.output_path, "ingestion-proof")
        FileUtils.mkdir_p(@ingestion_evidence, mode: 0o700)
        @ingestion_inventory = {}
        ingestion_inventory!
        verify_ingestion_corrections!
        raise Failure, "required existing correction matrix failed" unless @failures.empty?

        verify_ingestion_attributes!
        record[:completed] = true
      rescue StandardError => error
        record[:error] = "#{error.class}: #{error.message}"
        raise
      ensure
        begin
          attributes_cleanup!
          record[:cleanup] = true
        rescue Failure => error
          record[:cleanup_error] = error.message
          raise
        ensure
          record[:checks] = @checks
          record[:cleanup_records] = @attributes_cleanup_records
          record[:evidence] = @attributes_evidence
          record[:passed] = record[:completed] && record[:cleanup] && @failures.empty?
          path = File.join(@contract.output_path, "attributes-live-#{Process.pid}-#{SecureRandom.hex(4)}.json")
          File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(JSON.pretty_generate(record) + "\n") }
          puts "database-ingestion-attributes-live: report=#{path} passed=#{record.fetch(:passed)} d3_complete=false"
        end
      end
    end

    private

    def assert(value, message)
      raise Failure, message unless value

      @assertions += 1
    end

    def rejected(message)
      yield
    rescue Failure => error
      assert(error.message.include?(message), "unexpected rejection: #{error.message}")
    else
      raise Failure, "expected rejection: #{message}"
    end

    def copy(value)
      Marshal.load(Marshal.dump(value))
    end

    def attributes_test_inputs(clock, rank, trust_key: "public")
      inputs = IngestionPolicy::POLICY_READ_TABLES.to_h { |table| [table, []] }
      inputs["indexer_definition"] = [{ "indexer_definition_id" => 569001, "created_at" => clock, "updated_at" => clock, "upstream_slug" => "ingestion-proof", "definition_hash" => "a" * 64 }]
      inputs["indexer_instance"] = [{ "indexer_instance_id" => 569001, "created_at" => clock, "updated_at" => clock,
        "indexer_instance_public_id" => "56900000-0000-4000-8000-000000000001", "indexer_definition_id" => 569001,
        "is_enabled" => true, "deleted_at" => nil, "migration_state" => "ready", "trust_tier_key" => trust_key }]
      inputs["policy_snapshot"] = [{ "policy_snapshot_id" => 569001, "created_at" => clock, "snapshot_hash" => "b" * 64 }]
      inputs["search_request"] = [{ "search_request_id" => 569001, "created_at" => clock,
        "search_request_public_id" => "56900000-0000-4000-8000-000000000002", "policy_snapshot_id" => 569001,
        "status" => "running", "page_size" => 10, "query_text" => "Ingestion proof", "finished_at" => nil, "canceled_at" => nil, "failure_class" => nil }]
      ranks = [["semi_private", 20], ["private", 30], ["invite_only", 40]]
      ranks.unshift(["public", rank]) unless rank.nil?
      inputs["trust_tier"] = ranks.map do |key, value|
        { "trust_tier_key" => key, "created_at" => clock, "rank" => value }
      end
      inputs["media_domain"] = [{ "media_domain_key" => "movies", "created_at" => clock }]
      inputs["search_request_indexer_run"] = [{ "search_request_id" => 569001, "indexer_instance_id" => 569001, "status" => "queued" }]
      inputs
    end

    def attributes_test_frame(role, number, before, after, first: false)
      identities = attributes_identities(after)
      { "backend" => first ? "100" : "101", "clock" => "2026-09-12T00:00:0#{number}+00:00", "before" => "error", "after" => "error",
        "role" => { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" },
        "state" => "00000", "within" => "true", "outside" => (role == "postgres").to_s,
        "tables_before" => before, "tables_after" => after, "tables_finish" => after,
        "result" => identities.merge("observation_created" => first, "durable_source_created" => first, "canonical_changed" => first) }
    end

    def attributes_test_transport(frames, mode)
      frame = frames.first
      context = { "backend" => frame.fetch("backend"), "session" => frame.fetch("role").fetch("session"),
                  "current" => frame.fetch("role").fetch("current"), "setting" => "error" }
      lines = ["attribute_context:#{JSON.generate(context)}"]
      lines << "helpers:#{JSON.generate(ingestion_helper_expectations)}" if mode == "helpers-first"
      frames.each do |item|
        IngestionCorrections::CORRECTION_RECORDS.each do |key|
          lines << JSON.generate(item.fetch("result")) if key == "state" && item.key?("result")
          value = item.fetch(key)
          lines << "#{key}:#{%w[role clock tables_before tables_after tables_finish].include?(key) ? JSON.generate(value) : value}"
        end
      end
      stderr = frames.any? { |item| item.fetch("state") == "42P07" } ? policy_d4_expected : ""
      { "context" => context, "frames" => frames, "stdout" => lines.join("\n") + "\n", "stderr" => stderr }
    end

    def attributes_test_evidence(test_case, mode, variant)
      role = variant == "reference" ? "postgres" : @runtime
      identities = { "canonical_torrent_public_id" => "56900000-0000-4000-8000-000000000090",
                     "canonical_torrent_source_public_id" => "56900000-0000-4000-8000-000000000091" }
      initial = attributes_initial_tables(test_case, "2026-09-12T00:00:01+00:00", identities)
      fixture = attributes_test_frame(role, 1, attributes_empty, initial, first: true)
      prior = initial
      frames = (0..2).map do |index|
        frame = attributes_test_frame(role, index + 2, prior, prior)
        frame["tables_after"] = attributes_repeated_tables(test_case, frame, index)
        frame["tables_finish"] = index.zero? ? initial : frame.fetch("tables_after")
        frame["outside"] = (variant == "reference" && index.positive?).to_s
        if variant == "reference" && index == 2
          frame.delete("result")
          frame.merge!("state" => "42P07", "diagnostic" => ingestion_diagnostics(policy_d4_expected, role:).first,
                       "tables_after" => prior, "tables_finish" => prior)
        end
        prior = frame.fetch("tables_finish")
        frame
      end
      seed_clock = "2026-09-12T00:00:00+00:00"
      trust = attributes_trust_fixture(test_case)
      inputs = attributes_test_inputs(seed_clock, trust.fetch(:public_rank), trust_key: trust.fetch(:key))
      attributes_test_transport(frames, mode).merge("fixture" => fixture, "fixture_transport" => attributes_test_transport([fixture], "cold"),
        "before" => initial, "after" => prior, "seed_clock" => seed_clock, "inputs_before" => inputs,
        "inputs_fixture" => inputs, "inputs_after" => inputs).merge(test_case.slice(:trust_rank_expectation).transform_keys(&:to_s))
    end

    def attributes_refresh_transport!(evidence, mode)
      evidence.merge!(attributes_test_transport(evidence.fetch("frames"), mode))
      evidence["fixture_transport"] = attributes_test_transport([evidence.fetch("fixture")], "cold")
    end

    def attributes_case_tests!
      cases = attributes_cases
      assert(cases.length == 14 && cases.map { |item| item.fetch(:name) }.uniq.length == 14, "twelve existing cases plus exactly two K1 branches")
      assert(cases.first(4).map { |item| item.values_at(:rank, :confidence) } == [[19, 0.5], [20, 0.6], [30, 0.7], [40, 0.8]], "independent trust bucket boundary answers")
      assert(ATTRIBUTE_INPUTS.map { |_key, type, _value| type }.uniq.sort == %i[bigint bool int numeric text], "all valid channels, no fabricated UUID key")
      assert(ATTRIBUTE_INPUTS.map(&:first).sort == (IngestionValidation::VALIDATION_TYPES.values.flatten - %w[imdb_id tmdb_id tvdb_id]).sort, "all non-D5 typed keys")
      cases.each do |test_case|
        %w[cold helpers-first].each do |mode|
          query = attributes_query(attributes_session(test_case, mode))
          assert(query.scan("FROM public.search_result_ingest_v1(").length == 3 && query.scan("ROLLBACK;").length == 1 && query.scan("COMMIT;").length == 2, "real calls rollback and committed reuse")
          assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no repair privilege or compiler setting changes")
          assert(query.index("attribute_context:") < query.index(mode == "helpers-first" ? "helpers:" : "BEGIN;"), "entry context precedes helpers and ingestion")
          pair = %w[reference final].to_h do |variant|
            evidence = attributes_test_evidence(test_case, mode, variant)
            role = variant == "reference" ? "postgres" : @runtime
            assert(attributes_validate!(evidence, test_case, mode, variant, role), "synthetic #{test_case.fetch(:name)} #{mode} #{variant}")
            [variant, attributes_comparable(evidence)]
          end
          assert(pair.fetch("reference") == pair.fetch("final"), "only validated roles/clocks/UUIDs and D4 are outside comparison")
        end
      end
      rejected("unknown attribute session") { attributes_session(cases.first, "reconnected") }
      fixture = attributes_test_evidence(cases.first, "cold", "final")
      signals = fixture.fetch("after").fetch("canonical_torrent_signal")
      assert(signals.map { |row| row.fetch("canonical_torrent_signal_id") } == (1..6).to_a + (13..24).to_a, "rollback sequence gaps and repeated signal rows retained")
      assert(signals.select { |row| row.fetch("signal_key") == "language" }.map { |row| row.fetch("confidence") } == [0.5, 0.5, 0.5], "NULL-distinct keys do not boost repeated confidence")
      tables = fixture.fetch("fixture").fetch("tables_after")
      reordered = copy(tables)
      rows = reordered.fetch("search_request_source_observation_attr")
      rows[0]["observation_attr_id"], rows[1]["observation_attr_id"] = 2, 1
      assert(attributes_first_tables?(reordered, tables), "unordered initial insertion still checks all values")
      assert(attributes_comparable(fixture) != attributes_comparable(fixture.merge("fixture" => fixture.fetch("fixture").merge("tables_after" => reordered))), "paired evidence retains exact generated attribute identities")
      rows[0]["observation_attr_id"] = 1
      assert(!attributes_first_tables?(reordered, tables), "duplicate generated attribute IDs rejected")
      rows[0]["observation_attr_id"] = 2
      rows[0]["value_text"] = "wrong"
      assert(!attributes_first_tables?(reordered, tables), "unordered matching cannot hide changed values")
    end

    def attributes_trust_tests!(source)
      cases = attributes_cases.last(2)
      assert(cases.map { |item| item.fetch(:name) } == %w[null-instance-trust-key missing-public-trust-tier], "two distinct K1 fixtures")
      assert(cases.map { |item| item.fetch(:trust_fixture) } == [{ key: nil, public_rank: 40 }, { key: "public", public_rank: nil }], "NULL key retains a discriminating rank-40 row; non-null key has no row")
      assert(cases.map { |item| item.fetch(:trust_rank_expectation).fetch("source_branch") } == ["0052:655 false; retain initialization at 471", "0052:655 true; 660 true; assign zero at 661"], "expectations distinguish skipped lookup from missing-row fallback")
      { 471 => "instance_trust_rank SMALLINT := 0;", 655 => "IF instance_trust_tier_key IS NOT NULL THEN",
        660 => "IF instance_trust_rank IS NULL THEN", 661 => "instance_trust_rank := 0;" }.each do |line, statement|
        assert(source.lines.fetch(line - 1).strip == statement, "source coordinate #{line} is a premise, not an executed observation")
      end
      cases.each do |test_case|
        expectation = test_case.fetch(:trust_rank_expectation)
        assert(expectation.values_at("rank", "bucket", "confidence", "native_branch_evidence") == [0, 0, 0.5, "pending"], "independent local-value expectations never claim native observation")
        assert(test_case.values_at(:rank, :confidence) == [0, 0.5] && test_case.fetch(:inputs) == [["language_primary", :text, "en"]], "literal K1 input and confidence answer")
        query = attributes_fixture_sql(test_case)
        if test_case.fetch(:name) == "null-instance-trust-key"
          assert(query.include?("SET trust_tier_key = NULL WHERE indexer_instance_id = 569001;") && query.include?("SET rank = 40 WHERE trust_tier_key = 'public';") && !query.include?("DELETE"), "nullable-key fixture keeps its public tier")
        else
          assert(query.include?("SET trust_tier_key = 'public' WHERE indexer_instance_id = 569001;") && query.include?("DELETE FROM public.trust_tier WHERE trust_tier_key = 'public';") && !query.include?("SET rank"), "missing-tier fixture keeps the public instance key and deletes only its tier")
        end
        assert(!query.match?(/DISABLE|TRIGGER|CONSTRAINT|ALTER|DROP|session_replication_role/), "ordinary constrained fixture DML only")
        %w[cold helpers-first].each do |mode|
          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = attributes_test_evidence(test_case, mode, variant)
            frames = [evidence.fetch("fixture")] + evidence.fetch("frames")
            assert(frames.all? { |frame| %w[tables_before tables_after tables_finish].all? { |key| frame.fetch(key).keys.sort == IngestionProof::INGESTION_TABLES.sort } }, "all 18 table images retained at every phase")
            assert(evidence.fetch("after").fetch("canonical_torrent_signal").all? { |row| row.values_at("signal_key", "value_text", "value_int", "confidence") == ["language", "en", nil, 0.5] }, "every retained language answer is independently fixed at en/0.5")
            assert(evidence.fetch("frames").first.fetch("tables_finish") == evidence.fetch("before") && evidence.fetch("frames").map { |frame| frame.fetch("state") } == (variant == "reference" ? %w[00000 00000 42P07] : %w[00000 00000 00000]), "whole rollback and frozen D4 versus final committed reuse retained")
            changed = copy(evidence)
            changed.fetch("trust_rank_expectation")["native_branch_evidence"] = "observed"
            rejected("pending native evidence") { attributes_validate!(changed, test_case, mode, variant, role) }
            changed = copy(evidence)
            changed.delete("trust_rank_expectation")
            rejected("trust-rank expectation") { attributes_validate!(changed, test_case, mode, variant, role) }
            inputs = copy(evidence.fetch("inputs_before"))
            if test_case.fetch(:name) == "null-instance-trust-key"
              inputs.fetch("indexer_instance").first["trust_tier_key"] = "public"
            else
              inputs.fetch("trust_tier") << { "trust_tier_key" => "public", "rank" => 0, "created_at" => evidence.fetch("seed_clock") }
            end
            changed = copy(evidence)
            %w[inputs_before inputs_fixture inputs_after].each { |key| changed[key] = copy(inputs) }
            rejected("read inputs") { attributes_validate!(changed, test_case, mode, variant, role) }
            attributes_trust_state_mutations!(evidence, test_case, mode, variant, role)
          end
        end
      end
      pair = cases.map { |test_case| attributes_comparable(attributes_test_evidence(test_case, "cold", "final")) }
      assert(pair.first.fetch("application") == pair.last.fetch("application") && pair.first.fetch("inputs") != pair.last.fetch("inputs"), "common application output cannot substitute for distinct read-input provenance")
      assert(pair.first.fetch("trust_rank_expectation") != pair.last.fetch("trust_rank_expectation"), "comparison retains distinct pending branch expectations")
    end

    def attributes_trust_state_mutations!(evidence, test_case, mode, variant, role)
      IngestionProof::INGESTION_TABLES.each do |table|
        changed = copy(evidence)
        changed.fetch("frames").fetch(1).fetch("tables_after").fetch(table) << { "unintended" => true }
        attributes_refresh_transport!(changed, mode)
        rejected("") { attributes_validate!(changed, test_case, mode, variant, role) }
      end
      IngestionPolicy::POLICY_READ_TABLES.each do |table|
        changed = copy(evidence)
        %w[inputs_before inputs_fixture inputs_after].each do |key|
          rows = changed.fetch(key).fetch(table)
          changed.fetch(key)[table] = rows.empty? ? [{ "unintended" => true }] : []
        end
        rejected("read inputs") { attributes_validate!(changed, test_case, mode, variant, role) }
      end
      ["fixture", 0, 1].each do |phase|
        changed = copy(evidence)
        frame = phase == "fixture" ? changed.fetch("fixture") : changed.fetch("frames").fetch(phase)
        frame.fetch("tables_after").fetch("canonical_torrent_signal").last["confidence"] = 0.8
        attributes_refresh_transport!(changed, mode)
        rejected("") { attributes_validate!(changed, test_case, mode, variant, role) }
      end
    end

    def attributes_trust_producer_tests!
      @contract.validate_output_path!
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      attributes_cases.last(2).each do |test_case|
        Dir.mktmpdir("attributes-trust-unit-", @contract.output_path) do |directory|
          @attributes_evidence = directory
          @attributes_validated_evidence = {}
          evidence = attributes_test_evidence(test_case, "cold", "final")
          outcomes = [evidence.fetch("fixture_transport"), evidence].map do |record|
            CommandRunner::Result.new(stdout: record.fetch("stdout"), stderr: record.fetch("stderr"), success: true)
          end
          commands = []
          seed = nil
          snapshots = [evidence.fetch("before"), evidence.fetch("after")]
          define_singleton_method(:sql) { |query, **_options| commands << query; "" }
          define_singleton_method(:policy_setup!) { |query, *_args| seed = query; evidence.fetch("seed_clock") }
          define_singleton_method(:correction_observer!) { |*_args| nil }
          define_singleton_method(:policy_read_snapshot) { |*_args| evidence.fetch("inputs_before") }
          define_singleton_method(:ingestion_snapshot) { |*_args| snapshots.shift }
          define_singleton_method(:result) { |*_args, **_options| outcomes.shift }
          assert(attributes_isolated(test_case, "cold", "final", @database, @runtime) == evidence, "real attribute producer retains serialized fixture/input provenance and pending expectation")
          assert(seed.end_with?(attributes_fixture_sql(test_case)) && seed.include?("INSERT INTO public.indexer_instance"), "K1 uses the existing seed plus its exact constrained delta")
          assert(@attributes_validated_evidence.length == 1 && outcomes.empty? && snapshots.empty?, "real producer validates both fixture and tested session before registration")
          assert(commands.last == 'DROP DATABASE "ingestion_attributes_final" WITH (FORCE)', "producer cleanup remains scoped to its owned clone")
        end
      end
    ensure
      %i[sql policy_setup! correction_observer! policy_read_snapshot ingestion_snapshot result].each do |name|
        singleton_class.remove_method(name) if singleton_methods.include?(name)
      end
    end

    def attributes_mutation_tests!
      test_case = attributes_cases.first
      evidence = attributes_test_evidence(test_case, "cold", "final")
      # Coherent transport rewrites exercise the independent semantic validator,
      # not merely disagreement between parsed and serialized copies.
      mutations = {
        "backend" => ->(record) { record.fetch("fixture")["backend"] = "101" },
        "required success" => ->(record) { record.fetch("frames").last.delete("result"); record.fetch("frames").last.merge!("state" => "42P07", "diagnostic" => ingestion_diagnostics(policy_d4_expected, role: "postgres").first) },
        "scratch lifetime" => ->(record) { record.fetch("frames").last["outside"] = "true" },
        "read inputs" => ->(record) { record.fetch("inputs_before").fetch("trust_tier").first["rank"] = 20 },
        "read inputs changed" => ->(record) { record.fetch("inputs_before").fetch("policy_rule") << { "unintended" => true } },
        "repeat results" => ->(record) { record.fetch("frames").last.fetch("result")["canonical_changed"] = true },
        "repeat results changed" => ->(record) { record.fetch("frames").last.fetch("tables_after").fetch("canonical_torrent_signal").last["confidence"] = 0.55 }
      }
      mutations.each do |message, mutate|
        changed = copy(evidence)
        mutate.call(changed)
        attributes_refresh_transport!(changed, "cold")
        # Final error diagnostic line is offset by the approved D3 directive.
        if message == "required success"
          changed["stderr"] = changed.fetch("stderr").sub(/line (\d+) at/) { "line #{Regexp.last_match(1).to_i + 1} at" }
          changed.fetch("frames").last["diagnostic"] = ingestion_diagnostics(changed.fetch("stderr"), role: @runtime).first
        end
        rejected(message) { attributes_validate!(changed, test_case, "cold", "final", @runtime) }
      end
      IngestionProof::INGESTION_TABLES.each do |table|
        changed = copy(evidence)
        last = changed.fetch("frames").last
        last.fetch("tables_after").fetch(table) << { "unintended" => true }
        attributes_refresh_transport!(changed, "cold")
        rejected("repeat results") { attributes_validate!(changed, test_case, "cold", "final", @runtime) }
      end
      changed = copy(evidence)
      changed.fetch("fixture").fetch("result")["canonical_torrent_public_id"] = "56900000-0000-4000-8000-000000000092"
      attributes_refresh_transport!(changed, "cold")
      rejected("first results") { attributes_validate!(changed, test_case, "cold", "final", @runtime) }
      changed = copy(evidence)
      changed.fetch("frames").last["tables_finish"] = attributes_empty
      attributes_refresh_transport!(changed, "cold")
      rejected("continuity") { attributes_validate!(changed, test_case, "cold", "final", @runtime) }
      changed = copy(evidence)
      changed.fetch("fixture").fetch("tables_after").fetch("canonical_torrent").first["title_display"] = "wrong"
      attributes_refresh_transport!(changed, "cold")
      rejected("first results") { attributes_validate!(changed, test_case, "cold", "final", @runtime) }
    end

    def attributes_transport_tests!
      test_case = attributes_cases.first
      evidence = attributes_test_evidence(test_case, "helpers-first", "reference")
      stdout, stderr = evidence.values_at("stdout", "stderr")
      session = attributes_session(test_case, "helpers-first")
      [stdout.lines.drop(1).join, "attribute_context:{\n" + stdout.lines.drop(1).join, stdout + stdout,
       stdout.sub(/^helpers:.*\n/, ""), stdout.sub(/^helpers:.*\n/, "helpers:{}\n")].each do |changed|
        rejected("") { attributes_parse(changed, stderr, session, "postgres") }
      end
      ["", stderr + stderr, stderr + "NOTICE: extra\n", stderr.sub("42P07", "P0001")].each do |changed|
        rejected("diagnostic") { attributes_parse(stdout, changed, session, "postgres") }
      end
      changed = copy(evidence)
      changed["stderr"] = stderr.sub("createas.c:406", "createas.c:407")
      changed.merge!(attributes_parse(stdout, changed.fetch("stderr"), session, "postgres"))
      rejected("exact D4") { attributes_validate!(changed, test_case, "helpers-first", "reference", "postgres") }
      evidence.fetch("frames").pop
      rejected("serialized transport") { attributes_validate!(evidence, test_case, "helpers-first", "reference", "postgres") }
    end

    def attributes_bytes_tests!
      test_case = attributes_cases.first
      evidence = attributes_test_evidence(test_case, "cold", "final")
      @contract.validate_output_path!
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      Dir.mktmpdir("attributes-unit-", @contract.output_path) do |directory|
        @attributes_evidence = directory
        bytes = JSON.pretty_generate(evidence) + "\n"
        path = attributes_write("synthetic.json", bytes)
        registry = { path => Digest::SHA256.hexdigest(bytes) }
        assert(dependency_read_observation(path, registry).first == evidence, "synthetic registry validates exact bytes")
        [bytes + " ", "{", JSON.generate(evidence.merge("frames" => []))].each do |changed|
          File.binwrite(path, changed)
          rejected("bytes changed") { dependency_read_observation(path, registry) }
        end
        File.binwrite(path, bytes)
        rejected("current-process") { dependency_read_observation(path, {}) }
        rejected("cannot retain") { attributes_write("synthetic.json", bytes) }
        File.symlink(path, File.join(directory, "link.json"))
        rejected("cannot retain") { attributes_write("link.json", bytes) }
        rejected("invalid attribute evidence") { attributes_write("../outside.json", bytes) }
        assert(File.stat(path).mode & 0o777 == 0o600, "private evidence permissions")
      end
    end

    def attributes_start!
      containers = @runner.run!(["docker", "container", "ls", "-a", "--format", "{{.Names}}", "--filter", "name=^/#{@container}$"])
      volumes = @runner.run!(["docker", "volume", "ls", "--format", "{{.Name}}", "--filter", "name=^#{@attributes_volume}$"])
      raise Failure, "refuse pre-existing attribute resources" unless containers.strip.empty? && volumes.strip.empty?

      @attributes_owned_volume = true
      @runner.run!(["docker", "volume", "create", @attributes_volume])
      @attributes_owned_container = true
      @runner.run!(["docker", "run", "-d", "--pull=never", "--network", "none", "--name", @container, "--shm-size", "1g",
                    "--mount", "type=volume,source=#{@attributes_volume},target=/var/lib/postgresql/data",
                    "-e", "POSTGRES_HOST_AUTH_METHOD=trust", "-e", "POSTGRES_INITDB_ARGS=--locale=C --encoding=UTF8 --data-checksums",
                    "-e", "TZ=UTC", @contract.postgres_image])
    end

    def attributes_cleanup!
      @attributes_cleanup_records = []
      [[@attributes_owned_container, ["docker", "rm", "-fv", @container]],
       [@attributes_owned_volume, ["docker", "volume", "rm", @attributes_volume]]].each do |owned, command|
        next unless owned

        outcome = @runner.capture(command)
        @attributes_cleanup_records << { command:, stdout: outcome.stdout, stderr: outcome.stderr, success: outcome.success }
      end
      raise Failure, "exact attribute resource cleanup failed" unless @attributes_cleanup_records.all? { |row| row.fetch(:success) }
    end

    def attributes_cleanup_tests!
      original = @runner
      commands = []
      fake = Object.new
      fake.define_singleton_method(:capture) do |command|
        commands << command
        CommandRunner::Result.new(stdout: "", stderr: "synthetic failure", success: false)
      end
      @runner = fake
      @attributes_volume = "synthetic-owned-volume"
      @attributes_owned_container = true
      @attributes_owned_volume = true
      rejected("cleanup failed") { attributes_cleanup! }
      assert(commands == [["docker", "rm", "-fv", @container], ["docker", "volume", "rm", @attributes_volume]], "exact volume cleanup attempted after container failure")
      @attributes_owned_container = false
      @attributes_owned_volume = false
      commands.clear
      attributes_cleanup!
      assert(commands.empty?, "no unowned resource removal")
    ensure
      @runner = original
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    valid = ARGV.empty? || ARGV == ["--unit"] || (ARGV.length == 2 && ARGV.first == "--live")
    raise RevaerDatabaseRebaseline::Failure, "usage: database-ingestion-attributes-test.rb [--unit|--live PINNED_CANDIDATE]" unless valid

    contract = RevaerDatabaseRebaseline::Contract.new(root: File.expand_path("../..", __dir__))
    proof = RevaerDatabaseRebaseline::IngestionAttributesTest.new(contract)
    ARGV.first == "--live" ? proof.run_live!(ARGV.fetch(1)) : proof.run_tests!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-attributes: #{error.message}"
    exit 1
  end
end
