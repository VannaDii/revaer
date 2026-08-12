use std::path::{Path, PathBuf};

use super::{
    MediaRootIdentity, MediaRootIdentityError, MediaRootIdentityResolver,
    StdMediaRootIdentityResolver,
};
use crate::config::run_migrations;
use chrono::{DateTime, Duration, Utc};
use revaer_test_support::postgres::{TestDatabase, start_postgres};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};
use uuid::Uuid;

const ACTOR_ID: &str = "00000000-0000-0000-0000-000000000000";

struct TestDb {
    _database: TestDatabase,
    pool: PgPool,
}

struct TestRoots {
    base: PathBuf,
    source: MediaRootIdentity,
    output: MediaRootIdentity,
}

impl Drop for TestRoots {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.base) {
            eprintln!("failed to remove media root test directory: {error}");
        }
    }
}

async fn setup_db(test_name: &str) -> anyhow::Result<Option<TestDb>> {
    let database = match start_postgres() {
        Ok(database) => database,
        Err(error) => {
            eprintln!("skipping {test_name}: {error}");
            return Ok(None);
        }
    };
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(database.connection_string())
        .await?;
    run_migrations(&pool).await?;
    Ok(Some(TestDb {
        _database: database,
        pool,
    }))
}

fn actor_id() -> anyhow::Result<Uuid> {
    Ok(Uuid::parse_str(ACTOR_ID)?)
}

fn path_text(path: &Path) -> anyhow::Result<&str> {
    path.to_str()
        .ok_or_else(|| anyhow::anyhow!("test path is not valid UTF-8: {}", path.display()))
}

fn identity_number(value: u64) -> anyhow::Result<i64> {
    Ok(i64::try_from(value)?)
}

fn make_test_roots() -> anyhow::Result<TestRoots> {
    let base = std::env::temp_dir().join(format!("revaer-media-data-{}", Uuid::new_v4()));
    let source_path = base.join("LibraryCase");
    let output_path = base.join("output");
    std::fs::create_dir_all(&source_path)?;
    std::fs::create_dir_all(&output_path)?;
    let resolver = StdMediaRootIdentityResolver;
    let source = resolver.resolve(&source_path)?;
    let output = resolver.resolve(&output_path)?;
    Ok(TestRoots {
        base,
        source,
        output,
    })
}

async fn create_profile(pool: &PgPool, key: &str, roots: &TestRoots) -> anyhow::Result<Uuid> {
    let profile_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_profile_create_v3($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
    )
    .bind(actor_id()?)
    .bind(key)
    .bind(path_text(roots.source.requested_path())?)
    .bind(path_text(roots.source.canonical_path())?)
    .bind(identity_number(roots.source.filesystem_device())?)
    .bind(identity_number(roots.source.filesystem_inode())?)
    .bind(path_text(roots.output.requested_path())?)
    .bind(path_text(roots.output.canonical_path())?)
    .bind(identity_number(roots.output.filesystem_device())?)
    .bind(identity_number(roots.output.filesystem_inode())?)
    .bind(30_i32)
    .bind(Option::<&str>::None)
    .bind("safe_dry_run")
    .fetch_one(pool)
    .await?;
    Ok(profile_id)
}

async fn create_job(
    pool: &PgPool,
    profile_id: Uuid,
    roots: &TestRoots,
    sequence: usize,
) -> anyhow::Result<Uuid> {
    let source = roots
        .source
        .canonical_path()
        .join(format!("source-{sequence}.mkv"));
    let output = roots
        .output
        .canonical_path()
        .join(format!("output-{sequence}.mkv"));
    let job_id = sqlx::query_scalar::<_, Uuid>("SELECT media_job_create_v1($1,$2,$3,$4,$5)")
        .bind(actor_id()?)
        .bind(profile_id)
        .bind(path_text(&source)?)
        .bind(path_text(&output)?)
        .bind(true)
        .fetch_one(pool)
        .await?;
    Ok(job_id)
}

fn database_message(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .map(sqlx::error::DatabaseError::message)
}

