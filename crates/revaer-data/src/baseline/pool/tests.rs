use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, Executor};
use std::str::FromStr;

use super::{BaselineReadReason, verify_runtime_pool};

#[tokio::test]
async fn baseline_pool_closed_error_does_not_expose_credentials() -> anyhow::Result<()> {
    let options = PgConnectOptions::new()
        .host("127.0.0.1")
        .username("private_runtime_login")
        .password("private_runtime_password");
    let pool = PgPoolOptions::new().connect_lazy_with(options);
    pool.close().await;
    let error = verify_runtime_pool(&pool).await.err().ok_or_else(|| {
        anyhow::anyhow!("closed pool unexpectedly verified a runtime database baseline")
    })?;
    assert_eq!(error.reason(), BaselineReadReason::StatementFailed);
    assert!(!format!("{error:?}").contains("private_"));
    assert_eq!(pool.size(), 0);
    Ok(())
}

#[tokio::test]
async fn baseline_pool_reads_exact_init_as_restricted_runtime() -> anyhow::Result<()> {
    let database = revaer_test_support::postgres::start_postgres()?;
    let options = PgConnectOptions::from_str(database.connection_string())?;
    let database_name = options
        .get_database()
        .ok_or_else(|| anyhow::anyhow!("owned fixture database name absent"))?;
    let owner_name = format!("{database_name}_owner");
    let runtime_name = format!("{database_name}_runtime");
    let password = uuid::Uuid::new_v4().to_string();
    let mut admin = sqlx::PgConnection::connect_with(&options).await?;
    let mut setup = admin.begin().await?;
    sqlx::query("SELECT set_config('revaer_test.fixture_password', $1, true)")
        .bind(&password)
        .execute(&mut *setup)
        .await?;
    sqlx::raw_sql(include_str!(
        "../../../../../scripts/tests/database-runtime-fixture-roles.sql"
    ))
    .execute(&mut *setup)
    .await
    .map_err(|_| anyhow::anyhow!("restricted fixture role provisioning failed"))?;
    setup.commit().await?;

    let result = initialized_runtime_read(
        &mut admin,
        &options.clone().username(&owner_name).password(&password),
        &options.username(&runtime_name).password(&password),
        &runtime_name,
    )
    .await;
    // Role cleanup runs even when initialization, verification or assertions fail.
    let cleanup = sqlx::raw_sql(include_str!(
        "../../../../../scripts/tests/database-runtime-fixture-cleanup.sql"
    ))
    .execute(&mut admin)
    .await;
    admin.close().await?;
    cleanup?;
    result
}

async fn initialized_runtime_read(
    admin: &mut sqlx::PgConnection,
    owner_options: &PgConnectOptions,
    runtime_options: &PgConnectOptions,
    runtime_name: &str,
) -> anyhow::Result<()> {
    let mut owner = sqlx::PgConnection::connect_with(owner_options).await?;
    let initialized = apply_and_seal(&mut owner, runtime_name).await;
    owner.close().await?;
    initialized?;
    admin
        .execute(include_str!(
            "../../../../../scripts/tests/database-runtime-fixture-owner-disable.sql"
        ))
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(
            runtime_options
                .clone()
                .options([("statement_timeout", "10000")]),
        )
        .await?;
    let verified = verify_runtime_pool(&pool).await;
    let direct_table_read = sqlx::query("SELECT media_job_id FROM public.media_job LIMIT 0")
        .execute(&pool)
        .await;
    let readiness = crate::media::root_catalog::read_root_catalog_readiness(&pool).await;
    let missing_job_costs =
        crate::media::policy_snapshot::list_operation_costs(&pool, uuid::Uuid::new_v4()).await;
    let profile_versions = profile_versions::check(admin, &pool).await;
    let catalog = check_catalog_pages(admin, &pool).await;
    let reset = sqlx::query("SELECT revaer_config.factory_reset()")
        .execute(&pool)
        .await;
    let reset_readiness = crate::media::root_catalog::read_root_catalog_readiness(&pool).await;
    let reset_baseline = verify_runtime_pool(&pool).await;
    pool.close().await;
    verified?;
    assert!(missing_job_costs?.is_empty());
    profile_versions?;
    catalog?;
    let denied = direct_table_read.err().ok_or_else(|| {
        anyhow::anyhow!("runtime role unexpectedly read the media job table directly")
    })?;
    assert_eq!(
        denied
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .as_deref(),
        Some("42501")
    );
    reset?;
    reset_baseline?;
    assert_initial_readiness(&readiness?)?;
    assert_initial_readiness(&reset_readiness?)
}

