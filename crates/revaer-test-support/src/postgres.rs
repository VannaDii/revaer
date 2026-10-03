//! Helpers for creating disposable databases on an externally managed Postgres instance.

use std::{
    sync::atomic::{AtomicU64, Ordering},
    thread,
};

use anyhow::{Context, Result};
use sqlx::Connection;
use sqlx::postgres::{PgConnection, PgPoolOptions};
use sqlx::{AssertSqlSafe, raw_sql};
use url::Url;

const TEST_DATABASE_URL_IS_REQUIRED: &str = "test database url is required";

#[doc = "Handle to a disposable Postgres database used in tests."]
#[rustfmt::skip]
pub struct TestDatabase { connection_string: String, admin_url: String, database: String, runtime_roles: bool, closed: bool }

impl std::fmt::Debug for TestDatabase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TestDatabase")
            .field("database", &self.database)
            .field("runtime_roles", &self.runtime_roles)
            .field("closed", &self.closed)
            .finish_non_exhaustive()
    }
}

impl TestDatabase {
    #[doc = "Connection string that can be passed to `sqlx` or other Postgres clients."]
    #[must_use]
    #[rustfmt::skip]
    pub fn connection_string(&self) -> &str { &self.connection_string }
}

impl Drop for TestDatabase {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup() {
            eprintln!("owned test database cleanup failed: {error}");
        }
    }
}

impl TestDatabase {
    /// Initialize and seal this owned database, then expose only its runtime login.
    ///
    /// Raw fixtures remain empty until this method is explicitly called. The
    /// server hashes the exact UTF-8 initializer submitted by the caller, using
    /// the same SHA-256 bytes as the packaged runtime baseline verifier.
    ///
    /// # Errors
    /// Returns initialization, sealing or connection errors. The handle retains
    /// cleanup ownership on failure; it must not be used as an initialized fixture.
    pub async fn initialize_runtime(&mut self, init: &str) -> Result<()> {
        anyhow::ensure!(
            !self.closed && !self.runtime_roles,
            "fixture is already initialized or closed"
        );
        anyhow::ensure!(
            !init.trim().is_empty(),
            "fixture initializer must not be empty"
        );
        let mut admin = PgConnection::connect(&self.connection_string).await?;
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&mut admin)
            .await?;
        anyhow::ensure!(
            database == self.database,
            "fixture connection reached another database"
        );
        let password: String = sqlx::query_scalar("SELECT gen_random_uuid()::text")
            .fetch_one(&mut admin)
            .await?;
        let mut transaction = admin.begin().await?;
        sqlx::query("SELECT set_config('revaer_test.fixture_password', $1, true)")
            .bind(&password)
            .execute(&mut *transaction)
            .await?;
        sqlx::raw_sql(include_str!(
            "../../../scripts/tests/database-runtime-fixture-roles.sql"
        ))
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        self.runtime_roles = true;
        let owner_name = format!("{}_owner", self.database);
        let runtime_name = format!("{}_runtime", self.database);
        let mut owner_url = Url::parse(&self.connection_string)?;
        owner_url
            .set_username(&owner_name)
            .map_err(|()| anyhow::anyhow!("invalid owner URL"))?;
        owner_url
            .set_password(Some(&password))
            .map_err(|()| anyhow::anyhow!("invalid owner password field"))?;
        let mut owner = PgConnection::connect(owner_url.as_str()).await?;
        let mut transaction = owner.begin().await?;
        sqlx::raw_sql(sqlx::AssertSqlSafe(init))
            .execute(&mut *transaction)
            .await?;
        sqlx::query("SELECT * FROM revaer_system.seal_database_baseline_v1(1::smallint, sha256(convert_to($1, 'UTF8')), $2)")
            .bind(init).bind(&runtime_name).execute(&mut *transaction).await?;
        transaction.commit().await?;
        owner.close().await?;
        sqlx::raw_sql(include_str!(
            "../../../scripts/tests/database-runtime-fixture-owner-disable.sql"
        ))
        .execute(&mut admin)
        .await?;
        admin.close().await?;
        owner_url
            .set_username(&runtime_name)
            .map_err(|()| anyhow::anyhow!("invalid runtime URL"))?;
        let mut runtime = PgConnection::connect(owner_url.as_str()).await?;
        let identity: (String, String) = sqlx::query_as("SELECT current_database(), current_user")
            .fetch_one(&mut runtime)
            .await?;
        runtime.close().await?;
        anyhow::ensure!(
            identity == (self.database.clone(), runtime_name),
            "fixture runtime connection has the wrong database or role"
        );
        self.connection_string = owner_url.to_string();
        Ok(())
    }

    /// Remove the owned database and any fixture roles, reporting cleanup errors.
    ///
    /// # Errors
    /// Returns the administrative cleanup error. Drop also attempts cleanup on
    /// early returns, and reports failures rather than silently discarding them.
    pub fn close(mut self) -> Result<()> {
        self.cleanup()
    }

    fn cleanup(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        run_admin_operation(
            &self.admin_url,
            &drop_database_sql(&self.database),
            "failed to drop test database",
        )?;
        if self.runtime_roles {
            let sql = format!(
                "DROP ROLE IF EXISTS \"{}_runtime\", \"{}_owner\"",
                self.database, self.database
            );
            run_admin_operation(&self.admin_url, &sql, "failed to drop test fixture roles")?;
        }
        self.closed = true;
        Ok(())
    }
}

#[doc = "Start a disposable test database on an externally managed Postgres instance."]
#[doc = ""]
#[doc = "# Errors"]
#[doc = "Returns an error when no test database URL is configured or provisioning fails."]
#[rustfmt::skip]
pub fn start_postgres() -> Result<TestDatabase> { std::env::var("REVAER_TEST_DATABASE_URL").ok().or_else(|| std::env::var("DATABASE_URL").ok()).context(TEST_DATABASE_URL_IS_REQUIRED).and_then(|url| start_postgres_at(&url)) }