async fn claim_job(pool: &PgPool) -> anyhow::Result<(Uuid, i32, i64)> {
    let row = sqlx::query(
        "SELECT media_job_public_id, attempt_number, claim_generation \
         FROM media_job_worker_claim_next_v2()",
    )
    .fetch_one(pool)
    .await?;
    Ok((row.get(0), row.get(1), row.get(2)))
}

#[cfg(unix)]
#[test]
fn normalized_profiles_validate_identity_snapshot_and_safe_create() -> anyhow::Result<()> {
    let roots = make_test_roots()?;
    let resolver = StdMediaRootIdentityResolver;

    let relative_error = resolver.resolve(Path::new("relative-media-root"));
    assert!(matches!(
        relative_error,
        Err(MediaRootIdentityError::PathNotAbsolute(_))
    ));
    let file_path = roots.base.join("not-a-directory");
    std::fs::write(&file_path, b"identity fixture")?;
    let file_error = resolver.resolve(&file_path);
    assert!(matches!(
        file_error,
        Err(MediaRootIdentityError::NotDirectory(_))
    ));

    Ok(())
}

#[test]
fn media_root_identity_errors_preserve_diagnostics_and_sources() {
    let relative = MediaRootIdentityError::PathNotAbsolute(PathBuf::from("relative-root"));
    assert_eq!(
        relative.to_string(),
        "media root is not absolute: relative-root"
    );
    assert!(std::error::Error::source(&relative).is_none());

    let io = MediaRootIdentityError::from(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "identity denied",
    ));
    assert_eq!(
        io.to_string(),
        "media root filesystem operation failed: identity denied"
    );
    assert!(std::error::Error::source(&io).is_some());

    let not_directory = MediaRootIdentityError::NotDirectory(PathBuf::from("/media/file.mkv"));
    assert_eq!(
        not_directory.to_string(),
        "media root is not a directory: /media/file.mkv"
    );
    assert!(std::error::Error::source(&not_directory).is_none());

    let unsupported = MediaRootIdentityError::UnsupportedPlatform;
    assert_eq!(
        unsupported.to_string(),
        "media root identity requires filesystem device and inode support"
    );
    assert!(std::error::Error::source(&unsupported).is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn normalized_profiles_reject_filesystem_aliases() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("normalized_profiles_reject_filesystem_aliases").await? else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "alias-profile", &roots).await?;
    let resolver = StdMediaRootIdentityResolver;

    let sibling_path = roots.base.join("librarycase");
    std::fs::create_dir_all(&sibling_path)?;
    let sibling = resolver.resolve(&sibling_path)?;
    if !roots.source.matches(&sibling) {
        sqlx::query_scalar::<_, Uuid>(
            "SELECT media_profile_root_add_v1($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(profile_id)
        .bind("source")
        .bind(path_text(sibling.requested_path())?)
        .bind(path_text(sibling.canonical_path())?)
        .bind(identity_number(sibling.filesystem_device())?)
        .bind(identity_number(sibling.filesystem_inode())?)
        .bind("movies")
        .bind(1_i32)
        .bind(true)
        .fetch_one(&test_db.pool)
        .await?;
    }

    let alias_path = roots.base.join("source-alias");
    std::os::unix::fs::symlink(roots.source.canonical_path(), &alias_path)?;
    let alias = resolver.resolve(&alias_path)?;
    assert!(roots.source.matches(&alias));
    let alias_error = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_profile_root_add_v1($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(profile_id)
    .bind("source")
    .bind(path_text(alias.requested_path())?)
    .bind(path_text(alias.canonical_path())?)
    .bind(identity_number(alias.filesystem_device())?)
    .bind(identity_number(alias.filesystem_inode())?)
    .bind("movies")
    .bind(2_i32)
    .bind(true)
    .fetch_one(&test_db.pool)
    .await;
    match alias_error {
        Err(error) => assert_eq!(database_message(&error), Some("filesystem roots overlap")),
        Ok(_) => {
            return Err(anyhow::anyhow!(
                "symlink alias unexpectedly passed identity validation"
            ));
        }
    }

    let bind_alias_error = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_profile_root_add_v1($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(profile_id)
    .bind("source")
    .bind("/mnt/revaer-bind-alias")
    .bind("/mnt/revaer-bind-alias")
    .bind(identity_number(roots.source.filesystem_device())?)
    .bind(identity_number(roots.source.filesystem_inode())?)
    .bind("movies")
    .bind(3_i32)
    .bind(true)
    .fetch_one(&test_db.pool)
    .await;
    match bind_alias_error {
        Err(error) => assert_eq!(database_message(&error), Some("filesystem roots overlap")),
        Ok(_) => {
            return Err(anyhow::anyhow!(
                "bind alias unexpectedly passed identity validation"
            ));
        }
    }

    Ok(())
}

#[tokio::test]
async fn discovery_revalidates_root_identity() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("discovery_revalidates_root_identity").await? else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "discovery-identity-profile", &roots).await?;
    let resolver = StdMediaRootIdentityResolver;

    let secondary_path = roots.base.join("secondary");
    std::fs::create_dir_all(&secondary_path)?;
    let secondary = resolver.resolve(&secondary_path)?;
    let secondary_root_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_profile_root_add_v1($1,$2,$3,$4,$5,$6,$7,$8,$9)",
    )
    .bind(profile_id)
    .bind("source")
    .bind(path_text(secondary.requested_path())?)
    .bind(path_text(secondary.canonical_path())?)
    .bind(identity_number(secondary.filesystem_device())?)
    .bind(identity_number(secondary.filesystem_inode())?)
    .bind("movies")
    .bind(4_i32)
    .bind(true)
    .fetch_one(&test_db.pool)
    .await?;
    let source_root_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_profile_root_public_id FROM media_profile_root_list_v1($1) \
         WHERE root_kind='source' AND sort_order=0",
    )
    .bind(profile_id)
    .fetch_one(&test_db.pool)
    .await?;
    let due_at = Utc::now() - Duration::minutes(1);
    let schedule_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_discovery_schedule_create_v1($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(profile_id)
    .bind(source_root_id)
    .bind(15_i32)
    .bind("minutes")
    .bind(0_i32)
    .bind(true)
    .bind(due_at)
    .fetch_one(&test_db.pool)
    .await?;
    let watcher_id =
        sqlx::query_scalar::<_, Uuid>("SELECT media_discovery_watcher_create_v1($1,$2,$3,$4,$5)")
            .bind(profile_id)
            .bind(secondary_root_id)
            .bind(500_i32)
            .bind(0_i32)
            .bind(true)
            .fetch_one(&test_db.pool)
            .await?;
    sqlx::query("SELECT * FROM media_discovery_schedule_claim_v1($1,$2,$3,$4,$5)")
        .bind(schedule_id)
        .bind(path_text(roots.source.canonical_path())?)
        .bind(identity_number(roots.source.filesystem_device())?)
        .bind(identity_number(roots.source.filesystem_inode())?)
        .bind(Utc::now())
        .execute(&test_db.pool)
        .await?;
    sqlx::query("SELECT * FROM media_discovery_watcher_start_v1($1,$2,$3,$4)")
        .bind(watcher_id)
        .bind(path_text(secondary.canonical_path())?)
        .bind(identity_number(secondary.filesystem_device())?)
        .bind(identity_number(secondary.filesystem_inode())?)
        .execute(&test_db.pool)
        .await?;
    let changed_identity =
        sqlx::query("SELECT * FROM media_discovery_watcher_start_v1($1,$2,$3,$4)")
            .bind(watcher_id)
            .bind(path_text(secondary.canonical_path())?)
            .bind(identity_number(secondary.filesystem_device())?)
            .bind(identity_number(secondary.filesystem_inode())? + 1)
            .execute(&test_db.pool)
            .await;
    assert!(changed_identity.is_err());

    Ok(())
}

