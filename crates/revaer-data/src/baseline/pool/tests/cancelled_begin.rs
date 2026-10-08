use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgPool};
use std::str::FromStr;
use std::time::Duration;

#[tokio::test]
async fn cancelled_begin_returns_idle_pool_connection() -> anyhow::Result<()> {
    let mut database = revaer_test_support::postgres::start_postgres()?;
    let admin_options = PgConnectOptions::from_str(database.connection_string())?;
    let result = async {
        database
            .initialize_runtime(include_str!("../../../../init.sql"))
            .await?;
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .min_connections(0)
            .connect(database.connection_string())
            .await?;
        let mut admin = sqlx::PgConnection::connect_with(&admin_options).await?;
        let observed = cancel_and_read_state(&pool, &mut admin).await;
        pool.close().await;
        admin.close().await?;
        let (cancelled, state) = observed?;
        anyhow::ensure!(
            cancelled,
            "transaction startup unexpectedly completed before cancellation"
        );
        anyhow::ensure!(
            state.as_deref() == Some("idle"),
            "cancelled transaction returned a non-idle connection to the pool: {state:?}"
        );
        Ok(())
    }
    .await;
    database.close()?;
    result
}

async fn cancel_and_read_state(
    pool: &PgPool,
    admin: &mut sqlx::PgConnection,
) -> anyhow::Result<(bool, Option<String>)> {
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(pool)
        .await?;
    // The fixture delays the BEGIN reply so cancellation occurs inside startup,
    // rather than before acquiring a connection or after creating a transaction.
    let cancelled = tokio::time::timeout(
        Duration::from_millis(100),
        pool.begin_with(include_str!(
            "../../../../../../scripts/tests/database-cancelled-begin.sql"
        )),
    )
    .await;
    let cancelled = match cancelled {
        Err(_) => true,
        Ok(transaction) => {
            transaction?.rollback().await?;
            false
        }
    };
    // Acquisition waits for the abandoned reply and queued rollback to flush.
    // With one connection, the same socket must be safe for its next borrower.
    let connection = pool.acquire().await?;
    let state = sqlx::query_scalar(include_str!(
        "../../../../../../scripts/tests/database-connection-state.sql"
    ))
    .bind(pid)
    .fetch_optional(admin)
    .await?;
    drop(connection);
    Ok((cancelled, state))
}
