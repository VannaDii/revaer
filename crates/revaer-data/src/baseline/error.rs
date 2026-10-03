use std::error::Error;
use std::fmt::{self, Display, Formatter};

use sqlx::postgres::PgDatabaseError;

/// ADR 551 reason codes that can arise while reading a runtime baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselineReadReason {
    /// The injected runtime login is not a valid `PostgreSQL` role name.
    InvalidConfiguration,
    /// The baseline does not identify `PostgreSQL` 16.14.
    PostgresIdentityUnsupported,
    /// The baseline projection or its singleton state is malformed or absent.
    BaselineShapeInvalid,
    /// The baseline has an unsupported contract version.
    BaselineContractUnsupported,
    /// The sealed digest differs from the exact packaged init bytes.
    BaselineDigestMismatch,
    /// The sealed runtime login differs from the connection login.
    BaselineRuntimeRoleMismatch,
    /// The server denied the baseline read.
    BaselineReadDenied,
    /// The query failed without a more specific verified classification.
    StatementFailed,
    /// The runtime pool could not provide a connection within ten seconds.
    PoolAcquireTimeout,
    /// The baseline statement exceeded its ten-second budget.
    StatementTimeout,
    /// The verification connection could not be closed within its cleanup bound.
    CleanupFailed,
}

impl BaselineReadReason {
    /// Return the exact version-one reason code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InvalidConfiguration => "invalid_configuration",
            Self::PostgresIdentityUnsupported => "postgres_identity_unsupported",
            Self::BaselineShapeInvalid => "baseline_shape_invalid",
            Self::BaselineContractUnsupported => "baseline_contract_unsupported",
            Self::BaselineDigestMismatch => "baseline_digest_mismatch",
            Self::BaselineRuntimeRoleMismatch => "baseline_runtime_role_mismatch",
            Self::BaselineReadDenied => "baseline_read_denied",
            Self::StatementFailed => "statement_failed",
            Self::PoolAcquireTimeout => "pool_acquire_timeout",
            Self::StatementTimeout => "statement_timeout",
            Self::CleanupFailed => "cleanup_failed",
        }
    }

    fn from_detail(detail: &str) -> Option<Self> {
        match detail {
            "postgres_identity_unsupported" => Some(Self::PostgresIdentityUnsupported),
            "baseline_shape_invalid" => Some(Self::BaselineShapeInvalid),
            "baseline_contract_unsupported" => Some(Self::BaselineContractUnsupported),
            "baseline_digest_mismatch" => Some(Self::BaselineDigestMismatch),
            "baseline_runtime_role_mismatch" => Some(Self::BaselineRuntimeRoleMismatch),
            "baseline_read_denied" => Some(Self::BaselineReadDenied),
            _ => None,
        }
    }
}

/// A baseline-read failure containing only a closed reason and optional SQLSTATE.
///
/// Raw `SQLx` errors and `PostgreSQL` messages are deliberately not retained as an
/// error source: they can contain credentials, role names, SQL, or object names.
/// Translating them into this bounded contract is not permission to suppress
/// failures; every query failure still returns an error. SQLSTATE `57014` must
/// be classified and logged once by the lifecycle that owns cancellation and
/// deadlines. Other failures are logged at this translation boundary.
#[derive(Debug, Eq, PartialEq)]
pub struct BaselineReadError {
    reason: BaselineReadReason,
    sqlstate: Option<String>,
}

impl BaselineReadError {
    pub(super) fn new(reason: BaselineReadReason) -> Self {
        Self::record(reason, None)
    }

    fn record(reason: BaselineReadReason, sqlstate: Option<String>) -> Self {
        // Only the owner knows whether this shared SQLSTATE represents its
        // cancellation, its timeout, or an otherwise unexplained query failure.
        if sqlstate.as_deref() != Some("57014") {
            tracing::error!(
                reason = reason.code(),
                sqlstate = sqlstate.as_deref(),
                "database baseline read failed"
            );
        }
        Self { reason, sqlstate }
    }

    pub(super) fn from_query(source: &sqlx::Error) -> Self {
        if let sqlx::Error::Database(database) = source {
            let code = database.code();
            let detail = database
                .try_downcast_ref::<PgDatabaseError>()
                .and_then(PgDatabaseError::detail);
            return Self::from_database(code.as_deref(), detail);
        }
        let reason = match source {
            sqlx::Error::RowNotFound
            | sqlx::Error::ColumnNotFound(_)
            | sqlx::Error::ColumnIndexOutOfBounds { .. }
            | sqlx::Error::ColumnDecode { .. }
            | sqlx::Error::Decode(_)
            | sqlx::Error::TypeNotFound { .. } => BaselineReadReason::BaselineShapeInvalid,
            _ => BaselineReadReason::StatementFailed,
        };
        Self::new(reason)
    }

    pub(super) fn from_database(code: Option<&str>, detail: Option<&str>) -> Self {
        let reason = match code {
            Some("42501") => BaselineReadReason::BaselineReadDenied,
            Some("42883" | "42P01" | "42703" | "3F000" | "42704") => {
                BaselineReadReason::BaselineShapeInvalid
            }
            Some("P0001") => detail
                .and_then(BaselineReadReason::from_detail)
                .unwrap_or(BaselineReadReason::StatementFailed),
            _ => BaselineReadReason::StatementFailed,
        };
        let sqlstate = code
            .filter(|value| {
                value.len() == 5
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
            })
            .map(str::to_owned);
        Self::record(reason, sqlstate)
    }

    /// Return the closed failure classification.
    #[must_use]
    pub const fn reason(&self) -> BaselineReadReason {
        self.reason
    }

    /// Return a validated five-character SQLSTATE when the server supplied one.
    #[must_use]
    pub fn sqlstate(&self) -> Option<&str> {
        self.sqlstate.as_deref()
    }
}

impl Display for BaselineReadError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.reason.code())
    }
}

impl Error for BaselineReadError {}
