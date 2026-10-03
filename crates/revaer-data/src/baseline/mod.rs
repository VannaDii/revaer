//! Read-only verification of the ADR 551 packaged database baseline.
//!
//! The caller supplies a connection, the exact packaged digest, and the login
//! identity derived during bootstrap. This module never creates a pool, reads
//! the environment, executes DDL, or adopts an unmanaged database. Connection,
//! statement, cancellation, and whole-operation bounds belong to the owning
//! bootstrap lifecycle; successful row verification is not that lifecycle.

mod error;
mod pool;
mod row;

pub use error::{BaselineReadError, BaselineReadReason};
pub use pool::verify_runtime_pool;
pub use row::VerifiedBaseline;

/// SHA-256 of the exact init bytes built into this artifact's database contract.
pub const PACKAGED_INIT_SHA256: &[u8; 32] =
    include_bytes!(concat!(env!("OUT_DIR"), "/init-sha256.bin"));

use row::BaselineRow;
use sqlx::PgConnection;

const READ_BASELINE: &str = "SELECT contract_version, init_sha256, postgres_version_num, schema_owner_role, runtime_role, sealed_at FROM revaer_system.read_database_baseline_v1() LIMIT 2";

/// Read and verify the sealed baseline for a runtime login.
///
/// The stored procedure authenticates the actual `PostgreSQL` session. The
/// caller must derive `runtime_login` from that connection's configuration,
/// never from an API request or an owner-supplied substitute. The extra runtime
/// comparison rejects an owner-side read as proof of runtime access.
///
/// # Errors
///
/// Returns a bounded, credential-free error for invalid expectations, missing
/// or malformed lifecycle state, mismatched bytes or login, unsupported
/// versions, and database failures. No absent row is a successful result.
/// SQLSTATE `57014` remains unlogged here: the owning lifecycle must classify
/// cancellation versus timeout from its own state and log the final reason once.
pub async fn read_runtime_baseline(
    connection: &mut PgConnection,
    expected_digest: &[u8; 32],
    runtime_login: &str,
) -> Result<VerifiedBaseline, BaselineReadError> {
    row::validate_role(runtime_login)
        .map_err(|()| BaselineReadError::new(BaselineReadReason::InvalidConfiguration))?;
    let rows = sqlx::query_as::<_, BaselineRow>(READ_BASELINE)
        .fetch_all(connection)
        .await
        .map_err(|source| BaselineReadError::from_query(&source))?;
    row::verify(rows, expected_digest, runtime_login).map_err(BaselineReadError::new)
}

#[cfg(test)]
mod tests;
