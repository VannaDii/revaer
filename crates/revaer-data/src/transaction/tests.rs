use super::*;
use crate::indexers::jobs::JobClaimOutcome;
use crate::indexers::jobs::job_claim_next;
use revaer_test_support::postgres::start_postgres;
use sqlx::postgres::PgPoolOptions;

#[tokio::test]
async fn failed_transaction_rolls_back_before_reusing_connection() -> anyhow::Result<()> {
    let mut database = start_postgres()?;
    database
        .initialize_runtime(include_str!("../../init.sql"))
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(database.fixture_connect_options()?)
        .await?;
    sqlx::query("UPDATE job_schedule SET enabled = false WHERE job_key = 'retention_purge'")
        .execute(&pool)
        .await?;
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    let mut connection = pool.acquire().await?;
    let mut tx = begin_transaction(&mut connection).await?;
    let failed = sqlx::query("SELECT 1 / 0").execute(tx.as_mut()).await;
    assert!(
        failed.is_err(),
        "the regression must abort a real transaction"
    );
    rollback_transaction(tx).await?;
    connection.return_to_pool().await;
    assert_eq!(
        job_claim_next(&pool, "retention_purge").await?,
        JobClaimOutcome::Disabled
    );
    let revision = crate::config::fetch_revision(&pool).await;
    assert!(
        revision.is_ok(),
        "rollback left the connection in an aborted transaction: {revision:?}"
    );
    let reused: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        backend, reused,
        "rollback must restore and reuse the same connection"
    );
    sqlx::query("DELETE FROM job_schedule WHERE job_key = 'retention_purge'")
        .execute(&pool)
        .await?;
    let failure = job_claim_next(&pool, "retention_purge")
        .await
        .expect_err("missing schedule must remain a genuine error");
    assert_eq!(failure.database_detail(), Some("job_not_found"));
    crate::config::fetch_revision(&pool).await?;
    let reused_after_failure: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        backend, reused_after_failure,
        "a failed autocommit call must also preserve connection usability"
    );
    pool.close().await;
    database.close()?;
    Ok(())
}

#[tokio::test]
async fn expected_job_outcomes_keep_the_transaction_usable() -> anyhow::Result<()> {
    let mut database = start_postgres()?;
    database
        .initialize_runtime(include_str!("../../init.sql"))
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(database.fixture_connect_options()?)
        .await?;
    let mut connection = pool.acquire().await?;
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(connection.as_mut())
        .await?;
    let mut tx = begin_transaction(&mut connection).await?;
    for (enabled, due, locked, expected) in [
        (false, false, false, JobClaimOutcome::Disabled),
        (true, false, false, JobClaimOutcome::NotDue),
        (true, true, true, JobClaimOutcome::Locked),
    ] {
        sqlx::query("UPDATE job_schedule SET enabled = $1, next_run_at = now() + make_interval(secs => CASE WHEN $2 THEN -60 ELSE 60 END), locked_until = CASE WHEN $3 THEN now() + interval '1 minute' ELSE NULL END WHERE job_key = 'retention_purge'")
            .bind(enabled).bind(due).bind(locked).execute(tx.as_mut()).await?;
        assert_eq!(
            job_claim_next(tx.as_mut(), "retention_purge").await?,
            expected
        );
        crate::config::fetch_revision(tx.as_mut()).await?;
    }
    commit_transaction(tx).await?;
    connection.return_to_pool().await;
    let reused: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    assert_eq!(backend, reused);
    pool.close().await;
    database.close()?;
    Ok(())
}

