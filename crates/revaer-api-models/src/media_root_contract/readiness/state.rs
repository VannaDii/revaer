use std::num::NonZeroU64;

use super::RootReadinessError;

/// A closed ADR 550 source failure; no active generation survives this state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSourceFailure {
    /// No catalog document was supplied.
    Missing,
    /// The document or its location failed source trust checks.
    Untrusted,
    /// The complete document failed the version 1 format contract.
    Invalid,
    /// The document exceeded the accepted source bound.
    BoundExceeded,
    /// Source loading is unsupported on the current platform.
    Unsupported,
}

impl RootSourceFailure {
    /// Return the exact bounded diagnostic for this source state.
    #[must_use]
    pub const fn reason_code(self) -> &'static str {
        match self {
            Self::Missing => "media_root_catalog_source_missing",
            Self::Untrusted => "media_root_catalog_source_untrusted",
            Self::Invalid => "media_root_catalog_format_invalid",
            Self::BoundExceeded => "media_root_catalog_bound_exceeded",
            Self::Unsupported => "media_root_platform_unsupported",
        }
    }

    pub(super) const fn source_state(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Untrusted => "untrusted",
            Self::Invalid => "invalid",
            Self::BoundExceeded => "bound_exceeded",
            Self::Unsupported => "unsupported",
        }
    }

    fn from_wire(state: &str, reason: &str) -> Result<Self, RootReadinessError> {
        let failure = match state {
            "missing" => Self::Missing,
            "untrusted" => Self::Untrusted,
            "invalid" => Self::Invalid,
            "bound_exceeded" => Self::BoundExceeded,
            "unsupported" => Self::Unsupported,
            _ => return Err(RootReadinessError),
        };
        if failure.reason_code() != reason {
            return Err(RootReadinessError);
        }
        Ok(failure)
    }
}

/// A closed ADR 557 attestation failure after a valid source was loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootAttestationFailure {
    /// Root proof failed an attestation invariant.
    Invalid,
    /// Separate slots overlap physically.
    Overlap,
    /// A path's ancestry is unsafe.
    UnsafeAncestry,
    /// Required restart persistence was not proved.
    DurabilityUnproven,
    /// Required sole-writer control was not proved.
    WriterControlUnproven,
    /// Observed root identity differs from the expected identity.
    IdentityMismatch,
}

impl RootAttestationFailure {
    /// Return the exact bounded diagnostic for this failed attestation.
    #[must_use]
    pub const fn reason_code(self) -> &'static str {
        match self {
            Self::Invalid => "media_root_attestation_invalid",
            Self::Overlap => "media_root_overlap",
            Self::UnsafeAncestry => "media_root_unsafe_ancestry",
            Self::DurabilityUnproven => "media_root_durability_unproven",
            Self::WriterControlUnproven => "media_root_writer_control_unproven",
            Self::IdentityMismatch => "media_root_identity_mismatch",
        }
    }

    fn from_wire(reason: &str) -> Result<Self, RootReadinessError> {
        match reason {
            "media_root_attestation_invalid" => Ok(Self::Invalid),
            "media_root_overlap" => Ok(Self::Overlap),
            "media_root_unsafe_ancestry" => Ok(Self::UnsafeAncestry),
            "media_root_durability_unproven" => Ok(Self::DurabilityUnproven),
            "media_root_writer_control_unproven" => Ok(Self::WriterControlUnproven),
            "media_root_identity_mismatch" => Ok(Self::IdentityMismatch),
            _ => Err(RootReadinessError),
        }
    }
}

/// Coherent source and attestation states, without filesystem or write authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCatalogReadinessState {
    /// The source is unavailable and attestation was not evaluated.
    SourceUnavailable(RootSourceFailure),
    /// The source is ready but attestation failed, clearing the active generation.
    AttestationInvalid(RootAttestationFailure),
    /// Both stages are ready, including a valid catalog with zero slots.
    Ready {
        /// Positive catalog-wide generation, bounded to PostgreSQL `bigint`.
        generation: NonZeroU64,
    },
}

impl RootCatalogReadinessState {
    pub(super) fn validate(self) -> Result<(), RootReadinessError> {
        if let Self::Ready { generation } = self {
            i64::try_from(generation.get()).map_err(|_| RootReadinessError)?;
        }
        Ok(())
    }

    pub(super) fn from_wire(
        source: &str,
        source_reason: Option<&str>,
        attestation: &str,
        attestation_reason: Option<&str>,
        generation: Option<&str>,
    ) -> Result<Self, RootReadinessError> {
        match (
            source,
            source_reason,
            attestation,
            attestation_reason,
            generation,
        ) {
            (source, Some(reason), "not_evaluated", None, None) => Ok(Self::SourceUnavailable(
                RootSourceFailure::from_wire(source, reason)?,
            )),
            ("ready", None, "invalid", Some(reason), None) => Ok(Self::AttestationInvalid(
                RootAttestationFailure::from_wire(reason)?,
            )),
            ("ready", None, "ready", None, Some(generation)) => {
                let parsed = generation
                    .parse::<NonZeroU64>()
                    .map_err(|_| RootReadinessError)?;
                if parsed.to_string() != generation {
                    return Err(RootReadinessError);
                }
                let state = Self::Ready { generation: parsed };
                state.validate()?;
                Ok(state)
            }
            _ => Err(RootReadinessError),
        }
    }
}