#[tokio::test]
async fn discovery_rejects_ambiguous_associations_and_unbounded_policy() -> anyhow::Result<()> {
    let Some(test_db) =
        setup_db("discovery_rejects_ambiguous_associations_and_unbounded_policy").await?
    else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "association-profile", &roots).await?;

    let other_roots = make_test_roots()?;
    let other_profile_id = create_profile(&test_db.pool, "other-association", &other_roots).await?;
    let other_source_root_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_profile_root_public_id FROM media_profile_root_list_v1($1) \
         WHERE root_kind='source' AND sort_order=0",
    )
    .bind(other_profile_id)
    .fetch_one(&test_db.pool)
    .await?;
    let ambiguous_association = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_discovery_schedule_create_v1($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(profile_id)
    .bind(other_source_root_id)
    .bind(15_i32)
    .bind("minutes")
    .bind(1_i32)
    .bind(true)
    .bind(Utc::now())
    .fetch_one(&test_db.pool)
    .await;
    assert!(ambiguous_association.is_err());

    let unbounded_runtime = sqlx::query(
        "SELECT media_policy_runtime_limit_set_v2($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
    )
    .bind(actor_id()?)
    .bind("safe_dry_run")
    .bind(1_i32)
    .bind(0_i32)
    .bind(0_i32)
    .bind(21_600_i32)
    .bind(1024_i32)
    .bind(10_737_418_240_i64)
    .bind(true)
    .bind(20_i32)
    .bind("serious")
    .bind(true)
    .execute(&test_db.pool)
    .await;
    assert!(unbounded_runtime.is_err());

    Ok(())
}