#[tokio::test]
async fn failed_commit_rolls_back_before_reusing_connection() -> anyhow::Result<()> {
    let mut database = start_postgres()?;
    database
        .initialize_runtime(include_str!("../../init.sql"))
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(database.fixture_connect_options()?)
        .await?;
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    let mut connection = pool.acquire().await?;
    sqlx::raw_sql(sqlx::AssertSqlSafe("CREATE TEMP TABLE commit_parent(id integer PRIMARY KEY); CREATE TEMP TABLE commit_child(parent_id integer REFERENCES commit_parent(id) DEFERRABLE INITIALLY DEFERRED)"))
        .execute(connection.as_mut()).await?;
    let mut tx = begin_transaction(&mut connection).await?;
    sqlx::query("INSERT INTO commit_child VALUES (1)")
        .execute(tx.as_mut())
        .await?;
    let error = commit_transaction(tx)
        .await
        .expect_err("deferred constraint must fail at commit");
    assert_eq!(error.database_code().as_deref(), Some("23503"));
    assert!(
        matches!(
            error,
            DataError::QueryFailed {
                operation: "transaction commit",
                ..
            }
        ),
        "successful rollback must preserve the commit failure: {error}"
    );
    connection.return_to_pool().await;
    let reused: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    assert_eq!(backend, reused);
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM commit_child")
        .fetch_one(&pool)
        .await?;
    assert_eq!(rows, 0);
    pool.close().await;
    database.close()?;
    Ok(())
}

#[tokio::test]
async fn stored_routine_boundaries_have_explicit_outcomes_and_no_forwarders() -> anyhow::Result<()>
{
    let mut database = start_postgres()?;
    database
        .initialize_runtime(include_str!("../../init.sql"))
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(database.fixture_connect_options()?)
        .await?;
    let void_routines: Vec<String> = sqlx::query_scalar("SELECT p.oid::regprocedure::text FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime') AND p.prokind = 'f' AND p.prorettype = 'void'::regtype AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e')")
        .fetch_all(&pool).await?;
    assert!(
        void_routines.is_empty(),
        "routines without an explicit outcome: {void_routines:?}"
    );
    let forwarders: Vec<String> = sqlx::query_scalar(r"SELECT p.oid::regprocedure::text FROM pg_proc p JOIN pg_namespace n ON n.oid = p.pronamespace WHERE n.nspname IN ('public', 'revaer_config', 'revaer_runtime') AND p.prosrc ~* '^\s*(BEGIN\s+)?(SELECT\s+(\*\s+FROM\s+)?|PERFORM\s+|RETURN\s+QUERY\s+SELECT\s+(\*\s+FROM\s+)?)[a-z_][a-z_0-9]*\(\s*([a-z_][a-z_0-9]*(\s*=>\s*[a-z_][a-z_0-9]*)?(\s*,\s*[a-z_][a-z_0-9]*(\s*=>\s*[a-z_][a-z_0-9]*)?)*)?\s*\);?\s*(END;?)?\s*$' AND NOT EXISTS (SELECT 1 FROM pg_depend d WHERE d.classid = 'pg_proc'::regclass AND d.objid = p.oid AND d.deptype = 'e')")
        .fetch_all(&pool).await?;
    assert!(
        forwarders.is_empty(),
        "pure routine forwarding layers: {forwarders:?}"
    );
    pool.close().await;
    database.close()?;
    Ok(())
}

#[tokio::test]
async fn failed_cleanup_closes_connection_before_pool_replacement() -> anyhow::Result<()> {
    let mut database = start_postgres()?;
    database
        .initialize_runtime(include_str!("../../init.sql"))
        .await?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(database.connection_string())
        .await?;
    let mut connection = pool.acquire().await?;
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(connection.as_mut())
        .await?;
    let mut transaction = begin_transaction(&mut connection).await?;
    let mut control = database.admin_connection().await?;
    let terminated: bool = sqlx::query_scalar("SELECT pg_terminate_backend($1)")
        .bind(backend)
        .fetch_one(&mut control)
        .await?;
    assert!(terminated);
    let failed = job_claim_next(transaction.as_mut(), "retention_purge").await;
    let finished = finish_transaction(transaction, failed).await;
    let error = release_connection(connection, finished)
        .await
        .expect_err("a terminated backend must preserve operation and cleanup failure");
    assert!(matches!(error, DataError::RollbackFailed { .. }), "{error}");
    let replacement: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    assert_ne!(backend, replacement);
    crate::config::fetch_revision(&pool).await?;
    sqlx::Connection::close(control).await?;
    pool.close().await;
    database.close()?;
    Ok(())
}
