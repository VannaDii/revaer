# frozen_string_literal: true

require "digest"
require "json"
require_relative "support"

module RevaerDatabaseRebaseline
  # Source predictions only, never counts learned from native observations.
  # Compilation: fk-readback-20260913-70500-c1fasy, source_commit c2484dba.
  # Remaining: fk-remaining-readback-20260913-90176-t4oe99, revision b6d649014131c03ace63e3a5d6df08748db1a703.
  # Both independently qualified against PostgreSQL REL_16_14 RI dispatch.
  # Historical revisions identify provenance; current definition bytes are the gate.
  class NativeFkExpectations
    SOURCE_PINS = {
      "crates/revaer-data/migrations/0052_indexer_search_result_ingest_proc.sql" => "621c1421a6cab3731e4e6939c2687942a3bf343ce578f87cbc62cd2deb571930",
      "crates/revaer-data/migrations/0120_search_result_ingest_seed_best_source_context.sql" => "14082d5b23fb3129e612c0bf32b6880b0fc2642fa66de0b3c566b3e4e45b6c82",
      "scripts/database_rebaseline/final_sql.rb" => "3310e98ad17292b72d35c605bb6cd234fdd582140ae146ba3ca6afd80cd4cd91",
      "scripts/database_rebaseline/ingestion_compilation.rb" => "4873b4e04fc7385491e8c6026a6eda63406b45f871754c9d804ff5c562eb7641",
      "scripts/database_rebaseline/ingestion_corrections.rb" => "a15e54cabaf54c054a178cac9318ef0eb50fdde1c571c559fcee65f52ff43df3",
      "scripts/database_rebaseline/ingestion_existing.rb" => "3688b4f198f816554f6227e2d36aab660f4a9230710ef0f43296c2fd72e5abcd",
      "scripts/database_rebaseline/ingestion_wrapper.rb" => "fe79a8bad14747cb892c363befab09c0ad82e6116643b8fc0bd2e996efd926cb",
      "scripts/database_rebaseline/ingestion_proof.rb" => "b80a09d02367eced3daf6f4a87ade75457a2eccd2b8c4c23d0a9147931ab9361",
      "scripts/tests/database-ingestion-proof-seed.sql" => "6ec60a196da7dbedc8ec08f27af2550f3cd11c77ca416a5451c45992a0b72fa8",
      "scripts/tests/database-ingestion-corrections-seed.sql" => "da7d068dbe4644dd83b80d0d863a8810458f3216b41c66265cf62d071049e9cc"
    }.freeze
    SIGNATURES = {
      "search_result_ingest_v1" => "search_result_ingest_v1(uuid,uuid,character varying,character varying,character varying,character varying,character varying,bigint,character,character,character,integer,integer,timestamp with time zone,character varying,timestamp with time zone,observation_attr_key[],attr_value_type[],character varying[],integer[],bigint[],numeric[],boolean[],uuid[])",
      "log_source_metadata_conflict_v1" => "log_source_metadata_conflict_v1(bigint,bigint,conflict_type,text,text,timestamp with time zone)",
      "search_result_ingest" => "search_result_ingest(uuid,uuid,character varying,character varying,character varying,character varying,character varying,bigint,character,character,character,integer,integer,timestamp with time zone,character varying,timestamp with time zone,observation_attr_key[],attr_value_type[],character varying[],integer[],bigint[],numeric[],boolean[],uuid[])"
    }.freeze
    BODY_PINS = {
      "search_result_ingest_v1" => {
        "reference_proof" => "29c026228e9f4a7c69adc166a224d4c34cc00a945266a8e2a21ea55b3c6afc62",
        "final_proof" => "afe1e4cce523fa3135c0c6952dce1b2c5a81e71346a6fbf31907f92e90c3c43d"
      }.freeze,
      "log_source_metadata_conflict_v1" => {
        "reference_proof" => "f2034eb8b990f29acf804dcbef44b2a5205b10f72d83eaac431a053ab3e00202",
        "final_proof" => "f2034eb8b990f29acf804dcbef44b2a5205b10f72d83eaac431a053ab3e00202"
      }.freeze,
      "search_result_ingest" => {
        "reference_proof" => "d5b233041227a0e7750bfcbf63dcb49f8dc3758285bfe07286278e23f6acdfb7",
        "final_proof" => "d5b233041227a0e7750bfcbf63dcb49f8dc3758285bfe07286278e23f6acdfb7"
      }.freeze
    }.freeze

    # Deduplicated rows retain ownership, ambient GUC and owner compilation mode.
    # Zero compilation rows are deliberate queue-suppression predictions.
    DATA_JSON = <<~'JSON'
      {
        "compilation": {
          "operations": [
            {"scenario":"cold-logger-setting","operation_ordinal":1,"operation":"ingest b*40","compiler_setting":{"reference":2,"final":0},"multisets":{"logger":2,"new_canonical_existing_source":1},"exact_total":19,"expected_function_counts":{"RI_FKey_check_ins":18,"RI_FKey_check_upd":1,"RI_FKey_noaction_upd":0}},
            {"scenario":"logger-first-setting","operation_ordinal":1,"operation":"direct logger tracker_name NULL/NULL/NULL","compiler_setting":{"reference":0,"final":0},"multisets":{"logger":1},"exact_total":5,"expected_function_counts":{"RI_FKey_check_ins":5}},
            {"scenario":"logger-first-setting","operation_ordinal":2,"operation":"direct logger tracker_category A*257/B*258/fixed time","compiler_setting":{"reference":0,"final":0},"multisets":{"logger":1},"exact_total":5,"expected_function_counts":{"RI_FKey_check_ins":5}},
            {"scenario":"logger-first-setting","operation_ordinal":3,"operation":"ingest b*40","compiler_setting":{"reference":2,"final":0},"multisets":{"logger":2,"new_canonical_existing_source":1},"exact_total":19,"expected_function_counts":{"RI_FKey_check_ins":18,"RI_FKey_check_upd":1,"RI_FKey_noaction_upd":0}}
          ],
          "multisets": {
            "logger": [
              {"function":"RI_FKey_check_ins","table":"source_metadata_conflict","constraints":["source_metadata_conflict_canonical_torrent_source_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_ins","table":"source_metadata_conflict_audit_log","constraints":["source_metadata_conflict_audit_log_conflict_id_fkey","source_metadata_conflict_audit_log_actor_user_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_ins","table":"indexer_health_event","constraints":["indexer_health_event_indexer_instance_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_ins","table":"source_metadata_conflict","constraints":["source_metadata_conflict_resolved_by_user_id_fkey"],"count_each":1}
            ],
            "new_canonical_existing_source": [
              {"function":"RI_FKey_check_ins","table":"canonical_torrent_source_context_score","constraints":["canonical_torrent_source_conte_canonical_torrent_source_id_fkey","canonical_torrent_source_context_scor_canonical_torrent_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_upd","table":"search_request_source_observation","constraints":["search_request_source_observation_canonical_torrent_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_ins","table":"canonical_size_sample","constraints":["canonical_size_sample_canonical_torrent_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_ins","table":"canonical_size_rollup","constraints":["canonical_size_rollup_canonical_torrent_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_ins","table":"search_request_canonical","constraints":["search_request_canonical_canonical_torrent_id_fkey","search_request_canonical_search_request_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_ins","table":"search_page_item","constraints":["search_page_item_search_page_id_fkey","search_page_item_search_request_canonical_id_fkey"],"count_each":1},
              {"function":"RI_FKey_check_upd","table":"canonical_torrent_source","constraints":["canonical_torrent_source_indexer_instance_id_fkey"],"count_each":0},
              {"function":"RI_FKey_check_upd","table":"search_request_source_observation","constraints":["search_request_source_observat_canonical_torrent_source_id_fkey","search_request_source_observation_indexer_instance_id_fkey","search_request_source_observation_search_request_id_fkey"],"count_each":0},
              {"function":"RI_FKey_noaction_upd","table":"canonical_torrent","constraints":["acquisition_attempt_canonical_torrent_id_fkey","canonical_external_id_canonical_torrent_id_fkey","canonical_size_rollup_canonical_torrent_id_fkey","canonical_size_sample_canonical_torrent_id_fkey","canonical_torrent_best_source_context_canonical_torrent_id_fkey","canonical_torrent_best_source_global_canonical_torrent_id_fkey","canonical_torrent_signal_canonical_torrent_id_fkey","canonical_torrent_source_base_score_canonical_torrent_id_fkey","canonical_torrent_source_context_scor_canonical_torrent_id_fkey","search_filter_decision_canonical_torrent_id_fkey","search_request_canonical_canonical_torrent_id_fkey","search_request_source_observation_canonical_torrent_id_fkey","user_result_action_canonical_torrent_id_fkey"],"count_each":0},
              {"function":"RI_FKey_noaction_upd","table":"canonical_torrent_source","constraints":["acquisition_attempt_canonical_torrent_source_id_fkey","canonical_external_id_source_canonical_torrent_source_id_fkey","canonical_torrent_best_source__canonical_torrent_source_id_fkey","canonical_torrent_best_source_canonical_torrent_source_id_fkey1","canonical_torrent_source_attr_canonical_torrent_source_id_fkey","canonical_torrent_source_base__canonical_torrent_source_id_fkey","canonical_torrent_source_conte_canonical_torrent_source_id_fkey","search_filter_decision_canonical_torrent_source_id_fkey","search_request_source_observat_canonical_torrent_source_id_fkey","source_metadata_conflict_canonical_torrent_source_id_fkey"],"count_each":0},
              {"function":"RI_FKey_noaction_upd","table":"search_request_source_observation","constraints":["search_filter_decision_observation_id_fkey","search_request_source_observation_attr_observation_id_fkey"],"count_each":0}
            ]
          }
        },
        "row_fields": ["function","table","constraint","count","sql_owner","scope","v1_active","ambient_variable_conflict","owner_compile_resolution"],
        "rows": [
          ["RI_FKey_check_ins","public.canonical_torrent_source","canonical_torrent_source_indexer_instance_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.canonical_torrent_source_context_score","canonical_torrent_source_conte_canonical_torrent_source_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.canonical_torrent_source_context_score","canonical_torrent_source_context_scor_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation","search_request_source_observat_canonical_torrent_source_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation","search_request_source_observation_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation","search_request_source_observation_indexer_instance_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation","search_request_source_observation_search_request_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation_attr","search_request_source_observation_attr_observation_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.canonical_torrent_source_attr","canonical_torrent_source_attr_canonical_torrent_source_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.canonical_torrent_source","canonical_torrent_source_indexer_instance_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.canonical_torrent_source_context_score","canonical_torrent_source_conte_canonical_torrent_source_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.canonical_torrent_source_context_score","canonical_torrent_source_context_scor_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation","search_request_source_observat_canonical_torrent_source_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation","search_request_source_observation_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation","search_request_source_observation_indexer_instance_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation","search_request_source_observation_search_request_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_request_source_observation_attr","search_request_source_observation_attr_observation_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.canonical_torrent_source_attr","canonical_torrent_source_attr_canonical_torrent_source_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.canonical_external_id","canonical_external_id_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.canonical_external_id","canonical_external_id_source_canonical_torrent_source_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.canonical_size_sample","canonical_size_sample_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.canonical_size_rollup","canonical_size_rollup_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_request_canonical","search_request_canonical_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_request_canonical","search_request_canonical_search_request_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_page","search_page_search_request_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_page_item","search_page_item_search_page_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.search_page_item","search_page_item_search_request_canonical_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"],
          ["RI_FKey_check_ins","public.source_metadata_conflict","source_metadata_conflict_canonical_torrent_source_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.source_metadata_conflict","source_metadata_conflict_resolved_by_user_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.source_metadata_conflict_audit_log","source_metadata_conflict_audit_log_actor_user_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.source_metadata_conflict_audit_log","source_metadata_conflict_audit_log_conflict_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.indexer_health_event","indexer_health_event_indexer_instance_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"use_column","use_column"],
          ["RI_FKey_check_upd","public.search_request_source_observation","search_request_source_observation_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.canonical_size_sample","canonical_size_sample_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.canonical_size_rollup","canonical_size_rollup_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_request_canonical","search_request_canonical_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_request_canonical","search_request_canonical_search_request_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_page_item","search_page_item_search_page_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.search_page_item","search_page_item_search_request_canonical_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"use_column","use_column"],
          ["RI_FKey_check_ins","public.canonical_torrent_best_source_context","canonical_torrent_best_source_context_canonical_torrent_id_fkey",1,"public.search_result_ingest","wrapper_post_v1",false,"error","error"],
          ["RI_FKey_check_ins","public.canonical_torrent_best_source_context","canonical_torrent_best_source_canonical_torrent_source_id_fkey1",1,"public.search_result_ingest","wrapper_post_v1",false,"error","error"],
          ["RI_FKey_check_ins","public.source_metadata_conflict","source_metadata_conflict_canonical_torrent_source_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"error","error"],
          ["RI_FKey_check_ins","public.source_metadata_conflict","source_metadata_conflict_resolved_by_user_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"error","error"],
          ["RI_FKey_check_ins","public.source_metadata_conflict_audit_log","source_metadata_conflict_audit_log_actor_user_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"error","error"],
          ["RI_FKey_check_ins","public.source_metadata_conflict_audit_log","source_metadata_conflict_audit_log_conflict_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"error","error"],
          ["RI_FKey_check_ins","public.indexer_health_event","indexer_health_event_indexer_instance_id_fkey",2,"public.log_source_metadata_conflict_v1","v1_nested_logger",true,"error","error"],
          ["RI_FKey_check_upd","public.search_request_source_observation","search_request_source_observation_canonical_torrent_id_fkey",1,"public.search_result_ingest_v1","v1_direct",true,"error","use_column"]
        ],
        "cases": [{"name":"imdb-upsert","variants":{"reference":{"operations":[{"ordinal":1,"expected_sqlstate":"42P10","finish":"ROLLBACK TO SAVEPOINT operation; COMMIT","total_callback_entries":9,"scope_totals":{"wrapper_pre_v1":0,"v1_direct":9,"v1_nested_logger":0,"wrapper_post_v1":0},"multiset":[0,1,2,3,4,5,6,7,8]},{"ordinal":2,"expected_sqlstate":"42P10","finish":"ROLLBACK TO SAVEPOINT operation; COMMIT","total_callback_entries":9,"scope_totals":{"wrapper_pre_v1":0,"v1_direct":9,"v1_nested_logger":0,"wrapper_post_v1":0},"multiset":[0,1,2,3,4,5,6,7,8]}]},"final":{"operations":[{"ordinal":1,"expected_sqlstate":"00000","finish":"COMMIT","total_callback_entries":18,"scope_totals":{"wrapper_pre_v1":0,"v1_direct":18,"v1_nested_logger":0,"wrapper_post_v1":0},"multiset":[9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26]},{"ordinal":2,"expected_sqlstate":"00000","finish":"COMMIT","total_callback_entries":1,"scope_totals":{"wrapper_pre_v1":0,"v1_direct":1,"v1_nested_logger":0,"wrapper_post_v1":0},"multiset":[20]}]}}},{"name":"existing-v2-hash-conflict-warm-rollback","variants":{"reference":{"operations":[{"ordinal":1,"expected_sqlstate":"00000","finish":"ROLLBACK","total_callback_entries":21,"scope_totals":{"wrapper_pre_v1":0,"v1_direct":9,"v1_nested_logger":10,"wrapper_post_v1":2},"multiset":[27,28,29,30,31,1,2,32,33,34,35,36,37,38,39,40]},{"ordinal":2,"expected_sqlstate":"00000","finish":"COMMIT","total_callback_entries":21,"scope_totals":{"wrapper_pre_v1":0,"v1_direct":9,"v1_nested_logger":10,"wrapper_post_v1":2},"multiset":[27,28,29,30,31,1,2,32,33,34,35,36,37,38,39,40]}]},"final":{"operations":[{"ordinal":1,"expected_sqlstate":"00000","finish":"ROLLBACK","total_callback_entries":21,"scope_totals":{"wrapper_pre_v1":0,"v1_direct":9,"v1_nested_logger":10,"wrapper_post_v1":2},"multiset":[41,42,43,44,45,10,11,46,20,21,22,23,25,26,39,40]},{"ordinal":2,"expected_sqlstate":"00000","finish":"COMMIT","total_callback_entries":21,"scope_totals":{"wrapper_pre_v1":0,"v1_direct":9,"v1_nested_logger":10,"wrapper_post_v1":2},"multiset":[41,42,43,44,45,10,11,46,20,21,22,23,25,26,39,40]}]}}}],
        "target_constraints": ["canonical_external_id_canonical_torrent_id_fkey","canonical_external_id_source_canonical_torrent_source_id_fkey","canonical_torrent_source_attr_canonical_torrent_source_id_fkey","canonical_torrent_best_source_context_canonical_torrent_id_fkey","canonical_torrent_best_source_canonical_torrent_source_id_fkey1"]
      }
    JSON
    private_constant :SOURCE_PINS, :SIGNATURES, :BODY_PINS, :DATA_JSON

    attr_reader :compilation, :remaining, :target_constraints

    def initialize(root:, inventory:)
      validate_sources!(root)
      validate_inventory!(inventory)
      data = JSON.parse(DATA_JSON)
      fields = data.fetch("row_fields")
      rows = data.fetch("rows").map { |values| fields.zip(values).to_h }
      cases = data.fetch("cases")
      cases.each do |entry|
        entry.fetch("variants").each_value do |variant|
          variant.fetch("operations").each do |operation|
            operation["multiset"] = operation.fetch("multiset").map { |index| rows.fetch(index) }
          end
        end
      end
      @compilation = deep_freeze(data.fetch("compilation"))
      @remaining = deep_freeze({ "cases" => cases })
      @target_constraints = deep_freeze(data.fetch("target_constraints"))
    end

    private

    def validate_sources!(root)
      SOURCE_PINS.each do |path, expected|
        actual = Digest::SHA256.file(File.join(root, path)).hexdigest
        raise Failure, "native FK source definition changed: #{path}" unless actual == expected
      rescue SystemCallError, IOError
        raise Failure, "native FK required source unreadable: #{path}"
      end
    end

    def validate_inventory!(inventory)
      raise Failure, "native FK inventory must be an object" unless inventory.is_a?(Hash)

      %w[reference_proof final_proof].each do |variant|
        proof = inventory[variant]
        routines = proof.is_a?(Hash) ? proof["routines"] : nil
        unless routines.is_a?(Array) && routines.all? { |row| row.is_a?(Hash) && row["name"].is_a?(String) }
          raise Failure, "native FK routine inventory missing or malformed: #{variant}"
        end
        SIGNATURES.each do |name, signature|
          matches = routines.select { |row| row.fetch("name") == name }
          raise Failure, "native FK routine missing or duplicated: #{variant}/#{name}" unless matches.length == 1

          routine = matches.fetch(0)
          source = routine["source"]
          valid = routine["signature"] == signature && source.is_a?(String) &&
            Digest::SHA256.hexdigest(source) == BODY_PINS.fetch(name).fetch(variant) &&
            routine.key?("settings") && routine["settings"] == expected_settings(variant, name)
          raise Failure, "native FK routine binding changed: #{variant}/#{name}" unless valid
        end
      end
    end

    def expected_settings(variant, name)
      return ["search_path=pg_catalog, public"] if variant == "final_proof"
      return ["plpgsql.variable_conflict=use_column"] if name == "search_result_ingest_v1"

      nil
    end

    def deep_freeze(value)
      case value
      when Hash
        value.each { |key, item| deep_freeze(key); deep_freeze(item) }
      when Array
        value.each { |item| deep_freeze(item) }
      end
      value.freeze
    end
  end
end
