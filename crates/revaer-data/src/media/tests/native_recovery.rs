//! Native admission for database recovery tests; catalog proof is synthetic.

use super::{TestRoots, actor_id, path_text};
use crate::media::association_jobs::{
    AssociationFingerprint, AssociationJobInput, enqueue_association_job,
};
use crate::media::associations::{CreateAssociationInput, create_association};
use crate::media::configuration::{
    AppendMediaDesiredTargetStreamInput, CreateMediaDesiredTargetInput, MediaPolicyOutputRow,
    UpsertMediaPolicyProfileInput, append_media_desired_target_stream, create_media_desired_target,
    upsert_media_policy_profile,
};
use crate::media::profile_versions::{CreateProfileVersionInput, create_profile_version};
use revaer_test_support::postgres::TestDatabase;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn enqueue(
    database: &TestDatabase,
    pool: &PgPool,
    roots: &TestRoots,
    key: &str,
    dry_run: bool,
    prefix: &str,
) -> anyhow::Result<Uuid> {
    let sha256 = "11".repeat(32);
    enqueue_with_fingerprint(
        database,
        pool,
        roots,
        key,
        dry_run,
        AssociationFingerprint {
            identity: "0000000000000001:0000000000000001",
            size_bytes: 6,
            modified_ns: 1,
            changed_ns: 1,
            sha256: &sha256,
        },
        prefix,
    )
    .await
}

pub(super) async fn enqueue_with_fingerprint(
    database: &TestDatabase,
    pool: &PgPool,
    roots: &TestRoots,
    key: &str,
    dry_run: bool,
    fingerprint: AssociationFingerprint<'_>,
    prefix: &str,
) -> anyhow::Result<Uuid> {
    initialize_catalog(database, pool, roots, dry_run).await?;
    let association = create_profile_association(pool, key, dry_run, prefix, 1).await?;
    let path = if prefix.is_empty() {
        "source.mkv".to_string()
    } else {
        format!("{prefix}/source.mkv")
    };
    enqueue_candidate(pool, &association, &path, dry_run, fingerprint).await
}

pub(crate) async fn initialize_catalog(
    database: &TestDatabase,
    pool: &PgPool,
    roots: &TestRoots,
    dry_run: bool,
) -> anyhow::Result<()> {
    let source_device = format!("{:016x}", roots.source.filesystem_device());
    let source_inode = format!("{:016x}", roots.source.filesystem_inode());
    let workspace_device = format!("{:016x}", roots.output.filesystem_device());
    let workspace_inode = format!("{:016x}", roots.output.filesystem_inode());
    database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/media-native-worker-catalog.sql"),
            &[
                (
                    "revaer_test.source_path",
                    path_text(roots.source.canonical_path())?,
                ),
                ("revaer_test.source_device", &source_device),
                ("revaer_test.source_inode", &source_inode),
                (
                    "revaer_test.workspace_path",
                    path_text(roots.output.canonical_path())?,
                ),
                ("revaer_test.workspace_device", &workspace_device),
                ("revaer_test.workspace_inode", &workspace_inode),
            ],
        )
        .await?;
    create_target(pool).await?;
    create_policy(pool, dry_run, 1).await?;
    Ok(())
}

pub(crate) async fn create_profile_association(
    pool: &PgPool,
    key: &str,
    dry_run: bool,
    prefix: &str,
    policy_version: i32,
) -> anyhow::Result<crate::media::associations::AssociationRow> {
    create_profile_association_for_input(
        pool,
        &CreateProfileVersionInput {
            actor_public_id: actor_id()?,
            profile_key: key,
            display_name: "Native recovery fixture",
            description: "Synthetic database admission",
            enabled: true,
            dry_run_only: dry_run,
            desired_target_key: "recovery-target",
            desired_target_version: 1,
            policy_key: "recovery-policy",
            policy_version,
            output_root_key: "worker-source",
            workspace_root_key: "worker-workspace",
            backup_root_key: None,
            quarantine_root_key: None,
        },
        prefix,
    )
    .await
}

