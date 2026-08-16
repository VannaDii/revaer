//! Bounded, versioned media-root catalog parsing and trusted local loading.
//!
//! Catalog parsing establishes only deployment-provided logical authority. It
//! does not attest a root, bind a profile, or grant write or destructive
//! readiness. Callers must inject a [`RootCatalogSource`] during bootstrap and
//! pass the resulting typed catalog to the separately reviewed root resolver.

mod local_file;
mod model;
mod parse;
mod source;

pub use local_file::{PACKAGED_ROOT_CATALOG_PATH, TrustedLocalRootCatalogSource};
pub use model::{
    DurabilityClass, DurabilityEvidence, RootCatalog, RootCatalogSlot, RootKind, SoleWriterClass,
    SoleWriterEvidence,
};
pub use parse::{
    MAX_ROOT_CATALOG_DOCUMENT_BYTES, MAX_ROOT_CATALOG_KEY_BYTES, MAX_ROOT_CATALOG_PATH_BYTES,
    MAX_ROOT_CATALOG_SLOTS, RootCatalogParseError, parse_root_catalog_v1,
};
pub use source::{
    RootCatalogFileEvidence, RootCatalogLoad, RootCatalogSource, RootCatalogSourceError,
    RootCatalogSourceState, RootCatalogSourceTrust, RootCatalogTrustViolation,
};

#[cfg(test)]
mod tests;
