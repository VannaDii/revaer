use revaer_test_support::postgres::start_postgres;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

const EXPECTED_TABLES: &[&str] = &[
    "media_profile",
    "media_target",
    "media_job",
    "media_job_phase",
    "media_job_operation",
    "media_job_violation",
    "media_job_verification_check",
    "media_job_artifact",
    "media_job_compact_audit",
    "media_capability_snapshot",
    "media_capability_snapshot_encoder",
    "media_capability_snapshot_feature",
    "media_capability_snapshot_run",
    "media_compatibility_target",
    "media_policy_profile",
    "media_job_retention_policy",
    "media_desired_target_profile",
    "media_desired_target_container",
    "media_desired_target_stream",
    "media_desired_target_audio_stream",
    "media_job_desired_target_stream",
    "media_discovery_source_fingerprint",
];

const EXPECTED_PROCS: &[&str] = &[
    "media_profile_upsert_v1",
    "media_profile_upsert_v2",
    "media_profile_list_v1",
    "media_profile_list_v2",
    "media_profile_list_v3",
    "media_profile_get_v1",
    "media_profile_get_v2",
    "media_profile_get_v3",
    "media_profile_update_v1",
    "media_profile_normalized_root_v1",
    "media_profile_validate_discovery_root_overlap_v1",
    "media_job_normalized_absolute_path_v1",
    "media_job_validate_path_within_root_v1",
    "media_job_create_v1",
    "media_discovery_job_enqueue_v1",
    "media_discovery_job_enqueue_v2",
    "media_job_recent_page_v1",
    "media_job_snapshot_source_fingerprint_v1",
    "media_job_phase_append_v1",
    "media_job_phase_list_v1",
    "media_job_operation_append_v1",
    "media_job_operation_list_v1",
    "media_job_violation_append_v1",
    "media_job_violation_list_v1",
    "media_job_verification_check_append_v1",
    "media_job_verification_check_list_v1",
    "media_job_artifact_path_is_managed_v1",
    "media_job_artifact_append_v1",
    "media_job_artifact_list_v1",
    "media_job_compact_audit_append_v1",
    "media_job_compact_audit_list_v1",
    "media_capability_snapshot_record_v1",
    "media_capability_snapshot_latest_v1",
    "media_capability_snapshot_encoder_record_v1",
    "media_capability_snapshot_encoder_list_v1",
    "media_capability_snapshot_feature_record_v1",
    "media_capability_snapshot_feature_list_v1",
    "media_capability_snapshot_run_start_v1",
    "media_capability_snapshot_run_complete_v1",
    "media_compatibility_target_list_v1",
    "media_compatibility_target_upsert_v1",
    "media_policy_profile_list_v1",
    "media_policy_profile_upsert_v1",
    "media_job_retention_policy_get_v1",
    "media_job_retention_policy_update_v1",
    "media_job_retention_policy_get_v2",
    "media_job_retention_policy_update_v2",
    "media_job_retention_run_v1",
    "media_workspace_retention_snapshot_v1",
    "media_retention_mode_age_v1",
    "media_retention_mode_count_v1",
    "media_desired_target_create_v1",
    "media_desired_target_stream_append_v1",
    "media_desired_target_stream_append_v2",
    "media_desired_target_stream_append_v3",
    "media_desired_target_stream_append_v4",
    "media_video_level_known_v1",
    "media_audio_channel_layout_count_v1",
    "media_desired_target_list_v1",
    "media_desired_target_graph_page_v1",
    "media_desired_target_stream_list_v1",
    "media_desired_target_stream_list_v2",
    "media_desired_target_stream_list_v3",
    "media_desired_target_stream_list_v4",
    "media_profile_desired_target_set_v1",
    "media_job_desired_target_stream_list_v1",
    "media_job_desired_target_stream_list_v2",
    "media_job_desired_target_stream_list_v3",
    "media_job_desired_target_stream_list_v4",
    "media_job_desired_target_audio_constraints_snapshot_v1",
    "media_key_valid_v1",
    "media_display_valid_v1",
    "media_job_list_v1",
    "media_job_get_v1",
    "media_job_cancel_v1",
    "media_job_cancel_v2",
    "media_job_retry_v1",
    "media_job_worker_claim_next_v2",
    "media_job_worker_claim_next_v3",
    "media_job_worker_poll_control_v1",
    "media_job_worker_acknowledge_cancel_v1",
    "media_job_worker_complete_v1",
    "media_job_worker_complete_finalized_v1",
    "media_job_worker_recover_stale_v1",
    "media_job_mark_completed_v1",
    "media_job_cleanup_completed_v1",
    "media_job_cleanup_failed_terminal_diagnostics_v1",
];
const SYSTEM_USER_PUBLIC_ID: Uuid = Uuid::from_u128(0);