pub(super) async fn create_profile_association_for_input(
    pool: &PgPool,
    input: &CreateProfileVersionInput<'_>,
    prefix: &str,
) -> anyhow::Result<crate::media::associations::AssociationRow> {
    let profiles = create_profile_version(pool, input).await?;
    let profile = profiles
        .first()
        .ok_or_else(|| anyhow::anyhow!("native profile missing"))?;
    Ok(create_association(
        pool,
        &CreateAssociationInput {
            actor_public_id: actor_id()?,
            association_key: input.profile_key,
            media_profile_public_id: profile.media_profile_public_id,
            profile_version: 1,
            source_root_key: "worker-source",
            root_relative_path: prefix,
            manual_enabled: true,
            watcher_enabled: false,
            schedule_enabled: false,
        },
    )
    .await?)
}

pub(super) async fn enqueue_candidate(
    pool: &PgPool,
    association: &crate::media::associations::AssociationRow,
    relative_path: &str,
    dry_run: bool,
    fingerprint: AssociationFingerprint<'_>,
) -> anyhow::Result<Uuid> {
    let readiness = crate::media::root_catalog::read_root_catalog_readiness(pool).await?;
    let generation = readiness
        .first()
        .and_then(|row| row.attestation_generation)
        .ok_or_else(|| anyhow::anyhow!("native generation missing"))?;
    let job = enqueue_association_job(
        pool,
        &AssociationJobInput {
            actor_public_id: actor_id()?,
            association_public_id: association.media_discovery_association_public_id,
            association_version: association.latest_version,
            relative_path,
            dry_run,
            trigger: "manual",
            generation,
            generation_sha256: [0x33; 32],
            fingerprint,
        },
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("native recovery job missing"))?;
    assert_eq!(
        job.dry_run, dry_run,
        "fixture admission changed requested mode"
    );
    Ok(job.media_job_public_id)
}

pub(super) async fn create_policy(
    pool: &PgPool,
    dry_run: bool,
    version: i32,
) -> anyhow::Result<()> {
    upsert_media_policy_profile(
        pool,
        UpsertMediaPolicyProfileInput {
            actor_public_id: actor_id()?,
            policy_key: "recovery-policy",
            version,
            display_name: "Recovery policy",
            video_intent: "general",
            verification_strictness: "balanced",
            verification_duration_tolerance_millis: 2000,
            verification_mux_validation: true.into(),
            verification_decode_all_streams: true.into(),
            verification_keyframe_seek: true.into(),
            verification_playback_probe: false.into(),
            output: MediaPolicyOutputRow {
                dry_run,
                replacement_mode: if dry_run {
                    "disabled"
                } else {
                    "atomic_replace"
                }
                .to_string(),
                quarantine_enabled: false,
                ..MediaPolicyOutputRow::default()
            },
        },
    )
    .await?;
    Ok(())
}

async fn create_target(pool: &PgPool) -> anyhow::Result<()> {
    let target = create_media_desired_target(
        pool,
        CreateMediaDesiredTargetInput {
            actor_public_id: actor_id()?,
            target_key: "recovery-target",
            version: 1,
            display_name: "Recovery target",
            container_format: "matroska",
        },
    )
    .await?;
    append_media_desired_target_stream(
        pool,
        AppendMediaDesiredTargetStreamInput {
            media_desired_target_profile_public_id: target,
            stream_key: "video-main",
            stream_kind: "video",
            semantic_role: None,
            language_code: None,
            optional: false,
            sort_order: 0,
            codec: "h264",
            channel_count: None,
            channel_layout: None,
            audio_bitrate_bps: None,
            audio_sample_rate_hz: None,
            audio_loudness_profile: None,
            audio_dynamic_range: None,
            video_profile: None,
            video_level: None,
            video_bitrate_bps: None,
            color_primaries: None,
            color_transfer: None,
            color_space: None,
            hdr_format: None,
            title: None,
            default_disposition: false,
            forced_disposition: false,
            subtitle_placement: None,
            image_subtitle_action: None,
        },
    )
    .await?;
    Ok(())
}