async fn append_profile_file_rules(pool: &PgPool, profile_id: Uuid) -> anyhow::Result<()> {
    sqlx::query_scalar::<_, i64>("SELECT media_profile_file_rule_append_v1($1,$2,$3,$4,$5,$6)")
        .bind(profile_id)
        .bind("include")
        .bind("extension")
        .bind("mkv")
        .bind(0_i32)
        .bind(true)
        .fetch_one(pool)
        .await?;
    sqlx::query_scalar::<_, i64>("SELECT media_profile_file_rule_append_v1($1,$2,$3,$4,$5,$6)")
        .bind(profile_id)
        .bind("exclude")
        .bind("glob")
        .bind("**/sample/**")
        .bind(1_i32)
        .bind(true)
        .fetch_one(pool)
        .await?;
    Ok(())
}

#[tokio::test]
async fn profile_rules_are_ordered_and_bounded() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("profile_rules_are_ordered_and_bounded").await? else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "profile-rule-bounds", &roots).await?;
    append_profile_file_rules(&test_db.pool, profile_id).await?;
    let rules = sqlx::query("SELECT * FROM media_profile_file_rule_list_v1($1)")
        .bind(profile_id)
        .fetch_all(&test_db.pool)
        .await?;
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].get::<i32, _>("sort_order"), 0);
    assert_eq!(rules[1].get::<i32, _>("sort_order"), 1);

    let unbounded_filter =
        sqlx::query("SELECT media_profile_filter_set_v1($1,$2,$3,$4,$5,$6,$7,$8,$9)")
            .bind(profile_id)
            .bind(Option::<i64>::None)
            .bind(Option::<i64>::None)
            .bind(Option::<i64>::None)
            .bind(Option::<i64>::None)
            .bind(false)
            .bind(false)
            .bind(true)
            .bind(true)
            .execute(&test_db.pool)
            .await;
    assert!(unbounded_filter.is_err());

    Ok(())
}

