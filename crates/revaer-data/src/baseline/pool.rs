use std::time::Duration;

use sqlx::{Connection, PgPool};
use tokio::time::timeout;

use super::{
    BaselineReadError, BaselineReadReason, PACKAGED_INIT_SHA256, VerifiedBaseline,
    read_runtime_baseline,
};

const ACQUIRE_BOUND: Duration = Duration::from_secs(10);
const STATEMENT_BOUND: Duration = Duration::from_secs(10);
const CLEANUP_BOUND: Duration = Duration::from_secs(10);

/// Verify a runtime pool against this artifact without applying schema changes.
///
/// The expected login comes only from the pool's connection options. The
/// verification connection is detached and closed, including after a failed or
/// cancelled read; it cannot return to the runtime pool carrying pending work.
/// Bootstrap must configure the verification pool's `PostgreSQL` startup options
/// with `statement_timeout=10000`; this reader issues no session-setting SQL.
/// Acquisition, read and cleanup each have a ten-second upper bound. Bootstrap
/// remains responsible for process signals and its whole-operation deadline.
///
/// # Errors
///
/// Rejects missing, malformed or mismatched baselines, denied runtime access,
/// acquisition/read timeouts and failed cleanup. Never creates or repairs schema.
pub async fn verify_runtime_pool(pool: &PgPool) -> Result<VerifiedBaseline, BaselineReadError> {
    let options = pool.connect_options();
    let acquired = timeout(ACQUIRE_BOUND, pool.acquire())
        .await
        .map_err(|_| BaselineReadError::new(BaselineReadReason::PoolAcquireTimeout))?
        .map_err(|error| match error {
            sqlx::Error::PoolTimedOut => {
                BaselineReadError::new(BaselineReadReason::PoolAcquireTimeout)
            }
            _ => BaselineReadError::from_query(&error),
        })?;
    // Owning the raw connection makes cancellation drop its socket instead of
    // scheduling an unbounded pool-return flush of a cancelled statement.
    let mut connection = acquired.detach();
    let result = timeout(
        STATEMENT_BOUND,
        read_runtime_baseline(
            &mut connection,
            PACKAGED_INIT_SHA256,
            options.get_username(),
        ),
    )
    .await
    .map_err(|_| BaselineReadError::new(BaselineReadReason::StatementTimeout))
    .and_then(std::convert::identity)
    .map_err(|error| {
        if error.sqlstate() == Some("57014") {
            BaselineReadError::new(BaselineReadReason::StatementTimeout)
        } else {
            error
        }
    });
    timeout(CLEANUP_BOUND, connection.close())
        .await
        .map_err(|_| BaselineReadError::new(BaselineReadReason::CleanupFailed))?
        .map_err(|_| BaselineReadError::new(BaselineReadReason::CleanupFailed))?;
    result
}

#[cfg(test)]
mod tests;
