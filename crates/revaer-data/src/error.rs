//! Error types for the data access layer.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::path::PathBuf;

use sqlx::postgres::PgDatabaseError;

/// Result alias for data layer operations.
pub type Result<T> = std::result::Result<T, DataError>;

/// Wrap external operation errors with a consistent data-layer error.
pub(crate) async fn try_op<T, F, Fut>(operation: &'static str, call: F) -> Result<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = sqlx::Result<T>>,
{
    call()
        .await
        .map_err(|source| DataError::QueryFailed { operation, source })
}

/// Errors raised by the data access layer.
#[derive(Debug)]
pub enum DataError {
    /// The runtime database does not satisfy the packaged baseline contract.
    BaselineFailed {
        /// Credential-free baseline failure.
        source: crate::baseline::BaselineReadError,
    },
    /// The owner of a database boundary could not complete.
    BoundaryFailed {
        /// Operation identifier.
        operation: &'static str,
        /// Underlying worker error.
        source: Box<dyn Error + Send + Sync>,
    },
    /// An interaction and its awaited rollback both failed.
    RollbackFailed {
        /// Original interaction failure.
        original: Box<Self>,
        /// Failure while rolling back the interaction.
        cleanup: Box<Self>,
    },
    /// Migration execution failed.
    MigrationFailed {
        /// Underlying migration error.
        source: sqlx::migrate::MigrateError,
    },
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
            Self::RollbackFailed { original, .. } => original.database_code(),
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
            Self::RollbackFailed { original, .. } => original.database_detail(),
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
            Self::BaselineFailed { .. } => {
                formatter.write_str("database baseline verification failed")
            }
            Self::BoundaryFailed { operation, source } => {
                write!(formatter, "{operation}: {source}")
            }
            Self::RollbackFailed { original, cleanup } => {
                write!(formatter, "{original}; rollback failed: {cleanup}")
            }
            Self::MigrationFailed { .. } => formatter.write_str("migration failed"),
            Self::QueryFailed { operation, source } => write!(formatter, "{operation}: {source}"),
            Self::JobFailed { .. } => formatter.write_str("job run failed"),
            Self::PathNotUtf8 { .. } => formatter.write_str("path contained invalid utf-8"),
        }
    }
}

impl Error for DataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BaselineFailed { source } => Some(source),
            Self::BoundaryFailed { source, .. } => Some(source.as_ref()),
            Self::RollbackFailed { original, .. } => Some(original),
            Self::MigrationFailed { source } => Some(source),
            Self::QueryFailed { source, .. } => Some(source),
            Self::JobFailed { .. } | Self::PathNotUtf8 { .. } => None,
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
        let migration = DataError::MigrationFailed {
            source: sqlx::migrate::MigrateError::VersionMissing(1),
        };
        assert_eq!(migration.to_string(), "migration failed");
        assert!(migration.source().is_some());

        let query = DataError::QueryFailed {
            operation: "fetch",
            source: sqlx::Error::RowNotFound,
        };
        assert!(query.to_string().starts_with("fetch: "));
        assert!(query.to_string().contains("no rows returned"));
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
        assert!(from.to_string().starts_with("sqlx operation: "));
        assert!(from.source().is_some());
    }
}
