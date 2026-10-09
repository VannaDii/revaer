//! Path-free operator state; readiness counts never confer execution authority.

use revaer_api_models::media_root_contract::{
    RootAttestationFailure, RootCatalogReadinessResponse, RootCatalogReadinessState, RootKind,
    RootSourceFailure,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RootLoadFailure {
    Authentication,
    Forbidden,
    Unavailable,
    Request,
}

impl RootLoadFailure {
    pub(crate) const fn from_status(status: u16) -> Self {
        match status {
            401 => Self::Authentication,
            403 => Self::Forbidden,
            404 => Self::Unavailable,
            _ => Self::Request,
        }
    }

    pub(crate) const fn message(self) -> &'static str {
        match self {
            Self::Authentication => "Authentication required to read root readiness.",
            Self::Forbidden => "Access to root readiness was denied.",
            Self::Unavailable => "Root readiness is unavailable on this server.",
            Self::Request => "Root readiness could not be loaded. Retry the request.",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RootCounts {
    pub kind: &'static str,
    pub attested: u16,
    pub binding: u16,
    pub destructive: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RootSummary {
    pub source: &'static str,
    pub attestation: &'static str,
    pub reason: Option<&'static str>,
    pub generation: Option<u64>,
    pub kinds: [RootCounts; 5],
}

impl From<RootCatalogReadinessResponse> for RootSummary {
    fn from(response: RootCatalogReadinessResponse) -> Self {
        let (source, attestation, reason, generation) = match response.state() {
            RootCatalogReadinessState::SourceUnavailable(reason) => (
                "Unavailable",
                "Not evaluated",
                Some(source_remediation(reason)),
                None,
            ),
            RootCatalogReadinessState::AttestationInvalid(reason) => (
                "Ready",
                "Invalid",
                Some(attestation_remediation(reason)),
                None,
            ),
            RootCatalogReadinessState::Ready { generation } => {
                ("Ready", "Ready", None, Some(generation.get()))
            }
        };
        Self {
            source,
            attestation,
            reason,
            generation,
            kinds: response.kinds().map(|row| RootCounts {
                kind: match row.kind {
                    RootKind::Source => "Source",
                    RootKind::Output => "Output",
                    RootKind::Workspace => "Workspace",
                    RootKind::Backup => "Backup",
                    RootKind::Quarantine => "Quarantine",
                },
                attested: row.attested_slot_count,
                binding: row.binding_ready_slot_count,
                destructive: row.destructive_ready_slot_count,
            }),
        }
    }
}

const fn source_remediation(reason: RootSourceFailure) -> &'static str {
    match reason {
        RootSourceFailure::Missing => "Supply the deployment root catalog.",
        RootSourceFailure::Untrusted => "Correct catalog ownership, permissions, and location.",
        RootSourceFailure::Invalid => "Correct the catalog document format.",
        RootSourceFailure::BoundExceeded => "Reduce the catalog to the supported bounds.",
        RootSourceFailure::Unsupported => "Use a supported Linux deployment.",
    }
}

const fn attestation_remediation(reason: RootAttestationFailure) -> &'static str {
    match reason {
        RootAttestationFailure::Invalid => "Resolve the deployment root attestation failure.",
        RootAttestationFailure::Overlap => "Remove physical overlap between separate root slots.",
        RootAttestationFailure::UnsafeAncestry => "Correct unsafe root ancestry and permissions.",
        RootAttestationFailure::DurabilityUnproven => {
            "Establish required restart durability for the roots."
        }
        RootAttestationFailure::WriterControlUnproven => {
            "Establish required sole-writer control for the roots."
        }
        RootAttestationFailure::IdentityMismatch => {
            "Restore the expected root identity before reattestation."
        }
    }
}

#[cfg(test)]
mod tests;
