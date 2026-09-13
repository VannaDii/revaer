//! Helpers for creating disposable databases on an externally managed Postgres instance.

use std::{
    cmp::Reverse,
    sync::atomic::{AtomicU64, Ordering},
    thread,
};

use anyhow::{Context, Result};
use sqlx::{Connection, postgres::PgConnection};
use url::{Position, Url, form_urlencoded};

const TEST_DATABASE_URL_IS_REQUIRED: &str = "test database url is required";

#[doc = "Handle to a disposable Postgres database used in tests."]
#[derive(Debug)]
#[rustfmt::skip]
pub struct TestDatabase { connection_string: String, admin_url: String, database: String }

impl TestDatabase {
    #[doc = "Connection string that can be passed to `sqlx` or other Postgres clients."]
    #[must_use]
    #[rustfmt::skip]
    pub fn connection_string(&self) -> &str { &self.connection_string }
}

#[rustfmt::skip]
impl Drop for TestDatabase { fn drop(&mut self) { let _ = run_admin_operation(&self.admin_url, &drop_database_sql(&self.database), "failed to drop test database"); } }

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
#[doc = "Provisioning errors retain endpoint/admin attempt order with URL credentials redacted."]
pub fn start_postgres_at(base_url: &str) -> Result<TestDatabase> {
    let parsed = Url::parse(base_url).context("invalid postgres connection url")?;
    try_postgres_candidates(&parsed, create_test_database)
}

fn try_postgres_candidates<T>(
    parsed: &Url,
    mut create: impl FnMut(&Url) -> Result<T>,
) -> Result<T> {
    // Retain redacted rendered chains so Debug/source traversal cannot expose URL credentials.
    let mut failures = Vec::new();
    for (index, candidate) in postgres_url_candidates(parsed).iter().enumerate() {
        match create(candidate) {
            Ok(database) => return Ok(database),
            Err(error) => failures.push(format!(
                "endpoint attempt {} ({}): {}",
                index + 1,
                &candidate[Position::BeforeHost..Position::AfterPort],
                redact_url_credentials(&format!("{error:#}"), candidate),
            )),
        }
    }
    Err(anyhow::Error::msg(failures.join("; ")).context("failed to create database"))
}

fn create_test_database(parsed: &Url) -> Result<TestDatabase> {
    let database = unique_database_name();
    let connection_string = database_connection_string(parsed, &database);
    let create_sql = format!("CREATE DATABASE \"{database}\"");
    let admin_url =
        create_and_probe_database(parsed, &connection_string, &create_sql, run_admin_operation)?;
    Ok(TestDatabase {
        connection_string,
        admin_url,
        database,
    })
}