async fn check_catalog_pages(
    admin: &mut sqlx::PgConnection,
    pool: &sqlx::PgPool,
) -> anyhow::Result<()> {
    use crate::media::root_catalog::read_root_catalog_page;
    let missing = read_root_catalog_page(pool, 50, None, None).await?;
    anyhow::ensure!(
        missing.len() == 1 && missing[0].slot.is_none(),
        "expected empty missing catalog"
    );
    sqlx::raw_sql(include_str!(
        "../../../../../scripts/tests/database-root-catalog-reader-fixture.sql"
    ))
    .execute(&mut *admin)
    .await?;
    let first = read_root_catalog_page(pool, 1, None, None).await?;
    anyhow::ensure!(
        first.len() == 2,
        "one slot must retain both allowed-kind rows"
    );
    let slot = first[0]
        .slot
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing first slot"))?;
    anyhow::ensure!(
        slot.logical_key == "reader-1"
            && slot.allowed_root_kind == "source"
            && slot.binding_ready
            && slot.destructive_ready
            && slot.page_has_more,
        "unexpected first-page readiness"
    );
    let second = read_root_catalog_page(
        pool,
        1,
        Some(&slot.logical_key),
        Some(slot.media_root_catalog_slot_public_id),
    )
    .await?;
    anyhow::ensure!(second.len() == 1, "expected one final slot");
    let last = second[0]
        .slot
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing final slot"))?;
    anyhow::ensure!(
        last.logical_key == "reader-2"
            && last.allowed_root_kind == "workspace"
            && !last.page_has_more
            && last.binding_ready
            && last.destructive_ready,
        "unexpected final page"
    );
    let empty = read_root_catalog_page(
        pool,
        1,
        Some(&last.logical_key),
        Some(last.media_root_catalog_slot_public_id),
    )
    .await?;
    anyhow::ensure!(
        empty.len() == 1 && empty[0].slot.is_none(),
        "expected empty continuation"
    );
    for (limit, key, id) in [
        (0, None, None),
        (201, None, None),
        (1, Some("reader-1"), None),
        (1, Some("reader-1"), Some(uuid::Uuid::nil())),
    ] {
        let result = read_root_catalog_page(pool, limit, key, id).await;
        anyhow::ensure!(result.err().is_some_and(|error| error.database_detail() == Some("media_configuration_invalid")),
            "invalid page input was not rejected");
    }
    profile_creation::check(admin, pool).await?;
    let profile_id = profile_versions::check_resolved(admin, pool).await?;
    check_catalog_invalidations(admin, pool).await?;
    profile_versions::check_stale(pool, profile_id).await?;
    check_catalog_digests(admin, pool).await?;
    root_reconciliation::check(pool).await?;
    Ok(())
}

mod profile_creation;
mod profile_versions;
mod root_reconciliation;

async fn check_catalog_digests(
    admin: &mut sqlx::PgConnection,
    pool: &sqlx::PgPool,
) -> anyhow::Result<()> {
    let mut transaction = admin.begin().await?;
    let result = sqlx::raw_sql(include_str!(
        "../../../../../scripts/tests/database-root-catalog-digest-fixture.sql"
    ))
    .execute(&mut *transaction)
    .await;
    transaction.rollback().await?;
    result?;
    for query in [
        "SELECT media_root_catalog_slot_identity_v1(1)",
        "SELECT * FROM media_root_catalog_digests_v1(1)",
    ] {
        let denied = sqlx::query(query)
            .execute(pool)
            .await
            .err()
            .ok_or_else(|| anyhow::anyhow!("runtime called an internal digest helper"))?;
        anyhow::ensure!(
            denied
                .as_database_error()
                .is_some_and(|error| error.code().as_deref() == Some("42501")),
            "wrong digest helper denial"
        );
    }
    Ok(())
}

