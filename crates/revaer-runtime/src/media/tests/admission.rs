//! Current admission for facade regressions; catalog state is synthetic.

use super::MediaStore;
use revaer_data::media::association_jobs::{
    AssociationFingerprint, AssociationJobInput, enqueue_association_job,
};
use revaer_data::media::associations::{CreateAssociationInput, create_association};
use revaer_data::media::configuration::{
    AppendMediaDesiredTargetStreamInput, CreateMediaDesiredTargetInput, MediaPolicyOutputRow,
    UpsertMediaPolicyProfileInput, append_media_desired_target_stream, create_media_desired_target,
    upsert_media_policy_profile,
};
use revaer_data::media::profile_versions::{CreateProfileVersionInput, create_profile_version};
use revaer_test_support::postgres::TestDatabase;
use uuid::Uuid;

pub(super) async fn enqueue(
    database: &TestDatabase,
    store: &MediaStore,
    actor: Uuid,
    key: &str,
) -> anyhow::Result<(Uuid, Uuid)> {
    let pool = store.pool();
    initialize_catalog(database).await?;
    create_target(pool, actor).await?;
    upsert_media_policy_profile(
        pool,
        UpsertMediaPolicyProfileInput {
            actor_public_id: actor,
            policy_key: "runtime-policy",
            version: 1,
            display_name: "Runtime policy",
            video_intent: "general",
            verification_strictness: "balanced",
            verification_duration_tolerance_millis: 2000,
            verification_mux_validation: true.into(),
            verification_decode_all_streams: true.into(),
            verification_keyframe_seek: true.into(),
            verification_playback_probe: false.into(),
            output: MediaPolicyOutputRow {
                dry_run: true,
                replacement_mode: "disabled".to_string(),
                quarantine_enabled: false,
                ..MediaPolicyOutputRow::default()
            },
        },
    )
    .await?;
    let profiles = create_profile_version(
        pool,
        &CreateProfileVersionInput {
            actor_public_id: actor,
            profile_key: key,
            display_name: "Runtime fixture",
            description: "Synthetic database admission",
            enabled: true,
            dry_run_only: true,
            desired_target_key: "runtime-target",
            desired_target_version: 1,
            policy_key: "runtime-policy",
            policy_version: 1,
            output_root_key: "worker-source",
            workspace_root_key: "worker-workspace",
            backup_root_key: None,
            quarantine_root_key: None,
        },
    )
    .await?;
    let profile = profiles
        .first()
        .ok_or_else(|| anyhow::anyhow!("runtime profile missing"))?;
    let association = create_association(
        pool,
        &CreateAssociationInput {
            actor_public_id: actor,
            association_key: key,
            media_profile_public_id: profile.media_profile_public_id,
            profile_version: 1,
            source_root_key: "worker-source",
            root_relative_path: "",
            manual_enabled: true,
            watcher_enabled: false,
            schedule_enabled: false,
        },
    )
    .await?;
    let readiness = revaer_data::media::root_catalog::read_root_catalog_readiness(pool).await?;
    let generation = readiness
        .first()
        .and_then(|row| row.attestation_generation)
        .ok_or_else(|| anyhow::anyhow!("runtime catalog generation missing"))?;
    let job = enqueue_association_job(
        pool,
        &AssociationJobInput {
            actor_public_id: actor,
            association_public_id: association.media_discovery_association_public_id,
            association_version: association.latest_version,
            relative_path: "show.mkv",
            dry_run: true,
            trigger: "manual",
            generation,
            generation_sha256: [0x33; 32],
            fingerprint: AssociationFingerprint {
                identity: "0000000000000001:0000000000000001",
                size_bytes: 1,
                modified_ns: 1,
                changed_ns: 1,
                sha256: &"1".repeat(64),
            },
        },
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("runtime job missing"))?;
    assert!(job.dry_run);
    Ok((profile.media_profile_public_id, job.media_job_public_id))
}

async fn initialize_catalog(database: &TestDatabase) -> anyhow::Result<()> {
    database
        .apply_fixture_script(
            include_str!("../../../../../scripts/tests/media-native-worker-catalog.sql"),
            &[
                ("revaer_test.source_path", "/input/tv"),
                ("revaer_test.source_device", "0000000000000001"),
                ("revaer_test.source_inode", "0000000000000001"),
                ("revaer_test.workspace_path", "/output/tv"),
                ("revaer_test.workspace_device", "0000000000000001"),
                ("revaer_test.workspace_inode", "0000000000000002"),
            ],
        )
        .await?;
    Ok(())
}

async fn create_target(pool: &sqlx::PgPool, actor: Uuid) -> anyhow::Result<()> {
    let target = create_media_desired_target(
        pool,
        CreateMediaDesiredTargetInput {
            actor_public_id: actor,
            target_key: "runtime-target",
            version: 1,
            display_name: "Runtime target",
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
