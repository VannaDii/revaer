//! Rescan requests survive without a scanner lease or takeover protocol.

use super::{make_test_roots, native_recovery, setup_db};
use crate::media::rescan::{RescanFence, read_rescan_state, request_rescan, satisfy_rescan};
use revaer_test_support::postgres::TestDatabase;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[tokio::test]
async fn guarded_rescan_preserves_newer_requests_and_rejects_stale_authority() -> anyhow::Result<()>
{
    let db = setup_db().await?;
    let roots = make_test_roots()?;
    native_recovery::initialize_catalog(&db.database, &db.pool, &roots, true).await?;
    let association =
        native_recovery::create_profile_association(&db.pool, "rescan-fence", true, "", 1).await?;
    let id = association.media_discovery_association_public_id;
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT latest_media_discovery_association_version_id FROM media_discovery_association WHERE media_discovery_association_public_id = $1",
    ).bind(id).fetch_one(&db.pool).await?;
    let generation = crate::media::root_catalog::read_root_catalog_readiness(&db.pool)
        .await?
        .first()
        .and_then(|row| row.attestation_generation)
        .ok_or_else(|| anyhow::anyhow!("synthetic catalog generation missing"))?;
    let runtime = PgPoolOptions::new()
        .max_connections(1)
        .connect(db.database.connection_string())
        .await?;
    let mut fence = RescanFence {
        association: id,
        version: 1,
        generation,
        generation_sha256: [0x33; 32],
        trigger: "watcher",
    };
    let disabled = request_rescan(&runtime, &fence, "restart_reconcile").await;
    assert!(
        matches!(disabled, Err(ref error) if error.database_detail() == Some("media_discovery_watcher_disabled"))
    );
    let private =
        sqlx::query_scalar::<_, i64>("SELECT media_discovery_rescan_fence_v1($1,$2,$3,$4,$5)")
            .bind(id)
            .bind(1_i32)
            .bind(generation)
            .bind([0x33_u8; 32].as_slice())
            .bind("watcher")
            .fetch_one(&runtime)
            .await;
    assert!(
        matches!(private, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("42501"))
    );
    // Synthetic mode transition is only for procedure qualification; this is
    // not filesystem/runtime or ordinary operator activation proof.
    db.database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/media-rescan-guarded-mode.sql"),
            &[("revaer_test.association_version", &version.to_string())],
        )
        .await?;
    let captured = request_rescan(&runtime, &fence, "restart_reconcile").await?;
    let newer = request_rescan(&runtime, &fence, "overflow").await?;
    assert_eq!(newer, captured + 1);
    satisfy_rescan(&runtime, &fence, captured).await?;
    assert_pending_sequence(&runtime, id, newer, captured).await?;
    let reasons = read_rescan_state(&runtime, id).await?;
    assert!(
        reasons
            .iter()
            .any(|row| row.reason_code == "overflow" && row.last_requested_sequence == newer)
    );
    let future = satisfy_rescan(&runtime, &fence, newer + 1).await;
    assert!(
        matches!(future, Err(ref error) if error.database_detail() == Some("media_configuration_invalid"))
    );
    fence.generation_sha256 = [0x34; 32];
    let stale = satisfy_rescan(&runtime, &fence, newer).await;
    assert!(
        matches!(stale, Err(ref error) if error.database_detail() == Some("media_root_attestation_stale"))
    );
    fence.generation_sha256 = [0x33; 32];
    fence.version = 2;
    let stale = request_rescan(&runtime, &fence, "overflow").await;
    assert!(
        matches!(stale, Err(ref error) if error.database_detail() == Some("media_root_attestation_stale"))
    );
    fence.version = 1;
    let forged = request_rescan(&runtime, &fence, "configuration_activated").await;
    assert!(
        matches!(forged, Err(ref error) if error.database_detail() == Some("media_configuration_invalid"))
    );
    assert_pending_sequence(&runtime, id, newer, captured).await?;
    satisfy_rescan(&runtime, &fence, newer).await?;
    satisfy_rescan(&runtime, &fence, captured).await?;
    assert_pending_sequence(&runtime, id, newer, newer).await?;
    runtime.close().await;
    db.close().await
}

async fn assert_pending_sequence(
    pool: &PgPool,
    id: uuid::Uuid,
    requested: i64,
    satisfied: i64,
) -> anyhow::Result<()> {
    let rows = read_rescan_state(pool, id).await?;
    assert!(!rows.is_empty());
    for row in rows {
        assert_eq!(row.requested_sequence, requested);
        assert_eq!(row.satisfied_sequence, satisfied);
    }
    Ok(())
}

