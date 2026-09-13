# frozen_string_literal: true

module RevaerDatabaseRebaseline
  # A matched prevent_merge rule retains the frozen unique-key failure, not a split.
  module IngestionDisambiguation
    DISAMBIGUATION_IDENTITIES = { "infohash_v1" => "a" * 40, "infohash_v2" => "b" * 64, "magnet_hash" => "c" * 64 }.freeze
    DISAMBIGUATION_MODES = %w[cold helpers-first].freeze

    private

    def disambiguation_cases
      DISAMBIGUATION_IDENTITIES.keys.product(%w[left right]).map do |identity, canonical_side|
        { name: "#{identity}-canonical-#{canonical_side}", identity:, canonical_side: }
      end
    end

    def disambiguation_arguments(spec)
      wrapper_arguments("size-source", title: IngestionSize::SIZE_TITLE).merge(
        infohash_v1_input: "NULL::char(40)", infohash_v2_input: "NULL::char(64)", magnet_hash_input: "NULL::char(64)",
        "#{spec.fetch(:identity)}_input".to_sym => "#{literal(DISAMBIGUATION_IDENTITIES.fetch(spec.fetch(:identity)))}::char(#{spec.fetch(:identity) == 'infohash_v1' ? 40 : 64})")
    end

    def disambiguation_session(spec, mode, fixture: false)
      raise Failure, "unknown disambiguation mode" unless DISAMBIGUATION_MODES.include?(mode)

      { calls: Array.new(fixture ? 1 : 2) { disambiguation_arguments(spec) }, wrapper: true,
        helpers: !fixture && mode == "helpers-first", finish_setting: true }
    end

    def disambiguation_rule(spec, uuid, clock)
      row = { "canonical_disambiguation_rule_id" => 1, "created_by_user_id" => 0, "created_at" => clock,
        "rule_type" => "prevent_merge", "reason" => "isolated ingestion proof" }
      %w[left right].each do |side|
        canonical = side == spec.fetch(:canonical_side)
        row.merge!("identity_#{side}_type" => canonical ? "canonical_public_id" : spec.fetch(:identity),
          "identity_#{side}_value_text" => canonical ? nil : DISAMBIGUATION_IDENTITIES.fetch(spec.fetch(:identity)),
          "identity_#{side}_value_uuid" => canonical ? uuid : nil)
      end
      row
    end

    def disambiguation_fixture_tables(spec, frame)
      tables = size_expected_tables(wrapper_size_cases.first.fetch(:size_case), frame, 0)
      identity = spec.fetch(:identity)
      hash = DISAMBIGUATION_IDENTITIES.fetch(identity)
      magnet = identity == "magnet_hash" ? hash : Digest::SHA256.hexdigest([hash].pack("H*"))
      %w[canonical_torrent canonical_torrent_source search_request_source_observation].each do |table|
        tables.fetch(table).first.merge!("infohash_v1" => identity == "infohash_v1" ? hash : nil,
          "infohash_v2" => identity == "infohash_v2" ? hash : nil, "magnet_hash" => magnet)
      end
      tables.fetch("canonical_torrent").first.merge!("identity_strategy" => identity,
        "identity_confidence" => identity == "magnet_hash" ? 0.85 : 1.0)
      tables
    end

    def disambiguation_wrapper_stack
      routine = @ingestion_inventory.fetch("reference_proof").fetch("routines").find { |row| row.fetch("name") == "search_result_ingest" }
      source = routine.fetch("source")
      statements = source.scan(/SELECT \*\n.*?FROM search_result_ingest_v1\(.*?\);/m)
      raise Failure, "frozen disambiguation wrapper site changed" unless statements.length == 1 && source.lines.fetch(8).strip == "SELECT *"

      # PL/pgSQL replaces its INTO target bytes with spaces in the SPI query.
      statement = statements.first.delete_suffix(";").sub(/INTO\n.*?(?=FROM)/m) { |target| " " * target.bytesize }
      "SQL statement \"#{statement}\"\nPL/pgSQL function #{routine.fetch('signature')} line 9 at SQL statement\n"
    end

    def disambiguation_diagnostic(spec, role)
      identity = spec.fetch(:identity)
      constraint = "canonical_torrent_#{identity}_uq"
      "ERROR:  23505: duplicate key value violates unique constraint \"#{constraint}\"\n" \
        "DETAIL:  Key (#{identity})=(#{DISAMBIGUATION_IDENTITIES.fetch(identity)}) already exists.\n" \
        "CONTEXT:  #{setting_path_stack(role)}#{disambiguation_wrapper_stack}" \
        "SCHEMA NAME:  public\nTABLE NAME:  canonical_torrent\nCONSTRAINT NAME:  #{constraint}\n" \
        "LOCATION:  _bt_check_unique, nbtinsert.c:666\n"
    end

    def disambiguation_parse(raw, session, spec, role, fixture: false)
      metadata_transport_json!(raw.fetch("stdout"))
      frames = correction_frames(raw.fetch("stdout"), session)
      states = Array.new(session.fetch(:calls).length, fixture ? "00000" : "23505")
      diagnostic = fixture ? "" : disambiguation_diagnostic(spec, role) * states.length
      raise Failure, "disambiguation exact outcome or native diagnostic changed" unless frames.map { |frame| frame.fetch("state") } == states && raw.fetch("stderr") == diagnostic

      frames
    end

    def disambiguation_execute(spec, mode, database, role, name, fixture: false)
      session = disambiguation_session(spec, mode, fixture:)
      raw = metadata_transport(correction_session(session), database, role, name)
      raw.merge("frames" => disambiguation_parse(raw, session, spec, role, fixture:))
    end

    def disambiguation_read(database, name)
      pairs = IngestionPolicy::POLICY_READ_TABLES.map do |table|
        "#{literal(table)}, (SELECT COALESCE(json_agg(row_to_json(t) ORDER BY to_jsonb(t)::text), '[]') FROM public.#{identifier(table)} t)"
      end
      query = <<~SQL
        SELECT json_build_object('tables', ingestion_observation.snapshot(),
          'inputs', json_build_object(#{pairs.join(',')}),
          'canonical_sequence', (SELECT last_value FROM pg_catalog.pg_sequences WHERE schemaname = 'public'
            AND sequencename = 'canonical_torrent_canonical_torrent_id_seq'));
      SQL
      raw = metadata_transport(query, database, "postgres", "#{name}-read")
      raw.merge("data" => metadata_read_parse(raw))
    end

    def disambiguation_read?(record, tables, inputs, sequence)
      data = metadata_read_parse(record)
      JSON.generate(data) == JSON.generate(record.fetch("data")) && data.keys.sort == %w[canonical_sequence inputs tables] &&
        size_tables_equal?(data.fetch("tables"), tables) && size_tables_equal?(data.fetch("inputs"), inputs) &&
        data.fetch("canonical_sequence").eql?(sequence)
    end

    def disambiguation_validate!(evidence, spec, mode, variant, role)
      fixture = evidence.fetch("fixture")
      tested = evidence.fetch("tested")
      [true, false].zip([fixture, tested]).each do |is_fixture, record|
        session = disambiguation_session(spec, mode, fixture: is_fixture)
        parsed = disambiguation_parse(record, session, spec, role, fixture: is_fixture)
        raise Failure, "disambiguation raw and declared frames differ" unless size_tables_equal?({ "frames" => parsed }, { "frames" => record.fetch("frames") })
      end
      first = fixture.fetch("frames").fetch(0)
      frames = tested.fetch("frames")
      all = [first, *frames]
      roles = { "session" => role, "current" => role, "superuser" => role == "postgres", "create_role" => role == "postgres", "bypass_rls" => role == "postgres" }
      raise Failure, "disambiguation role or settings changed" unless all.all? { |frame| frame.fetch("role") == roles && frame.values_at("before", "after", "finished_setting") == %w[error error error] }

      backends = frames.map { |frame| frame.fetch("backend") }
      raise Failure, "disambiguation backend provenance changed" unless all.all? { |frame| frame.fetch("backend").match?(/\A[1-9][0-9]*\z/) } && backends.uniq.length == 1 && !backends.include?(first.fetch("backend"))

      clocks = [evidence.fetch("seed_clock"), first.fetch("clock"), evidence.fetch("rule_clock"), *frames.map { |frame| frame.fetch("clock") }]
      raise Failure, "disambiguation transaction clocks changed" unless clocks.all? { |clock| metadata_clock?(clock) } && clocks.uniq.length == clocks.length

      answer = first.fetch("result")
      ids = answer.values_at("canonical_torrent_public_id", "canonical_torrent_source_public_id")
      expected_result = answer.slice("canonical_torrent_public_id", "canonical_torrent_source_public_id").merge(
        "canonical_changed" => true, "observation_created" => true, "durable_source_created" => true)
      raise Failure, "disambiguation fixture result changed" unless ids.uniq.length == 2 && ids.all? { |id| id.is_a?(String) && IngestionProof::INGESTION_UUID.match?(id) } && answer == expected_result

      tables = disambiguation_fixture_tables(spec, first)
      inputs = metadata_read_tables(evidence.fetch("seed_clock"))
      raise Failure, "disambiguation initial state changed" unless disambiguation_read?(evidence.fetch("initial"), attributes_empty, inputs, nil)
      raise Failure, "disambiguation fixture state changed" unless first.fetch("tables_before") == attributes_empty && %w[tables_after tables_finish].all? { |key| size_tables_equal?(first.fetch(key), tables) } && disambiguation_read?(evidence.fetch("before_rule"), tables, inputs, 1)

      inputs.fetch("canonical_disambiguation_rule") << disambiguation_rule(spec, ids.first, evidence.fetch("rule_clock"))
      raise Failure, "disambiguation rule or durable state changed" unless disambiguation_read?(evidence.fetch("before"), tables, inputs, 1) && disambiguation_read?(evidence.fetch("after"), tables, inputs, 3)
      raise Failure, "disambiguation failed call changed tables" unless frames.all? { |frame| %w[tables_before tables_after tables_finish].all? { |key| size_tables_equal?(frame.fetch(key), tables) } }
      raise Failure, "disambiguation temporary lifetime changed" unless first.values_at("within", "outside") == ["true", (variant == "reference").to_s] && frames.all? { |frame| frame.values_at("within", "outside") == %w[false false] }

      true
    end

    def disambiguation_isolated(spec, mode, variant, source, role)
      database = "ingestion_disambiguation_#{variant}"
      name = "#{spec.fetch(:name)}-#{mode}-#{variant}"
      created = false
      begin
        sql("CREATE DATABASE #{identifier(database)} TEMPLATE #{identifier(source)} OWNER #{identifier(@owner)}", role: "postgres", database: "postgres")
        created = true
        sql("REVOKE ALL ON DATABASE #{identifier(database)} FROM PUBLIC; GRANT CONNECT ON DATABASE #{identifier(database)} TO #{identifier(@runtime)}", role: "postgres", database:)
        seed = File.binread(File.join(@contract.root, "scripts/tests/database-ingestion-proof-seed.sql")) + "\nUPDATE public.trust_tier SET created_at = transaction_timestamp();\nUPDATE public.media_domain SET created_at = transaction_timestamp();"
        clock = policy_setup!(seed, database, File.join(@disambiguation_evidence, "#{name}-seed"))
        correction_observer!(database, variant)
        evidence = { "seed_clock" => clock, "initial" => disambiguation_read(database, "#{name}-initial") }
        evidence["fixture"] = disambiguation_execute(spec, mode, database, role, "#{name}-fixture", fixture: true)
        evidence["before_rule"] = disambiguation_read(database, "#{name}-before-rule")
        first = evidence.fetch("fixture").fetch("frames").fetch(0)
        uuid = first.fetch("result").fetch("canonical_torrent_public_id")
        raise Failure, "invalid fixture identity" unless IngestionProof::INGESTION_UUID.match?(uuid)

        row = disambiguation_rule(spec, uuid, clock).reject { |key, _value| %w[canonical_disambiguation_rule_id created_at].include?(key) }
        query = "INSERT INTO public.canonical_disambiguation_rule (#{row.keys.map { |key| identifier(key) }.join(',')}) VALUES (#{row.values.map { |value| policy_sql_value(value) }.join(',')});"
        evidence["rule_clock"] = policy_setup!(query, database, File.join(@disambiguation_evidence, "#{name}-rule"))
        evidence["before"] = disambiguation_read(database, "#{name}-before")
        evidence["tested"] = disambiguation_execute(spec, mode, database, role, name)
        evidence["after"] = disambiguation_read(database, "#{name}-after")
        metadata_write("#{name}.json", JSON.pretty_generate(evidence) + "\n")
        check("disambiguation #{name} independently modeled fixture rule native error rollback and warm retry", disambiguation_validate!(evidence, spec, mode, variant, role))
        { name:, validated: true, states: evidence.fetch("tested").fetch("frames").map { |frame| frame.fetch("state") }, application_succeeded: false }
      ensure
        sql("DROP DATABASE #{identifier(database)} WITH (FORCE)", role: "postgres", database: "postgres") if created
      end
    end

    def verify_ingestion_disambiguation!
      directory = File.join(@contract.output_path, "ingestion-disambiguation")
      raise Failure, "disambiguation evidence must not be a symlink" if File.symlink?(directory)

      FileUtils.mkdir_p(directory, mode: 0o700)
      @disambiguation_evidence = Dir.mktmpdir("run-", directory)
      previous_metadata, previous_correction = @metadata_evidence, @correction_evidence
      @metadata_evidence = @correction_evidence = @disambiguation_evidence
      hashes = wrapper_source_hashes.merge("scripts/tests/database-ingestion-disambiguation-test.rb" => Digest::SHA256.file(File.join(@contract.root, "scripts/tests/database-ingestion-disambiguation-test.rb")).hexdigest)
      first = @checks.length
      cases = []
      completed = false
      begin
        disambiguation_cases.each do |spec|
          DISAMBIGUATION_MODES.each do |mode|
            { "reference" => ["reference_proof", "postgres"], "final" => [@database, @runtime] }.each do |variant, (source, role)|
              cases << disambiguation_isolated(spec, mode, variant, source, role)
            end
          end
        end
        check("disambiguation source bytes unchanged", hashes.all? { |path, hash| Digest::SHA256.file(File.join(@contract.root, path)).hexdigest == hash })
        completed = true
      ensure
        checks = @checks.drop(first)
        report = { completed:, passed: completed && checks.all? { |entry| entry.fetch(:passed) }, d3_complete: false,
          candidate_sha256: @contract.expected_candidate_sha256, final_sha256: @contract.final_sha256,
          postgres_image: @contract.postgres_image, source_sha256: hashes, checks:, cases:,
          native_source: { url: "https://raw.githubusercontent.com/postgres/postgres/REL_16_14/src/backend/access/nbtree/nbtinsert.c", sha256: "3babaf9404d5f93dc20a5aea3dfdfffb0be636bf9ab1794c2fbb3b0bc92fe79e", location: 666 },
          limitations: ["Privileged isolated rule fixtures do not prove rule-management API reachability.", "Only the canonical identity sequence is observed; no concurrent identity race or native closure certificate.", "Matching rules retain 23505 as application failure, not successful prevent-merge behavior."] }
        metadata_write("report.json", JSON.pretty_generate(report) + "\n")
        @metadata_evidence, @correction_evidence = previous_metadata, previous_correction
      end
    end
  end
end
