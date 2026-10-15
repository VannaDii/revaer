use crate::media::profile_versions::{
    CreateProfileVersionInput, create_profile_version, read_profile_page, read_profile_version,
};
use sqlx::{PgConnection, PgPool};

fn input() -> CreateProfileVersionInput<'static> {
    CreateProfileVersionInput {
        actor_public_id: uuid::Uuid::nil(),
        profile_key: "saved-profile",
        display_name: " Exact save ",
        description: "Exact description",
        enabled: false,
        dry_run_only: true,
        desired_target_key: "profile-save-target",
        desired_target_version: 1,
        policy_key: "profile-save-policy",
        policy_version: 1,
        output_root_key: "reader-1",
        workspace_root_key: "reader-2",
        backup_root_key: None,
        quarantine_root_key: None,
    }
}

pub(super) async fn check(admin: &mut PgConnection, runtime: &PgPool) -> anyhow::Result<()> {
    sqlx::raw_sql(include_str!(
        "../../../../../../scripts/tests/database-profile-version-create-fixture.sql"
    ))
    .execute(&mut *admin)
    .await?;
    let missing_target = CreateProfileVersionInput {
        desired_target_version: 2,
        ..input()
    };
    rejected(runtime, &missing_target, "media_desired_target_not_found").await?;
    let missing_policy = CreateProfileVersionInput {
        policy_version: 2,
        ..input()
    };
    rejected(runtime, &missing_policy, "media_policy_profile_not_found").await?;
    let unmapped = CreateProfileVersionInput {
        workspace_root_key: "absent",
        ..input()
    };
    rejected(runtime, &unmapped, "media_configuration_root_unmapped").await?;
    let forbidden = CreateProfileVersionInput {
        output_root_key: "reader-2",
        ..input()
    };
    rejected(runtime, &forbidden, "media_root_kind_forbidden").await?;
    for optional in [
        CreateProfileVersionInput {
            backup_root_key: Some("reader-1"),
            ..input()
        },
        CreateProfileVersionInput {
            quarantine_root_key: Some("reader-1"),
            ..input()
        },
    ] {
        rejected(runtime, &optional, "media_root_binding_incomplete").await?;
    }
    let absent: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM public.media_profile WHERE profile_key = 'saved-profile'",
    )
    .fetch_one(&mut *admin)
    .await?;
    anyhow::ensure!(absent == 0, "failed create left a parent or head behind");
    let rows = create_profile_version(runtime, &input()).await?;
    let first = rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("saved profile unreadable"))?;
    anyhow::ensure!(
        rows.len() == 2
            && first.latest_version == 1
            && first.active_version == Some(1)
            && first.lifecycle_state == "active"
            && !first.enabled
            && first.dry_run_only
            && first.display_name == " Exact save "
            && first.description == "Exact description"
            && first.desired_target_key == "profile-save-target"
            && first.desired_target_version == 1
            && first.policy_key == "profile-save-policy"
            && first.policy_version == 1,
        "save changed explicit values or failed to advance disabled heads"
    );
    anyhow::ensure!(
        read_profile_version(runtime, first.media_profile_public_id).await? == rows,
        "committed profile differs from save response snapshot"
    );
    let path_free: bool = sqlx::query_scalar("SELECT source_root IS NULL AND output_root IS NULL FROM public.media_profile WHERE media_profile_public_id = $1")
        .bind(first.media_profile_public_id).fetch_one(&mut *admin).await?;
    anyhow::ensure!(path_free, "save persisted a path placeholder");
    rejected(runtime, &input(), "media_profile_key_conflict").await?;
    anyhow::ensure!(
        read_profile_version(runtime, first.media_profile_public_id).await? == rows,
        "duplicate create changed existing version"
    );
    check_pages(runtime, first.media_profile_public_id).await?;
    let frozen = super::attempt_referenced_policy_mutation(admin).await?;
    anyhow::ensure!(
        frozen
            .as_database_error()
            .and_then(|e| e.try_downcast_ref::<sqlx::postgres::PgDatabaseError>())
            .and_then(sqlx::postgres::PgDatabaseError::detail)
            == Some("media_policy_version_immutable"),
        "unexpected policy freeze rejection"
    );
    Ok(())
}

async fn check_pages(runtime: &PgPool, saved: uuid::Uuid) -> anyhow::Result<()> {
    let all = read_profile_page(runtime, 200, None, None).await?;
    let expected: std::collections::BTreeSet<_> =
        all.iter().map(|row| row.media_profile_public_id).collect();
    anyhow::ensure!(
        expected.contains(&saved),
        "path-free save absent from collection"
    );
    let mut seen = std::collections::BTreeSet::new();
    let mut cursor = None;
    for _ in 0..=expected.len() {
        let page = read_profile_page(
            runtime,
            1,
            cursor
                .as_ref()
                .map(|(key, _): &(String, uuid::Uuid)| key.as_str()),
            cursor.as_ref().map(|(_, id)| *id),
        )
        .await?;
        let Some(first) = page.first() else {
            break;
        };
        anyhow::ensure!(
            seen.insert(first.media_profile_public_id),
            "pagination repeated a parent"
        );
        let parent_count = page
            .iter()
            .map(|row| row.media_profile_public_id)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        anyhow::ensure!(
            parent_count <= 2,
            "limit applied to bindings instead of parents"
        );
        if parent_count == 1 {
            break;
        }
        cursor = Some((first.profile_key.clone(), first.media_profile_public_id));
    }
    anyhow::ensure!(
        seen == expected,
        "pagination skipped complete profile parents"
    );
    for (limit, key, id) in [
        (0, None, None),
        (201, None, None),
        (1, Some("missing"), Some(uuid::Uuid::new_v4())),
        (1, Some("saved-profile"), None),
    ] {
        let error = read_profile_page(runtime, limit, key, id)
            .await
            .err()
            .ok_or_else(|| anyhow::anyhow!("invalid collection request accepted"))?;
        anyhow::ensure!(error.database_detail() == Some("media_configuration_invalid"));
    }
    Ok(())
}

async fn rejected(
    runtime: &PgPool,
    input: &CreateProfileVersionInput<'_>,
    code: &str,
) -> anyhow::Result<()> {
    let error = create_profile_version(runtime, input)
        .await
        .err()
        .ok_or_else(|| anyhow::anyhow!("invalid profile creation accepted"))?;
    anyhow::ensure!(
        error.database_detail() == Some(code),
        "unexpected profile rejection: {error}"
    );
    Ok(())
}
