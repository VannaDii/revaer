use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;
mod adapter;

const BEGIN_EMPTY: &str = "SELECT * FROM media_root_catalog_reconcile_begin_v1(1::smallint, decode('8f0b256ad26c139e8c51bd426ba510eb43aefca9cb1d58fb0737b6518e6bc867', 'hex'), decode('8f02f1258c4d32e0a9919e632d0ab9ec9c17a96f888ee26ee90fb8b9007568cd', 'hex'), decode('c69e6c6c3b5624cb119e895a8dac30a7989143139f9bde93199b8268295e2bc4', 'hex'), 0::smallint)";
const ACTIVATE: &str = "SELECT * FROM media_root_catalog_reconcile_activate_v1($1)";

async fn begin_empty(
    transaction: &mut Transaction<'_, Postgres>,
) -> anyhow::Result<(Uuid, i64, bool)> {
    Ok(sqlx::query_as(BEGIN_EMPTY)
        .fetch_one(&mut **transaction)
        .await?)
}

async fn reject_without_begin(pool: &PgPool, current: Uuid) -> anyhow::Result<()> {
    let mut transaction = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await?;
    let error = sqlx::query(ACTIVATE)
        .bind(current)
        .execute(&mut *transaction)
        .await
        .err()
        .ok_or_else(|| anyhow::anyhow!("activation without begin was accepted"))?;
    anyhow::ensure!(
        error
            .as_database_error()
            .is_some_and(|error| error.message() == "media_root_attestation_invalid"),
        "wrong missing-begin rejection"
    );
    transaction.rollback().await?;
    Ok(())
}

pub(super) async fn check(pool: &PgPool) -> anyhow::Result<()> {
    use crate::media::root_catalog::{
        RootCatalogUnavailable, mark_root_catalog_unavailable, read_root_catalog_readiness,
    };
    let mut incomplete = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await?;
    begin_empty(&mut incomplete).await?;
    let error = incomplete
        .commit()
        .await
        .err()
        .ok_or_else(|| anyhow::anyhow!("incomplete generation committed"))?;
    anyhow::ensure!(
        error
            .as_database_error()
            .is_some_and(|error| error.code().as_deref() == Some("23514")
                && error.message() == "media_root_catalog_generation_not_activated"),
        "wrong incomplete-commit error"
    );

    let mut first = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await?;
    let initial = begin_empty(&mut first).await?;
    anyhow::ensure!(
        !initial.2,
        "first reconciliation unexpectedly reused history"
    );
    sqlx::query(ACTIVATE)
        .bind(initial.0)
        .execute(&mut *first)
        .await?;
    first.commit().await?;
    reject_without_begin(pool, initial.0).await?;
    let ready = read_root_catalog_readiness(pool).await?;
    anyhow::ensure!(
        ready.len() == 5
            && ready.iter().all(|row| row.source_state == "ready"
                && row.attestation_state == "ready"
                && row.attestation_generation == Some(initial.1)
                && row.attested_slot_count == 0),
        "loaded empty catalog was not reconciled"
    );

    let mut unchanged = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await?;
    let reused = begin_empty(&mut unchanged).await?;
    anyhow::ensure!(
        reused == (initial.0, initial.1, true),
        "current generation was not idempotent"
    );
    sqlx::query(ACTIVATE)
        .bind(reused.0)
        .execute(&mut *unchanged)
        .await?;
    unchanged.commit().await?;

    mark_root_catalog_unavailable(pool, RootCatalogUnavailable::Missing).await?;
    let mut historical = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await?;
    begin_empty(&mut historical).await?;
    let error = sqlx::query(ACTIVATE)
        .bind(initial.0)
        .execute(&mut *historical)
        .await
        .err()
        .ok_or_else(|| anyhow::anyhow!("historical generation was reactivated"))?;
    anyhow::ensure!(
        error
            .as_database_error()
            .is_some_and(|error| error.message() == "media_root_attestation_invalid"),
        "wrong stale generation rejection"
    );
    historical.rollback().await?;
    let mut replacement = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await?;
    let next = begin_empty(&mut replacement).await?;
    anyhow::ensure!(
        !next.2 && next.0 != initial.0 && next.1 > initial.1,
        "lost continuity reused historical identity"
    );
    sqlx::query(ACTIVATE)
        .bind(next.0)
        .execute(&mut *replacement)
        .await?;
    replacement.commit().await?;
    check_populated(pool).await?;
    adapter::check(pool).await?;
    Ok(())
}

async fn check_populated(pool: &PgPool) -> anyhow::Result<()> {
    let mut transaction = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await?;
    let result = sqlx::raw_sql(include_str!(
        "../../../../../../scripts/tests/database-root-catalog-reconcile-fixture.sql"
    ))
    .execute(&mut *transaction)
    .await;
    transaction.rollback().await?;
    result?;
    Ok(())
}
