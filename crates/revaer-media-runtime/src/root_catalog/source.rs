use std::io;
use std::sync::Arc;

use thiserror::Error;

use super::model::RootCatalog;
use super::parse::RootCatalogParseError;

/// Injected source boundary for startup-only root-catalog loading.
pub trait RootCatalogSource: Send + Sync {
    /// Load one immutable startup snapshot.
    ///
    /// A missing source returns an empty catalog with remediation state. No
    /// result from this boundary constitutes root attestation or readiness.
    ///
    /// # Errors
    ///
    /// Returns [`RootCatalogSourceError`] when the source is untrusted,
    /// changes while loading, exceeds a bound, or has invalid version-1 data.
    fn load(&self) -> Result<RootCatalogLoad, RootCatalogSourceError>;
}

/// Trust class used to admit the local catalog document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCatalogSourceTrust {
    /// Fixed release-package location owned by root and unreadied for service writes.
    Packaged,
    /// Native operator-owned override used for local read-only development flows.
    NativeOverride,
}

/// Bounded source state returned to bootstrap and the future root resolver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCatalogSourceState {
    /// A trusted, valid document was loaded once.
    Loaded,
    /// The configured document was absent and remediation is required.
    Missing,
}

impl RootCatalogSourceState {
    /// Return the stable remediation reason when this state requires one.
    #[must_use]
    pub const fn remediation_reason(self) -> Option<&'static str> {
        match self {
            Self::Loaded => None,
            Self::Missing => Some("media_root_catalog_source_missing"),
        }
    }
}

/// Trusted descriptor metadata retained with a loaded catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RootCatalogFileEvidence {
    pub(super) trust: RootCatalogSourceTrust,
    pub(super) owner_uid: u32,
    pub(super) mode: u32,
    pub(super) document_bytes: usize,
}

impl RootCatalogFileEvidence {
    /// Return the injected source trust class.
    #[must_use]
    pub const fn trust(&self) -> RootCatalogSourceTrust {
        self.trust
    }

    /// Return the owner observed on the stable file descriptor.
    #[must_use]
    pub const fn owner_uid(&self) -> u32 {
        self.owner_uid
    }

    /// Return the Unix mode observed on the stable file descriptor.
    #[must_use]
    pub const fn mode(&self) -> u32 {
        self.mode
    }

    /// Return the exact raw document byte count read from the descriptor.
    #[must_use]
    pub const fn document_bytes(&self) -> usize {
        self.document_bytes
    }
}

/// One startup catalog result, including missing-source remediation state.
#[derive(Debug, Clone)]
pub struct RootCatalogLoad {
    state: RootCatalogSourceState,
    catalog: RootCatalog,
    file_evidence: Option<RootCatalogFileEvidence>,
    retained_source: Option<Arc<super::local_file::RetainedSource>>,
}

impl RootCatalogLoad {
    pub(super) fn missing() -> Self {
        Self {
            state: RootCatalogSourceState::Missing,
            catalog: RootCatalog::empty(),
            file_evidence: None,
            retained_source: None,
        }
    }

    pub(super) fn loaded(
        catalog: RootCatalog,
        file_evidence: RootCatalogFileEvidence,
        retained_source: super::local_file::RetainedSource,
    ) -> Self {
        Self {
            state: RootCatalogSourceState::Loaded,
            catalog,
            file_evidence: Some(file_evidence),
            retained_source: Some(Arc::new(retained_source)),
        }
    }

    /// Revalidate the retained source and its protected directory chain.
    /// Clones share the same descriptors, not a newly loaded document.
    /// A missing-source result has no descriptor to revalidate and remains
    /// missing; this method never grants root readiness.
    ///
    /// # Errors
    /// Rejects changed identity, metadata, ancestry, or source trust.
    pub fn revalidate(&self) -> Result<(), RootCatalogSourceError> {
        match (&self.retained_source, self.state) {
            (Some(source), _) => source.revalidate(),
            (None, RootCatalogSourceState::Missing) => Ok(()),
            (None, RootCatalogSourceState::Loaded) => {
                Err(RootCatalogSourceError::SourceUntrusted {
                    violation: RootCatalogTrustViolation::SourceChanged,
                })
            }
        }
    }