#[tokio::test]
async fn job_snapshots_and_selected_policy_are_immutable() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("job_snapshots_and_selected_policy_are_immutable").await? else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "snapshot-profile", &roots).await?;
    append_profile_file_rules(&test_db.pool, profile_id).await?;

    let job_id = create_job(&test_db.pool, profile_id, &roots, 0).await?;
    let snapshot_roots = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM media_job_root_snapshot snapshot \
         JOIN media_job job USING (media_job_id) WHERE job.media_job_public_id=$1",
    )
    .bind(job_id)
    .fetch_one(&test_db.pool)
    .await?;
    assert!(snapshot_roots >= 2);
    sqlx::query_scalar::<_, i64>("SELECT media_profile_file_rule_append_v1($1,$2,$3,$4,$5,$6)")
        .bind(profile_id)
        .bind("include")
        .bind("extension")
        .bind("mp4")
        .bind(2_i32)
        .bind(true)
        .fetch_one(&test_db.pool)
        .await?;
    let current_rule_count =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM media_profile_file_rule_list_v1($1)")
            .bind(profile_id)
            .fetch_one(&test_db.pool)
            .await?;
    let snapshot_rule_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM media_job_file_rule_snapshot snapshot \
         JOIN media_job job USING (media_job_id) WHERE job.media_job_public_id=$1",
    )
    .bind(job_id)
    .fetch_one(&test_db.pool)
    .await?;
    assert_eq!(current_rule_count, 3);
    assert_eq!(snapshot_rule_count, 2);
    let snapshot_mutation = sqlx::query(
        "UPDATE media_job_file_rule_snapshot SET enabled=FALSE \
         WHERE media_job_id=(SELECT media_job_id FROM media_job WHERE media_job_public_id=$1)",
    )
    .bind(job_id)
    .execute(&test_db.pool)
    .await;
    assert!(snapshot_mutation.is_err());
    let selected_policy_mutation = sqlx::query(
        "SELECT media_policy_runtime_limit_set_v2($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)",
    )
    .bind(actor_id()?)
    .bind("safe_dry_run")
    .bind(1_i32)
    .bind(1_i32)
    .bind(0_i32)
    .bind(21_600_i32)
    .bind(1024_i32)
    .bind(10_737_418_240_i64)
    .bind(true)
    .bind(20_i32)
    .bind("serious")
    .bind(true)
    .execute(&test_db.pool)
    .await;
    assert!(selected_policy_mutation.is_err());

    Ok(())
}

#[tokio::test]
async fn profile_create_is_insert_only_and_preserves_live_state() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("profile_create_is_insert_only_and_preserves_live_state").await?
    else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "identity-profile", &roots).await?;

    sqlx::query("SELECT media_profile_update_v1($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
        .bind(actor_id()?)
        .bind(profile_id)
        .bind(path_text(roots.source.canonical_path())?)
        .bind(path_text(roots.output.canonical_path())?)
        .bind(false)
        .bind(30_i32)
        .bind(Option::<&str>::None)
        .bind("safe_dry_run")
        .bind(false)
        .bind(false)
        .bind(Option::<i32>::None)
        .execute(&test_db.pool)
        .await?;

    let duplicate = sqlx::query_scalar::<_, Uuid>(
        "SELECT media_profile_upsert_v2($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)",
    )
    .bind(actor_id()?)
    .bind("identity-profile")
    .bind("/different/source")
    .bind("/different/output")
    .bind(true)
    .bind(30_i32)
    .bind(Option::<&str>::None)
    .bind("safe_dry_run")
    .bind(true)
    .bind(true)
    .bind(5_i32)
    .fetch_one(&test_db.pool)
    .await;
    assert!(duplicate.is_err());
    let profile = sqlx::query("SELECT dry_run_only, source_root FROM media_profile_get_v3($1)")
        .bind(profile_id)
        .fetch_one(&test_db.pool)
        .await?;
    assert!(!profile.get::<bool, _>("dry_run_only"));
    assert_eq!(
        profile.get::<String, _>("source_root"),
        path_text(roots.source.canonical_path())?
    );

    Ok(())
}

