//! Startup-only catalog authority and fail-closed persistence.

use revaer_data::media::root_catalog::{RootCatalogUnavailable, mark_root_catalog_unavailable};
use revaer_media_runtime::root_catalog::{
    RootCatalogSource, RootCatalogSourceError, RootCatalogSourceState, RootCatalogTrustViolation,
    TrustedLocalRootCatalogSource,
};
use tracing::warn;

use crate::{AppError, AppResult};

#[cfg(target_os = "linux")]
mod native;

pub(super) type CatalogSource = Result<Box<dyn RootCatalogSource>, RootCatalogSourceError>;

pub(super) fn source_from_env() -> CatalogSource {
    let uid = rustix::process::geteuid().as_raw();
    match std::env::var_os("REVAER_MEDIA_ROOT_CATALOG_FILE") {
        None => Ok(Box::new(TrustedLocalRootCatalogSource::packaged(uid))),
        Some(location) => {
            let location = location
                .to_str()
                .ok_or(RootCatalogSourceError::SourceUntrusted {
                    violation: RootCatalogTrustViolation::InvalidLocation,
                })?;
            Ok(Box::new(TrustedLocalRootCatalogSource::native_override(
                location, uid,
            )?))
        }
    }
}

pub(super) async fn start(
    config: &revaer_config::ConfigService,
    source: CatalogSource,
) -> AppResult<Option<std::sync::Arc<dyn crate::media::source::AssociationSource>>> {
    let loaded = match source.and_then(|source| source.load()) {
        Ok(loaded) => loaded,
        Err(error) => {
            warn!(
                reason = error.reason_code(),
                "media root catalog source unavailable"
            );
            mark_root_catalog_unavailable(config.pool(), unavailable(&error))
                .await
                .map_err(storage)?;
            return Ok(None);
        }
    };
    if loaded.state() == RootCatalogSourceState::Missing {
        mark_root_catalog_unavailable(config.pool(), RootCatalogUnavailable::Missing)
            .await
            .map_err(storage)?;
        return Ok(None);
    }
    #[cfg(target_os = "linux")]
    {
        native::start(config, loaded).await.map(|roots| {
            roots.map(|root| {
                let source: std::sync::Arc<dyn crate::media::source::AssociationSource> =
                    std::sync::Arc::new(root);
                source
            })
        })
    }
    #[cfg(not(target_os = "linux"))]
    {
        mark_root_catalog_unavailable(config.pool(), RootCatalogUnavailable::Unsupported)
            .await
            .map_err(storage)?;
        Ok(None)
    }
}

fn unavailable(error: &RootCatalogSourceError) -> RootCatalogUnavailable {
    match error {
        RootCatalogSourceError::SourceUntrusted { .. }
        | RootCatalogSourceError::Filesystem { .. } => RootCatalogUnavailable::Untrusted,
        RootCatalogSourceError::BoundExceeded { .. } => RootCatalogUnavailable::BoundExceeded,
        RootCatalogSourceError::FormatInvalid(error)
            if error.reason_code() == "media_root_catalog_bound_exceeded" =>
        {
            RootCatalogUnavailable::BoundExceeded
        }
        RootCatalogSourceError::FormatInvalid(_) => RootCatalogUnavailable::Invalid,
        RootCatalogSourceError::PlatformUnsupported => RootCatalogUnavailable::Unsupported,
    }
}

fn storage(source: revaer_data::DataError) -> AppError {
    tracing::error!("media root catalog startup persistence failed");
    AppError::RootCatalogStorage { source }
}

#[cfg(test)]
mod tests;
