//! A single statement keeps catalog metadata and its slot page in one snapshot.

use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Row, postgres::PgRow};
use uuid::Uuid;

use crate::error::{Result, try_op};

const PAGE: &str = "SELECT s.*, p.* FROM media_root_catalog_state_get_v1() AS s LEFT JOIN media_root_catalog_slot_page_v1($1, $2, $3) AS p ON true";

/// Persisted singleton and active-generation metadata. Absence is not readiness.
#[derive(Clone, PartialEq, Eq, FromRow)]
pub struct RootCatalogStateRow {
    /// Closed source state.
    pub source_state: String,
    /// Closed source failure reason.
    pub source_reason_code: Option<String>,
    /// Closed attestation state.
    pub attestation_state: String,
    /// Closed attestation failure reason.
    pub attestation_reason_code: Option<String>,
    /// Public active-generation identity.
    pub media_root_catalog_generation_public_id: Option<Uuid>,
    /// Positive active-generation number.
    pub attestation_generation: Option<i64>,
    /// Version of the attested source format.
    pub source_format_version: Option<i16>,
    /// Source digest bytes.
    pub source_sha256: Option<Vec<u8>>,
    /// Generation digest bytes.
    pub generation_sha256: Option<Vec<u8>>,
    /// Total slots in the active generation.
    pub slot_count: Option<i16>,
    /// First activation time.
    pub activated_at: Option<DateTime<Utc>>,
    /// Latest reconciliation time.
    pub reconciled_at: DateTime<Utc>,
}

/// One allowed-kind row of a bounded slot page; deliberately has no Debug output.
#[derive(Clone, PartialEq, Eq, FromRow)]
pub struct RootCatalogSlotRow {
    /// Stable slot public identity.
    pub media_root_catalog_slot_public_id: Uuid,
    /// Logical selector.
    pub logical_key: String,
    /// Ordered allowed root kind.
    pub allowed_root_kind: String,
    /// Operator-supplied absolute path.
    pub requested_path: String,
    /// Attested canonical path.
    pub canonical_path: String,
    /// Unsigned device identity bytes.
    pub filesystem_device: Vec<u8>,
    /// Unsigned inode identity bytes.
    pub filesystem_inode: Vec<u8>,
    /// Attested mount identity.
    pub mount_id: i64,
    /// Attested filesystem name.
    pub filesystem_type: String,
    /// Seven explicit probe bits in contract order.
    pub capability_mask: i16,
    /// Closed durability class.
    pub durability_class: String,
    /// Closed durability proof class.
    pub durability_evidence: String,
    /// Closed writer class.
    pub sole_writer_class: String,
    /// Closed writer proof class.
    pub sole_writer_evidence: String,
    /// Unsigned owner identity stored in bigint.
    pub owner_uid: i64,
    /// Unsigned group identity stored in bigint.
    pub owner_gid: i64,
    /// Permission and special mode bits.
    pub mode_bits: i32,
    /// Attestation observation time.
    pub validated_at: DateTime<Utc>,
    /// Attested identity digest bytes.
    pub root_identity_sha256: Vec<u8>,
    /// Eligibility for this kind.
    pub binding_ready: bool,
    /// Closed binding failure reason.
    pub binding_reason_code: Option<String>,
    /// Eligibility for destructive use of this kind.
    pub destructive_ready: bool,
    /// Closed destructive failure reason.
    pub destructive_reason_code: Option<String>,
    /// Whether another slot follows this page.
    pub page_has_more: bool,
}

/// Metadata repeats on each joined kind row, including one row for an empty page.
pub struct RootCatalogPageRow {
    /// Snapshot metadata.
    pub state: RootCatalogStateRow,
    /// Absent only for an empty page.
    pub slot: Option<RootCatalogSlotRow>,
}

impl<'row> FromRow<'row, PgRow> for RootCatalogPageRow {
    fn from_row(row: &'row PgRow) -> std::result::Result<Self, sqlx::Error> {
        let identity: Option<Uuid> = row.try_get("media_root_catalog_slot_public_id")?;
        Ok(Self {
            state: RootCatalogStateRow::from_row(row)?,
            slot: identity
                .map(|_| RootCatalogSlotRow::from_row(row))
                .transpose()?,
        })
    }
}

/// Read metadata and at most 200 slots through the approved procedures.
///
/// # Errors
/// Propagates invalid cursor, input, and database failures without fallback data.
pub async fn read_root_catalog_page(
    pool: &PgPool,
    limit: i16,
    cursor_key: Option<&str>,
    cursor_id: Option<Uuid>,
) -> Result<Vec<RootCatalogPageRow>> {
    sqlx::query_as(PAGE)
        .bind(limit)
        .bind(cursor_key)
        .bind(cursor_id)
        .fetch_all(pool)
        .await
        .map_err(try_op("read root catalog page"))
}
