//! Canonical native portable export, without host paths or filesystem authority.

use std::collections::BTreeSet;

use revaer_api::models::media_root_contract::{
    ProfileVersionRequest, ProfileVersionRequestFields, validate_root_association_prefix,
    validate_root_logical_key,
};
use revaer_data::media::portable::{PortableAssociationRow, PortableProfileRow, PortableSnapshot};
use serde::{Deserialize, Deserializer, Serialize, de};

use super::{
    MediaServiceError, MediaServiceErrorKind, MediaYamlCompatibilityTarget, MediaYamlDesiredTarget,
    MediaYamlMetadata, MediaYamlPolicy, map_desired_target_stream, policy_output_response,
};

#[derive(Serialize)]
struct PortableProfile {
    version: i32,
    #[serde(flatten)]
    profile: ProfileVersionRequest,
}

impl<'de> Deserialize<'de> for PortableProfile {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut value = serde_yaml::Value::deserialize(deserializer)?;
        let mapping = value
            .as_mapping_mut()
            .ok_or_else(|| de::Error::custom("invalid portable profile"))?;
        let version = mapping
            .remove(serde_yaml::Value::from("version"))
            .and_then(|v| v.as_i64())
            .and_then(|v| i32::try_from(v).ok())
            .filter(|v| *v > 0)
            .ok_or_else(|| de::Error::custom("invalid portable version"))?;
        let profile = serde_yaml::from_value(value)
            .map_err(|_| de::Error::custom("invalid native profile"))?;
        Ok(Self { version, profile })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableAssociation {
    association_key: String,
    profile_key: String,
    profile_version: i32,
    source_root_key: String,
    root_relative_path: String,
    manual_enabled: bool,
    watcher_enabled: bool,
    schedule_enabled: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PortableBundle {
    format_version: u32,
    kind: String,
    metadata: MediaYamlMetadata,
    #[serde(default)]
    compatibility_targets: Vec<MediaYamlCompatibilityTarget>,
    #[serde(default)]
    targets: Vec<MediaYamlDesiredTarget>,
    #[serde(default)]
    policies: Vec<MediaYamlPolicy>,
    #[serde(default)]
    profiles: Vec<PortableProfile>,
    #[serde(default)]
    discovery_associations: Vec<PortableAssociation>,
}

pub(super) fn serialize(snapshot: PortableSnapshot) -> Result<String, MediaServiceError> {
    if snapshot.resource_count() > 128 || snapshot.child_count() > 4_096 {
        return Err(bound_error());
    }
    let bundle = bundle(snapshot)?;
    let yaml = serde_yaml::to_string(&bundle).map_err(|_| invalid_snapshot())?;
    if yaml.len() > revaer_api::models::MEDIA_YAML_BUNDLE_MAX_BYTES {
        return Err(bound_error());
    }
    Ok(yaml)
}

pub(super) fn serialize_local(
    snapshot: PortableSnapshot,
    paths: Vec<revaer_data::media::portable::LocalRootPathRow>,
) -> Result<String, MediaServiceError> {
    #[derive(Serialize)]
    struct LocalPath {
        logical_key: String,
        canonical_path: Option<String>,
    }
    #[derive(Serialize)]
    struct LocalExport {
        #[serde(flatten)]
        configuration: PortableBundle,
        local_root_paths: Vec<LocalPath>,
    }
    if snapshot.resource_count() > 128 || snapshot.child_count() > 4_096 || paths.len() > 4_096 {
        return Err(bound_error());
    }
    let mut configuration = bundle(snapshot)?;
    configuration.kind = "revaer.media.local_snapshot".into();
    let export = LocalExport {
        configuration,
        local_root_paths: paths
            .into_iter()
            .map(|row| LocalPath {
                logical_key: row.logical_key,
                canonical_path: row.canonical_path,
            })
            .collect(),
    };
    let yaml = serde_yaml::to_string(&export).map_err(|_| invalid_snapshot())?;
    if yaml.len() > revaer_api::models::MEDIA_YAML_BUNDLE_MAX_BYTES {
        return Err(bound_error());
    }
    Ok(yaml)
}

pub(super) async fn prepare_import(
    transaction: &mut super::MediaImportTransaction<'_>,
    actor: uuid::Uuid,
    input: &str,
    preconditions: &[revaer_api::models::MediaYamlResourcePrecondition],
) -> Result<(), MediaServiceError> {
    use revaer_api::models::{MediaYamlResourceKind, MediaYamlResourcePrecondition};
    let bundle = validation::parse(input)?;
    let mut sources = bundle
        .profiles
        .iter()
        .map(|row| row.profile.fields().output_root_key.clone())
        .chain(
            bundle
                .discovery_associations
                .iter()
                .map(|row| row.source_root_key.clone()),
        )
        .collect::<Vec<_>>();
    sources.sort();
    sources.dedup();
    let rows = preconditions
        .iter()
        .map(|row| {
            let (kind, key) = row.resource();
            let kind = match kind {
                MediaYamlResourceKind::CompatibilityTargets => "compatibility_targets",
                MediaYamlResourceKind::Targets => "targets",
                MediaYamlResourceKind::Policies => "policies",
                MediaYamlResourceKind::Profiles => "profiles",
                MediaYamlResourceKind::DiscoveryAssociations => "discovery_associations",
            };
            let expected_version = expected_import_head(row);
            revaer_data::media::configuration::ImportResourcePrecondition {
                kind,
                key,
                create: matches!(row, MediaYamlResourcePrecondition::Create { .. }),
                expected_version,
            }
        })
        .collect::<Vec<_>>();
    revaer_data::media::configuration::prepare_import(transaction, actor, &rows, &sources)
        .await
        .map_err(|error| super::map_data_error(&error))
}

pub(super) async fn apply_import(
    service: &super::MediaService,
    transaction: &mut super::MediaImportTransaction<'_>,
    actor: uuid::Uuid,
    input: &str,
    preconditions: &[revaer_api::models::MediaYamlResourcePrecondition],
) -> Result<super::MediaYamlApplyResult, MediaServiceError> {
    use revaer_api::app::media::MediaFacade;
    use revaer_api::models::MediaYamlResourceKind;
    use sha2::Digest;
    let mut bundle = validation::parse(input)?;
    let count = bundle.profiles.len()
        + bundle.discovery_associations.len()
        + bundle.targets.len()
        + bundle.policies.len()
        + bundle.compatibility_targets.len();
    let mut heads = preconditions
        .iter()
        .map(|row| {
            let (kind, key) = row.resource();
            let expected = expected_import_head(row);
            ((kind, key.to_owned()), expected)
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let catalogs = super::MediaYamlBundle {
        format_version: bundle.format_version,
        kind: bundle.kind,
        metadata: bundle.metadata,
        compatibility_targets: bundle.compatibility_targets,
        targets: bundle.targets,
        policies: bundle.policies,
        profiles: Vec::new(),
    };
    super::import_yaml_catalogs(
        transaction,
        actor,
        &catalogs,
        &service.media_compatibility_target_list().await?,
        &service.media_desired_target_list().await?,
        &service.media_policy_list().await?,
    )
    .await?;
    bundle.profiles.sort_by(|a, b| {
        (&a.profile.fields().profile_key, a.version)
            .cmp(&(&b.profile.fields().profile_key, b.version))
    });
    let mut ids = BTreeSet::new();
    let mut drafts = BTreeSet::new();
    for profile in bundle.profiles {
        let fields = profile.profile.fields();
        let identity = (MediaYamlResourceKind::Profiles, fields.profile_key.clone());
        let head = heads.get_mut(&identity).ok_or_else(invalid_snapshot)?;
        let row = import_profile_row(&profile);
        let imported =
            revaer_data::media::configuration::import_profile(transaction, actor, &row, *head)
                .await
                .map_err(|error| super::map_data_error(&error))?;
        *head = Some(imported.latest_version);
        if imported.draft {
            ids.remove(&imported.profile_public_id);
            drafts.insert(imported.profile_public_id);
        } else {
            drafts.remove(&imported.profile_public_id);
            ids.insert(imported.profile_public_id);
        }
    }
    bundle
        .discovery_associations
        .sort_by(|a, b| a.association_key.cmp(&b.association_key));
    for association in bundle.discovery_associations {
        let head = heads
            .get(&(
                MediaYamlResourceKind::DiscoveryAssociations,
                association.association_key.clone(),
            ))
            .ok_or_else(invalid_snapshot)?;
        let row = PortableAssociationRow {
            association_key: association.association_key,
            profile_key: association.profile_key,
            profile_version: association.profile_version,
            source_root_key: association.source_root_key,
            root_relative_path: association.root_relative_path,
            manual_enabled: association.manual_enabled,
            watcher_enabled: association.watcher_enabled,
            schedule_enabled: association.schedule_enabled,
        };
        // The result is a host-local identity; the portable result contains only profiles.
        let _id =
            revaer_data::media::configuration::import_association(transaction, actor, &row, *head)
                .await
                .map_err(|error| super::map_data_error(&error))?;
    }
    revaer_data::media::configuration::audit_import(
        transaction,
        actor,
        &sha2::Sha256::digest(input.as_bytes()),
        i32::try_from(count).map_err(|_| bound_error())?,
    )
    .await
    .map_err(|error| super::map_data_error(&error))?;
    Ok(super::MediaYamlApplyResult {
        forced_dry_run: true,
        media_profile_public_ids: ids.into_iter().collect(),
        media_profile_import_draft_public_ids: drafts.into_iter().collect(),
    })
}

fn import_profile_row(profile: &PortableProfile) -> PortableProfileRow {
    let fields = profile.profile.fields();
    PortableProfileRow {
        profile_key: fields.profile_key.clone(),
        version: profile.version,
        display_name: fields.display_name.clone(),
        description: fields.description.clone(),
        enabled: fields.enabled,
        dry_run_only: true,
        desired_target_key: fields.desired_target_key.clone(),
        desired_target_version: fields.desired_target_version,
        policy_key: fields.policy_key.clone(),
        policy_version: fields.policy_version,
        output_root_key: fields.output_root_key.clone(),
        workspace_root_key: fields.workspace_root_key.clone(),
        backup_root_key: fields.backup_root_key.clone(),
        quarantine_root_key: fields.quarantine_root_key.clone(),
    }
}

const fn expected_import_head(
    row: &revaer_api::models::MediaYamlResourcePrecondition,
) -> Option<i32> {
    use revaer_api::models::MediaYamlResourcePrecondition;
    match row {
        MediaYamlResourcePrecondition::Create { .. } => None,
        MediaYamlResourcePrecondition::Match {
            expected_version, ..
        } => Some(*expected_version),
    }
}

fn bundle(snapshot: PortableSnapshot) -> Result<PortableBundle, MediaServiceError> {
    let mut profiles = snapshot
        .profiles
        .into_iter()
        .map(profile)
        .collect::<Result<Vec<_>, _>>()?;
    profiles.sort_by(|a, b| {
        (&a.profile.fields().profile_key, a.version)
            .cmp(&(&b.profile.fields().profile_key, b.version))
    });
    let profile_versions = profiles
        .iter()
        .map(|p| (p.profile.fields().profile_key.as_str(), p.version))
        .collect::<BTreeSet<_>>();
    if profile_versions.len() != profiles.len() {
        return Err(invalid_snapshot());
    }
    let mut discovery_associations = snapshot
        .associations
        .into_iter()
        .map(|row| {
            if !profile_versions.contains(&(row.profile_key.as_str(), row.profile_version)) {
                return Err(invalid_snapshot());
            }
            association(row)
        })
        .collect::<Result<Vec<_>, _>>()?;
    discovery_associations.sort_by(|a, b| a.association_key.cmp(&b.association_key));
    if discovery_associations
        .windows(2)
        .any(|pair| pair[0].association_key == pair[1].association_key)
    {
        return Err(invalid_snapshot());
    }
    let mut compatibility_targets = snapshot
        .compatibility_targets
        .into_iter()
        .map(|row| MediaYamlCompatibilityTarget {
            compatibility_target_key: row.compatibility_target_key,
            version: row.version,
            display_name: row.display_name,
            video_codec: row.video_codec,
            audio_codec: row.audio_codec,
            audio_channels: row.audio_channels,
            audio_channel_layout: row.audio_channel_layout,
            subtitle_policy: row.subtitle_policy,
        })
        .collect::<Vec<_>>();
    compatibility_targets.sort_by(|a, b| {
        (&a.compatibility_target_key, a.version).cmp(&(&b.compatibility_target_key, b.version))
    });
    let mut targets = snapshot
        .targets
        .into_iter()
        .map(|(row, streams)| MediaYamlDesiredTarget {
            target_key: row.target_key,
            version: row.version,
            display_name: row.display_name,
            container_format: row.container_format,
            streams: streams.into_iter().map(map_desired_target_stream).collect(),
        })
        .collect::<Vec<_>>();
    targets.sort_by(|a, b| (&a.target_key, a.version).cmp(&(&b.target_key, b.version)));
    let mut policies = snapshot
        .policies
        .into_iter()
        .map(|row| MediaYamlPolicy {
            output: policy_output_response(&row.output),
            policy_key: row.policy_key,
            version: row.version,
            display_name: row.display_name,
            video_intent: row.video_intent,
            verification_strictness: row.verification_strictness,
            verification_duration_tolerance_millis: row.verification_duration_tolerance_millis,
            verification_mux_validation: row.verification_mux_validation.enabled().into(),
            verification_decode_all_streams: row.verification_decode_all_streams.enabled().into(),
            verification_keyframe_seek: row.verification_keyframe_seek.enabled().into(),
            verification_playback_probe: row.verification_playback_probe.enabled().into(),
        })
        .collect::<Vec<_>>();
    policies.sort_by(|a, b| (&a.policy_key, a.version).cmp(&(&b.policy_key, b.version)));
    validate_references(&profiles, &targets, &policies)?;
    Ok(PortableBundle {
        format_version: 1,
        kind: "revaer.media.profile_bundle".into(),
        metadata: MediaYamlMetadata {
            name: "Revaer media configuration".into(),
            description: Some("Portable native profiles and exact referenced versions".into()),
        },
        compatibility_targets,
        targets,
        policies,
        profiles,
        discovery_associations,
    })
}

fn validate_references(
    profiles: &[PortableProfile],
    targets: &[MediaYamlDesiredTarget],
    policies: &[MediaYamlPolicy],
) -> Result<(), MediaServiceError> {
    if targets.iter().any(super::yaml_desired_target_shape_invalid) {
        return Err(invalid_snapshot());
    }
    let target_versions = targets
        .iter()
        .map(|t| (t.target_key.as_str(), t.version))
        .collect::<BTreeSet<_>>();
    let policy_versions = policies
        .iter()
        .map(|p| (p.policy_key.as_str(), p.version))
        .collect::<BTreeSet<_>>();
    for profile in profiles {
        let fields = profile.profile.fields();
        if !target_versions.contains(&(
            fields.desired_target_key.as_str(),
            fields.desired_target_version,
        )) || !policy_versions.contains(&(fields.policy_key.as_str(), fields.policy_version))
        {
            return Err(invalid_snapshot());
        }
    }
    Ok(())
}

fn profile(row: PortableProfileRow) -> Result<PortableProfile, MediaServiceError> {
    if row.version <= 0 {
        return Err(invalid_snapshot());
    }
    let version = row.version;
    let profile = ProfileVersionRequest::new(ProfileVersionRequestFields {
        profile_key: row.profile_key,
        display_name: row.display_name,
        description: row.description,
        enabled: row.enabled,
        dry_run_only: row.dry_run_only,
        desired_target_key: row.desired_target_key,
        desired_target_version: row.desired_target_version,
        policy_key: row.policy_key,
        policy_version: row.policy_version,
        output_root_key: row.output_root_key,
        workspace_root_key: row.workspace_root_key,
        backup_root_key: row.backup_root_key,
        quarantine_root_key: row.quarantine_root_key,
    })
    .map_err(|_| invalid_snapshot())?;
    Ok(PortableProfile { version, profile })
}

fn association(row: PortableAssociationRow) -> Result<PortableAssociation, MediaServiceError> {
    for key in [&row.association_key, &row.profile_key, &row.source_root_key] {
        validate_root_logical_key(key).map_err(|_| invalid_snapshot())?;
    }
    validate_root_association_prefix(&row.root_relative_path).map_err(|_| invalid_snapshot())?;
    if row.profile_version <= 0 {
        return Err(invalid_snapshot());
    }
    Ok(PortableAssociation {
        association_key: row.association_key,
        profile_key: row.profile_key,
        profile_version: row.profile_version,
        source_root_key: row.source_root_key,
        root_relative_path: row.root_relative_path,
        manual_enabled: row.manual_enabled,
        watcher_enabled: row.watcher_enabled,
        schedule_enabled: row.schedule_enabled,
    })
}

fn bound_error() -> MediaServiceError {
    MediaServiceError::new(MediaServiceErrorKind::Invalid)
        .with_code("media_configuration_bound_exceeded")
}

fn invalid_snapshot() -> MediaServiceError {
    MediaServiceError::new(MediaServiceErrorKind::Storage)
        .with_code("media_yaml_export_snapshot_invalid")
}

mod validation;

pub(super) use validation::{validate, validate_preconditions};
