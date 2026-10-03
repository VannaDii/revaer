use super::*;

fn profile_row(version: i32) -> PortableProfileRow {
    PortableProfileRow {
        profile_key: "library".into(),
        version,
        display_name: "Library".into(),
        description: "Explicit immutable version".into(),
        enabled: false,
        dry_run_only: true,
        desired_target_key: "h264".into(),
        desired_target_version: 2,
        policy_key: "preserve".into(),
        policy_version: 3,
        output_root_key: "output".into(),
        workspace_root_key: "workspace".into(),
        backup_root_key: None,
        quarantine_root_key: Some("quarantine".into()),
    }
}

fn association_row() -> PortableAssociationRow {
    PortableAssociationRow {
        association_key: "movies".into(),
        profile_key: "library".into(),
        profile_version: 1,
        source_root_key: "source".into(),
        root_relative_path: String::new(),
        manual_enabled: false,
        watcher_enabled: false,
        schedule_enabled: false,
    }
}

fn empty_snapshot() -> PortableSnapshot {
    PortableSnapshot {
        profiles: Vec::new(),
        associations: Vec::new(),
        compatibility_targets: Vec::new(),
        targets: Vec::new(),
        policies: Vec::new(),
    }
}

#[test]
fn profile_preserves_exact_native_body_and_version() -> anyhow::Result<()> {
    let serialized = serde_yaml::to_value(profile(profile_row(7))?)?;
    assert_eq!(serialized["version"].as_i64(), Some(7));
    assert_eq!(serialized["desired_target_version"].as_i64(), Some(2));
    assert_eq!(serialized["policy_version"].as_i64(), Some(3));
    assert_eq!(serialized["output_root_key"].as_str(), Some("output"));
    assert_eq!(serialized["workspace_root_key"].as_str(), Some("workspace"));
    assert_eq!(
        serialized["quarantine_root_key"].as_str(),
        Some("quarantine")
    );
    let mapping = serialized
        .as_mapping()
        .ok_or_else(|| anyhow::anyhow!("expected mapping"))?;
    for forbidden in [
        "source_root",
        "output_root",
        "media_profile_public_id",
        "root_bindings",
        "backup_root_key",
    ] {
        assert!(!mapping.contains_key(serde_yaml::Value::from(forbidden)));
    }
    assert!(profile(profile_row(0)).is_err());
    Ok(())
}

#[test]
fn whole_root_association_is_explicit_and_has_only_portable_fields() -> anyhow::Result<()> {
    let serialized = serde_yaml::to_value(association(association_row())?)?;
    assert_eq!(serialized["root_relative_path"].as_str(), Some(""));
    assert_eq!(serialized["profile_key"].as_str(), Some("library"));
    assert_eq!(serialized["profile_version"].as_i64(), Some(1));
    assert_eq!(
        serialized.as_mapping().map(serde_yaml::Mapping::len),
        Some(8)
    );
    let mut invalid = association_row();
    invalid.root_relative_path = "/physical/path".into();
    assert!(association(invalid).is_err());
    Ok(())
}

#[test]
fn unexported_association_pin_fails_instead_of_dropping_the_association() {
    let mut snapshot = empty_snapshot();
    snapshot.associations.push(association_row());
    assert!(serialize(snapshot).is_err());
}

#[test]
fn overflow_is_rejected_before_serializing_a_partial_bundle() -> anyhow::Result<()> {
    let mut snapshot = empty_snapshot();
    snapshot.profiles = (1..=129).map(profile_row).collect();
    let result = serialize(snapshot);
    let Err(error) = result else {
        anyhow::bail!("overflow must fail");
    };
    assert_eq!(error.code(), Some("media_configuration_bound_exceeded"));
    Ok(())
}

#[test]
fn empty_bundle_is_deterministic_and_includes_associations() -> anyhow::Result<()> {
    let first = serialize(empty_snapshot())?;
    assert_eq!(first, serialize(empty_snapshot())?);
    let parsed: serde_yaml::Value = serde_yaml::from_str(&first)?;
    assert_eq!(parsed["format_version"].as_i64(), Some(1));
    assert_eq!(
        parsed["discovery_associations"].as_sequence().map(Vec::len),
        Some(0)
    );
    Ok(())
}

#[test]
fn catalog_preconditions_use_catalog_key_grammar() -> anyhow::Result<()> {
    use revaer_api::models::{MediaYamlResourceKind, MediaYamlResourcePrecondition};
    let yaml = "format_version: 1\nkind: revaer.media.profile_bundle\nmetadata:\n  name: Catalog key regression\ncompatibility_targets:\n  - compatibility_target_key: safe_dry_run\n    version: 1\n    display_name: Catalog\n    video_codec: h264\n    audio_codec: aac\n    audio_channels: 2\n    audio_channel_layout: stereo\n    subtitle_policy: selected\n";
    let fence = MediaYamlResourcePrecondition::Match {
        kind: MediaYamlResourceKind::CompatibilityTargets,
        key: "safe_dry_run".into(),
        expected_version: 1,
    };
    validation::validate_preconditions(yaml, &[fence])?;
    let mut row = profile_row(1);
    row.profile_key = "invalid_profile".into();
    assert!(profile(row).is_err());
    Ok(())
}

#[test]
fn local_snapshot_is_explicit_and_never_import_authority() -> anyhow::Result<()> {
    let yaml = serialize_local(
        empty_snapshot(),
        vec![
            revaer_data::media::portable::LocalRootPathRow {
                logical_key: "source".into(),
                canonical_path: Some("/srv/media".into()),
            },
            revaer_data::media::portable::LocalRootPathRow {
                logical_key: "unmapped".into(),
                canonical_path: None,
            },
        ],
    )?;
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml)?;
    assert_eq!(parsed["kind"].as_str(), Some("revaer.media.local_snapshot"));
    assert_eq!(
        parsed["local_root_paths"][0]["canonical_path"].as_str(),
        Some("/srv/media")
    );
    assert!(parsed["local_root_paths"][1]["canonical_path"].is_null());
    assert!(validation::parse(&yaml).is_err());
    let portable = serialize(empty_snapshot())?;
    assert!(!portable.contains("local_root_paths"));
    assert!(!portable.contains("/srv/media"));
    Ok(())
}
