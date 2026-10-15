use std::num::NonZeroU64;

use revaer_api_models::media_root_contract::{RootKindReadiness, RootReadinessError};

use super::*;

fn empty_counts() -> [RootKindReadiness; 5] {
    [
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
    })
}

#[test]
fn empty_catalog_keeps_generation_but_has_no_ready_slots() -> Result<(), RootReadinessError> {
    let summary = RootSummary::from(RootCatalogReadinessResponse::new(
        RootCatalogReadinessState::Ready {
            generation: NonZeroU64::MIN,
        },
        empty_counts(),
    )?);
    assert_eq!(summary.source, "Ready");
    assert_eq!(summary.attestation, "Ready");
    assert_eq!(summary.generation, Some(1));
    assert_eq!(summary.reason, None);
    assert!(
        summary
            .kinds
            .iter()
            .all(|row| row.attested == 0 && row.binding == 0 && row.destructive == 0)
    );
    assert_eq!(
        summary.kinds.map(|row| row.kind),
        ["Source", "Output", "Workspace", "Backup", "Quarantine"]
    );
    Ok(())
}

#[test]
fn binding_and_destructive_counts_remain_distinct() -> Result<(), RootReadinessError> {
    let counts = empty_counts().map(|row| RootKindReadiness {
        attested_slot_count: 3,
        binding_ready_slot_count: 2,
        destructive_ready_slot_count: u16::from(row.kind == RootKind::Output),
        ..row
    });
    let summary = RootSummary::from(RootCatalogReadinessResponse::new(
        RootCatalogReadinessState::Ready {
            generation: NonZeroU64::MIN,
        },
        counts,
    )?);
    assert!(
        summary
            .kinds
            .iter()
            .all(|row| row.attested == 3 && row.binding == 2)
    );
    assert_eq!(summary.kinds.map(|row| row.destructive), [0, 1, 0, 0, 0]);
    Ok(())
}

#[test]
fn source_failures_do_not_claim_attestation() -> Result<(), RootReadinessError> {
    for (failure, expected) in [
        (
            RootSourceFailure::Missing,
            "Supply the deployment root catalog.",
        ),
        (
            RootSourceFailure::Untrusted,
            "Correct catalog ownership, permissions, and location.",
        ),
        (
            RootSourceFailure::Invalid,
            "Correct the catalog document format.",
        ),
        (
            RootSourceFailure::BoundExceeded,
            "Reduce the catalog to the supported bounds.",
        ),
        (
            RootSourceFailure::Unsupported,
            "Use a supported Linux deployment.",
        ),
    ] {
        let summary = RootSummary::from(RootCatalogReadinessResponse::new(
            RootCatalogReadinessState::SourceUnavailable(failure),
            empty_counts(),
        )?);
        assert_eq!(summary.source, "Unavailable");
        assert_eq!(summary.attestation, "Not evaluated");
        assert_eq!(summary.generation, None);
        assert_eq!(summary.reason, Some(expected));
    }
    Ok(())
}

#[test]
fn invalid_attestation_preserves_source_state() -> Result<(), RootReadinessError> {
    for failure in [
        RootAttestationFailure::Invalid,
        RootAttestationFailure::Overlap,
        RootAttestationFailure::UnsafeAncestry,
        RootAttestationFailure::DurabilityUnproven,
        RootAttestationFailure::WriterControlUnproven,
        RootAttestationFailure::IdentityMismatch,
    ] {
        let summary = RootSummary::from(RootCatalogReadinessResponse::new(
            RootCatalogReadinessState::AttestationInvalid(failure),
            empty_counts(),
        )?);
        assert_eq!(summary.source, "Ready");
        assert_eq!(summary.attestation, "Invalid");
        assert_eq!(summary.generation, None);
        assert_eq!(summary.reason, Some(attestation_remediation(failure)));
    }
    Ok(())
}

#[test]
fn transport_failures_have_bounded_messages() {
    for (status, expected, message) in [
        (
            401,
            RootLoadFailure::Authentication,
            "Authentication required to read root readiness.",
        ),
        (
            403,
            RootLoadFailure::Forbidden,
            "Access to root readiness was denied.",
        ),
        (
            404,
            RootLoadFailure::Unavailable,
            "Root readiness is unavailable on this server.",
        ),
        (
            0,
            RootLoadFailure::Request,
            "Root readiness could not be loaded. Retry the request.",
        ),
        (
            500,
            RootLoadFailure::Request,
            "Root readiness could not be loaded. Retry the request.",
        ),
    ] {
        let failure = RootLoadFailure::from_status(status);
        assert_eq!(failure, expected);
        assert_eq!(failure.message(), message);
    }
}
