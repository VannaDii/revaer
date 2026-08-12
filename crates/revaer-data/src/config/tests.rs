use super::*;
use revaer_test_support::postgres::{TestDatabase, start_postgres};
use sqlx::postgres::PgPoolOptions;

#[test]
fn schema_baseline_revision_matches_initializer_marker() {
    let schema = include_str!("../../init.sql");
    let marker = format!("CHECK (baseline = '{SCHEMA_BASELINE}')");

    assert!(schema.contains(&marker));
    assert_eq!(schema_source_digest().len(), 64);
}

#[test]
fn factory_reset_is_limited_to_baseline_registered_tables() {
    let schema = include_str!("../../init.sql");
    let reset_start = schema
        .find("CREATE FUNCTION revaer_config.factory_reset_without_media_defaults_v1()")
        .expect("factory reset routine must exist");
    let reset_body = &schema[reset_start..];
    let reset_end = reset_body
        .find("ON CONFLICT (id) DO UPDATE")
        .expect("factory reset seed boundary must exist");
    let table_selection = &reset_body[..reset_end];

    assert!(table_selection.contains("FROM public.revaer_schema_table"));
    assert!(!table_selection.contains("FROM pg_tables"));
    assert!(!table_selection.contains("CASCADE"));
}

#[test]
fn schema_catalog_digest_uses_stable_cast_function_identity() {
    assert!(SCHEMA_CATALOG_DIGEST_QUERY.contains("pg_get_function_identity_arguments"));
    assert!(
        !SCHEMA_CATALOG_DIGEST_QUERY
            .contains("cast_value.castmethod::text || '|' || cast_value.castfunc",)
    );
}

#[test]
fn schema_catalog_digest_covers_trigger_state_and_composites() {
    assert!(SCHEMA_CATALOG_DIGEST_QUERY.contains("trigger_value.tgenabled"));
    assert!(SCHEMA_CATALOG_DIGEST_QUERY.contains("composite_relation.relkind = 'c'"));
    assert!(SCHEMA_CATALOG_DIGEST_QUERY.contains("attribute.attnotnull"));
}

async fn fresh_test_pool() -> anyhow::Result<(TestDatabase, PgPool)> {
    let postgres = start_postgres()?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(postgres.connection_string())
        .await?;
    Ok((postgres, pool))
}

#[tokio::test]
async fn concurrent_first_initialization_serializes_and_validates() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;

    let (first, second) = tokio::join!(initialize_schema(&pool), initialize_schema(&pool));
    first?;
    second?;
    initialize_schema(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn initializer_rejects_marker_source_tampering() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query("UPDATE public.revaer_schema_state SET source_digest = repeat('0', 64)")
        .execute(&pool)
        .await?;

    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaSourceMismatch)
    ));
    Ok(())
}

#[tokio::test]
async fn initializer_rejects_catalog_corruption() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query("DROP INDEX public.ix_media_job_profile_status")
        .execute(&pool)
        .await?;

    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));
    Ok(())
}

#[tokio::test]
async fn initializer_ignores_and_factory_reset_preserves_operator_tables() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query("CREATE TABLE public.operator_state (value text PRIMARY KEY)")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO public.operator_state (value) VALUES ('preserve')")
        .execute(&pool)
        .await?;

    initialize_schema(&pool).await?;
    super::factory_reset(&pool).await?;
    let value: String = sqlx::query_scalar("SELECT value FROM public.operator_state")
        .fetch_one(&pool)
        .await?;
    assert_eq!(value, "preserve");
    Ok(())
}

#[tokio::test]
async fn factory_reset_rejects_operator_foreign_keys_without_erasing_data() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query(
        "CREATE TABLE public.operator_profile_state (\
             value text PRIMARY KEY, \
             app_profile_id uuid NOT NULL REFERENCES public.app_profile(id)\
         )",
    )
    .execute(&pool)
    .await?;
    sqlx::query(
        "INSERT INTO public.operator_profile_state (value, app_profile_id) \
         VALUES ('preserve', '00000000-0000-0000-0000-000000000001')",
    )
    .execute(&pool)
    .await?;

    assert!(super::factory_reset(&pool).await.is_err());
    let value: String = sqlx::query_scalar("SELECT value FROM public.operator_profile_state")
        .fetch_one(&pool)
        .await?;
    assert_eq!(value, "preserve");
    let profile_count: i64 = sqlx::query_scalar("SELECT count(*) FROM public.app_profile")
        .fetch_one(&pool)
        .await?;
    assert_eq!(profile_count, 1);
    Ok(())
}

#[tokio::test]
async fn factory_reset_rejects_registry_tampering_before_truncation() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query("DELETE FROM public.revaer_schema_table WHERE table_name = 'app_profile'")
        .execute(&pool)
        .await?;

    assert!(super::factory_reset(&pool).await.is_err());
    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));
    Ok(())
}

#[tokio::test]
async fn initializer_rejects_sequence_and_cast_corruption() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query("ALTER SEQUENCE public.engine_tracker_endpoints_id_seq INCREMENT BY 2")
        .execute(&pool)
        .await?;
    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));

    sqlx::query("ALTER SEQUENCE public.engine_tracker_endpoints_id_seq INCREMENT BY 1")
        .execute(&pool)
        .await?;
    initialize_schema(&pool).await?;
    sqlx::query("DROP CAST (public.policy_action AS public.decision_type)")
        .execute(&pool)
        .await?;
    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));
    Ok(())
}

