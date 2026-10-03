//! Root catalog readers and bootstrap-only fail-closed stored-procedure calls.

use sqlx::PgPool;

mod invalidation;
mod page;
mod reconciliation;
pub use invalidation::{
    RootAttestationFailure, RootCatalogUnavailable, mark_root_attestation_invalid,
    mark_root_catalog_unavailable,
};
pub use page::{
    RootCatalogPageRow, RootCatalogSlotRow, RootCatalogStateRow, read_root_catalog_page,
};
pub use reconciliation::{
    ActivatedRootCatalog, RootCatalogGenerationInput, RootCatalogReconciliation,
    RootCatalogSlotInput,
};

use crate::error::{Result, try_op};

pub(super) const READINESS: &str = "SELECT source_state, source_reason_code, attestation_state, attestation_reason_code, attestation_generation, root_kind, attested_slot_count, binding_ready_slot_count, destructive_ready_slot_count FROM media_root_catalog_readiness_get_v1()";

/// One of the five ordered kind rows from a single catalog snapshot.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct RootCatalogReadinessRow {
    /// Closed catalog source state.
    pub source_state: String,
    /// Closed reason when the source is unavailable.
    pub source_reason_code: Option<String>,
    /// Closed attestation state.
    pub attestation_state: String,
    /// Closed reason when attestation is invalid.
    pub attestation_reason_code: Option<String>,
    /// Active positive generation, absent when unavailable.
    pub attestation_generation: Option<i64>,
    /// Root kind in source-through-quarantine order.
    pub root_kind: String,
    /// Active allowed-kind attestations.
    pub attested_slot_count: i32,
    /// Attestations satisfying binding requirements.
    pub binding_ready_slot_count: i32,
    /// Binding-ready attestations satisfying destructive requirements.
    pub destructive_ready_slot_count: i32,
}

/// Read all five path-free rows in one database statement/snapshot.
///
/// # Errors
/// Returns the database failure; no fallback state or counts are synthesized.
pub async fn read_root_catalog_readiness(pool: &PgPool) -> Result<Vec<RootCatalogReadinessRow>> {
    sqlx::query_as(READINESS)
        .fetch_all(pool)
        .await
        .map_err(try_op("read root catalog readiness"))
}