#[tokio::test]
async fn rescan_coalesces_bounded_reasons_without_scanner_leases() -> anyhow::Result<()> {
    let db = setup_db().await?;
    let roots = make_test_roots()?;
    native_recovery::initialize_catalog(&db.database, &db.pool, &roots, true).await?;
    let association =
        native_recovery::create_profile_association(&db.pool, "rescan", true, "", 1).await?;
    let id = association.media_discovery_association_public_id;
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT latest_media_discovery_association_version_id FROM media_discovery_association WHERE media_discovery_association_public_id = $1",
    )
    .bind(id)
    .fetch_one(&db.pool)
    .await?;
    let runtime = PgPoolOptions::new()
        .max_connections(1)
        .connect(db.database.connection_string())
        .await?;

    assert_no_scanner_leases(&db.pool).await?;
    let denied =
        sqlx::query_scalar::<_, i64>("SELECT media_discovery_rescan_publish_v1($1, 'overflow')")
            .bind(version)
            .fetch_one(&runtime)
            .await;
    assert!(
        matches!(denied, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("42501"))
    );
    for reason in [
        "overflow",
        "overflow",
        "manual",
        "schedule",
        "watcher_uncertain",
        "directory_changed",
        "restart_reconcile",
    ] {
        sqlx::query_scalar::<_, i64>("SELECT media_discovery_rescan_publish_v1($1, $2)")
            .bind(version)
            .bind(reason)
            .fetch_one(&db.pool)
            .await?;
    }
    let rows = crate::media::rescan::read_rescan_state(&runtime, id).await?;
    assert_eq!(rows.len(), 7);
    for row in rows {
        assert_eq!(row.association_version, 1);
        assert_eq!(row.requested_sequence, 8);
        assert_eq!(row.satisfied_sequence, 0);
    }
    assert_overflow_rolls_back(&db.database, &db.pool, version).await?;
    runtime.close().await;
    db.close().await
}

async fn assert_no_scanner_leases(pool: &PgPool) -> anyhow::Result<()> {
    let absent = sqlx::query_scalar::<_, bool>(
        "SELECT to_regclass('public.media_discovery_execution_slot') IS NULL",
    )
    .fetch_one(pool)
    .await?;
    assert!(absent);
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname = 'public' AND p.proname IN ('media_discovery_execution_claim_v1', 'media_discovery_execution_renew_v1', 'media_discovery_execution_release_v1')",
    )
    .fetch_one(pool)
    .await?;
    assert_eq!(count, 0);
    Ok(())
}

async fn assert_overflow_rolls_back(
    database: &TestDatabase,
    pool: &PgPool,
    version: i64,
) -> anyhow::Result<()> {
    database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/media-rescan-overflow.sql"),
            &[("revaer_test.association_version", &version.to_string())],
        )
        .await?;
    let sequence = sqlx::query_scalar::<_, i64>("SELECT requested_sequence FROM media_discovery_rescan WHERE media_discovery_association_version_id = $1")
        .bind(version)
        .fetch_one(pool)
        .await?;
    assert_eq!(sequence, 8);
    Ok(())
}

#[tokio::test]
async fn guarded_schedule_coalesces_due_intervals_and_rolls_back_failed_publication()
-> anyhow::Result<()> {
    use crate::media::schedules::{create_schedule_configuration, observe_schedule_due};

    let db = setup_db().await?;
    let roots = make_test_roots()?;
    native_recovery::initialize_catalog(&db.database, &db.pool, &roots, true).await?;
    let association =
        native_recovery::create_profile_association(&db.pool, "schedule-due-fence", true, "", 1)
            .await?;
    let id = association.media_discovery_association_public_id;
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT latest_media_discovery_association_version_id FROM media_discovery_association WHERE media_discovery_association_public_id = $1",
    ).bind(id).fetch_one(&db.pool).await?;
    let generation = crate::media::root_catalog::read_root_catalog_readiness(&db.pool)
        .await?
        .first()
        .and_then(|row| row.attestation_generation)
        .ok_or_else(|| anyhow::anyhow!("synthetic catalog generation missing"))?;
    let runtime = PgPoolOptions::new()
        .max_connections(1)
        .connect(db.database.connection_string())
        .await?;
    let mut fence = RescanFence {
        association: id,
        version: 1,
        generation,
        generation_sha256: [0x33; 32],
        trigger: "schedule",
    };
    let disabled = observe_schedule_due(&runtime, &fence).await;
    assert!(matches!(disabled, Err(ref error)
        if error.database_detail() == Some("media_discovery_schedule_disabled")));
    db.database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/media-schedule-guarded-mode.sql"),
            &[("revaer_test.association_version", &version.to_string())],
        )
        .await?;
    assert_eq!(observe_schedule_due(&runtime, &fence).await?, None);
    create_schedule_configuration(&runtime, super::actor_id()?, id, 1, 2, "minutes").await?;
    assert!(observe_schedule_due(&runtime, &fence).await?.is_some());
    assert_eq!(observe_schedule_due(&runtime, &fence).await?, None);
    db.database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/media-schedule-overdue.sql"),
            &[("revaer_test.association_version", &version.to_string())],
        )
        .await?;
    let before = read_rescan_state(&runtime, id).await?;
    let previous = before
        .first()
        .ok_or_else(|| anyhow::anyhow!("rescan missing"))?
        .requested_sequence;
    assert_eq!(
        observe_schedule_due(&runtime, &fence).await?,
        Some(previous + 1)
    );
    assert_eq!(observe_schedule_due(&runtime, &fence).await?, None);
    let coalesced = sqlx::query_as::<_, (i64, bool, bool)>(
        "SELECT last_coalesced_count, last_coalesced_last_due_at - last_coalesced_first_due_at = interval '20 minutes', next_due_at > clock_timestamp() FROM media_discovery_schedule_state WHERE media_discovery_association_version_id = $1",
    ).bind(version).fetch_one(&db.pool).await?;
    assert_eq!(coalesced, (11, true, true));
    assert!(read_rescan_state(&runtime, id).await?.iter().any(|row|
        row.reason_code == "schedule" && row.last_requested_sequence == previous + 1));
    fence.generation_sha256 = [0x34; 32];
    let stale = observe_schedule_due(&runtime, &fence).await;
    assert!(matches!(stale, Err(ref error)
        if error.database_detail() == Some("media_root_attestation_stale")));
    fence.generation_sha256 = [0x33; 32];
    fence.trigger = "watcher";
    let forged = observe_schedule_due(&runtime, &fence).await;
    assert!(matches!(forged, Err(ref error)
        if error.database_detail() == Some("media_configuration_invalid")));
    db.database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/media-schedule-overflow.sql"),
            &[
                ("revaer_test.association_version", &version.to_string()),
                ("revaer_test.association", &id.to_string()),
                ("revaer_test.generation", &generation.to_string()),
            ],
        )
        .await?;
    runtime.close().await;
    db.close().await
}

