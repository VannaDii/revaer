# frozen_string_literal: true

require "tmpdir"
require_relative "../database_rebaseline/final_proof"
require_relative "../database_rebaseline/ingestion_validation"

module RevaerDatabaseRebaseline
  class IngestionValidationTest < FinalProof
    include IngestionValidation

    def run_tests!
      @assertions = 0
      @runtime = "validation_unit_runtime"
      source = File.binread(File.join(@contract.root, "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql"))
      body = source.split("CREATE OR REPLACE FUNCTION search_result_ingest_v1(", 2).fetch(1).split("AS $$", 2).fetch(1).split("$$;", 2).first
      @ingestion_inventory = { "reference_proof" => { "routines" => [{ "name" => "search_result_ingest_v1", "signature" => "search_result_ingest_v1(uuid)", "source" => body }] } }
      validation_sites!
      validation_case_tests!
      validation_mutation_tests!
      validation_control_tests!
      validation_bytes_tests!
      validation_cleanup_tests!
      puts "database-ingestion-validation-test: #{@assertions} assertions passed"
    end

    # Fixture-only execution on the exact pinned pair, not canonical D3 wiring.
    def run_live!(candidate_path)
      @contract.freeze!
      @candidate = File.binread(candidate_path)
      @contract.verify_candidate_source!(@candidate)
      @contract.validate_output_path!
      FileUtils.mkdir_p(@contract.output_path, mode: 0o700)
      raise Failure, "validation candidate output must not be a symlink" if File.symlink?(@contract.candidate_path)

      File.binwrite(@contract.candidate_path, @candidate)
      final = FinalSql.new(@contract).verify!
      @routines = FinalSql.new(@contract).routines(@candidate)
      @container = "revaer-validation-proof-#{Process.pid}-#{SecureRandom.hex(8)}"
      @validation_volume = "#{@container}-data"
      @validation_owned_container = false
      @validation_owned_volume = false
      record = { completed: false, passed: false, d3_complete: false, cleanup: false,
                 source_commit: @runner.run!(["git", "rev-parse", "HEAD"], chdir: @contract.root).strip,
                 source_status: @runner.run!(["git", "status", "--porcelain=v1"], chdir: @contract.root),
                 source_sha256: validation_source_hashes, postgres_image: @contract.postgres_image,
                 candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
                 container: @container, volume: @validation_volume }
      begin
        validation_start!
        wait_ready!
        provision!
        check("validation pinned PostgreSQL identity", sql("SELECT current_setting('server_version_num')", role: "postgres") == "160014")
        apply!("reference_proof", @candidate, role: "postgres")
        apply!(@database, final)
        seal!
        @ingestion_evidence = File.join(@contract.output_path, "ingestion-proof")
        FileUtils.mkdir_p(@ingestion_evidence, mode: 0o700)
        @ingestion_inventory = {}
        ingestion_inventory!
        verify_ingestion_corrections!
        raise Failure, "existing correction matrix failed" unless @failures.empty?

        verify_ingestion_validation!
        record[:completed] = true
      rescue StandardError => error
        record[:error] = "#{error.class}: #{error.message}"
        raise
      ensure
        begin
          validation_cleanup!
          record[:cleanup] = true
        rescue Failure => error
          record[:cleanup_error] = error.message
          raise
        ensure
          record[:checks] = @checks
          record[:cleanup_records] = @validation_cleanup_records
          record[:evidence] = @validation_evidence
          record[:passed] = record[:completed] && record[:cleanup] && @failures.empty?
          path = File.join(@contract.output_path, "validation-live-#{Process.pid}-#{SecureRandom.hex(4)}.json")
          File.open(path, File::WRONLY | File::CREAT | File::EXCL, 0o600) { |file| file.write(JSON.pretty_generate(record) + "\n") }
          puts "database-ingestion-validation-live: report=#{path} passed=#{record.fetch(:passed)} d3_complete=false"
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
      assert(error.message.include?(message), "unexpected failure: #{error.message}")
    else
      raise Failure, "expected rejection: #{message}"
    end

    def copy(value)
      Marshal.load(Marshal.dump(value))
    end

    def validation_case_tests!
      cases = validation_cases
      assert(VALIDATION_SITES.length == 32 && cases.length == 108, "exact bounded guard and case inventory")
      assert(cases.map { |item| item.fetch(:name) }.uniq.length == cases.length, "unique validation cases")
      assert(cases.filter_map { |item| item[:site] }.uniq.sort == VALIDATION_SITES.keys.sort, "every declared exact guard site executes")
      assert(cases.count { |item| item[:site].nil? } == 4, "four legitimate acceptance controls")
      assert(cases.count { |item| item[:site] == :companion } == 7, "every missing companion")
      assert(cases.count { |item| item[:site] == :length } == 15, "every shorter/longer companion and empty keys")
      assert(cases.count { |item| item[:site] == :type } == 19, "all declared key/type contracts")
      cases.each do |test_case|
        assert(ingestion_call(test_case.fetch(:arguments)).scan("_input =>").length == 24, "#{test_case.fetch(:name)} uses exact real entry signature")
        assert((test_case.fetch(:fixture).keys - %i[enabled deleted run status migration]).empty?, "fixture keys cannot hide ingestion arguments")
        %w[cold helpers-first].each do |mode|
          session = validation_session_case(test_case, mode)
          query = correction_session(session)
          assert(query.scan("FROM public.search_result_ingest_v1(").length == session.fetch(:calls).length, "actual cold/warm calls")
          assert(query.scan("COMMIT;").length == 2, "two natural commits without backend replacement")
          assert(!query.match?(/DISCARD|DROP TABLE|SET ROLE|SET plpgsql|\\connect/), "no compiler or scratch repair")
          next unless test_case[:site]

          %w[reference final].each do |variant|
            role = variant == "reference" ? "postgres" : @runtime
            evidence = validation_error_fixture(test_case, mode, role)
            assert(validation_validate!(evidence, test_case, mode, variant, role), "synthetic #{test_case.fetch(:name)} #{mode} #{variant}")
          end
        end
      end
      rejected("unknown validation mode") { validation_session_case(cases.first, "reconnected") }
      source = @ingestion_inventory.fetch("reference_proof").fetch("routines").first
      previous = source.fetch("source")
      source["source"] = "\n" + previous
      rejected("guard coordinate changed") { validation_sites! }
      source["source"] = previous
    end

    def validation_test_inputs(test_case, clock)
      inputs = IngestionPolicy::POLICY_READ_TABLES.to_h { |table| [table, []] }
      fixture = test_case.fetch(:fixture)
      inputs["indexer_definition"] = [{ "indexer_definition_id" => 569001, "created_at" => clock, "updated_at" => clock, "upstream_slug" => "ingestion-proof", "definition_hash" => "a" * 64 }]
      inputs["indexer_instance"] = [{ "indexer_instance_id" => 569001, "created_at" => clock, "updated_at" => clock,
        "indexer_instance_public_id" => "56900000-0000-4000-8000-000000000001", "indexer_definition_id" => 569001,
        "is_enabled" => fixture.fetch(:enabled, true), "deleted_at" => fixture[:deleted] ? "2026-09-10T00:00:00+00:00" : nil,
        "migration_state" => fixture.fetch(:migration, "ready"), "trust_tier_key" => "public" }]
      inputs["policy_snapshot"] = [{ "policy_snapshot_id" => 569001, "created_at" => clock, "snapshot_hash" => "b" * 64 }]
      inputs["search_request"] = [{ "search_request_id" => 569001, "created_at" => clock,
        "search_request_public_id" => "56900000-0000-4000-8000-000000000002", "policy_snapshot_id" => 569001,
        "status" => fixture.fetch(:status, "running"), "page_size" => 10, "query_text" => "Ingestion proof",
        "finished_at" => fixture.key?(:status) ? clock : nil, "canceled_at" => fixture[:status] == "canceled" ? clock : nil,
        "failure_class" => fixture[:status] == "failed" ? "coordinator_error" : nil }]
      inputs["trust_tier"] = [{ "trust_tier_key" => "public", "created_at" => clock, "rank" => 10 }]
      inputs["media_domain"] = [{ "media_domain_key" => "movies", "created_at" => clock }]
      inputs["search_request_indexer_run"] = [{ "search_request_id" => 569001, "indexer_instance_id" => 569001, "status" => "queued" }] unless fixture[:run] == false
      inputs
    end

    def validation_error_fixture(test_case, mode, role)
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      frames = [1, 2].map do |number|
        { "backend" => "123", "role" => { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" },
          "clock" => "2026-09-11T01:00:0#{number}+00:00", "before" => "error", "after" => "error", "state" => "P0001",
          "within" => "false", "outside" => "false", "tables_before" => copy(empty), "tables_after" => copy(empty), "tables_finish" => copy(empty),
          "diagnostic" => validation_guard_diagnostic(test_case.fetch(:site)) }
      end
      clock = "2026-09-11T01:00:00+00:00"
      inputs = validation_test_inputs(test_case, clock)
      evidence = { "name" => "explicit-synthetic-unit-fixture", "seed_clock" => clock, "before" => empty, "after" => copy(empty),
                   "inputs_before" => inputs, "inputs_after" => copy(inputs), "frames" => frames }
      validation_encode(evidence, mode, role)
    end

    def validation_encode(evidence, mode, role)
      lines = mode == "helpers-first" ? ["helpers:#{JSON.generate(ingestion_helper_expectations)}"] : []
      diagnostics = []
      evidence.fetch("frames").each do |frame|
        IngestionCorrections::CORRECTION_RECORDS.each do |key|
          lines << JSON.generate(frame.fetch("result")) if key == "state" && frame.key?("result")
          value = frame.fetch(key)
          value = JSON.generate(value) if %w[role clock tables_before tables_after tables_finish].include?(key)
          lines << "#{key}:#{value}"
        end
        next unless frame.key?("diagnostic")

        diagnostic = frame.fetch("diagnostic")
        line = diagnostic.fetch("line") + (role == @runtime ? 1 : 0)
        detail = diagnostic.fetch("detail") ? "DETAIL:  #{diagnostic.fetch('detail')}\n" : ""
        diagnostics << "ERROR:  #{diagnostic.fetch('error')}\n#{detail}CONTEXT:  #{diagnostic.fetch('routine')} line #{line} at #{diagnostic.fetch('operation')}\nLOCATION:  #{diagnostic.fetch('location')}\n"
      end
      evidence.merge("stdout" => lines.join("\n") + "\n", "stderr" => diagnostics.join)
    end

    def validation_mutation_tests!
      test_case = validation_cases.find { |item| item[:name] == "negative-year" }
      original = validation_error_fixture(test_case, "cold", @runtime)
      %w[error detail routine line operation location hint statement].each do |key|
        changed = copy(original)
        changed.fetch("frames").first.fetch("diagnostic")[key] = key == "line" ? 904 : "changed"
        assert(!validation_states?(changed.fetch("frames"), test_case, "final", @runtime), "reject diagnostic #{key}")
      end
      # Coherent raw + parsed mutation, same SQLSTATE/DETAIL but another real site.
      changed = copy(original)
      changed.fetch("frames").each { |frame| frame["diagnostic"] = validation_guard_diagnostic(:tracker_category) }
      changed = validation_encode(changed, "cold", @runtime)
      rejected("guard site changed") { validation_validate!(changed, test_case, "cold", "final", @runtime) }
      changed = copy(original)
      changed.fetch("frames").each { |frame| frame["state"] = "00000"; frame["result"] = {}; frame.delete("diagnostic") }
      changed = validation_encode(changed, "cold", @runtime)
      rejected("guard site changed") { validation_validate!(changed, test_case, "cold", "final", @runtime) }
      [nil, "0", "-1", 123, "different"].each do |backend|
        changed = copy(original)
        changed.fetch("frames").each { |frame| frame["backend"] = backend }
        assert(!validation_context?(changed.fetch("frames"), @runtime), "reject invalid coherent backend #{backend.inspect}")
      end
      %w[backend before after clock role].each do |key|
        changed = copy(original)
        changed.fetch("frames").last[key] = key == "role" ? {} : "changed"
        assert(!validation_context?(changed.fetch("frames"), @runtime), "reject caller context #{key}")
      end
      IngestionProof::INGESTION_TABLES.each do |table|
        %w[tables_after tables_finish].each do |phase|
          changed = copy(original)
          changed.fetch("frames").first.fetch(phase)[table] << { "unintended" => "late write" }
          changed = validation_encode(changed, "cold", @runtime)
          rejected("18-table rollback") { validation_validate!(changed, test_case, "cold", "final", @runtime) }
        end
        changed = copy(original)
        changed.fetch("frames").each { |frame| frame.fetch("tables_before").delete(table) }
        assert(!validation_images?(changed, test_case), "reject missing #{table} comparison")
      end
      %w[inputs_before inputs_after].each do |key|
        changed = copy(original)
        changed.fetch(key).fetch("search_request").first["page_size"] = 11
        assert(!validation_inputs?(changed, test_case), "read inputs must remain unchanged")
      end
      changed = copy(original)
      %w[inputs_before inputs_after].each { |key| changed.fetch(key).fetch("search_request").first["status"] = "finished" }
      assert(!validation_inputs?(changed, test_case), "coherent incorrect eligibility fixture cannot pass paired equality")
      changed = copy(original)
      %w[inputs_before inputs_after].each { |key| changed.fetch(key).fetch("search_request").first["created_at"] = "2026-09-11T00:00:00+00:00" }
      assert(!validation_inputs?(changed, test_case), "read clock normalization requires exact seed provenance")
      changed = copy(original)
      changed["seed_clock"] = "2026-09-11T00:30:00+00:00"
      changed["inputs_before"] = validation_test_inputs(test_case, changed.fetch("seed_clock"))
      changed["inputs_after"] = copy(changed.fetch("inputs_before"))
      assert(validation_inputs?(changed, test_case), "separately recorded seed transaction accepted")
      assert(validation_comparable(original, test_case) == validation_comparable(changed, test_case), "only validated read clocks normalize")
      %w[inputs_before inputs_after].each { |key| changed.fetch(key).fetch("trust_tier").first["rank"] = 40 }
      assert(validation_comparable(original, test_case) != validation_comparable(changed, test_case), "lookup values remain in paired evidence")
      validation_framing_tests!(original, test_case)
    end

    def validation_framing_tests!(evidence, test_case)
      stdout = evidence.fetch("stdout")
      stderr = evidence.fetch("stderr")
      lines = stdout.lines
      [lines.drop(1).join, stdout + lines.first, stdout + stdout, lines[0...-1].join].each do |changed|
        rejected("correction") { validation_parse(changed, stderr, test_case, "cold", @runtime) }
      end
      ["", stderr + stderr, stderr + "NOTICE: extra\n", stderr.sub("3897", "3898")].each do |changed|
        if changed.include?("3898")
          parsed = validation_parse(stdout, changed, test_case, "cold", @runtime)
          assert(!validation_states?(parsed, test_case, "final", @runtime), "native raise location must be exact")
        else
          rejected("diagnostic") { validation_parse(stdout, changed, test_case, "cold", @runtime) }
        end
      end
      rejected("helper-first") { validation_parse(stdout, stderr, test_case, "helpers-first", @runtime) }
      helpers = "helpers:#{JSON.generate(ingestion_helper_expectations)}\n"
      rejected("record") { validation_parse(helpers + helpers + stdout, stderr, test_case, "helpers-first", @runtime) }
      wrong = ingestion_helper_expectations.merge("normalize_title_v1" => [nil, "wrong"])
      rejected("helper-first") { validation_parse("helpers:#{JSON.generate(wrong)}\n" + stdout, stderr, test_case, "helpers-first", @runtime) }
      changed = copy(evidence)
      changed.fetch("frames").pop
      rejected("serialized frames") { validation_validate!(changed, test_case, "cold", "final", @runtime) }
    end

    def validation_control_tests!
      control = validation_cases.find { |item| item[:name] == "aligned-empty-arrays" }
      query = correction_session(validation_session_case(control, "cold"))
      assert(query.scan("ROLLBACK;").length == 1 && query.scan("COMMIT;").length == 2, "warm rollback is not frozen committed reuse")
      %w[reference final].each do |variant|
        frames = [0, 1, 2].map { |index| { "state" => "00000", "result" => {}, "within" => "true", "outside" => (variant == "reference" && index.positive?).to_s } }
        if variant == "reference"
          frames.last.delete("result")
          frames.last.merge!("state" => "42P07", "diagnostic" => ingestion_diagnostics(policy_d4_expected, role: "postgres").first)
        end
        role = variant == "reference" ? "postgres" : @runtime
        assert(validation_states?(frames, control, variant, role), "exact #{variant} control SQLSTATEs")
        assert(validation_lifetime?(frames, control, variant), "exact #{variant} scratch lifetime")
        changed = copy(frames)
        changed.last["state"] = variant == "reference" ? "00000" : "42P07"
        assert(!validation_states?(changed, control, variant, role), "no manufactured frozen reuse or final failure acceptance")
        frames.last["outside"] = "changed"
        assert(!validation_lifetime?(frames, control, variant), "no scratch lifetime normalization without validation")
      end
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      empty["canonical_torrent"] = [{ "canonical_torrent_id" => 1, "updated_at" => "old" }]
      changed = copy(empty)
      changed.fetch("canonical_torrent").first["updated_at"] = "new"
      frame = { "tables_before" => empty, "tables_after" => changed, "clock" => "new" }
      assert(validation_control_repeat?(frame), "repeat delta preserves all non-clock fields")
      IngestionProof::INGESTION_TABLES.each do |table|
        mutation = copy(frame)
        mutation.fetch("tables_after").fetch(table) << { "unintended" => true }
        assert(!validation_control_repeat?(mutation), "third final success cannot conceal #{table} drift")
      end
      frames = validation_control_fixture
      assert(validation_control?(frames), "shared result helper receives case identity for actual reused-row flags")
      frames.last.fetch("result")["durable_source_created"] = true
      assert(!validation_control?(frames), "reused source cannot claim new-row success")
    end

    def validation_control_fixture
      empty = IngestionProof::INGESTION_TABLES.to_h { |table| [table, []] }
      tables = copy(empty)
      %w[canonical_torrent_source_context_score canonical_size_sample canonical_size_rollup search_request_canonical search_page search_page_item].each { |table| tables[table] = [{}] }
      canonical_id = "56900000-0000-4000-8000-000000000090"
      source_id = "56900000-0000-4000-8000-000000000091"
      tables["canonical_torrent"] = [{ "canonical_torrent_id" => 1, "canonical_torrent_public_id" => canonical_id,
        "infohash_v1" => "a" * 40, "title_display" => "Ingestion proof title", "title_normalized" => "ingestion proof title",
        "size_bytes" => 1024, "identity_strategy" => "infohash_v1", "updated_at" => "old" }]
      tables["canonical_torrent_source"] = [{ "canonical_torrent_source_id" => 1, "canonical_torrent_source_public_id" => source_id,
        "indexer_instance_id" => 569001, "source_guid" => "ingestion-proof-source", "last_seen_seeders" => 5, "last_seen_leechers" => 2, "updated_at" => "old" }]
      tables["search_request_source_observation"] = [{ "search_request_id" => 569001, "indexer_instance_id" => 569001,
        "canonical_torrent_id" => 1, "canonical_torrent_source_id" => 1, "title_raw" => "Ingestion proof title", "size_bytes" => 1024, "seeders" => 5, "leechers" => 2 }]
      result = { "canonical_torrent_public_id" => canonical_id, "canonical_torrent_source_public_id" => source_id,
                 "observation_created" => true, "durable_source_created" => true, "canonical_changed" => true }
      first = { "state" => "00000", "tables_before" => empty, "tables_after" => tables, "result" => result }
      after = copy(tables)
      { "canonical_torrent" => "updated_at", "canonical_torrent_source" => "updated_at",
        "canonical_torrent_source_context_score" => "computed_at", "canonical_size_rollup" => "updated_at" }.each do |table, column|
        after.fetch(table).first[column] = "new"
      end
      second = { "state" => "00000", "tables_before" => copy(tables), "tables_after" => after, "clock" => "new",
                 "result" => result.merge("observation_created" => false, "durable_source_created" => false, "canonical_changed" => false) }
      [first, second]
    end

    def validation_bytes_tests!
      test_case = validation_cases.first
      evidence = validation_error_fixture(test_case, "cold", @runtime)
      Dir.mktmpdir("revaer-validation-bytes.") do |directory|
        @validation_evidence = directory
        bytes = JSON.pretty_generate(evidence) + "\n"
        path = validation_write("synthetic.json", bytes)
        # Explicit synthetic unit registry, never a live replay provenance claim.
        registry = { path => Digest::SHA256.hexdigest(bytes) }
        assert(dependency_read_observation(path, registry).first == evidence, "original serialized bytes accepted")
        ["{", bytes + " ", JSON.generate(evidence.merge("frames" => [])), JSON.generate(evidence.merge("frames" => [evidence.fetch('frames').first] * 2))].each do |changed|
          File.binwrite(path, changed)
          rejected("bytes changed") { dependency_read_observation(path, registry) }
        end
        File.binwrite(path, bytes)
        rejected("current-process") { dependency_read_observation(path, {}) }
        rejected("current-process") { dependency_read_observation(path, nil) }
        File.unlink(path)
        rejected("cannot read") { dependency_read_observation(path, registry) }
        path = validation_write("synthetic.json", bytes)
        File.symlink(path, File.join(directory, "linked.json"))
        rejected("symlink") { validation_write("linked.json", bytes) }
        rejected("invalid validation evidence name") { validation_write("../escaped.json", bytes) }
        assert(File.stat(path).mode & 0o777 == 0o600, "private evidence bytes")
      end
    end

    def validation_start!
      containers = @runner.run!(["docker", "container", "ls", "-a", "--format", "{{.Names}}", "--filter", "name=^/#{@container}$"])
      volumes = @runner.run!(["docker", "volume", "ls", "--format", "{{.Name}}", "--filter", "name=^#{@validation_volume}$"])
      raise Failure, "refuse pre-existing validation resources" unless containers.strip.empty? && volumes.strip.empty?

      @validation_owned_volume = true
      @runner.run!(["docker", "volume", "create", @validation_volume])
      @validation_owned_container = true
      @runner.run!(["docker", "run", "-d", "--pull=never", "--network", "none", "--name", @container, "--shm-size", "1g",
                    "--mount", "type=volume,source=#{@validation_volume},target=/var/lib/postgresql/data",
                    "-e", "POSTGRES_HOST_AUTH_METHOD=trust", "-e", "POSTGRES_INITDB_ARGS=--locale=C --encoding=UTF8 --data-checksums",
                    "-e", "TZ=UTC", @contract.postgres_image])
    end

    def validation_cleanup!
      @validation_cleanup_records = []
      [[@validation_owned_container, ["docker", "rm", "-fv", @container]],
       [@validation_owned_volume, ["docker", "volume", "rm", @validation_volume]]].each do |owned, command|
        next unless owned

        outcome = @runner.capture(command)
        @validation_cleanup_records << { command:, stdout: outcome.stdout, stderr: outcome.stderr, success: outcome.success }
      end
      raise Failure, "exact validation resource cleanup failed" unless @validation_cleanup_records.all? { |row| row.fetch(:success) }
    end

    def validation_cleanup_tests!
      runner = @runner
      commands = []
      fake = Object.new
      fake.define_singleton_method(:capture) do |command|
        commands << command
        CommandRunner::Result.new(stdout: "", stderr: "synthetic failure", success: false)
      end
      @runner = fake
      @validation_volume = "synthetic-owned-volume"
      @validation_owned_container = true
      @validation_owned_volume = true
      rejected("cleanup failed") { validation_cleanup! }
      assert(commands == [["docker", "rm", "-fv", @container], ["docker", "volume", "rm", @validation_volume]], "attempt exact volume cleanup even after container failure")
      @validation_owned_container = false
      @validation_owned_volume = false
      commands.clear
      validation_cleanup!
      assert(commands.empty?, "never remove unowned resources")
    ensure
      @runner = runner
    end
  end
end

if $PROGRAM_NAME == __FILE__
  begin
    valid = ARGV.empty? || ARGV == ["--unit"] || (ARGV.length == 2 && ARGV.first == "--live")
    raise RevaerDatabaseRebaseline::Failure, "usage: database-ingestion-validation-test.rb [--unit|--live PINNED_CANDIDATE]" unless valid

    contract = RevaerDatabaseRebaseline::Contract.new(root: File.expand_path("../..", __dir__))
    proof = RevaerDatabaseRebaseline::IngestionValidationTest.new(contract)
    ARGV.first == "--live" ? proof.run_live!(ARGV.fetch(1)) : proof.run_tests!
  rescue RevaerDatabaseRebaseline::Failure => error
    warn "database-ingestion-validation: #{error.message}"
    exit 1
  end
end
