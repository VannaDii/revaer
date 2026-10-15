//! Validate persisted root readiness before exposing a path-free response.

use revaer_api::models::media_root_contract::{
    RootCatalogReadinessResponse, RootCatalogReadinessState, RootKind, RootKindReadiness,
    RootReadinessError,
};
use revaer_data::media::root_catalog::RootCatalogReadinessRow;

pub(super) fn response(
    rows: Vec<RootCatalogReadinessRow>,
) -> Result<RootCatalogReadinessResponse, RootReadinessError> {
    let rows: [RootCatalogReadinessRow; 5] = rows.try_into().map_err(|_| RootReadinessError)?;
    let state = row_state(&rows[0])?;
    let mut kinds = Vec::with_capacity(5);
    for row in &rows {
        if row_state(row)? != state {
            return Err(RootReadinessError);
        }
        kinds.push(RootKindReadiness {
            kind: match row.root_kind.as_str() {
                "source" => RootKind::Source,
                "output" => RootKind::Output,
                "workspace" => RootKind::Workspace,
                "backup" => RootKind::Backup,
                "quarantine" => RootKind::Quarantine,
                _ => return Err(RootReadinessError),
            },
            attested_slot_count: row
                .attested_slot_count
                .try_into()
                .map_err(|_| RootReadinessError)?,
            binding_ready_slot_count: row
                .binding_ready_slot_count
                .try_into()
                .map_err(|_| RootReadinessError)?,
            destructive_ready_slot_count: row
                .destructive_ready_slot_count
                .try_into()
                .map_err(|_| RootReadinessError)?,
        });
    }
    RootCatalogReadinessResponse::new(state, kinds.try_into().map_err(|_| RootReadinessError)?)
}

fn row_state(
    row: &RootCatalogReadinessRow,
) -> Result<RootCatalogReadinessState, RootReadinessError> {
    let generation = row
        .attestation_generation
        .map(|generation| generation.to_string());
    RootCatalogReadinessState::from_wire(
        &row.source_state,
        row.source_reason_code.as_deref(),
        &row.attestation_state,
        row.attestation_reason_code.as_deref(),
        generation.as_deref(),
    )
}

#[cfg(test)]
pub(super) mod tests;
