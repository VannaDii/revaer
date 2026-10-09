//! Catalog state wire adapter reusing the readiness contract's closed states.

use std::num::NonZeroU64;

use super::{
    RootAttestationFailure, RootCatalogError, RootCatalogReadinessState, RootSourceFailure,
};

pub(super) const fn source_name(reason: RootSourceFailure) -> &'static str {
    match reason {
        RootSourceFailure::Missing => "missing",
        RootSourceFailure::Untrusted => "untrusted",
        RootSourceFailure::Invalid => "invalid",
        RootSourceFailure::BoundExceeded => "bound_exceeded",
        RootSourceFailure::Unsupported => "unsupported",
    }
}

pub(super) fn decode(
    source: &str,
    source_reason: Option<&str>,
    attestation: &str,
    attestation_reason: Option<&str>,
    generation: Option<u64>,
) -> Result<RootCatalogReadinessState, RootCatalogError> {
    match (
        source,
        source_reason,
        attestation,
        attestation_reason,
        generation,
    ) {
        ("ready", None, "ready", None, Some(generation)) => {
            let generation = NonZeroU64::new(generation).ok_or(RootCatalogError)?;
            Ok(RootCatalogReadinessState::Ready { generation })
        }
        (source, Some(reason), "not_evaluated", None, None) => [
            RootSourceFailure::Missing,
            RootSourceFailure::Untrusted,
            RootSourceFailure::Invalid,
            RootSourceFailure::BoundExceeded,
            RootSourceFailure::Unsupported,
        ]
        .into_iter()
        .find(|failure| source_name(*failure) == source && failure.reason_code() == reason)
        .map(RootCatalogReadinessState::SourceUnavailable)
        .ok_or(RootCatalogError),
        ("ready", None, "invalid", Some(reason), None) => [
            RootAttestationFailure::Invalid,
            RootAttestationFailure::Overlap,
            RootAttestationFailure::UnsafeAncestry,
            RootAttestationFailure::DurabilityUnproven,
            RootAttestationFailure::WriterControlUnproven,
            RootAttestationFailure::IdentityMismatch,
        ]
        .into_iter()
        .find(|failure| failure.reason_code() == reason)
        .map(RootCatalogReadinessState::AttestationInvalid)
        .ok_or(RootCatalogError),
        _ => Err(RootCatalogError),
    }
}