#[doc = "Start a disposable test database using an explicit Postgres base URL."]
#[doc = ""]
#[doc = "# Errors"]
#[doc = "Returns an error when the URL is invalid or the database cannot be created and probed."]
#[rustfmt::skip]
pub fn start_postgres_at(base_url: &str) -> Result<TestDatabase> { let parsed = Url::parse(base_url).context("invalid postgres connection url")?; let mut last_error = anyhow::Error::msg("failed to create database"); for candidate in postgres_url_candidates(&parsed) { match create_test_database(&candidate) { Ok(db) => return Ok(db), Err(err) => last_error = err, } } Err(last_error.context("failed to create database")) }

#[rustfmt::skip]
fn create_test_database(parsed: &Url) -> Result<TestDatabase> { let database = unique_database_name(); let connection_string = database_connection_string(parsed, &database); let create_sql = format!("CREATE DATABASE \"{database}\""); let mut last_error = anyhow::Error::msg("failed to create database"); for admin_url in admin_urls(parsed) { if let Err(err) = run_admin_operation(&admin_url, &create_sql, "failed to issue CREATE DATABASE") { last_error = err; continue; } run_admin_operation(&connection_string, "SELECT 1", "failed to probe test database")?; return Ok(TestDatabase { connection_string, admin_url, database, runtime_roles: false, closed: false }); } Err(last_error) }

#[must_use]
#[rustfmt::skip]
fn unique_database_name() -> String { static NEXT_DATABASE_ID: AtomicU64 = AtomicU64::new(1); format!("revaer_test_{}_{}", std::process::id(), NEXT_DATABASE_ID.fetch_add(1, Ordering::Relaxed)) }

#[must_use]
#[rustfmt::skip]
fn drop_database_sql(database: &str) -> String { format!("DROP DATABASE IF EXISTS \"{database}\" WITH (FORCE)") }

#[rustfmt::skip]
fn database_connection_string(base_url: &Url, database: &str) -> String { let mut database_url = base_url.clone(); database_url.set_path(&format!("/{database}")); database_url.to_string() }

#[rustfmt::skip]
fn postgres_url_candidates(base_url: &Url) -> Vec<Url> { let mut candidates = vec![base_url.clone()]; if let Some(fallback) = local_docker_host_fallback(base_url) { candidates.push(fallback); } candidates }

#[rustfmt::skip]
fn local_docker_host_fallback(base_url: &Url) -> Option<Url> { match base_url.host_str()? { "localhost" | "127.0.0.1" => { let mut fallback = base_url.clone(); fallback.set_host(Some("host.docker.internal")).ok()?; Some(fallback) } _ => None } }

#[rustfmt::skip]
fn admin_urls(base_url: &Url) -> Vec<String> { let mut admin_url = base_url.clone(); admin_url.set_path("/postgres"); if admin_url.path() == base_url.path() { vec![admin_url.to_string()] } else { vec![admin_url.to_string(), base_url.to_string()] } }

#[rustfmt::skip]
fn run_admin_operation(connection_string: &str, sql: &str, error_context: &'static str) -> Result<()> { let connection_string = connection_string.to_owned(); let sql = sql.to_owned(); thread::spawn(move || { let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().context("failed to create postgres admin runtime")?; runtime.block_on(async move { let pool = PgPoolOptions::new().max_connections(1).connect(&connection_string).await?; raw_sql(AssertSqlSafe(sql)).execute(&pool).await.map(|_| ()).context(error_context) }) }).join().map_err(|_| anyhow::Error::msg("postgres admin worker panicked"))? }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_connection_string_replaces_database_name() {
        let base_url =
            Url::parse("postgres://localhost:5432/postgres").expect("valid postgres url");

        let connection_string = database_connection_string(&base_url, "revaer_test_fixture");

        assert_eq!(
            connection_string,
            "postgres://localhost:5432/revaer_test_fixture"
        );
    }

    #[test]
    fn admin_urls_include_base_when_database_is_not_postgres() {
        let base_url = Url::parse("postgres://localhost:5432/revaer").expect("valid url");

        let admin_urls = admin_urls(&base_url);

        assert_eq!(
            admin_urls,
            vec![
                "postgres://localhost:5432/postgres".to_string(),
                "postgres://localhost:5432/revaer".to_string()
            ]
        );
    }

    #[test]
    fn admin_urls_deduplicate_postgres_database() {
        let base_url = Url::parse("postgres://localhost:5432/postgres").expect("valid url");

        let admin_urls = admin_urls(&base_url);

        assert_eq!(
            admin_urls,
            vec!["postgres://localhost:5432/postgres".to_string()]
        );
    }

    #[test]
    fn local_docker_host_fallback_rewrites_localhost_only() {
        let local = Url::parse("postgres://user:pass@localhost:55432/postgres")
            .expect("valid postgres url");
        let remote = Url::parse("postgres://user:pass@db.example.test:5432/postgres")
            .expect("valid postgres url");

        assert_eq!(
            local_docker_host_fallback(&local).map(|url| url.to_string()),
            Some("postgres://user:pass@host.docker.internal:55432/postgres".to_string())
        );
        assert!(local_docker_host_fallback(&remote).is_none());
    }

    #[test]
    fn drop_database_sql_forces_stale_pool_sessions_closed() {
        assert_eq!(
            drop_database_sql("revaer_test_42_7"),
            "DROP DATABASE IF EXISTS \"revaer_test_42_7\" WITH (FORCE)"
        );
    }
}
