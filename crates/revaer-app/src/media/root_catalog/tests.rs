use super::*;
use uuid::Uuid;

fn state() -> anyhow::Result<RootCatalogStateRow> {
    let at = "2026-09-21T00:00:00Z".parse()?;
    Ok(RootCatalogStateRow {
        source_state: "ready".into(),
        source_reason_code: None,
        attestation_state: "ready".into(),
        attestation_reason_code: None,
        media_root_catalog_generation_public_id: Some(Uuid::from_u128(3)),
        attestation_generation: Some(1),
        source_format_version: Some(1),
        source_sha256: Some(vec![1; 32]),
        generation_sha256: Some(vec![2; 32]),
        slot_count: Some(2),
        activated_at: Some(at),
        reconciled_at: at,
    })
}

fn slot() -> anyhow::Result<RootCatalogSlotRow> {
    Ok(RootCatalogSlotRow {
        media_root_catalog_slot_public_id: Uuid::from_u128(1),
        logical_key: "library".into(),
        allowed_root_kind: "source".into(),
        requested_path: "/fixture/library".into(),
        canonical_path: "/fixture/library".into(),
        filesystem_device: vec![1; 8],
        filesystem_inode: vec![2; 8],
        mount_id: 1,
        filesystem_type: "fixture".into(),
        capability_mask: 127,
        durability_class: "restart_persistent".into(),
        durability_evidence: "linux_dedicated_mount".into(),
        sole_writer_class: "revaer_exclusive".into(),
        sole_writer_evidence: "linux_dedicated_service".into(),
        owner_uid: 1000,
        owner_gid: 1000,
        mode_bits: 0o700,
        validated_at: "2026-09-21T00:00:00Z".parse()?,
        root_identity_sha256: vec![3; 32],
        binding_ready: true,
        binding_reason_code: None,
        destructive_ready: true,
        destructive_reason_code: None,
        page_has_more: true,
    })
}

fn rows() -> anyhow::Result<Vec<RootCatalogPageRow>> {
    let source = slot()?;
    let mut output = source.clone();
    output.allowed_root_kind = "output".into();
    Ok(vec![
        RootCatalogPageRow {
            state: state()?,
            slot: Some(source),
        },
        RootCatalogPageRow {
            state: state()?,
            slot: Some(output),
        },
    ])
}

#[test]
fn root_catalog_groups_kinds_and_emits_exact_continuation() -> anyhow::Result<()> {
    let page = response(rows()?)?;
    assert_eq!(page.slots().len(), 1);
    let slot = &page.slots()[0];
    assert_eq!(slot.allowed_kinds().len(), 2);
    assert_eq!(slot.fields().filesystem_device, "0101010101010101");
    assert_eq!(slot.fields().mode_bits, "0700");
    let cursor = RootCatalogCursor::decode(page.next_cursor().ok_or(RootCatalogError)?)?;
    assert_eq!(cursor.logical_key(), "library");
    assert_eq!(cursor.slot_public_id(), Uuid::from_u128(1));
    assert_eq!(
        serde_json::from_value::<RootCatalogPageResponse>(serde_json::to_value(&page)?)?,
        page
    );
    Ok(())
}

#[test]
fn root_catalog_rejects_inconsistent_or_fabricated_evidence() -> anyhow::Result<()> {
    let mut changed = rows()?;
    changed[1].state.attestation_generation = Some(2);
    assert!(response(changed).is_err());
    let mut changed = rows()?;
    changed[1]
        .slot
        .as_mut()
        .ok_or(RootCatalogError)?
        .canonical_path = "/different".into();
    assert!(response(changed).is_err());
    let mut changed = rows()?;
    changed[1]
        .slot
        .as_mut()
        .ok_or(RootCatalogError)?
        .page_has_more = false;
    assert!(response(changed).is_err());
    let mut changed = rows()?;
    changed[1]
        .slot
        .as_mut()
        .ok_or(RootCatalogError)?
        .allowed_root_kind = "source".into();
    assert!(response(changed).is_err());
    for row in [None, Some(slot()?)] {
        let mut state = state()?;
        state.source_state = "missing".into();
        state.source_reason_code = Some("media_root_catalog_source_missing".into());
        state.attestation_state = "not_evaluated".into();
        assert!(response(vec![RootCatalogPageRow { state, slot: row }]).is_err());
    }
    assert!(response(Vec::new()).is_err());
    Ok(())
}

#[test]
fn root_catalog_distinguishes_missing_from_valid_empty() -> anyhow::Result<()> {
    let mut empty = state()?;
    empty.slot_count = Some(0);
    let page = response(vec![RootCatalogPageRow {
        state: empty,
        slot: None,
    }])?;
    assert!(page.generation().is_some());
    assert!(page.slots().is_empty());
    let mut missing = state()?;
    missing.source_state = "missing".into();
    missing.source_reason_code = Some("media_root_catalog_source_missing".into());
    missing.attestation_state = "not_evaluated".into();
    missing.media_root_catalog_generation_public_id = None;
    missing.attestation_generation = None;
    missing.source_format_version = None;
    missing.source_sha256 = None;
    missing.generation_sha256 = None;
    missing.slot_count = None;
    missing.activated_at = None;
    let page = response(vec![RootCatalogPageRow {
        state: missing,
        slot: None,
    }])?;
    assert!(page.generation().is_none());
    assert!(page.slots().is_empty());
    Ok(())
}