fn create_and_probe_database(
    parsed: &Url,
    connection_string: &str,
    create_sql: &str,
    mut run: impl FnMut(&str, &str, &'static str) -> Result<()>,
) -> Result<String> {
    let mut failures = Vec::new();
    for (index, admin_url) in admin_urls(parsed).into_iter().enumerate() {
        if let Err(error) = run(&admin_url, create_sql, "failed to issue CREATE DATABASE") {
            failures.push(format!("admin attempt {}: {error:#}", index + 1));
            continue;
        }
        if let Err(error) = run(
            connection_string,
            "SELECT 1",
            "failed to probe test database",
        ) {
            failures.push(format!("admin attempt {} probe: {error:#}", index + 1));
            break;
        }
        return Ok(admin_url);
    }
    Err(anyhow::Error::msg(failures.join("; ")))
}

fn redact_url_credentials(message: &str, url: &Url) -> String {
    let mut secrets = vec![url.as_str().to_owned(), url.username().to_owned()];
    secrets.extend(
        [url.password(), url.query(), url.fragment()]
            .into_iter()
            .flatten()
            .map(str::to_owned),
    );
    secrets.extend(
        url.query_pairs()
            .filter(|(key, _)| {
                matches!(
                    key.as_ref(),
                    "user" | "username" | "password" | "sslpassword" | "passfile" | "sslkey"
                )
            })
            .map(|(_, value)| value.into_owned()),
    );
    // Error messages can repeat decoded userinfo; preserve literal '+' and '&'
    // while using the existing form parser for percent decoding.
    let decoded: Vec<_> = secrets
        .iter()
        .flat_map(|secret| {
            let encoded = format!("value={}", secret.replace('+', "%2B").replace('&', "%26"));
            form_urlencoded::parse(encoded.as_bytes())
                .map(|(_, value)| value.into_owned())
                .collect::<Vec<_>>()
        })
        .collect();
    secrets.extend(decoded);
    secrets.sort_by_key(|secret| Reverse(secret.len()));
    secrets.dedup();
    secrets
        .iter()
        .filter(|secret| !secret.is_empty())
        .fold(message.to_owned(), |message, secret| {
            message.replace(secret, "[redacted]")
        })
}

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
fn run_admin_operation(connection_string: &str, sql: &str, error_context: &'static str) -> Result<()> { let connection_string = connection_string.to_owned(); let sql = sql.to_owned(); thread::spawn(move || { let runtime = tokio::runtime::Builder::new_current_thread().enable_io().enable_time().build().context("failed to build postgres admin runtime")?; runtime.block_on(async move { let mut connection = PgConnection::connect(&connection_string).await?; sqlx::query(sqlx::AssertSqlSafe(sql)).execute(&mut connection).await.map(|_| ()).context(error_context) }) }).join().map_err(|_| anyhow::Error::msg("postgres admin worker panicked"))? }

#[cfg(test)]
mod tests {
    use super::*;

    fn provision_with(
        parsed: &Url,
        mut run: impl FnMut(&str, &str, &'static str) -> Result<()>,
    ) -> Result<String> {
        try_postgres_candidates(parsed, |candidate| {
            create_and_probe_database(
                candidate,
                &database_connection_string(candidate, "revaer_test_fixture"),
                "CREATE DATABASE \"revaer_test_fixture\"",
                &mut run,
            )
        })
    }

    #[test]
    fn provisioning_preserves_endpoint_and_admin_failures_in_order() -> Result<()> {
        let base_url = Url::parse("postgres://127.0.0.1:55432/revaer")?;
        let mut attempts = Vec::new();
        let result = provision_with(&base_url, |url, sql, context| {
            attempts.push(url.to_owned());
            assert_eq!(sql, "CREATE DATABASE \"revaer_test_fixture\"");
            Err(anyhow::Error::msg(format!("failure {}", attempts.len())).context(context))
        });
        let Err(error) = result else {
            anyhow::bail!("provisioning should fail");
        };

        assert_eq!(
            attempts,
            [
                "postgres://127.0.0.1:55432/postgres",
                "postgres://127.0.0.1:55432/revaer",
                "postgres://host.docker.internal:55432/postgres",
                "postgres://host.docker.internal:55432/revaer",
            ]
        );
        assert_eq!(
            format!("{error:#}"),
            concat!(
                "failed to create database: ",
                "endpoint attempt 1 (127.0.0.1:55432): ",
                "admin attempt 1: failed to issue CREATE DATABASE: failure 1; ",
                "admin attempt 2: failed to issue CREATE DATABASE: failure 2; ",
                "endpoint attempt 2 (host.docker.internal:55432): ",
                "admin attempt 1: failed to issue CREATE DATABASE: failure 3; ",
                "admin attempt 2: failed to issue CREATE DATABASE: failure 4",
            )
        );
        Ok(())
    }

    #[test]
    fn provisioning_stops_after_primary_or_later_admin_success() -> Result<()> {
        let base_url = Url::parse("postgres://127.0.0.1:55432/revaer?sslmode=disable")?;
        for successful_admin in [1, 2] {
            let mut attempts = Vec::new();
            let admin_url = provision_with(&base_url, |url, sql, context| {
                attempts.push(url.to_owned());
                if attempts.len() < successful_admin {
                    anyhow::bail!("first admin refused");
                }
                if attempts.len() == successful_admin {
                    assert_eq!(sql, "CREATE DATABASE \"revaer_test_fixture\"");
                    assert_eq!(context, "failed to issue CREATE DATABASE");
                } else {
                    assert_eq!(sql, "SELECT 1");
                    assert_eq!(context, "failed to probe test database");
                }
                Ok(())
            })?;

            let mut expected = admin_urls(&base_url);
            expected.truncate(successful_admin);
            assert_eq!(Some(&admin_url), expected.last());
            expected.push(database_connection_string(&base_url, "revaer_test_fixture"));
            assert_eq!(attempts, expected);
        }
        Ok(())
    }

    #[test]
    fn provisioning_returns_fallback_success_without_extra_attempts() -> Result<()> {
        let base_url = Url::parse("postgres://127.0.0.1:55432/revaer")?;
        for fail_probe in [false, true] {
            let mut attempts = Vec::new();
            let admin_url = provision_with(&base_url, |url, sql, _| {
                attempts.push(url.to_owned());
                let parsed = Url::parse(url)?;
                if parsed.host_str() == Some("127.0.0.1") && (!fail_probe || sql == "SELECT 1") {
                    anyhow::bail!("primary failed");
                }
                Ok(())
            })?;

            assert_eq!(admin_url, "postgres://host.docker.internal:55432/postgres");
            assert_eq!(
                attempts,
                [
                    "postgres://127.0.0.1:55432/postgres",
                    if fail_probe {
                        "postgres://127.0.0.1:55432/revaer_test_fixture"
                    } else {
                        "postgres://127.0.0.1:55432/revaer"
                    },
                    "postgres://host.docker.internal:55432/postgres",
                    "postgres://host.docker.internal:55432/revaer_test_fixture",
                ]
            );
        }
        Ok(())
    }

    #[test]
    fn probe_failure_keeps_the_preceding_admin_failure() -> Result<()> {
        let base_url = Url::parse("postgres://db.example.test:55432/revaer")?;
        let mut attempts = Vec::new();
        let result = provision_with(&base_url, |url, sql, context| {
            attempts.push(url.to_owned());
            if attempts.len() == 1 {
                anyhow::bail!("first admin refused");
            }
            if sql == "SELECT 1" {
                return Err(anyhow::Error::msg("probe refused").context(context));
            }
            Ok(())
        });
        let Err(error) = result else {
            anyhow::bail!("probe should fail");
        };

        assert_eq!(
            attempts,
            [
                "postgres://db.example.test:55432/postgres",
                "postgres://db.example.test:55432/revaer",
                "postgres://db.example.test:55432/revaer_test_fixture",
            ]
        );
        assert_eq!(
            format!("{error:#}"),
            concat!(
                "failed to create database: endpoint attempt 1 (db.example.test:55432): ",
                "admin attempt 1: first admin refused; ",
                "admin attempt 2 probe: failed to probe test database: probe refused",
            )
        );
        Ok(())
    }

    #[test]
    fn provisioning_keeps_single_remote_postgres_admin_attempt() -> Result<()> {
        let base_url = Url::parse("postgres://db.example.test:55432/postgres")?;
        let mut attempts = Vec::new();
        let result = provision_with(&base_url, |url, _, _| {
            attempts.push(url.to_owned());
            anyhow::bail!("only attempt refused");
        });
        let Err(error) = result else {
            anyhow::bail!("provisioning should fail");
        };

        assert_eq!(attempts, [base_url.as_str()]);
        assert_eq!(
            format!("{error:#}"),
            concat!(
                "failed to create database: endpoint attempt 1 (db.example.test:55432): ",
                "admin attempt 1: only attempt refused",
            )
        );
        Ok(())
    }

    #[test]
    fn provisioning_failure_context_redacts_credentials_and_url_extras() -> Result<()> {
        let mut base_url = Url::parse("postgres://127.0.0.1:55432/revaer")?;
        let username = format!("{}+&@role", unique_database_name());
        let password = format!("{}+&@password", unique_database_name());
        let query_password = format!("{}+&@query", unique_database_name());
        let fragment = unique_database_name();
        base_url
            .set_username(&username)
            .map_err(|()| anyhow::anyhow!("failed to set fixture username"))?;
        base_url
            .set_password(Some(&password))
            .map_err(|()| anyhow::anyhow!("failed to set fixture password"))?;
        base_url
            .query_pairs_mut()
            .append_pair("password", &query_password)
            .append_pair("sslmode", "disable");
        base_url.set_fragment(Some(&fragment));
        let result = provision_with(&base_url, |url, _, context| {
            Err(anyhow::anyhow!(
                "login refused for {username} with {password}; \
                 query credential {query_password}; connection {url}"
            )
            .context(context))
        });
        let Err(error) = result else {
            anyhow::bail!("provisioning should fail");
        };

        let secrets = [
            username.as_str(),
            password.as_str(),
            query_password.as_str(),
            fragment.as_str(),
            base_url.username(),
            base_url.password().context("fixture password missing")?,
            base_url.query().context("fixture query missing")?,
        ];
        for rendered in [
            format!("{error:#}"),
            format!("{error:?}"),
            format!("{error:#?}"),
        ] {
            for secret in secrets {
                assert!(!rendered.contains(secret));
            }
            assert!(rendered.contains("login refused"));
            assert!(rendered.contains("[redacted]"));
            assert!(rendered.contains("endpoint attempt 1 (127.0.0.1:55432)"));
            assert!(rendered.contains("endpoint attempt 2 (host.docker.internal:55432)"));
        }
        for source in error.chain() {
            for secret in secrets {
                assert!(!source.to_string().contains(secret));
            }
        }
        Ok(())
    }

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
