//! Error types for the data access layer.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::PathBuf;

use sqlx::postgres::PgDatabaseError;

/// Result alias for data layer operations.
pub type Result<T> = std::result::Result<T, DataError>;

/// Wrap external operation errors with a consistent data-layer error.
pub(crate) fn try_op(operation: &'static str) -> impl FnOnce(sqlx::Error) -> DataError {
    move |source| DataError::QueryFailed { operation, source }
}

/// Errors raised by the data access layer.
#[derive(Debug)]
pub enum DataError {
    /// Schema initialization failed.
    InitializationFailed {
        /// Underlying database error.
        source: sqlx::Error,
    },
    /// Authored schema objects exist without the required baseline marker.
    SchemaBaselineMissing,
    /// The schema marker does not identify the supported baseline.
    SchemaBaselineMismatch {
        /// Baseline value read from the marker table, if one exists.
        baseline: Option<String>,
    },
    /// The baseline marker exists but required v0 schema objects do not.
    SchemaBaselineIncomplete,
    /// The initializer source does not match the installed v0 baseline.
    SchemaSourceMismatch,
    /// The installed catalog no longer matches the initialized v0 baseline.
    SchemaCatalogMismatch,
    /// A database operation failed.
    QueryFailed {
        /// Operation identifier.
        operation: &'static str,
        /// Underlying SQL error.
        source: sqlx::Error,
    },
    /// A scheduled job failed but completed its schedule update.
    JobFailed {
        /// Operation identifier.
        operation: &'static str,
        /// Job key for the failed run.
        job_key: &'static str,
        /// SQLSTATE error code, when available.
        error_code: Option<String>,
        /// Database error detail, when available.
        error_detail: Option<String>,
    },
    /// A path could not be represented as UTF-8.
    PathNotUtf8 {
        /// Field name that contained the invalid path.
        field: &'static str,
        /// Path value.
        path: PathBuf,
    },
}

impl DataError {
    /// Returns the SQLSTATE error code when available.
    #[must_use]
    pub fn database_code(&self) -> Option<String> {
        match self {
            Self::QueryFailed {
                source: sqlx::Error::Database(db_err),
                ..
            } => db_err.code().map(std::borrow::Cow::into_owned),
            Self::JobFailed { error_code, .. } => error_code.clone(),
            _ => None,
        }
    }

    /// Returns the Postgres error detail field when available.
    #[must_use]
    pub fn database_detail(&self) -> Option<&str> {
        match self {
            Self::QueryFailed {
                source: sqlx::Error::Database(db_err),
                ..
            } => db_err
                .try_downcast_ref::<PgDatabaseError>()
                .and_then(PgDatabaseError::detail),
            Self::JobFailed { error_detail, .. } => error_detail.as_deref(),
            _ => None,
        }
    }
}

impl Display for DataError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InitializationFailed { .. } => {
                formatter.write_str("schema initialization failed")
            }
            Self::SchemaBaselineMissing => formatter.write_str("schema baseline marker is missing"),
            Self::SchemaBaselineMismatch { .. } => {
                formatter.write_str("schema baseline is unsupported")
            }
            Self::SchemaBaselineIncomplete => formatter.write_str("schema baseline is incomplete"),
            Self::SchemaSourceMismatch => formatter
                .write_str("schema initializer source does not match the installed baseline"),
            Self::SchemaCatalogMismatch => {
                formatter.write_str("database catalog does not match the installed baseline")
            }
            Self::QueryFailed { .. } => formatter.write_str("database operation failed"),
            Self::JobFailed { .. } => formatter.write_str("job run failed"),
            Self::PathNotUtf8 { .. } => formatter.write_str("path contained invalid utf-8"),
        }
    }
}

impl Error for DataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InitializationFailed { source } | Self::QueryFailed { source, .. } => {
                Some(source)
            }
            Self::SchemaBaselineMissing
            | Self::SchemaBaselineMismatch { .. }
            | Self::SchemaBaselineIncomplete
            | Self::SchemaSourceMismatch
            | Self::SchemaCatalogMismatch
            | Self::JobFailed { .. }
            | Self::PathNotUtf8 { .. } => None,
        }
    }
}

impl From<sqlx::Error> for DataError {
    fn from(source: sqlx::Error) -> Self {
        Self::QueryFailed {
            operation: "sqlx operation",
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_error_display_and_source() {
        let initialization = DataError::InitializationFailed {
            source: sqlx::Error::RowNotFound,
        };
        assert_eq!(initialization.to_string(), "schema initialization failed");
        assert!(initialization.source().is_some());

        let missing = DataError::SchemaBaselineMissing;
        assert_eq!(missing.to_string(), "schema baseline marker is missing");
        assert!(missing.source().is_none());

        let mismatch = DataError::SchemaBaselineMismatch {
            baseline: Some("future".to_string()),
        };
        assert_eq!(mismatch.to_string(), "schema baseline is unsupported");
        assert!(mismatch.source().is_none());

        let incomplete = DataError::SchemaBaselineIncomplete;
        assert_eq!(incomplete.to_string(), "schema baseline is incomplete");
        assert!(incomplete.source().is_none());

        let source_mismatch = DataError::SchemaSourceMismatch;
        assert_eq!(
            source_mismatch.to_string(),
            "schema initializer source does not match the installed baseline"
        );
        assert!(source_mismatch.source().is_none());

        let catalog_mismatch = DataError::SchemaCatalogMismatch;
        assert_eq!(
            catalog_mismatch.to_string(),
            "database catalog does not match the installed baseline"
        );
        assert!(catalog_mismatch.source().is_none());

        let query = DataError::QueryFailed {
            operation: "fetch",
            source: sqlx::Error::RowNotFound,
        };
        assert_eq!(query.to_string(), "database operation failed");
        assert!(query.source().is_some());
        assert_eq!(query.database_code(), None);
        assert_eq!(query.database_detail(), None);

        let job_failed = DataError::JobFailed {
            operation: "job_run",
            job_key: "retention_purge",
            error_code: Some("P0001".to_string()),
            error_detail: Some("job_error".to_string()),
        };
        assert_eq!(job_failed.to_string(), "job run failed");
        assert!(job_failed.source().is_none());
        assert_eq!(job_failed.database_code(), Some("P0001".to_string()));
        assert_eq!(job_failed.database_detail(), Some("job_error"));

        let path = DataError::PathNotUtf8 {
            field: "root",
            path: PathBuf::from(".server_root/revaer"),
        };
        assert_eq!(path.to_string(), "path contained invalid utf-8");
        assert!(path.source().is_none());

        let from = DataError::from(sqlx::Error::RowNotFound);
        assert_eq!(from.to_string(), "database operation failed");
        assert!(from.source().is_some());
    }
}
