use super::*;
use revaer_media_runtime::root_catalog::{MAX_ROOT_CATALOG_DOCUMENT_BYTES, RootCatalogParseError};

#[test]
fn source_failures_preserve_closed_classification() {
    for (error, expected) in [
        (
            RootCatalogSourceError::PlatformUnsupported,
            RootCatalogUnavailable::Unsupported,
        ),
        (
            RootCatalogSourceError::BoundExceeded {
                maximum_bytes: MAX_ROOT_CATALOG_DOCUMENT_BYTES,
            },
            RootCatalogUnavailable::BoundExceeded,
        ),
        (
            RootCatalogSourceError::FormatInvalid(RootCatalogParseError::MalformedDocument),
            RootCatalogUnavailable::Invalid,
        ),
        (
            RootCatalogSourceError::FormatInvalid(RootCatalogParseError::SlotBoundExceeded {
                maximum_slots: 256,
            }),
            RootCatalogUnavailable::Invalid,
        ),
        (
            RootCatalogSourceError::SourceUntrusted {
                violation: RootCatalogTrustViolation::SourceChanged,
            },
            RootCatalogUnavailable::Untrusted,
        ),
        (
            RootCatalogSourceError::Filesystem {
                operation: "load source",
                source: std::io::Error::other("injected"),
            },
            RootCatalogUnavailable::Untrusted,
        ),
    ] {
        assert_eq!(unavailable(&error), expected);
    }
}