async fn reject_catalog_invalid_inputs(
    pool: &sqlx::PgPool,
) -> anyhow::Result<crate::media::root_catalog::RootCatalogStateRow> {
    use crate::media::root_catalog::read_root_catalog_page;
    let original = read_root_catalog_page(pool, 1, None, None)
        .await?
        .remove(0)
        .state;
    anyhow::ensure!(
        original.attestation_generation.is_some(),
        "fixture must start ready"
    );
    for (state, reason) in [
        (None, None),
        (Some("ready"), None),
        (Some("missing"), None),
        (Some("missing"), Some("media_root_overlap")),
        (Some("invalid"), Some("/sensitive/path")),
    ] {
        let error = sqlx::query("SELECT media_root_catalog_mark_unavailable_v1($1, $2)")
            .bind(state)
            .bind(reason)
            .execute(pool)
            .await
            .err()
            .ok_or_else(|| anyhow::anyhow!("invalid source mutation accepted"))?;
        anyhow::ensure!(
            error
                .as_database_error()
                .is_some_and(|error| error.code().as_deref() == Some("P0001")
                    && error.message() == "media_configuration_invalid"),
            "unexpected source mutation error"
        );
    }
    for reason in [
        None,
        Some("media_root_catalog_source_missing"),
        Some("/sensitive/path"),
    ] {
        let error = sqlx::query("SELECT media_root_catalog_mark_attestation_invalid_v1($1)")
            .bind(reason)
            .execute(pool)
            .await
            .err()
            .ok_or_else(|| anyhow::anyhow!("invalid attestation mutation accepted"))?;
        anyhow::ensure!(
            error
                .as_database_error()
                .is_some_and(|error| error.code().as_deref() == Some("P0001")
                    && error.message() == "media_configuration_invalid"),
            "unexpected attestation mutation error"
        );
    }
    let mut rollback = pool.begin().await?;
    sqlx::query("SELECT media_root_catalog_mark_attestation_invalid_v1($1)")
        .bind("media_root_overlap")
        .execute(&mut *rollback)
        .await?;
    let invalidated: (String, String, Option<i64>) = sqlx::query_as(
        "SELECT source_state, attestation_state, attestation_generation FROM media_root_catalog_state_get_v1()",
    ).fetch_one(&mut *rollback).await?;
    anyhow::ensure!(
        invalidated == ("ready".into(), "invalid".into(), None),
        "attestation failure did not clear the active generation"
    );
    rollback.rollback().await?;
    let unchanged = read_root_catalog_page(pool, 1, None, None)
        .await?
        .remove(0)
        .state;
    anyhow::ensure!(
        original == unchanged,
        "rejected/rolled-back mutation changed state"
    );
    Ok(original)
}

async fn check_catalog_unavailable_states(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    use crate::media::root_catalog::{
        RootAttestationFailure as Failure, RootCatalogUnavailable as Source,
        mark_root_attestation_invalid, mark_root_catalog_unavailable, read_root_catalog_readiness,
    };
    for (source, expected) in [
        (Source::Missing, "missing"),
        (Source::Untrusted, "untrusted"),
        (Source::Invalid, "invalid"),
        (Source::BoundExceeded, "bound_exceeded"),
        (Source::Unsupported, "unsupported"),
    ] {
        mark_root_catalog_unavailable(pool, source).await?;
        let rows = read_root_catalog_readiness(pool).await?;
        anyhow::ensure!(
            rows.len() == 5
                && rows.iter().all(|row| row.source_state == expected
                    && row.attestation_state == "not_evaluated"
                    && row.attestation_generation.is_none()
                    && row.attestation_reason_code.is_none()
                    && row.attested_slot_count == 0
                    && row.binding_ready_slot_count == 0
                    && row.destructive_ready_slot_count == 0),
            "source invalidation retained readiness"
        );
    }
    for (failure, expected) in [
        (Failure::Invalid, "media_root_attestation_invalid"),
        (Failure::Overlap, "media_root_overlap"),
        (Failure::UnsafeAncestry, "media_root_unsafe_ancestry"),
        (
            Failure::DurabilityUnproven,
            "media_root_durability_unproven",
        ),
        (
            Failure::WriterControlUnproven,
            "media_root_writer_control_unproven",
        ),
        (Failure::IdentityMismatch, "media_root_identity_mismatch"),
    ] {
        mark_root_attestation_invalid(pool, failure).await?;
        let rows = read_root_catalog_readiness(pool).await?;
        anyhow::ensure!(
            rows.len() == 5
                && rows.iter().all(|row| row.source_state == "ready"
                    && row.source_reason_code.is_none()
                    && row.attestation_state == "invalid"
                    && row.attestation_reason_code.as_deref() == Some(expected)
                    && row.attestation_generation.is_none()
                    && row.attested_slot_count == 0
                    && row.binding_ready_slot_count == 0
                    && row.destructive_ready_slot_count == 0),
            "attestation invalidation retained readiness"
        );
    }
    Ok(())
}

