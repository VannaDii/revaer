//! Bootstrap-only fail-closed catalog transitions; never a readiness grant.

use sqlx::PgPool;

use crate::error::{Result, try_op};

/// Closed source failure states from ADR 557.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCatalogUnavailable {
    /// No configured catalog document exists.
    Missing,
    /// The document failed ownership, permissions or identity checks.
    Untrusted,
    /// The document failed its exact format contract.
    Invalid,
    /// The document exceeded its approved bound.
    BoundExceeded,
    /// The platform cannot establish the source trust contract.
    Unsupported,
}

impl RootCatalogUnavailable {
    const fn fields(self) -> (&'static str, &'static str) {
        match self {
            Self::Missing => ("missing", "media_root_catalog_source_missing"),
            Self::Untrusted => ("untrusted", "media_root_catalog_source_untrusted"),
            Self::Invalid => ("invalid", "media_root_catalog_format_invalid"),
            Self::BoundExceeded => ("bound_exceeded", "media_root_catalog_bound_exceeded"),
            Self::Unsupported => ("unsupported", "media_root_platform_unsupported"),
        }
    }
}

/// Closed attestation failures for a syntactically valid trusted source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootAttestationFailure {
    /// Attestation could not establish all required proof.
    Invalid,
    /// Distinct roots overlap physically.
    Overlap,
    /// Root ancestry is not protected.
    UnsafeAncestry,
    /// Required persistence was not proven.
    DurabilityUnproven,
    /// Required exclusive writer ownership was not proven.
    WriterControlUnproven,
    /// Observed identity changed or differed from the bound root.
    IdentityMismatch,
}

impl RootAttestationFailure {
    const fn reason(self) -> &'static str {
        match self {
            Self::Invalid => "media_root_attestation_invalid",
            Self::Overlap => "media_root_overlap",
            Self::UnsafeAncestry => "media_root_unsafe_ancestry",
            Self::DurabilityUnproven => "media_root_durability_unproven",
            Self::WriterControlUnproven => "media_root_writer_control_unproven",
            Self::IdentityMismatch => "media_root_identity_mismatch",
        }
    }
}

/// Clear current generation continuity when bootstrap cannot load its source.
/// Retained generations and their immutable child rows are never rewritten.
///
/// # Errors
/// Propagates begin, procedure or commit failure; callers must refuse media
/// startup on any database failure, not continue with previously persisted state.
pub async fn mark_root_catalog_unavailable(
    pool: &PgPool,
    state: RootCatalogUnavailable,
) -> Result<()> {
    let mut transaction = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await
        .map_err(try_op("begin root source invalidation"))?;
    let (state, reason) = state.fields();
    sqlx::query("SELECT media_root_catalog_mark_unavailable_v1($1, $2)")
        .bind(state)
        .bind(reason)
        .execute(&mut *transaction)
        .await
        .map_err(try_op("invalidate root catalog source"))?;
    transaction
        .commit()
        .await
        .map_err(try_op("commit root source invalidation"))
}

/// Clear current generation continuity after attestation fails, without
/// misclassifying the source. Bootstrap must retain its process/root locks.
///
/// # Errors
/// Propagates every database failure; no fallback generation is returned.
pub async fn mark_root_attestation_invalid(
    pool: &PgPool,
    failure: RootAttestationFailure,
) -> Result<()> {
    let mut transaction = pool
        .begin_with("BEGIN ISOLATION LEVEL SERIALIZABLE")
        .await
        .map_err(try_op("begin root attestation invalidation"))?;
    sqlx::query("SELECT media_root_catalog_mark_attestation_invalid_v1($1)")
        .bind(failure.reason())
        .execute(&mut *transaction)
        .await
        .map_err(try_op("invalidate root catalog attestation"))?;
    transaction
        .commit()
        .await
        .map_err(try_op("commit root attestation invalidation"))
}
