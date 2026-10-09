//! Clean absence is diagnostic evidence, never deletion authority.

use super::{TestDb, make_test_roots, native_recovery, setup_db};
use crate::media::association_jobs::AssociationFingerprint;
use crate::media::rescan::{
    RescanFence, observe_rescan_paths, read_rescan_state, read_source_diagnostics, request_rescan,
    satisfy_rescan,
};
use sqlx::{PgPool, postgres::PgPoolOptions};

async fn fixture() -> anyhow::Result<(TestDb, PgPool, RescanFence<'static>)> {
    let db = setup_db().await?;
    let roots = make_test_roots()?;
    native_recovery::initialize_catalog(&db.database, &db.pool, &roots, true).await?;
    let association =
        native_recovery::create_profile_association(&db.pool, "absence", true, "", 1).await?;
    native_recovery::enqueue_candidate(
        &db.pool,
        &association,
        "source.mkv",
        true,
        AssociationFingerprint {
            identity: "0000000000000001:0000000000000001",
            size_bytes: 6,
            modified_ns: 1,
            changed_ns: 1,
            sha256: &"11".repeat(32),
        },
    )
    .await?;
    let id = association.media_discovery_association_public_id;
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT latest_media_discovery_association_version_id FROM media_discovery_association WHERE media_discovery_association_public_id = $1",
    ).bind(id).fetch_one(&db.pool).await?;
    db.database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/media-rescan-guarded-mode.sql"),
            &[("revaer_test.association_version", &version.to_string())],
        )
        .await?;
    let generation = crate::media::root_catalog::read_root_catalog_readiness(&db.pool)
        .await?
        .first()
        .and_then(|row| row.attestation_generation)
        .ok_or_else(|| anyhow::anyhow!("catalog generation missing"))?;
    let runtime = PgPoolOptions::new()
        .max_connections(1)
        .connect(db.database.connection_string())
        .await?;
    Ok((
        db,
        runtime,
        RescanFence {
            association: id,
            version: 1,
            generation,
            generation_sha256: [0x33; 32],
            trigger: "watcher",
        },
    ))
}

async fn evidence(pool: &PgPool, fence: &RescanFence<'_>, count: i16) -> anyhow::Result<()> {
    let row = read_source_diagnostics(pool, fence.association, "source.mkv")
        .await?
        .ok_or_else(|| anyhow::anyhow!("fingerprint disappeared"))?;
    assert_eq!(row.source_path, "source.mkv");
    assert_eq!(row.absence_observations, count);
    assert_eq!(row.diagnostic_absent, count == 2);
    Ok(())
}

async fn requested(pool: &PgPool, fence: &RescanFence<'_>) -> anyhow::Result<i64> {
    Ok(read_rescan_state(pool, fence.association)
        .await?
        .first()
        .ok_or_else(|| anyhow::anyhow!("rescan request missing"))?
        .requested_sequence)
}

#[tokio::test]
async fn absence_needs_two_separated_clean_scans_and_reappearance_clears_it() -> anyhow::Result<()>
{
    let (db, runtime, fence) = fixture().await?;
    let first = request_rescan(&runtime, &fence, "restart_reconcile").await?;
    satisfy_rescan(&runtime, &fence, first).await?;
    evidence(&runtime, &fence, 1).await?;
    let second = requested(&runtime, &fence).await?;
    assert_eq!(second, first + 1);
    satisfy_rescan(&runtime, &fence, first).await?;
    evidence(&runtime, &fence, 1).await?;
    satisfy_rescan(&runtime, &fence, second).await?;
    evidence(&runtime, &fence, 1).await?;
    let third = requested(&runtime, &fence).await?;
    assert_eq!(third, second + 1);
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    satisfy_rescan(&runtime, &fence, third).await?;
    evidence(&runtime, &fence, 2).await?;
    let fourth = request_rescan(&runtime, &fence, "schedule").await;
    assert!(
        fourth.is_err(),
        "watcher cannot manufacture schedule authority"
    );
    let fourth = request_rescan(&runtime, &fence, "watcher_uncertain").await?;
    evidence(&runtime, &fence, 0).await?;
    satisfy_rescan(&runtime, &fence, fourth).await?;
    evidence(&runtime, &fence, 1).await?;
    let fifth = requested(&runtime, &fence).await?;
    let paths = vec!["source.mkv".to_string()];
    observe_rescan_paths(&runtime, &fence, fifth, &paths).await?;
    evidence(&runtime, &fence, 0).await?;
    satisfy_rescan(&runtime, &fence, fifth).await?;
    evidence(&runtime, &fence, 0).await?;
    assert_eq!(requested(&runtime, &fence).await?, fifth);
    assert!(
        observe_rescan_paths(&runtime, &fence, fifth, &paths)
            .await
            .is_err()
    );
    assert!(
        read_source_diagnostics(&runtime, fence.association, "unknown.mkv")
            .await?
            .is_none()
    );
    let jobs = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM media_job")
        .fetch_one(&db.pool)
        .await?;
    assert_eq!(jobs, 1, "absence must neither delete nor manufacture jobs");
    runtime.close().await;
    db.close().await
}

#[tokio::test]
async fn incomplete_newer_and_stale_scans_cannot_establish_absence() -> anyhow::Result<()> {
    let (db, runtime, mut fence) = fixture().await?;
    let first = request_rescan(&runtime, &fence, "restart_reconcile").await?;
    let newer = request_rescan(&runtime, &fence, "overflow").await?;
    satisfy_rescan(&runtime, &fence, first).await?;
    evidence(&runtime, &fence, 0).await?;
    let paths = vec!["source.mkv".to_string()];
    assert!(
        observe_rescan_paths(&runtime, &fence, newer + 1, &paths)
            .await
            .is_err()
    );
    assert!(
        observe_rescan_paths(&runtime, &fence, newer, &[])
            .await
            .is_err()
    );
    assert!(
        observe_rescan_paths(&runtime, &fence, newer, &vec![paths[0].clone(); 129])
            .await
            .is_err()
    );
    assert!(
        observe_rescan_paths(&runtime, &fence, newer, &["../outside.mkv".to_string()])
            .await
            .is_err()
    );
    fence.generation_sha256 = [0x34; 32];
    assert!(satisfy_rescan(&runtime, &fence, newer).await.is_err());
    assert!(
        observe_rescan_paths(&runtime, &fence, newer, &paths)
            .await
            .is_err()
    );
    evidence(&runtime, &fence, 0).await?;
    fence.generation_sha256 = [0x33; 32];
    observe_rescan_paths(&runtime, &fence, newer, &paths).await?;
    // Presence is safe to record before completion, but cannot acknowledge it.
    let state = read_rescan_state(&runtime, fence.association).await?;
    assert!(state.iter().all(|row| row.satisfied_sequence == first));
    evidence(&runtime, &fence, 0).await?;
    satisfy_rescan(&runtime, &fence, newer).await?;
    evidence(&runtime, &fence, 0).await?;
    runtime.close().await;
    db.close().await
}