    #[cfg(test)]
    pub(super) const fn loaded_for_encoding_test(
        catalog: RootCatalog,
        file_evidence: RootCatalogFileEvidence,
    ) -> Self {
        Self {
            state: RootCatalogSourceState::Loaded,
            catalog,
            file_evidence: Some(file_evidence),
            retained_source: None,
        }
    }

    /// Return whether a trusted document was loaded or remediation is required.
    #[must_use]
    pub const fn state(&self) -> RootCatalogSourceState {
        self.state
    }

    /// Return the validated catalog. Missing sources always return an empty catalog.
    #[must_use]
    pub const fn catalog(&self) -> &RootCatalog {
        &self.catalog
    }

    /// Return stable descriptor evidence when a document was loaded.
    #[must_use]
    pub const fn file_evidence(&self) -> Option<&RootCatalogFileEvidence> {
        self.file_evidence.as_ref()
    }
}

/// Exact local-source trust violation class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootCatalogTrustViolation {
    /// The configured location was not an exact bounded absolute path.
    InvalidLocation,
    /// A location component required path normalization or traversal.
    NonCanonicalLocation,
    /// The injected operator or service identity did not match the process.
    PolicyIdentityMismatch,
    /// A packaged service running as root could mutate a root-owned source.
    PackagedServiceIsRoot,
    /// A directory component had an unsafe type, owner, or mode.
    UnsafeDirectory,
    /// The final source was not one regular non-symlink file.
    UnsafeFileType,
    /// The regular source had more than one hard link.
    MultipleHardLinks,
    /// The source owner did not match the injected trust policy.
    OwnerMismatch,
    /// The source was group- or world-writable.
    UntrustedWritableMode,
    /// The packaged service could write the source or one of its directories.
    WritableByService,
    /// Descriptor identity or security metadata changed during loading.
    SourceChanged,
}

/// Fail-closed trusted local-source error.
#[derive(Debug, Error)]
pub enum RootCatalogSourceError {
    /// A trust-policy invariant failed without an operating-system error.
    #[error("root catalog source is untrusted: {violation:?}")]
    SourceUntrusted {
        /// Exact bounded trust violation.
        violation: RootCatalogTrustViolation,
    },
    /// A descriptor operation failed while proving the source.
    #[error("root catalog source operation failed: {operation}")]
    Filesystem {
        /// Bounded operation name without source path material.
        operation: &'static str,
        /// Original operating-system error.
        #[source]
        source: io::Error,
    },
    /// The descriptor yielded more than the reviewed raw byte bound.
    #[error("root catalog document exceeds {maximum_bytes} bytes")]
    BoundExceeded {
        /// Reviewed maximum raw document size.
        maximum_bytes: usize,
    },
    /// The complete descriptor contents failed exact version-1 parsing.
    #[error(transparent)]
    FormatInvalid(#[from] RootCatalogParseError),
    /// The host cannot provide the reviewed descriptor security semantics.
    #[error("root catalog source platform is unsupported")]
    PlatformUnsupported,
}

impl RootCatalogSourceError {
    /// Return the stable ADR 550 source-loading reason code.
    #[must_use]
    pub const fn reason_code(&self) -> &'static str {
        match self {
            Self::SourceUntrusted { .. } | Self::Filesystem { .. } => {
                "media_root_catalog_source_untrusted"
            }
            Self::BoundExceeded { .. } => "media_root_catalog_bound_exceeded",
            Self::FormatInvalid(source) => source.reason_code(),
            Self::PlatformUnsupported => "media_root_platform_unsupported",
        }
    }
}