#[tokio::test]
async fn ordinary_automatic_modes_and_import_publish_activation() -> anyhow::Result<()> {
    let db = setup_db().await?;
    let roots = make_test_roots()?;
    native_recovery::initialize_catalog(&db.database, &db.pool, &roots, true).await?;
    let association =
        native_recovery::create_profile_association(&db.pool, "automatic-modes", true, "manual", 1)
            .await?;
    let runtime = PgPoolOptions::new()
        .max_connections(1)
        .connect(db.database.connection_string())
        .await?;
    let result =
        qualify_automatic_mode_writers(&runtime, association.media_profile_public_id).await;
    runtime.close().await;
    let cleanup = db.close().await;
    match (result, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(cleanup)) => Err(anyhow::anyhow!("{error:#}; cleanup: {cleanup:#}")),
    }
}

async fn qualify_automatic_mode_writers(pool: &PgPool, profile: uuid::Uuid) -> anyhow::Result<()> {
    use crate::media::associations::{
        CreateAssociationInput, create_association, read_association,
    };
    use crate::media::configuration::{
        ImportResourcePrecondition, audit_import, begin_import_transaction, import_association,
        prepare_import,
    };
    let actor = super::actor_id()?;
    for (key, watcher, schedule) in [
        ("watcher", true, false),
        ("schedule", false, true),
        ("both", true, true),
    ] {
        let row = create_association(
            pool,
            &CreateAssociationInput {
                actor_public_id: actor,
                association_key: key,
                media_profile_public_id: profile,
                profile_version: 1,
                source_root_key: "worker-source",
                root_relative_path: key,
                manual_enabled: false,
                watcher_enabled: watcher,
                schedule_enabled: schedule,
            },
        )
        .await?;
        assert_eq!(
            (
                row.modes.manual_enabled,
                row.modes.watcher_enabled,
                row.modes.schedule_enabled
            ),
            (false, watcher, schedule)
        );
        assert_activation_request(pool, row.media_discovery_association_public_id).await?;
    }
    let mut transaction = begin_import_transaction(pool).await?;
    prepare_import(
        &mut transaction,
        actor,
        &[
            ImportResourcePrecondition {
                kind: "profiles",
                key: "automatic-modes",
                create: false,
                expected_version: Some(1),
            },
            ImportResourcePrecondition {
                kind: "discovery_associations",
                key: "imported-modes",
                create: true,
                expected_version: None,
            },
        ],
        &["worker-source".to_owned()],
    )
    .await?;
    let imported = import_association(
        &mut transaction,
        actor,
        &crate::media::portable::PortableAssociationRow {
            association_key: "imported-modes".into(),
            profile_key: "automatic-modes".into(),
            profile_version: 1,
            source_root_key: "worker-source".into(),
            root_relative_path: "imported".into(),
            manual_enabled: false,
            watcher_enabled: true,
            schedule_enabled: true,
        },
        None,
    )
    .await?;
    audit_import(&mut transaction, actor, &[0x61; 32], 1).await?;
    transaction.commit().await?;
    let row = read_association(pool, imported)
        .await?
        .ok_or_else(|| anyhow::anyhow!("imported association missing"))?;
    assert_eq!(
        (
            row.modes.manual_enabled,
            row.modes.watcher_enabled,
            row.modes.schedule_enabled
        ),
        (false, true, true)
    );
    assert_activation_request(pool, imported).await?;
    Ok(())
}

async fn assert_activation_request(pool: &PgPool, association: uuid::Uuid) -> anyhow::Result<()> {
    assert!(
        read_rescan_state(pool, association)
            .await?
            .iter()
            .any(|request| request.reason_code == "configuration_activated"
                && request.requested_sequence > request.satisfied_sequence)
    );
    Ok(())
}
