//! Native admission for injected worker regressions; synthetic catalog proof only.

use std::path::Path;

use revaer_data::media::association_jobs::{
    AssociationFingerprint, AssociationJobInput, enqueue_association_job,
};
use revaer_data::media::associations::{CreateAssociationInput, create_association};
use revaer_data::media::configuration::{
    MediaPolicyOutputRow, UpsertMediaPolicyProfileInput, upsert_media_policy_profile,
};
use revaer_data::media::profile_versions::{CreateProfileVersionInput, create_profile_version};
use revaer_data::media::{MediaRootIdentityResolver, StdMediaRootIdentityResolver};
use revaer_runtime::media::MediaStore;
use revaer_test_support::postgres::TestDatabase;
use uuid::Uuid;

pub(super) struct NativeJobInput<'a> {
    pub(super) actor: Uuid,
    pub(super) source_path: &'a Path,
    pub(super) source_root: &'a Path,
    pub(super) workspace_root: &'a Path,
    pub(super) target_key: &'a str,
    pub(super) target_version: i32,
    pub(super) dry_run: bool,
}

pub(super) async fn enqueue(
    postgres: &TestDatabase,
    store: &MediaStore,
    input: &NativeJobInput<'_>,
) -> anyhow::Result<Uuid> {
    seed_catalog(postgres, input.source_root, input.workspace_root).await?;
    create_policy(store, input.actor, input.dry_run).await?;
    let rows = create_profile_version(
        store.pool(),
        &CreateProfileVersionInput {
            actor_public_id: input.actor,
            profile_key: "worker",
            display_name: "Injected native worker fixture",
            description: "Synthetic catalog state, not filesystem qualification",
            enabled: true,
            dry_run_only: input.dry_run,
            desired_target_key: input.target_key,
            desired_target_version: input.target_version,
            policy_key: "worker-policy",
            policy_version: 1,
            output_root_key: "worker-source",
            workspace_root_key: "worker-workspace",
            backup_root_key: None,
            quarantine_root_key: None,
        },
    )
    .await?;
    let profile = rows
        .first()
        .ok_or_else(|| anyhow::anyhow!("native worker profile missing"))?;
    let association = create_association(
        store.pool(),
        &CreateAssociationInput {
            actor_public_id: input.actor,
            association_key: "worker-manual",
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
    let generation = revaer_data::media::root_catalog::read_root_catalog_readiness(store.pool())
        .await?
        .first()
        .and_then(|row| row.attestation_generation)
        .ok_or_else(|| anyhow::anyhow!("native worker generation missing"))?;
    let relative = input
        .source_path
        .strip_prefix(input.source_root)?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("native worker relative path is not UTF-8"))?;
    let fingerprint = crate::media_discovery_fingerprint::fingerprint_media_aggregate(
        input.source_path,
        input.source_root,
    )?
    .ok_or_else(|| anyhow::anyhow!("native worker source fingerprint was unstable"))?;
    let job = enqueue_association_job(
        store.pool(),
        &AssociationJobInput {
            actor_public_id: input.actor,
            association_public_id: association.media_discovery_association_public_id,
            association_version: association.latest_version,
            relative_path: relative,
            dry_run: input.dry_run,
            trigger: "manual",
            generation,
            generation_sha256: [0x33; 32],
            fingerprint: AssociationFingerprint {
                identity: &fingerprint.identity,
                size_bytes: fingerprint.size_bytes,
                modified_ns: fingerprint.modified_ns,
                changed_ns: fingerprint.changed_ns,
                sha256: &fingerprint.sha256,
            },
        },
    )
    .await?
    .ok_or_else(|| anyhow::anyhow!("native worker job was not admitted"))?;
    anyhow::ensure!(
        job.dry_run == input.dry_run,
        "worker dry-run intent changed"
    );
    Ok(job.media_job_public_id)
}

async fn create_policy(store: &MediaStore, actor: Uuid, dry_run: bool) -> anyhow::Result<()> {
    upsert_media_policy_profile(
        store.pool(),
        UpsertMediaPolicyProfileInput {
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
            actor_public_id: actor,
            policy_key: "worker-policy",
            version: 1,
            display_name: "Injected worker fixture",
            video_intent: "general",
            verification_strictness: "balanced",
            verification_duration_tolerance_millis: 2000,
            verification_mux_validation: true.into(),
            verification_decode_all_streams: true.into(),
            verification_keyframe_seek: true.into(),
            verification_playback_probe: false.into(),
        },
    )
    .await?;
    Ok(())
}

async fn seed_catalog(
    postgres: &TestDatabase,
    source: &Path,
    workspace: &Path,
) -> anyhow::Result<()> {
    let resolver = StdMediaRootIdentityResolver;
    let source_id = resolver.resolve(source)?;
    let workspace_id = resolver.resolve(workspace)?;
    let source_path = source
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("source root is not UTF-8"))?;
    let workspace_path = workspace
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("workspace root is not UTF-8"))?;
    let source_device = format!("{:016x}", source_id.filesystem_device());
    let source_inode = format!("{:016x}", source_id.filesystem_inode());
    let workspace_device = format!("{:016x}", workspace_id.filesystem_device());
    let workspace_inode = format!("{:016x}", workspace_id.filesystem_inode());
    postgres
        .apply_fixture_script(
            include_str!("../../../../scripts/tests/media-native-worker-catalog.sql"),
            &[
                ("revaer_test.source_path", source_path),
                ("revaer_test.source_device", &source_device),
                ("revaer_test.source_inode", &source_inode),
                ("revaer_test.workspace_path", workspace_path),
                ("revaer_test.workspace_device", &workspace_device),
                ("revaer_test.workspace_inode", &workspace_inode),
            ],
        )
        .await
}