#[tokio::test]
async fn retained_job_history_is_hard_capped_and_filterable() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("retained_job_history_is_hard_capped_and_filterable").await?
    else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "history-profile", &roots).await?;
    let mut job_ids = Vec::new();
    for sequence in 0..106 {
        job_ids.push(create_job(&test_db.pool, profile_id, &roots, sequence).await?);
    }
    let second_roots = make_test_roots()?;
    let second_profile_id =
        create_profile(&test_db.pool, "history-profile-second", &second_roots).await?;
    for sequence in 0..4 {
        create_job(&test_db.pool, second_profile_id, &second_roots, sequence).await?;
    }
    let (claimed_job_id, _, claim_token) = claim_job(&test_db.pool).await?;
    assert_eq!(claimed_job_id, job_ids[0]);
    sqlx::query("SELECT media_job_worker_mark_status_v1($1,$2,$3::media_job_status,$4)")
        .bind(claimed_job_id)
        .bind(claim_token)
        .bind("failed")
        .bind("pagination status fixture")
        .execute(&test_db.pool)
        .await?;

    let global_page =
        sqlx::query("SELECT * FROM media_job_list_v1($1,$2::media_job_status,$3,$4,$5)")
            .bind(Option::<Uuid>::None)
            .bind(Option::<&str>::None)
            .bind(100_i32)
            .bind(Option::<DateTime<Utc>>::None)
            .bind(Option::<Uuid>::None)
            .fetch_all(&test_db.pool)
            .await?;
    assert_eq!(global_page.len(), 100);
    let failed_page =
        sqlx::query("SELECT * FROM media_job_list_v1($1,$2::media_job_status,$3,$4,$5)")
            .bind(profile_id)
            .bind("failed")
            .bind(100_i32)
            .bind(Option::<DateTime<Utc>>::None)
            .bind(Option::<Uuid>::None)
            .fetch_all(&test_db.pool)
            .await?;
    assert_eq!(failed_page.len(), 1);
    let second_profile_page =
        sqlx::query("SELECT * FROM media_job_list_v1($1,$2::media_job_status,$3,$4,$5)")
            .bind(second_profile_id)
            .bind(Option::<&str>::None)
            .bind(100_i32)
            .bind(Option::<DateTime<Utc>>::None)
            .bind(Option::<Uuid>::None)
            .fetch_all(&test_db.pool)
            .await?;
    assert_eq!(second_profile_page.len(), 4);

    Ok(())
}

#[tokio::test]
async fn retained_job_history_keyset_is_stable() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("retained_job_history_keyset_is_stable").await? else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "history-keyset-profile", &roots).await?;
    for sequence in 0..106 {
        create_job(&test_db.pool, profile_id, &roots, sequence).await?;
    }

    let first_page = sqlx::query(
        "SELECT media_job_public_id, queued_at \
         FROM media_job_list_v1($1,$2::media_job_status,$3,$4,$5)",
    )
    .bind(profile_id)
    .bind(Option::<&str>::None)
    .bind(100_i32)
    .bind(Option::<DateTime<Utc>>::None)
    .bind(Option::<Uuid>::None)
    .fetch_all(&test_db.pool)
    .await?;
    assert_eq!(first_page.len(), 100);
    let cursor = first_page
        .last()
        .ok_or_else(|| anyhow::anyhow!("first history page was empty"))?;
    let cursor_id = cursor.get::<Uuid, _>("media_job_public_id");
    let cursor_time = cursor.get::<DateTime<Utc>, _>("queued_at");

    let inserted_after_first_page = create_job(&test_db.pool, profile_id, &roots, 999).await?;
    let second_page = sqlx::query(
        "SELECT media_job_public_id, queued_at \
         FROM media_job_list_v1($1,$2::media_job_status,$3,$4,$5)",
    )
    .bind(profile_id)
    .bind(Option::<&str>::None)
    .bind(100_i32)
    .bind(cursor_time)
    .bind(cursor_id)
    .fetch_all(&test_db.pool)
    .await?;
    assert_eq!(second_page.len(), 6);
    assert!(
        second_page
            .iter()
            .all(|row| row.get::<Uuid, _>(0) != inserted_after_first_page)
    );

    let oversized =
        sqlx::query("SELECT * FROM media_job_list_v1($1,$2::media_job_status,$3,$4,$5)")
            .bind(profile_id)
            .bind(Option::<&str>::None)
            .bind(101_i32)
            .bind(Option::<DateTime<Utc>>::None)
            .bind(Option::<Uuid>::None)
            .fetch_all(&test_db.pool)
            .await;
    match oversized {
        Err(error) => assert_eq!(
            database_message(&error),
            Some("media job page size is outside bounds")
        ),
        Ok(_) => {
            return Err(anyhow::anyhow!(
                "oversized history page unexpectedly succeeded"
            ));
        }
    }

    Ok(())
}

