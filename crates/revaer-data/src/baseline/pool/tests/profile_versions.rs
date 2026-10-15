use crate::media::profile_versions::read_profile_version;
use sqlx::{Connection, PgConnection, PgPool};

pub(super) async fn check(admin: &mut PgConnection, runtime: &PgPool) -> anyhow::Result<()> {
    let mut transaction = admin.begin().await?;
    let result = sqlx::raw_sql(include_str!(
        "../../../../../../scripts/tests/database-profile-version-schema-fixture.sql"
    ))
    .execute(&mut *transaction)
    .await;
    match result {
        Ok(_) => transaction.commit().await?,
        Err(error) => {
            transaction.rollback().await?;
            return Err(error.into());
        }
    }
    let id = sqlx::query_scalar("SELECT media_profile_public_id FROM public.media_profile WHERE profile_key = 'profile-schema-first'")
        .fetch_one(&mut *admin).await?;
    let rows = read_profile_version(runtime, id).await?;
    anyhow::ensure!(rows.len() == 2, "profile bindings missing or duplicated");
    for row in &rows {
        anyhow::ensure!(
            row.media_profile_public_id == id
                && row.profile_key == "profile-schema-first"
                && row.display_name == " Exact profile name "
                && row.description.is_empty()
                && !row.enabled
                && row.dry_run_only
                && row.desired_target_key == "profile-schema-test"
                && row.desired_target_version == 1
                && row.policy_key == "safe_dry_run"
                && row.policy_version == 1
                && row.latest_version == 1
                && row.active_version.is_none()
                && row.lifecycle_state == "draft"
                && row.resolution_state == "unmapped"
                && !row.readiness.binding_ready
                && !row.readiness.destructive_ready
                && row.readiness.binding_reason.as_deref() == Some("media_root_binding_incomplete")
                && row.readiness.destructive_reason.as_deref()
                    == Some("media_root_binding_incomplete"),
            "profile reader changed stored data or invented readiness"
        );
    }
    anyhow::ensure!(
        rows[0].root_kind == "output"
            && rows[0].logical_key == "output-key"
            && rows[1].root_kind == "workspace"
            && rows[1].logical_key == "workspace-key",
        "profile root ordinal or key drift"
    );
    anyhow::ensure!(
        read_profile_version(runtime, uuid::Uuid::new_v4())
            .await?
            .is_empty(),
        "unknown profile synthesized"
    );
    for statement in [
        "SELECT media_profile_version_id FROM public.media_profile_version LIMIT 0",
        "SELECT media_profile_version_id FROM public.media_profile_version_root_binding LIMIT 0",
    ] {
        let rejected = sqlx::query(statement)
            .execute(runtime)
            .await
            .err()
            .ok_or_else(|| anyhow::anyhow!("runtime accessed profile version tables directly"))?;
        anyhow::ensure!(
            rejected
                .as_database_error()
                .is_some_and(|error| error.code().as_deref() == Some("42501")),
            "unexpected profile table access rejection"
        );
    }
    Ok(())
}

pub(super) async fn check_resolved(
    admin: &mut PgConnection,
    runtime: &PgPool,
) -> anyhow::Result<uuid::Uuid> {
    sqlx::raw_sql(include_str!(
        "../../../../../../scripts/tests/database-profile-version-reader-fixture.sql"
    ))
    .execute(&mut *admin)
    .await?;
    let id = sqlx::query_scalar("SELECT media_profile_public_id FROM public.media_profile WHERE profile_key = 'profile-schema-second'")
        .fetch_one(&mut *admin).await?;
    let rows = read_profile_version(runtime, id).await?;
    anyhow::ensure!(rows.len() == 2, "resolved profile lost bindings");
    for row in &rows {
        anyhow::ensure!(
            row.lifecycle_state == "active"
                && row.latest_version == 1
                && row.active_version == Some(1)
                && !row.enabled
                && !row.dry_run_only
                && row.description == "Exact reader fixture"
                && row.resolution_state == "resolved"
                && row.readiness.binding_ready
                && row.readiness.binding_reason.is_none()
                && row.readiness.destructive_ready
                && row.readiness.destructive_reason.is_none(),
            "reader confused disabled profile settings with root readiness"
        );
    }
    anyhow::ensure!(
        rows[0].root_kind == "output"
            && rows[0].logical_key == "reader-1"
            && rows[1].root_kind == "workspace"
            && rows[1].logical_key == "reader-2",
        "resolved profile root ordinal or key drift"
    );
    Ok(id)
}

pub(super) async fn check_stale(runtime: &PgPool, id: uuid::Uuid) -> anyhow::Result<()> {
    let rows = read_profile_version(runtime, id).await?;
    anyhow::ensure!(rows.len() == 2, "stale profile disappeared");
    for row in rows {
        anyhow::ensure!(
            row.latest_version == 1
                && row.active_version == Some(1)
                && row.resolution_state == "resolved"
                && !row.readiness.binding_ready
                && !row.readiness.destructive_ready
                && row.readiness.binding_reason.as_deref() == Some("media_root_binding_incomplete")
                && row.readiness.destructive_reason.as_deref()
                    == Some("media_root_binding_incomplete"),
            "stale catalog rewrote history or manufactured readiness"
        );
    }
    Ok(())
}