#[tokio::test]
async fn initializer_rejects_disabled_triggers_and_composite_corruption() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query(
        "ALTER TABLE public.media_job DISABLE TRIGGER media_job_configuration_immutable_trigger",
    )
    .execute(&pool)
    .await?;
    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));

    sqlx::query(
        "ALTER TABLE public.media_job ENABLE TRIGGER media_job_configuration_immutable_trigger",
    )
    .execute(&pool)
    .await?;
    initialize_schema(&pool).await?;
    sqlx::query(
        "ALTER TYPE public.policy_rule_value_item RENAME ATTRIBUTE value_text TO value_label",
    )
    .execute(&pool)
    .await?;
    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));
    Ok(())
}

#[tokio::test]
async fn initializer_rejects_relation_durability_and_column_collation_drift() -> anyhow::Result<()>
{
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query("ALTER TABLE public.settings_secret SET UNLOGGED")
        .execute(&pool)
        .await?;
    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));

    sqlx::query("ALTER TABLE public.settings_secret SET LOGGED")
        .execute(&pool)
        .await?;
    initialize_schema(&pool).await?;
    sqlx::query(
        "ALTER TABLE public.app_profile ALTER COLUMN instance_name TYPE text COLLATE \"C\"",
    )
    .execute(&pool)
    .await?;
    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));
    Ok(())
}

#[tokio::test]
async fn initializer_rejects_markerless_partial_schema() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    sqlx::query("CREATE TABLE public.partial_schema_probe (id bigint PRIMARY KEY)")
        .execute(&pool)
        .await?;

    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaBaselineMissing)
    ));
    Ok(())
}

#[tokio::test]
async fn initializer_rejects_markerless_public_routine() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    sqlx::query(
        "CREATE FUNCTION public.partial_schema_probe() RETURNS bigint LANGUAGE sql AS 'SELECT 1'",
    )
    .execute(&pool)
    .await?;

    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaBaselineMissing)
    ));
    Ok(())
}

#[tokio::test]
async fn initializer_rejects_authorization_catalog_drift() -> anyhow::Result<()> {
    let (_database, pool) = fresh_test_pool().await?;
    initialize_schema(&pool).await?;
    sqlx::query("REVOKE EXECUTE ON FUNCTION public.media_job_status_queued_v1() FROM PUBLIC")
        .execute(&pool)
        .await?;

    assert!(matches!(
        initialize_schema(&pool).await,
        Err(DataError::SchemaCatalogMismatch)
    ));
    Ok(())
}

#[test]
fn fs_column_mappings_are_stable() {
    assert_eq!(FsStringField::LibraryRoot.column(), "library_root");
    assert_eq!(FsStringField::Par2.column(), "par2");
    assert_eq!(FsStringField::MoveMode.column(), "move_mode");
    assert_eq!(FsBooleanField::Extract.column(), "extract");
    assert_eq!(FsBooleanField::Flatten.column(), "flatten");
    assert_eq!(FsArrayField::CleanupKeep.column(), "cleanup_keep");
    assert_eq!(FsArrayField::CleanupDrop.column(), "cleanup_drop");
    assert_eq!(FsArrayField::AllowPaths.column(), "allow_paths");
    assert_eq!(FsOptionalStringField::ChmodFile.column(), "chmod_file");
    assert_eq!(FsOptionalStringField::ChmodDir.column(), "chmod_dir");
    assert_eq!(FsOptionalStringField::Owner.column(), "owner");
    assert_eq!(FsOptionalStringField::Group.column(), "group");
    assert_eq!(FsOptionalStringField::Umask.column(), "umask");
}

#[test]
fn queue_policy_set_flags_round_trip() {
    let set = QueuePolicySet::from_flags([true, false, true]);
    assert!(set.auto_managed());
    assert!(!set.prefer_seeds());
    assert!(set.dont_count_slow());
}

#[test]
fn seeding_toggle_set_flags_round_trip() {
    let set = SeedingToggleSet::from_flags([true, true, false]);
    assert!(set.sequential_default());
    assert!(set.super_seeding());
    assert!(!set.strict_super_seeding());
}

#[test]
fn storage_toggle_set_flags_round_trip() {
    let set = StorageToggleSet::from_flags([true, false, true, false]);
    assert!(set.use_partfile());
    assert!(!set.coalesce_reads());
    assert!(set.coalesce_writes());
    assert!(!set.use_disk_cache_pool());
}

#[test]
fn nat_toggle_set_flags_round_trip() {
    let set = NatToggleSet::from_flags([true, false, true, false]);
    assert!(set.lsd());
    assert!(!set.upnp());
    assert!(set.natpmp());
    assert!(!set.pex());
}

#[test]
fn privacy_toggle_set_flags_round_trip() {
    let set = PrivacyToggleSet::from_flags([true, true, false, false, true, false]);
    assert!(set.anonymous_mode());
    assert!(set.force_proxy());
    assert!(!set.prefer_rc4());
    assert!(!set.allow_multiple_connections_per_ip());
    assert!(set.enable_outgoing_utp());
    assert!(!set.enable_incoming_utp());
}
