use std::{
    path::{Path, PathBuf},
    time::Duration as StdDuration,
};

use super::{
    MediaRootIdentity, MediaRootIdentityError, MediaRootIdentityResolver,
    StdMediaRootIdentityResolver,
};
use crate::config::verify_database;
use crate::media::association_jobs::{
    AssociationFingerprint, AssociationJobInput, enqueue_association_job,
};
use crate::media::jobs::{get_media_job, media_job_worker_claim_next};
use crate::media::step_checkpoints::{StepCheckpoint, get_step_checkpoint, write_step_checkpoint};
use chrono::{DateTime, Duration, Utc};
use revaer_test_support::postgres::{TestDatabase, start_postgres};
use sqlx::postgres::{PgDatabaseError, PgPoolOptions};
use sqlx::{PgPool, Row};
use uuid::Uuid;

mod native_recovery;
mod rescan;
mod rescan_absence;

const ACTOR_ID: &str = "00000000-0000-0000-0000-000000000000";

struct TestDb {
    database: TestDatabase,
    pool: PgPool,
}

pub(super) struct TestRoots {
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

impl TestDb {
    async fn close(self) -> anyhow::Result<()> {
        self.pool.close().await;
        self.database.close()
    }
}

async fn setup_db() -> anyhow::Result<TestDb> {
    let mut database = start_postgres()?;
    // These database-layer tests deliberately mutate rows to exercise triggers.
    // Retain the owned administrative endpoint for those assertions, while
    // independently verifying the sealed baseline through the restricted role.
    let admin_url = database.connection_string().to_owned();
    database
        .initialize_runtime(include_str!("../../init.sql"))
        .await?;
    let runtime = PgPoolOptions::new()
        .max_connections(1)
        .connect(database.connection_string())
        .await?;
    let verified = verify_database(&runtime).await;
    runtime.close().await;
    verified?;
    let pool = PgPoolOptions::new()
        .max_connections(8)
        .connect(&admin_url)
        .await?;
    Ok(TestDb { database, pool })
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

pub(super) async fn native_job(
    database: &TestDatabase,
    pool: &PgPool,
    key: &str,
    dry_run: bool,
) -> anyhow::Result<(TestRoots, Uuid)> {
    let roots = make_test_roots()?;
    let job = native_recovery::enqueue(database, pool, &roots, key, dry_run, "").await?;
    Ok((roots, job))
}

pub(super) async fn native_configured_job(
    database: &TestDatabase,
    pool: &PgPool,
    input: &crate::media::profile_versions::CreateProfileVersionInput<'_>,
) -> anyhow::Result<(TestRoots, Uuid, Uuid)> {
    let roots = make_test_roots()?;
    native_recovery::initialize_catalog(database, pool, &roots, input.dry_run_only).await?;
    let association =
        native_recovery::create_profile_association_for_input(pool, input, "").await?;
    let job = additional_native_job(pool, input.profile_key, "source.mkv", true).await?;
    Ok((roots, association.media_profile_public_id, job))
}

pub(super) async fn additional_native_job(
    pool: &PgPool,
    key: &str,
    relative_path: &str,
    dry_run: bool,
) -> anyhow::Result<Uuid> {
    let association = native_association(pool, key).await?;
    let relative_path = if association.root_relative_path.is_empty() {
        relative_path.to_string()
    } else {
        format!("{}/{relative_path}", association.root_relative_path)
    };
    let sha256 = "11".repeat(32);
    native_recovery::enqueue_candidate(
        pool,
        &association,
        &relative_path,
        dry_run,
        crate::media::association_jobs::AssociationFingerprint {
            identity: "0000000000000001:0000000000000001",
            size_bytes: 6,
            modified_ns: 1,
            changed_ns: 1,
            sha256: &sha256,
        },
    )
    .await
}

async fn native_association(
    pool: &PgPool,
    key: &str,
) -> anyhow::Result<crate::media::associations::AssociationRow> {
    crate::media::associations::read_association_page(pool, 100, None, None)
        .await?
        .into_iter()
        .find(|row| row.association_key == key)
        .ok_or_else(|| anyhow::anyhow!("native association missing"))
}

pub(super) async fn native_job_fingerprint(
    database: &TestDatabase,
    pool: &PgPool,
    key: &str,
    dry_run: bool,
    fingerprint: crate::media::association_jobs::AssociationFingerprint<'_>,
) -> anyhow::Result<(TestRoots, Uuid)> {
    let roots = make_test_roots()?;
    let job = native_recovery::enqueue_with_fingerprint(
        database,
        pool,
        &roots,
        key,
        dry_run,
        fingerprint,
        "",
    )
    .await?;
    Ok((roots, job))
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

fn database_message(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .map(sqlx::error::DatabaseError::message)
}

fn database_detail(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|database_error| database_error.try_downcast_ref::<PgDatabaseError>())
        .and_then(PgDatabaseError::detail)
}

fn assert_configuration_immutable<T>(result: Result<T, sqlx::Error>) -> anyhow::Result<()> {
    let Err(error) = result else {
        return Err(anyhow::anyhow!(
            "media job configuration mutation unexpectedly succeeded"
        ));
    };
    assert_eq!(
        database_detail(&error),
        Some("media_job_configuration_immutable")
    );
    Ok(())
}

struct DryRunAdmissionCase<'a> {
    association: &'a crate::media::associations::AssociationRow,
    generation: i64,
    source_name: &'a str,
    requested_dry_run: bool,
    expected_dry_run: bool,
    source_identity: &'a str,
    fingerprint_version: i64,
    sha256_seed: &'a str,
}

async fn assert_dry_run_admission(
    pool: &PgPool,
    case: DryRunAdmissionCase<'_>,
) -> anyhow::Result<()> {
    let source_path = format!(
        "{}/{}",
        case.association.root_relative_path, case.source_name
    );
    let source_sha256 = case.sha256_seed.repeat(64);
    let enqueued = enqueue_association_job(
        pool,
        &AssociationJobInput {
            actor_public_id: actor_id()?,
            association_public_id: case.association.media_discovery_association_public_id,
            association_version: case.association.latest_version,
            relative_path: &source_path,
            dry_run: case.requested_dry_run,
            trigger: "manual",
            generation: case.generation,
            generation_sha256: [0x33; 32],
            fingerprint: AssociationFingerprint {
                identity: case.source_identity,
                size_bytes: case.fingerprint_version,
                modified_ns: case.fingerprint_version + 10,
                changed_ns: case.fingerprint_version + 20,
                sha256: &source_sha256,
            },
        },
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("{} was not enqueued", case.source_name))?;
    assert_eq!(enqueued.dry_run, case.expected_dry_run);

    let persisted = get_media_job(pool, enqueued.media_job_public_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("{} was not persisted", case.source_name))?;
    assert_eq!(persisted.dry_run, case.expected_dry_run);
    Ok(())
}

async fn claim_job(pool: &PgPool) -> anyhow::Result<(Uuid, i32, i64)> {
    let row = sqlx::query(
        "SELECT media_job_public_id, attempt_number, claim_generation \
         FROM media_job_worker_claim_next_v4()",
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
    let test_db = setup_db().await?;
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

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn discovery_revalidates_root_identity() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
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

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn discovery_rejects_ambiguous_associations_and_unbounded_policy() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
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

    test_db.close().await?;
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
    let test_db = setup_db().await?;
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

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn job_snapshots_and_selected_policy_are_immutable() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
    let roots = make_test_roots()?;
    let key = "snapshot-profile";
    native_recovery::initialize_catalog(&test_db.database, &test_db.pool, &roots, true).await?;
    let profile_id = native_recovery::create_profile_association(&test_db.pool, key, true, "", 1)
        .await?
        .media_profile_public_id;
    append_profile_file_rules(&test_db.pool, profile_id).await?;
    let job_id = additional_native_job(&test_db.pool, key, "source.mkv", true).await?;
    let snapshot_roots = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM media_job_root_snapshot snapshot \
         JOIN media_job job USING (media_job_id) WHERE job.media_job_public_id=$1",
    )
    .bind(job_id)
    .fetch_one(&test_db.pool)
    .await?;
    assert_eq!(snapshot_roots, 5);
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
    .bind("recovery-policy")
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

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn admitted_dry_run_mode_follows_complete_or_truth_table() -> anyhow::Result<()> {
    let test_db = setup_db().await?;

    let roots = make_test_roots()?;
    native_recovery::initialize_catalog(&test_db.database, &test_db.pool, &roots, false).await?;
    native_recovery::create_policy(&test_db.pool, true, 2).await?;
    let generation = crate::media::root_catalog::read_root_catalog_readiness(&test_db.pool)
        .await?
        .first()
        .and_then(|row| row.attestation_generation)
        .ok_or_else(|| anyhow::anyhow!("native generation missing"))?;
    for (policy_version, policy_dry_run) in [(1, false), (2, true)] {
        for profile_dry_run in [false, true] {
            let key = format!("truth-{policy_version}-{profile_dry_run}");
            let association = native_recovery::create_profile_association(
                &test_db.pool,
                &key,
                profile_dry_run,
                &key,
                policy_version,
            )
            .await?;
            for requested_dry_run in [false, true] {
                let source_name = format!("request-{requested_dry_run}.mkv");
                assert_dry_run_admission(
                    &test_db.pool,
                    DryRunAdmissionCase {
                        association: &association,
                        generation,
                        source_name: &source_name,
                        requested_dry_run,
                        expected_dry_run: requested_dry_run || profile_dry_run || policy_dry_run,
                        source_identity: "0000000000000009:0000000000000019",
                        fingerprint_version: 9,
                        sha256_seed: "9",
                    },
                )
                .await?;
            }
        }
    }

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn enqueue_rejects_profile_head_changed_under_row_lock() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
    let roots = make_test_roots()?;
    native_recovery::initialize_catalog(&test_db.database, &test_db.pool, &roots, false).await?;
    let association = native_recovery::create_profile_association(
        &test_db.pool,
        "profile-mode-lock",
        false,
        "",
        1,
    )
    .await?;
    let generation = crate::media::root_catalog::read_root_catalog_readiness(&test_db.pool)
        .await?
        .first()
        .and_then(|row| row.attestation_generation)
        .ok_or_else(|| anyhow::anyhow!("native generation missing"))?;
    let mut profile_update = test_db
        .pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await?;
    sqlx::query("SELECT media_profile_version_replace_v1($1,'profile-mode-lock','Native recovery fixture','Mode changed under lock',true,true,'recovery-target',1,'recovery-policy',1,'worker-source','worker-workspace',NULL,NULL,$2,1)")
        .bind(actor_id()?).bind(association.media_profile_public_id)
        .execute(&mut *profile_update).await?;
    let pool = test_db.pool.clone();
    let actor_public_id = actor_id()?;
    let mut enqueue_task = tokio::spawn(async move {
        let source_sha256 = "c".repeat(64);
        enqueue_association_job(
            &pool,
            &AssociationJobInput {
                actor_public_id,
                association_public_id: association.media_discovery_association_public_id,
                association_version: association.latest_version,
                relative_path: "lock-race.mkv",
                dry_run: false,
                trigger: "manual",
                generation,
                generation_sha256: [0x33; 32],
                fingerprint: AssociationFingerprint {
                    identity: "000000000000000c:000000000000001c",
                    size_bytes: 12,
                    modified_ns: 22,
                    changed_ns: 32,
                    sha256: &source_sha256,
                },
            },
        )
        .await
    });
    let early_result = tokio::time::timeout(StdDuration::from_millis(200), &mut enqueue_task).await;
    profile_update.commit().await?;
    let outcome = match early_result {
        Ok(join_result) => {
            let completed = join_result?;
            return Err(anyhow::anyhow!(
                "enqueue completed before the profile-row lock was released: {completed:?}"
            ));
        }
        Err(_) => enqueue_task.await?,
    };
    match outcome {
        Err(error) => assert_eq!(
            error.database_detail(),
            Some("media_root_binding_incomplete")
        ),
        Ok(_) => {
            return Err(anyhow::anyhow!(
                "stale association admitted changed profile mode"
            ));
        }
    }
    let jobs =
        sqlx::query("SELECT * FROM media_job_list_v1(NULL,NULL::media_job_status,100,NULL,NULL)")
            .fetch_all(&test_db.pool)
            .await?;
    assert!(jobs.is_empty());
    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn source_fingerprint_intent_is_immutable_and_claim_returns_original() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
    let roots = make_test_roots()?;
    let source_identity = "000000000000000d:000000000000001d";
    let source_size_bytes = 4_096_i64;
    let source_modified_ns = 5_000_i64;
    let source_changed_ns = 6_000_i64;
    let source_sha256 = "d".repeat(64);
    let job_id = native_recovery::enqueue_with_fingerprint(
        &test_db.database,
        &test_db.pool,
        &roots,
        "immutable-fingerprint",
        true,
        crate::media::association_jobs::AssociationFingerprint {
            identity: source_identity,
            size_bytes: source_size_bytes,
            modified_ns: source_modified_ns,
            changed_ns: source_changed_ns,
            sha256: &source_sha256,
        },
        "",
    )
    .await?;

    assert_configuration_immutable(
        sqlx::query("UPDATE media_job SET intent_source_identity=$2 WHERE media_job_public_id=$1")
            .bind(job_id)
            .bind("000000000000000e:000000000000001e")
            .execute(&test_db.pool)
            .await,
    )?;
    assert_configuration_immutable(
        sqlx::query(
            "UPDATE media_job SET intent_source_size_bytes=$2 WHERE media_job_public_id=$1",
        )
        .bind(job_id)
        .bind(source_size_bytes + 1)
        .execute(&test_db.pool)
        .await,
    )?;
    assert_configuration_immutable(
        sqlx::query(
            "UPDATE media_job SET intent_source_modified_ns=$2 WHERE media_job_public_id=$1",
        )
        .bind(job_id)
        .bind(source_modified_ns + 1)
        .execute(&test_db.pool)
        .await,
    )?;
    assert_configuration_immutable(
        sqlx::query(
            "UPDATE media_job SET intent_source_changed_ns=$2 WHERE media_job_public_id=$1",
        )
        .bind(job_id)
        .bind(source_changed_ns + 1)
        .execute(&test_db.pool)
        .await,
    )?;
    let replacement_sha256 = "e".repeat(64);
    assert_configuration_immutable(
        sqlx::query("UPDATE media_job SET intent_source_sha256=$2 WHERE media_job_public_id=$1")
            .bind(job_id)
            .bind(&replacement_sha256)
            .execute(&test_db.pool)
            .await,
    )?;
    assert_configuration_immutable(
        sqlx::query(
            "UPDATE media_job SET intent_source_identity=$2, intent_source_size_bytes=$3, \
             intent_source_modified_ns=$4, intent_source_changed_ns=$5, intent_source_sha256=$6 \
             WHERE media_job_public_id=$1",
        )
        .bind(job_id)
        .bind("000000000000000f:000000000000001f")
        .bind(source_size_bytes + 2)
        .bind(source_modified_ns + 2)
        .bind(source_changed_ns + 2)
        .bind("f".repeat(64))
        .execute(&test_db.pool)
        .await,
    )?;

    let claimed = media_job_worker_claim_next(&test_db.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("immutable fingerprint job was not claimable"))?;
    assert_eq!(claimed.media_job_public_id, job_id);
    assert_eq!(claimed.source_identity, source_identity);
    assert_eq!(claimed.source_size_bytes, source_size_bytes);
    assert_eq!(claimed.source_modified_ns, source_modified_ns);
    assert_eq!(claimed.source_changed_ns, source_changed_ns);
    assert_eq!(claimed.source_sha256, source_sha256);

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn profile_create_is_insert_only_and_preserves_live_state() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
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

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn retained_job_history_is_hard_capped_and_filterable() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
    let key = "history-profile";
    let roots = make_test_roots()?;
    let first_job =
        native_recovery::enqueue(&test_db.database, &test_db.pool, &roots, key, true, "first")
            .await?;
    let profile_id = native_association(&test_db.pool, key)
        .await?
        .media_profile_public_id;
    let mut job_ids = vec![first_job];
    for sequence in 1..106 {
        job_ids.push(
            additional_native_job(&test_db.pool, key, &format!("source-{sequence}.mkv"), true)
                .await?,
        );
    }
    let second_key = "history-profile-second";
    let second_profile_id =
        native_recovery::create_profile_association(&test_db.pool, second_key, true, "second", 1)
            .await?
            .media_profile_public_id;
    for sequence in 0..4 {
        additional_native_job(
            &test_db.pool,
            second_key,
            &format!("source-{sequence}.mkv"),
            true,
        )
        .await?;
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

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn retained_job_history_keyset_is_stable() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
    let key = "history-keyset-profile";
    let (_roots, _first_job) = native_job(&test_db.database, &test_db.pool, key, true).await?;
    let profile_id = native_association(&test_db.pool, key)
        .await?
        .media_profile_public_id;
    for sequence in 1..106 {
        additional_native_job(&test_db.pool, key, &format!("source-{sequence}.mkv"), true).await?;
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

    let inserted_after_first_page =
        additional_native_job(&test_db.pool, key, "source-999.mkv", true).await?;
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

    test_db.close().await?;
    Ok(())
}

async fn prepare_resumed_attempt(
    test_db: &TestDb,
    roots: &TestRoots,
    profile_key: &str,
) -> anyhow::Result<(Uuid, i64, i64)> {
    let pool = &test_db.pool;
    let job_id =
        native_recovery::enqueue(&test_db.database, pool, roots, profile_key, true, "").await?;
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

    let checkpoint = StepCheckpoint {
        step_signature: vec![0x55; 32],
        output_path: path_text(&roots.output.canonical_path().join("checkpoint.mkv"))?.to_string(),
        size_bytes: 6,
        output_sha256: vec![0x44; 32],
    };
    // Database evidence is synthetic here; filesystem validation is covered by
    // the application replay tests using actual synchronized writer outputs.
    write_step_checkpoint(pool, job_id, second_token, 0, &checkpoint).await?;

    let cancelled = sqlx::query_scalar::<_, bool>("SELECT media_job_worker_interrupt_v1($1,$2)")
        .bind(job_id)
        .bind(second_token)
        .fetch_one(pool)
        .await?;
    assert!(!cancelled);
    let (_, resumed_attempt, resumed_token) = claim_job(pool).await?;
    assert_eq!(resumed_attempt, second_attempt);
    assert_eq!(resumed_token, second_token);
    assert_eq!(
        get_step_checkpoint(pool, job_id, resumed_token, 0).await?,
        Some(checkpoint)
    );
    Ok((job_id, first_token, resumed_token))
}

#[tokio::test]
async fn prior_failed_attempt_cannot_heartbeat_resumed_job() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
    let roots = make_test_roots()?;
    let (job_id, stale_token, active_token) =
        prepare_resumed_attempt(&test_db, &roots, "stale-heartbeat-profile").await?;

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

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn retry_preserves_attempt_evidence() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
    let roots = make_test_roots()?;
    let (job_id, stale_token, active_token) =
        prepare_resumed_attempt(&test_db, &roots, "attempt-evidence-profile").await?;

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

    let checkpoint = get_step_checkpoint(&test_db.pool, job_id, active_token, 0)
        .await?
        .ok_or_else(|| anyhow::anyhow!("resumed checkpoint missing"))?;
    let read_error = get_step_checkpoint(&test_db.pool, job_id, stale_token, 0)
        .await
        .err()
        .ok_or_else(|| anyhow::anyhow!("failed attempt read current checkpoint"))?;
    assert_eq!(
        read_error.database_detail(),
        Some("media_job_worker_claim_stale")
    );
    let write_error = write_step_checkpoint(&test_db.pool, job_id, stale_token, 0, &checkpoint)
        .await
        .err()
        .ok_or_else(|| anyhow::anyhow!("failed attempt wrote current checkpoint"))?;
    assert_eq!(
        write_error.database_detail(),
        Some("media_job_worker_claim_stale")
    );

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
    assert_eq!(evidence[0].get::<i32, _>("attempt_number"), 2);
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
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].get::<i32, _>("attempt_number"), 2);
    assert!(attempts[0].get::<bool, _>("is_current"));
    assert_eq!(attempts[1].get::<i32, _>("attempt_number"), 1);
    assert_eq!(attempts[1].get::<String, _>("status"), "failed");

    test_db.close().await?;
    Ok(())
}

#[tokio::test]
async fn diagnostic_pruning_is_one_shot_and_bounded() -> anyhow::Result<()> {
    let test_db = setup_db().await?;
    let key = "pruning-profile";
    let (_roots, first_job) = native_job(&test_db.database, &test_db.pool, key, true).await?;
    assert_eq!(
        crate::media::jobs::list_media_job_desired_target_streams(&test_db.pool, first_job)
            .await?
            .len(),
        1
    );
    for sequence in 0..205 {
        let job_id = if sequence == 0 {
            first_job
        } else {
            additional_native_job(&test_db.pool, key, &format!("source-{sequence}.mkv"), true)
                .await?
        };
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
    // Each native job has one phase and one desired-target stream to prune.
    assert_eq!(batches, vec![(100, 200), (100, 200), (5, 10), (0, 0)]);

    test_db.close().await?;
    Ok(())
}
