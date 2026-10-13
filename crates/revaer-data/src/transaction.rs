//! Awaited transaction boundaries for pooled `PostgreSQL` interactions.

use sqlx::postgres::PgTransactionManager;
use sqlx::{PgConnection, Postgres, Transaction};
use sqlx_core::transaction::TransactionManager;

use crate::error::{DataError, Result, try_op};

/// Begin an owned transaction on a connection reserved for this interaction.
///
/// Completed boundaries await commit or rollback before returning.
/// Healthy connections remain available for reuse by the pool.
///
/// # Errors
/// Returns the underlying acquisition or BEGIN failure.
pub async fn begin_transaction(connection: &mut PgConnection) -> Result<Transaction<'_, Postgres>> {
    try_op("transaction begin", || Transaction::begin(connection, None)).await
}

/// Commit a transaction, awaiting rollback if the commit fails.
///
/// # Errors
/// Returns the commit failure and preserves any subsequent rollback failure.
pub async fn commit_transaction(mut transaction: Transaction<'_, Postgres>) -> Result<()> {
    match try_op("transaction commit", || {
        PgTransactionManager::commit(transaction.as_mut())
    })
    .await
    {
        Ok(()) => Ok(()),
        Err(original) => match rollback_transaction(transaction).await {
            Ok(()) => Err(original),
            Err(cleanup) => Err(DataError::RollbackFailed {
                original: Box::new(original),
                cleanup: Box::new(cleanup),
            }),
        },
    }
}

/// Roll back a transaction before its connection can be released.
///
/// # Errors
/// Returns the underlying rollback failure.
pub async fn rollback_transaction(mut transaction: Transaction<'_, Postgres>) -> Result<()> {
    try_op("transaction rollback", || {
        PgTransactionManager::rollback(transaction.as_mut())
    })
    .await
}

/// Finish a stored-routine interaction before returning its result.
///
/// # Errors
/// Preserves the original operation error and any rollback failure.
pub async fn finish_transaction<T>(tx: Transaction<'_, Postgres>, result: Result<T>) -> Result<T> {
    match result {
        Ok(value) => {
            commit_transaction(tx).await?;
            Ok(value)
        }
        Err(original) => match rollback_transaction(tx).await {
            Ok(()) => Err(original),
            Err(cleanup) => Err(DataError::RollbackFailed {
                original: Box::new(original),
                cleanup: Box::new(cleanup),
            }),
        },
    }
}

#[cfg(test)]
#[path = "transaction/tests.rs"]
mod tests;