async fn check_catalog_invalidations(
    admin: &mut sqlx::PgConnection,
    pool: &sqlx::PgPool,
) -> anyhow::Result<()> {
    use crate::media::root_catalog::read_root_catalog_page;
    let original = reject_catalog_invalid_inputs(pool).await?;
    check_catalog_unavailable_states(pool).await?;
    let hidden = read_root_catalog_page(pool, 1, None, None).await?;
    anyhow::ensure!(
        hidden.len() == 1 && hidden[0].slot.is_none(),
        "invalidated rows remain visible"
    );
    let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM public.media_root_catalog_generation WHERE media_root_catalog_generation_id = $1")
        .bind(original.attestation_generation).fetch_one(&mut *admin).await?;
    anyhow::ensure!(retained == 1, "invalidation removed historical generation");
    let denied = sqlx::query("SELECT source_state FROM public.media_root_catalog_state")
        .execute(pool)
        .await
        .err()
        .ok_or_else(|| anyhow::anyhow!("runtime read state table directly"))?;
    anyhow::ensure!(
        denied
            .as_database_error()
            .is_some_and(|error| error.code().as_deref() == Some("42501")),
        "wrong direct-read denial"
    );
    Ok(())
}

fn assert_initial_readiness(
    readiness: &[crate::media::root_catalog::RootCatalogReadinessRow],
) -> anyhow::Result<()> {
    anyhow::ensure!(readiness.len() == 5, "expected five root kinds");
    anyhow::ensure!(
        readiness.iter().map(|row| row.root_kind.as_str()).eq([
            "source",
            "output",
            "workspace",
            "backup",
            "quarantine"
        ]),
        "expected canonical root kind order"
    );
    anyhow::ensure!(
        readiness.iter().all(|row| row.source_state == "missing"
            && row.source_reason_code.as_deref() == Some("media_root_catalog_source_missing")
            && row.attestation_state == "not_evaluated"
            && row.attestation_reason_code.is_none()
            && row.attestation_generation.is_none()
            && row.attested_slot_count == 0
            && row.binding_ready_slot_count == 0
            && row.destructive_ready_slot_count == 0),
        "unexpected initial readiness"
    );
    Ok(())
}

async fn apply_and_seal(owner: &mut sqlx::PgConnection, runtime_name: &str) -> anyhow::Result<()> {
    let mut transaction = owner.begin().await?;
    sqlx::raw_sql(include_str!("../../../init.sql"))
        .execute(&mut *transaction)
        .await?;
    sqlx::query("SELECT * FROM revaer_system.seal_database_baseline_v1($1, $2, $3)")
        .bind(1_i16)
        .bind(super::PACKAGED_INIT_SHA256.as_slice())
        .bind(runtime_name)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(())
}

#[tokio::test]
async fn baseline_pool_rejects_uninitialized_database_without_returning_connection()
-> anyhow::Result<()> {
    let database = revaer_test_support::postgres::start_postgres()?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database.connection_string())
        .await?;
    let result = verify_runtime_pool(&pool).await;
    let remaining = pool.size();
    pool.close().await;
    assert_eq!(
        result.map_err(|error| error.reason()),
        Err(BaselineReadReason::BaselineShapeInvalid)
    );
    assert_eq!(remaining, 0);
    Ok(())
}

async fn attempt_referenced_policy_mutation(
    admin: &mut sqlx::PgConnection,
) -> anyhow::Result<sqlx::Error> {
    sqlx::query("UPDATE public.media_policy_output SET quarantine_enabled = true WHERE media_policy_profile_id = (SELECT media_policy_profile_id FROM public.media_policy_profile WHERE policy_key = 'profile-save-policy')")
        .execute(admin).await.err().ok_or_else(|| anyhow::anyhow!("referenced policy component mutated"))
}

mod cancelled_begin;