async fn prepare_recovered_attempt(
    pool: &PgPool,
    roots: &TestRoots,
    profile_key: &str,
) -> anyhow::Result<(Uuid, i64, i64)> {
    let profile_id = create_profile(pool, profile_key, roots).await?;
    let job_id = create_job(pool, profile_id, roots, 0).await?;
    let (claimed_job_id, first_attempt, first_token) = claim_job(pool).await?;
    assert_eq!(claimed_job_id, job_id);
    assert_eq!(first_attempt, 1);
    sqlx::query("SELECT media_job_verification_check_append_v1($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(job_id)
        .bind(first_token)
        .bind(0_i32)
        .bind("runtime")
        .bind("failed")
        .bind(Option::<&str>::None)
        .bind(Option::<&str>::None)
        .bind("attempt one")
        .execute(pool)
        .await?;
    sqlx::query("SELECT media_job_worker_mark_status_v1($1,$2,$3::media_job_status,$4)")
        .bind(job_id)
        .bind(first_token)
        .bind("failed")
        .bind("first failure")
        .execute(pool)
        .await?;
    let second_attempt = sqlx::query_scalar::<_, i32>("SELECT media_job_retry_v1($1)")
        .bind(job_id)
        .fetch_one(pool)
        .await?;
    assert_eq!(second_attempt, 2);
    let (_, claimed_second_attempt, second_token) = claim_job(pool).await?;
    assert_eq!(claimed_second_attempt, 2);
    assert!(second_token > first_token);

    let recovered_at = Utc::now() + Duration::minutes(2);
    let recovered_attempt =
        sqlx::query_scalar::<_, i32>("SELECT media_job_worker_recover_stale_v1($1,$2,$3,$4)")
            .bind(job_id)
            .bind(second_token)
            .bind(recovered_at - Duration::minutes(1))
            .bind(recovered_at)
            .fetch_one(pool)
            .await?;
    assert_eq!(recovered_attempt, 3);
    let (_, claimed_recovered_attempt, recovered_token) = claim_job(pool).await?;
    assert_eq!(claimed_recovered_attempt, 3);
    assert!(recovered_token > second_token);
    Ok((job_id, second_token, recovered_token))
}

#[tokio::test]
async fn stale_workers_cannot_heartbeat_recovered_jobs() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("stale_workers_cannot_heartbeat_recovered_jobs").await? else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let (job_id, stale_token, active_token) =
        prepare_recovered_attempt(&test_db.pool, &roots, "stale-heartbeat-profile").await?;

    let stale_pool = test_db.pool.clone();
    let active_pool = test_db.pool.clone();
    let stale = async move {
        sqlx::query("SELECT media_job_worker_heartbeat_v1($1,$2)")
            .bind(job_id)
            .bind(stale_token)
            .execute(&stale_pool)
            .await
    };
    let active = async move {
        sqlx::query("SELECT media_job_worker_heartbeat_v1($1,$2)")
            .bind(job_id)
            .bind(active_token)
            .execute(&active_pool)
            .await
    };
    let (stale_result, active_result) = tokio::join!(stale, active);
    match stale_result {
        Err(error) => assert_eq!(database_message(&error), Some("stale worker claim")),
        Ok(_) => {
            return Err(anyhow::anyhow!(
                "stale worker heartbeat unexpectedly succeeded"
            ));
        }
    }
    active_result?;

    Ok(())
}

