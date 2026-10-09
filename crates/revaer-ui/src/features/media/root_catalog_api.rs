//! Complete bounded catalog loading before publishing logical source choices.

use std::collections::HashSet;

use revaer_api_models::media_root_contract::{
    RootCatalogGeneration, RootCatalogPageResponse, RootCatalogReadinessResponse,
    RootCatalogReadinessState, RootCatalogSlot, RootKind, RootKindReadiness,
};

use super::{
    association::SourceChoice,
    profile_roots::RootChoice,
    root_catalog_view::{CatalogDisplay, KindDisplay, SlotDisplay},
    root_readiness::RootSummary,
};
use crate::services::api::ApiClient;

const CATALOG_INVALID: &str =
    "The catalog response is inconsistent. Reload before configuring associations.";

pub(crate) async fn fetch_root_catalog(client: &ApiClient) -> Result<CatalogDisplay, &'static str> {
    let mut cursor: Option<String> = None;
    let mut visited = HashSet::new();
    let mut slots = Vec::new();
    let mut generation: Option<RootCatalogGeneration> = None;
    loop {
        let mut path = "/v1/media/root-catalog?limit=200".to_owned();
        if let Some(cursor) = &cursor {
            path.push_str("&cursor=");
            path.push_str(&urlencoding::encode(cursor));
        }
        let page = client
            .get_api_private::<RootCatalogPageResponse>(&path)
            .await
            .map_err(|error| match error.status {
                401 => "Authentication required to read the root catalog.",
                403 => "Access to the root catalog was denied.",
                404 | 405 => "The root catalog is unavailable on this server.",
                _ => "The root catalog could not be loaded. No root choices were published.",
            })?;
        if cursor.is_some() && generation.as_ref() != page.generation() {
            return Err(CATALOG_INVALID);
        }
        generation = page.generation().cloned();
        slots.extend_from_slice(page.slots());
        if slots.len() > 256 {
            return Err(CATALOG_INVALID);
        }
        if let Some(next) = page.next_cursor() {
            if page.slots().is_empty() || !visited.insert(next.to_owned()) {
                return Err(CATALOG_INVALID);
            }
            cursor = Some(next.to_owned());
        } else {
            let expected_count = generation
                .as_ref()
                .map_or(0, |value| usize::from(value.fields().slot_count));
            if slots.len() != expected_count {
                return Err(CATALOG_INVALID);
            }
            return display_catalog(page.state(), &slots);
        }
    }
}

fn display_catalog(
    state: RootCatalogReadinessState,
    slots: &[RootCatalogSlot],
) -> Result<CatalogDisplay, &'static str> {
    let mut kinds = [
        RootKind::Source,
        RootKind::Output,
        RootKind::Workspace,
        RootKind::Backup,
        RootKind::Quarantine,
    ]
    .map(|kind| RootKindReadiness {
        kind,
        attested_slot_count: 0,
        binding_ready_slot_count: 0,
        destructive_ready_slot_count: 0,
    });
    let mut seen = HashSet::new();
    let mut sources = Vec::new();
    let mut profile_roots = Vec::new();
    for slot in slots {
        if !seen.insert(slot.logical_key()) {
            return Err(CATALOG_INVALID);
        }
        for allowed in slot.allowed_kinds() {
            let row = kinds
                .iter_mut()
                .find(|row| row.kind == allowed.kind)
                .ok_or(CATALOG_INVALID)?;
            row.attested_slot_count += 1;
            row.binding_ready_slot_count += u16::from(allowed.binding_ready);
            row.destructive_ready_slot_count += u16::from(allowed.destructive_ready);
            if allowed.kind == RootKind::Source {
                sources.push(SourceChoice {
                    key: slot.logical_key().to_owned(),
                    binding_ready: allowed.binding_ready,
                    destructive_ready: allowed.destructive_ready,
                });
            } else {
                profile_roots.push(RootChoice {
                    key: slot.logical_key().to_owned(),
                    kind: allowed.kind,
                    binding_ready: allowed.binding_ready,
                    destructive_ready: allowed.destructive_ready,
                });
            }
        }
    }
    let readiness = RootCatalogReadinessResponse::new(state, kinds).map_err(|_| CATALOG_INVALID)?;
    Ok(CatalogDisplay {
        summary: RootSummary::from(readiness),
        slots: slots.iter().map(display_slot).collect(),
        sources,
        profile_roots,
    })
}

fn display_slot(slot: &RootCatalogSlot) -> SlotDisplay {
    let fields = slot.fields();
    let evidence = vec![
        (
            "Slot ID",
            fields.media_root_catalog_slot_public_id.to_string(),
        ),
        ("Device", fields.filesystem_device.clone()),
        ("Inode", fields.filesystem_inode.clone()),
        ("Mount", fields.mount_id.clone()),
        ("Filesystem", fields.filesystem_type.clone()),
        ("Durability", fields.durability_class.clone()),
        ("Durability evidence", fields.durability_evidence.clone()),
        ("Writer control", fields.sole_writer_class.clone()),
        ("Writer evidence", fields.sole_writer_evidence.clone()),
        ("Owner UID", fields.owner_uid.to_string()),
        ("Owner GID", fields.owner_gid.to_string()),
        ("Mode", fields.mode_bits.clone()),
        ("Validated", fields.validated_at.clone()),
        ("Identity digest", fields.root_identity_sha256.clone()),
        ("Read", slot.read_capable().to_string()),
        ("Write", slot.write_capable().to_string()),
        ("Create new", slot.create_new_capable().to_string()),
        ("Synchronize", slot.fsync_capable().to_string()),
        ("Rename", slot.rename_capable().to_string()),
        ("Delete", slot.delete_capable().to_string()),
        ("Capacity probe", slot.capacity_probe_capable().to_string()),
    ];
    SlotDisplay {
        key: slot.logical_key().to_owned(),
        requested_path: slot.requested_path().to_owned(),
        canonical_path: slot.canonical_path().to_owned(),
        evidence,
        kinds: slot
            .allowed_kinds()
            .iter()
            .map(|kind| KindDisplay {
                name: match kind.kind {
                    RootKind::Source => "Source",
                    RootKind::Output => "Output",
                    RootKind::Workspace => "Workspace",
                    RootKind::Backup => "Backup",
                    RootKind::Quarantine => "Quarantine",
                },
                binding_ready: kind.binding_ready,
                binding_reason: kind.binding_reason.clone(),
                destructive_ready: kind.destructive_ready,
                destructive_reason: kind.destructive_reason.clone(),
            })
            .collect(),
    }
}
