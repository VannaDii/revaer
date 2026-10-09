//! Validate and group the single-snapshot administrative catalog page.

use std::fmt::Write;

use revaer_api::models::media_root_contract::{
    RootCatalogAllowedKind, RootCatalogCapability, RootCatalogCursor, RootCatalogError,
    RootCatalogGeneration, RootCatalogGenerationFields, RootCatalogPageResponse,
    RootCatalogReadinessState, RootCatalogSlot, RootCatalogSlotFields, RootKind,
};
use revaer_data::media::root_catalog::{
    RootCatalogPageRow, RootCatalogSlotRow, RootCatalogStateRow,
};

pub(super) fn response(
    rows: Vec<RootCatalogPageRow>,
) -> Result<RootCatalogPageResponse, RootCatalogError> {
    let mut rows = rows.into_iter();
    let first = rows.next().ok_or(RootCatalogError)?;
    let state = first.state;
    let generation = state.attestation_generation.map(|value| value.to_string());
    let readiness = RootCatalogReadinessState::from_wire(
        &state.source_state,
        state.source_reason_code.as_deref(),
        &state.attestation_state,
        state.attestation_reason_code.as_deref(),
        generation.as_deref(),
    )
    .map_err(|_| RootCatalogError)?;
    let metadata = metadata(&state)?;
    let mut slots: Vec<RootCatalogSlotFields> = Vec::new();
    let has_more = first.slot.as_ref().is_some_and(|row| row.page_has_more);
    let empty = first.slot.is_none();
    if let Some(row) = first.slot {
        slots.push(slot_fields(row)?);
    }
    for row in rows {
        if empty || row.state != state {
            return Err(RootCatalogError);
        }
        let row = row.slot.ok_or(RootCatalogError)?;
        if row.page_has_more != has_more {
            return Err(RootCatalogError);
        }
        let mut fields = slot_fields(row)?;
        if let Some(last) = slots.last_mut()
            && last.media_root_catalog_slot_public_id == fields.media_root_catalog_slot_public_id
        {
            let kind = fields.allowed_kinds.pop().ok_or(RootCatalogError)?;
            fields.allowed_kinds.clone_from(&last.allowed_kinds);
            if &fields != last {
                return Err(RootCatalogError);
            }
            last.allowed_kinds.push(kind);
        } else {
            slots.push(fields);
        }
    }
    let next_cursor = if has_more {
        let last = slots.last().ok_or(RootCatalogError)?;
        Some(
            RootCatalogCursor::new(&last.logical_key, last.media_root_catalog_slot_public_id)
                .and_then(|cursor| cursor.encode())
                .map_err(|_| RootCatalogError)?,
        )
    } else {
        None
    };
    let slots = slots
        .into_iter()
        .map(RootCatalogSlot::new)
        .collect::<Result<_, _>>()?;
    RootCatalogPageResponse::new(readiness, metadata, slots, next_cursor)
}

fn metadata(
    state: &RootCatalogStateRow,
) -> Result<Option<RootCatalogGeneration>, RootCatalogError> {
    let Some(generation) = state.attestation_generation else {
        if state.media_root_catalog_generation_public_id.is_some()
            || state.source_format_version.is_some()
            || state.source_sha256.is_some()
            || state.generation_sha256.is_some()
            || state.slot_count.is_some()
            || state.activated_at.is_some()
        {
            return Err(RootCatalogError);
        }
        return Ok(None);
    };
    if state.source_format_version != Some(1) {
        return Err(RootCatalogError);
    }
    RootCatalogGeneration::new(RootCatalogGenerationFields {
        media_root_catalog_generation_public_id: state
            .media_root_catalog_generation_public_id
            .ok_or(RootCatalogError)?,
        generation: generation.to_string(),
        source_sha256: hex(state.source_sha256.as_deref().ok_or(RootCatalogError)?)?,
        generation_sha256: hex(state.generation_sha256.as_deref().ok_or(RootCatalogError)?)?,
        slot_count: state
            .slot_count
            .ok_or(RootCatalogError)?
            .try_into()
            .map_err(|_| RootCatalogError)?,
        activated_at: state.activated_at.ok_or(RootCatalogError)?.to_rfc3339(),
        reconciled_at: state.reconciled_at.to_rfc3339(),
    })
    .map(Some)
}

fn slot_fields(row: RootCatalogSlotRow) -> Result<RootCatalogSlotFields, RootCatalogError> {
    if !(0..=127).contains(&row.capability_mask) || !(0..=4095).contains(&row.mode_bits) {
        return Err(RootCatalogError);
    }
    let capability = |bit| RootCatalogCapability::new(row.capability_mask & bit != 0);
    Ok(RootCatalogSlotFields {
        media_root_catalog_slot_public_id: row.media_root_catalog_slot_public_id,
        logical_key: row.logical_key,
        requested_path: row.requested_path,
        canonical_path: row.canonical_path,
        filesystem_device: hex(&row.filesystem_device)?,
        filesystem_inode: hex(&row.filesystem_inode)?,
        mount_id: row.mount_id.to_string(),
        filesystem_type: row.filesystem_type,
        read_capable: capability(1),
        write_capable: capability(2),
        create_new_capable: capability(4),
        fsync_capable: capability(8),
        rename_capable: capability(16),
        delete_capable: capability(32),
        capacity_probe_capable: capability(64),
        durability_class: row.durability_class,
        durability_evidence: row.durability_evidence,
        sole_writer_class: row.sole_writer_class,
        sole_writer_evidence: row.sole_writer_evidence,
        owner_uid: row.owner_uid.try_into().map_err(|_| RootCatalogError)?,
        owner_gid: row.owner_gid.try_into().map_err(|_| RootCatalogError)?,
        mode_bits: format!("{:04o}", row.mode_bits),
        validated_at: row.validated_at.to_rfc3339(),
        root_identity_sha256: hex(&row.root_identity_sha256)?,
        allowed_kinds: vec![RootCatalogAllowedKind {
            kind: match row.allowed_root_kind.as_str() {
                "source" => RootKind::Source,
                "output" => RootKind::Output,
                "workspace" => RootKind::Workspace,
                "backup" => RootKind::Backup,
                "quarantine" => RootKind::Quarantine,
                _ => return Err(RootCatalogError),
            },
            binding_ready: row.binding_ready,
            binding_reason: row.binding_reason_code,
            destructive_ready: row.destructive_ready,
            destructive_reason: row.destructive_reason_code,
        }],
    })
}

fn hex(bytes: &[u8]) -> Result<String, RootCatalogError> {
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(value, "{byte:02x}").map_err(|_| RootCatalogError)?;
    }
    Ok(value)
}