#[tokio::test]
async fn retry_preserves_attempt_evidence() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("retry_preserves_attempt_evidence").await? else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let (job_id, stale_token, active_token) =
        prepare_recovered_attempt(&test_db.pool, &roots, "attempt-evidence-profile").await?;

    let stale_evidence =
        sqlx::query("SELECT media_job_verification_check_append_v1($1,$2,$3,$4,$5,$6,$7,$8)")
            .bind(job_id)
            .bind(stale_token)
            .bind(1_i32)
            .bind("runtime")
            .bind("passed")
            .bind(Option::<&str>::None)
            .bind(Option::<&str>::None)
            .bind("stale")
            .execute(&test_db.pool)
            .await;
    assert!(stale_evidence.is_err());

    sqlx::query("SELECT media_job_verification_check_append_v1($1,$2,$3,$4,$5,$6,$7,$8)")
        .bind(job_id)
        .bind(active_token)
        .bind(0_i32)
        .bind("runtime")
        .bind("passed")
        .bind(Option::<&str>::None)
        .bind(Option::<&str>::None)
        .bind("attempt two")
        .execute(&test_db.pool)
        .await?;
    let cancelled = sqlx::query_scalar::<_, bool>("SELECT media_job_worker_complete_v1($1,$2,$3)")
        .bind(job_id)
        .bind(active_token)
        .bind(0_i64)
        .fetch_one(&test_db.pool)
        .await?;
    assert!(!cancelled);

    let evidence = sqlx::query(
        "SELECT attempt_number,is_current,check_status \
         FROM media_job_verification_check_list_v1($1)",
    )
    .bind(job_id)
    .fetch_all(&test_db.pool)
    .await?;
    assert_eq!(evidence.len(), 2);
    assert_eq!(evidence[0].get::<i32, _>("attempt_number"), 3);
    assert!(evidence[0].get::<bool, _>("is_current"));
    assert_eq!(evidence[0].get::<String, _>("check_status"), "passed");
    assert_eq!(evidence[1].get::<i32, _>("attempt_number"), 1);
    assert!(!evidence[1].get::<bool, _>("is_current"));
    assert_eq!(evidence[1].get::<String, _>("check_status"), "failed");

    let attempts = sqlx::query(
        "SELECT attempt_number,is_current,status::text AS status \
         FROM media_job_attempt_list_v1($1)",
    )
    .bind(job_id)
    .fetch_all(&test_db.pool)
    .await?;
    assert_eq!(attempts.len(), 3);
    assert_eq!(attempts[0].get::<i32, _>("attempt_number"), 3);
    assert!(attempts[0].get::<bool, _>("is_current"));
    assert_eq!(attempts[1].get::<i32, _>("attempt_number"), 2);
    assert_eq!(attempts[1].get::<String, _>("status"), "failed");

    Ok(())
}

#[tokio::test]
async fn diagnostic_pruning_is_one_shot_and_bounded() -> anyhow::Result<()> {
    let Some(test_db) = setup_db("diagnostic_pruning_is_one_shot_and_bounded").await? else {
        return Ok(());
    };
    let roots = make_test_roots()?;
    let profile_id = create_profile(&test_db.pool, "pruning-profile", &roots).await?;
    for sequence in 0..205 {
        let job_id = create_job(&test_db.pool, profile_id, &roots, sequence).await?;
        let (claimed_job_id, _, token) = claim_job(&test_db.pool).await?;
        assert_eq!(claimed_job_id, job_id);
        sqlx::query("SELECT media_job_phase_append_v1($1,$2,$3,$4,$5,$6)")
            .bind(job_id)
            .bind(token)
            .bind(0_i32)
            .bind("execute")
            .bind("running")
            .bind("bounded diagnostic")
            .execute(&test_db.pool)
            .await?;
        sqlx::query("SELECT media_job_worker_mark_status_v1($1,$2,$3::media_job_status,$4)")
            .bind(job_id)
            .bind(token)
            .bind("failed")
            .bind("retained diagnostic")
            .execute(&test_db.pool)
            .await?;
    }

    let as_of = Utc::now() + Duration::days(31);
    let mut batches = Vec::new();
    for _ in 0..4 {
        let row = sqlx::query(
            "SELECT completed_jobs_deleted, failed_jobs_pruned, failed_detail_rows_deleted \
             FROM media_job_retention_run_v1($1)",
        )
        .bind(as_of)
        .fetch_one(&test_db.pool)
        .await?;
        batches.push((
            row.get::<i32, _>("failed_jobs_pruned"),
            row.get::<i32, _>("failed_detail_rows_deleted"),
        ));
    }
    assert_eq!(batches, vec![(100, 100), (100, 100), (5, 5), (0, 0)]);

    Ok(())
}
