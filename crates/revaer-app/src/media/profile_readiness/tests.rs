use super::response;
use revaer_data::media::profile_versions::ProfileReadinessRows;

fn snapshot() -> anyhow::Result<ProfileReadinessRows> {
    let latest = crate::media::profile_versions::tests::fixture()?;
    let mut roots = crate::media::root_readiness::tests::rows();
    for row in &mut roots {
        row.destructive_ready_slot_count = 1;
    }
    Ok(ProfileReadinessRows {
        active: latest.clone(),
        latest,
        roots,
        active_association_count: 1,
    })
}

#[test]
fn preserves_distinct_latest_and_active_bodies_without_granting_enablement() -> anyhow::Result<()> {
    let mut rows = snapshot()?;
    for row in &mut rows.latest {
        row.active_version = Some(3);
        row.lifecycle_state = "draft".into();
    }
    for row in &mut rows.active {
        row.latest_version = 3;
        row.active_version = Some(3);
        row.description = "Earlier active body".into();
    }
    let result = response(rows)?;
    assert_eq!(result.profile.fields().latest_version, 4);
    assert_eq!(result.profile.fields().active_version, Some(3));
    let active = result
        .active_profile
        .ok_or_else(|| anyhow::anyhow!("missing active body"))?;
    assert_eq!(active.fields().description, "Earlier active body");
    assert!(!active.fields().enabled);
    assert!(result.binding_ready && result.destructive_ready);
    assert_eq!(result.active_association_count, 1);
    Ok(())
}

#[test]
fn absent_active_head_is_not_ready_and_does_not_fabricate_bindings() -> anyhow::Result<()> {
    let mut rows = snapshot()?;
    for row in &mut rows.latest {
        row.active_version = None;
        row.lifecycle_state = "draft".into();
    }
    rows.active.clear();
    rows.active_association_count = 0;
    let result = response(rows)?;
    assert!(result.active_profile.is_none());
    assert!(result.active_root_bindings.is_empty());
    assert!(!result.binding_ready && !result.destructive_ready);
    assert_eq!(
        result.binding_reason.as_deref(),
        Some("media_root_binding_incomplete")
    );
    Ok(())
}

#[test]
fn rejects_foreign_or_mismatched_heads_and_unbounded_counts() -> anyhow::Result<()> {
    let mut rows = snapshot()?;
    for row in &mut rows.active {
        row.media_profile_public_id = uuid::Uuid::from_u128(1);
    }
    assert!(response(rows).is_err());
    let mut rows = snapshot()?;
    for row in &mut rows.latest {
        row.active_version = Some(3);
    }
    assert!(response(rows).is_err());
    for count in [-1, 129] {
        let mut rows = snapshot()?;
        rows.active_association_count = count;
        assert!(response(rows).is_err());
    }
    Ok(())
}
