//! Bounded, versioned media-root catalog parsing and trusted local loading.
//!
//! Catalog parsing establishes only deployment-provided logical authority. It
//! does not attest a root, bind a profile, or grant write or destructive
//! readiness. Callers must inject a [`RootCatalogSource`] during bootstrap and
//! pass the resulting typed catalog to the separately reviewed root resolver.

mod directory;
mod identity;
mod local_file;
mod model;
#[cfg(target_os = "linux")]
mod mount_source;
#[cfg(target_os = "linux")]
mod mounts;
#[cfg(target_os = "linux")]
mod observation;
#[cfg(target_os = "linux")]
mod opened;
mod parse;
mod source;
mod strict_json;
mod write_probe;

pub use directory::{OpenedRootDirectory, RootDirectoryError};
pub use identity::{
    RootCatalogIdentityEncoding, RootIdentityEncodingError, RootSlotIdentityClaims,
    RootSlotIdentityEncoding, encode_root_catalog_identity_v1,
};
pub use local_file::{PACKAGED_ROOT_CATALOG_PATH, TrustedLocalRootCatalogSource};
pub use model::{
    DurabilityClass, DurabilityEvidence, RootCatalog, RootCatalogSlot, RootKind, SoleWriterClass,
    SoleWriterEvidence,
};
#[cfg(target_os = "linux")]
pub use mount_source::{ProcRootMountSource, RootMountReadError, RootMountSource};
#[cfg(target_os = "linux")]
pub use mounts::{RootMountError, RootMountTopology};
#[cfg(target_os = "linux")]
pub use observation::RootDirectoryObservation;
#[cfg(target_os = "linux")]
pub use opened::OpenedRootCatalog;
pub use parse::{
    MAX_ROOT_CATALOG_DOCUMENT_BYTES, MAX_ROOT_CATALOG_KEY_BYTES, MAX_ROOT_CATALOG_PATH_BYTES,
    MAX_ROOT_CATALOG_SLOTS, RootCatalogParseError, parse_root_catalog_v1,
};
pub use source::{
    RootCatalogFileEvidence, RootCatalogLoad, RootCatalogSource, RootCatalogSourceError,
    RootCatalogSourceState, RootCatalogSourceTrust, RootCatalogTrustViolation,
};
pub use write_probe::RootWriteProbeError;

#[cfg(test)]
mod tests;