pub(super) struct MediaTestDb {
    _db: revaer_test_support::postgres::TestDatabase,
    pool: PgPool,
    pub(super) system_user_public_id: Uuid,
}

impl MediaTestDb {
    pub(super) const fn pool(&self) -> &PgPool {
        &self.pool
    }
}

pub(super) async fn setup_media_db(label: &str) -> anyhow::Result<Option<MediaTestDb>> {
    let postgres = match start_postgres() {
        Ok(db) => db,
        Err(err) => {
            eprintln!("skipping {label}: {err}");
            return Ok(None);
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(postgres.connection_string())
        .await?;

    let mut migrator = sqlx::migrate!("./migrations");
    migrator.set_ignore_missing(true);
    migrator.run(&pool).await?;

    let system_user_public_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_public_id FROM app_user WHERE user_public_id = $1",
    )
    .bind(SYSTEM_USER_PUBLIC_ID)
    .fetch_one(&pool)
    .await?;

    Ok(Some(MediaTestDb {
        _db: postgres,
        pool,
        system_user_public_id,
    }))
}

#[tokio::test]
async fn media_tables_exist() -> anyhow::Result<()> {
    let db = match setup_media_db("media_tables_exist").await {
        Ok(Some(db)) => db,
        Ok(None) => {
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let rows = sqlx::query_scalar::<_, String>(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public' AND table_name LIKE 'media_%' ORDER BY table_name",
    )
    .fetch_all(db.pool())
    .await?;

    for table in EXPECTED_TABLES {
        assert!(
            rows.iter().any(|item| item == table),
            "missing table {table}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn media_procedures_exist() -> anyhow::Result<()> {
    let db = match setup_media_db("media_procedures_exist").await {
        Ok(Some(db)) => db,
        Ok(None) => {
            return Ok(());
        }
        Err(err) => return Err(err),
    };

    let rows = sqlx::query(
        "SELECT proname FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = 'public' AND proname LIKE 'media_%'",
    )
    .fetch_all(db.pool())
    .await?;

    let procedure_names: Vec<String> = rows
        .iter()
        .map(|row| row.try_get::<String, _>("proname"))
        .collect::<Result<Vec<_>, _>>()?;

    for proc_name in EXPECTED_PROCS {
        assert!(
            procedure_names.iter().any(|item| item == proc_name),
            "missing procedure {proc_name}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn media_text_contract_enforces_utf8_and_key_boundaries() -> anyhow::Result<()> {
    let db = match setup_media_db("media_text_contract_enforces_utf8_and_key_boundaries").await {
        Ok(Some(db)) => db,
        Ok(None) => return Ok(()),
        Err(err) => return Err(err),
    };

    let exact_key = format!("a{}z", "b".repeat(126));
    let oversized_key = format!("a{}z", "b".repeat(127));
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT media_key_valid_v1($1)")
            .bind(exact_key)
            .fetch_one(db.pool())
            .await?
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT media_key_valid_v1($1)")
            .bind(oversized_key)
            .fetch_one(db.pool())
            .await?
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT media_key_valid_v1($1)")
            .bind("Profile Key")
            .fetch_one(db.pool())
            .await?
    );

    let exact_display = "é".repeat(128);
    let oversized_display = format!("{exact_display}é");
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT media_display_valid_v1($1)")
            .bind(exact_display)
            .fetch_one(db.pool())
            .await?
    );
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT media_display_valid_v1($1)")
            .bind(oversized_display)
            .fetch_one(db.pool())
            .await?
    );

    Ok(())
}
