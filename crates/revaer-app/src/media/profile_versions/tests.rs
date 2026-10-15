use super::{page, response};
use revaer_data::media::profile_versions::{ProfileBindingReadiness, ProfileVersionRow};

pub(in crate::media) fn fixture() -> anyhow::Result<Vec<ProfileVersionRow>> {
    let time = chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")?.to_utc();
    let row = ProfileVersionRow {
        media_profile_public_id: uuid::Uuid::nil(),
        profile_key: "movies".into(),
        display_name: " Exact name ".into(),
        description: "Exact description".into(),
        enabled: false,
        dry_run_only: true,
        desired_target_key: "target".into(),
        desired_target_version: 2,
        policy_key: "safe-dry-run".into(),
        policy_version: 3,
        latest_version: 4,
        active_version: Some(4),
        lifecycle_state: "active".into(),
        created_at: time,
        updated_at: time,
        root_kind: "output".into(),
        logical_key: "output".into(),
        resolution_state: "resolved".into(),
        readiness: ProfileBindingReadiness {
            binding_ready: true,
            binding_reason: None,
            destructive_ready: true,
            destructive_reason: None,
        },
    };
    let mut workspace = row.clone();
    workspace.root_kind = "workspace".into();
    workspace.logical_key = "workspace".into();
    Ok(vec![row, workspace])
}

#[test]
fn page_preserves_complete_parents_and_continuation() -> anyhow::Result<()> {
    let first = fixture()?;
    let mut second = fixture()?;
    for row in &mut second {
        row.profile_key = "tv".into();
        row.media_profile_public_id = uuid::Uuid::from_u128(1);
    }
    let rows: Vec<_> = first.iter().chain(&second).cloned().collect();
    let limited = serde_json::to_value(page(&rows, 1)?)?;
    assert_eq!(limited["profiles"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        limited["profiles"][0]["root_bindings"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    let token = limited["next_cursor"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing continuation"))?;
    let cursor = revaer_api::models::media_root_contract::ProfileCollectionCursor::decode(token)?;
    assert_eq!(cursor.profile_key(), "movies");
    assert_eq!(cursor.profile_public_id(), uuid::Uuid::nil());
    let final_page = serde_json::to_value(page(&rows, 2)?)?;
    assert_eq!(final_page["profiles"].as_array().map(Vec::len), Some(2));
    assert!(final_page.get("next_cursor").is_none());
    assert!(page(&rows, 0).is_err());
    assert!(page(&rows, 201).is_err());
    assert!(page(&rows, 1).is_ok());
    let reversed: Vec<_> = second.into_iter().chain(first).collect();
    assert!(page(&reversed, 2).is_err());
    assert!(page(&reversed, 1).is_err());
    Ok(())
}

#[test]
fn complete_snapshot_preserves_exact_values_without_paths() -> anyhow::Result<()> {
    let profile = response(&fixture()?)?.ok_or_else(|| anyhow::anyhow!("profile missing"))?;
    let fields = profile.fields();
    assert_eq!(fields.latest_version, 4);
    assert_eq!(fields.active_version, Some(4));
    assert!(!fields.profile.fields().enabled);
    assert!(fields.profile.fields().dry_run_only);
    assert_eq!(fields.profile.fields().display_name, " Exact name ");
    assert_eq!(fields.profile.fields().desired_target_version, 2);
    assert_eq!(fields.profile.fields().policy_version, 3);
    let value = serde_json::to_value(profile)?;
    for forbidden in [
        "source_root",
        "output_root",
        "canonical_path",
        "filesystem_inode",
        "slot_id",
    ] {
        assert!(value.get(forbidden).is_none());
    }
    assert!(response(&[])?.is_none());
    Ok(())
}

#[test]
fn mixed_profile_metadata_is_rejected() -> anyhow::Result<()> {
    let mutations: [fn(&mut ProfileVersionRow); 16] = [
        |r| r.media_profile_public_id = uuid::Uuid::new_v4(),
        |r| r.profile_key = "different".into(),
        |r| r.display_name = "Different".into(),
        |r| r.description = "Different".into(),
        |r| r.enabled = true,
        |r| r.dry_run_only = false,
        |r| r.desired_target_key = "different".into(),
        |r| r.desired_target_version += 1,
        |r| r.policy_key = "different".into(),
        |r| r.policy_version += 1,
        |r| r.latest_version += 1,
        |r| r.active_version = None,
        |r| r.lifecycle_state = "draft".into(),
        |r| r.created_at += chrono::Duration::seconds(1),
        |r| r.updated_at += chrono::Duration::seconds(1),
        |r| r.lifecycle_state = "unknown".into(),
    ];
    for mutate in mutations {
        let mut rows = fixture()?;
        mutate(&mut rows[1]);
        assert!(response(&rows).is_err());
    }
    Ok(())
}

#[test]
fn incomplete_or_incoherent_bindings_are_rejected() -> anyhow::Result<()> {
    let mut missing = fixture()?;
    assert!(missing.pop().is_some());
    assert!(response(&missing).is_err());
    let mut reordered = fixture()?;
    reordered.reverse();
    assert!(response(&reordered).is_err());
    let mut duplicate = fixture()?;
    duplicate.push(duplicate[1].clone());
    assert!(response(&duplicate).is_err());
    let mutations: [fn(&mut ProfileVersionRow); 4] = [
        |r| r.root_kind = "source".into(),
        |r| r.resolution_state = "unknown".into(),
        |r| r.resolution_state = "unmapped".into(),
        |r| r.readiness.binding_reason = Some("unknown".into()),
    ];
    for mutate in mutations {
        let mut rows = fixture()?;
        mutate(&mut rows[1]);
        assert!(response(&rows).is_err());
    }
    Ok(())
}
