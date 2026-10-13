use std::fs;
use std::path::{Path, PathBuf};

use revaer_test_support::fixtures::{docker_available, docker_available_with_host};
use revaer_test_support::postgres::{start_postgres, start_postgres_at};
use sqlx::{Connection, Row, postgres::PgConnection};
use url::Url;

fn current_database_name(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let runtime = test_runtime()?;
    runtime.block_on(async {
        let mut connection = PgConnection::connect(url).await?;
        let row = sqlx::query("SELECT current_database()")
            .fetch_one(&mut connection)
            .await?;
        let database = row.try_get(0)?;
        Ok(database)
    })
}

fn database_exists(url: &str, database_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let runtime = test_runtime()?;
    runtime.block_on(async {
        let mut connection = PgConnection::connect(url).await?;
        let row = sqlx::query("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
            .bind(database_name)
            .fetch_one(&mut connection)
            .await?;
        let exists = row.try_get(0)?;
        Ok(exists)
    })
}

fn test_runtime() -> Result<tokio::runtime::Runtime, Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_io()
        .enable_time()
        .build()?;
    Ok(runtime)
}

fn admin_database_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut admin_url = Url::parse(url)?;
    admin_url.set_path("/postgres");
    Ok(admin_url.to_string())
}

#[test]
fn docker_available_returns_boolean() {
    let _available = docker_available();
}

#[test]
fn docker_available_with_host_accepts_tcp_host() {
    assert!(docker_available_with_host(
        Some("tcp://127.0.0.1:2375"),
        Path::new("/definitely/missing.sock"),
    ));
}

#[test]
fn docker_available_with_host_rejects_missing_unix_socket() {
    assert!(!docker_available_with_host(
        Some("unix:///definitely/missing.sock"),
        Path::new("/definitely/missing.sock"),
    ));
}

#[test]
fn docker_available_with_host_accepts_existing_unix_socket()
-> Result<(), Box<dyn std::error::Error>> {
    let socket_dir = PathBuf::from(".server_root/test-support");
    fs::create_dir_all(&socket_dir)?;
    let socket_path = socket_dir.join("revaer-docker.sock");
    fs::write(&socket_path, "")?;
    let host = format!("unix://{}", socket_path.display());
    assert!(docker_available_with_host(
        Some(&host),
        Path::new("/definitely/missing.sock")
    ));
    fs::remove_file(socket_path)?;
    Ok(())
}

#[test]
fn docker_available_with_host_uses_existing_default_socket()
-> Result<(), Box<dyn std::error::Error>> {
    let socket_dir = PathBuf::from(".server_root/test-support");
    fs::create_dir_all(&socket_dir)?;
    let socket_path = socket_dir.join("revaer-docker-default.sock");
    fs::write(&socket_path, "")?;
    assert!(docker_available_with_host(None, socket_path.as_path()));
    fs::remove_file(socket_path)?;
    Ok(())
}

#[test]
fn docker_available_with_host_probes_default_channels_when_needed() {
    let _available = docker_available_with_host(None, Path::new("/definitely/missing.sock"));
}

#[test]
fn start_postgres_at_rejects_invalid_url() {
    let err = start_postgres_at("not-a-url").expect_err("invalid URL should fail");
    assert!(err.to_string().contains("invalid postgres connection url"));
}

#[test]
fn start_postgres_at_reports_unreachable_database() {
    let err = start_postgres_at("postgres://[::1]:1/revaer")
        .expect_err("unreachable database should fail");
    assert!(format!("{err:#}").contains("failed to create database"));
}

#[test]
fn start_postgres_uses_external_database_when_available() -> Result<(), Box<dyn std::error::Error>>
{
    let has_base_url = std::env::var("REVAER_TEST_DATABASE_URL")
        .ok()
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .is_some();
    if !has_base_url {
        eprintln!(
            "skipping start_postgres_uses_external_database_when_available: no DATABASE_URL configured"
        );
        return Ok(());
    }

    let db = match start_postgres() {
        Ok(database) => database,
        Err(err) => {
            eprintln!("skipping start_postgres_uses_external_database_when_available: {err:#}");
            return Ok(());
        }
    };

    let current_database = current_database_name(db.connection_string())?;
    assert!(current_database.starts_with("revaer_test_"));
    let admin_url = admin_database_url(db.connection_string())?;
    assert!(database_exists(&admin_url, &current_database)?);
    drop(db);
    assert!(!database_exists(&admin_url, &current_database)?);
    Ok(())
}

fn fixture_roles_exist(url: &str, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    test_runtime()?.block_on(async {
        let mut admin = PgConnection::connect(url).await?;
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_roles WHERE rolname = $1 OR rolname = $2)",
        )
        .bind(format!("{name}_owner"))
        .bind(format!("{name}_runtime"))
        .fetch_one(&mut admin)
        .await?;
        admin.close().await?;
        Ok(exists)
    })
}

#[test]
fn initialized_fixture_seals_restricted_runtime_and_removes_roles()
-> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = start_postgres()?;
    let database = current_database_name(fixture.connection_string())?;
    let admin_url = admin_database_url(fixture.connection_string())?;
    test_runtime()?.block_on(async {
        fixture.initialize_runtime(include_str!("../../revaer-data/init.sql")).await?;
        let url = Url::parse(fixture.connection_string())?;
        assert!(!format!("{fixture:?}").contains(url.password().unwrap_or_default()));
        let mut runtime = PgConnection::connect(fixture.connection_string()).await?;
        let identity: (String, String) = sqlx::query_as("SELECT current_database(), current_user")
            .fetch_one(&mut runtime).await?;
        assert_eq!(identity, (database.clone(), format!("{database}_runtime")));
        let denied = sqlx::query("SELECT media_job_id FROM public.media_job LIMIT 0")
            .execute(&mut runtime).await;
        assert_eq!(denied.err().and_then(|error| error.as_database_error()
            .and_then(|database| database.code().map(|code| code.into_owned()))).as_deref(), Some("42501"));
        runtime.close().await?;
        let mut admin = PgConnection::connect(&admin_url).await?;
        let roles: Vec<(String, bool, bool, bool, bool, bool)> = sqlx::query_as(
            "SELECT rolname, rolcanlogin, rolsuper, rolcreatedb, rolcreaterole, rolbypassrls FROM pg_roles WHERE rolname = $1 OR rolname = $2 ORDER BY rolname")
            .bind(format!("{database}_owner")).bind(format!("{database}_runtime"))
            .fetch_all(&mut admin).await?;
        assert_eq!(roles, vec![
            (format!("{database}_owner"), false, false, false, false, false),
            (format!("{database}_runtime"), true, false, false, false, false),
        ]);
        admin.close().await?;
        assert!(fixture.initialize_runtime("SELECT 1").await.is_err());
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    fixture.close()?;
    assert!(!database_exists(&admin_url, &database)?);
    assert!(!fixture_roles_exist(&admin_url, &database)?);
    Ok(())
}

#[test]
fn initialized_fixture_failure_cleans_database_and_roles() -> Result<(), Box<dyn std::error::Error>>
{
    for explicit_close in [false, true] {
        let mut fixture = start_postgres()?;
        let database = current_database_name(fixture.connection_string())?;
        let admin_url = admin_database_url(fixture.connection_string())?;
        let result = test_runtime()?.block_on(fixture.initialize_runtime(include_str!(
            "../../../scripts/tests/database-runtime-fixture-failing-init.sql"
        )));
        assert!(result.is_err());
        assert!(fixture_roles_exist(&admin_url, &database)?);
        if explicit_close {
            fixture.close()?;
        } else {
            drop(fixture);
        }
        assert!(!database_exists(&admin_url, &database)?);
        assert!(!fixture_roles_exist(&admin_url, &database)?);
    }
    Ok(())
}

#[test]
fn initialized_fixture_empty_input_preserves_raw_database() -> Result<(), Box<dyn std::error::Error>>
{
    let mut fixture = start_postgres()?;
    let database = current_database_name(fixture.connection_string())?;
    let admin_url = admin_database_url(fixture.connection_string())?;
    assert!(
        test_runtime()?
            .block_on(fixture.initialize_runtime(" "))
            .is_err()
    );
    assert!(database_exists(&admin_url, &database)?);
    assert!(!fixture_roles_exist(&admin_url, &database)?);
    fixture.close()?;
    Ok(())
}

#[test]
fn initialized_fixture_seals_runtime_identity_and_cleans_owned_roles()
-> Result<(), Box<dyn std::error::Error>> {
    const INITIALIZER: &str =
        include_str!("../../../scripts/tests/database-runtime-fixture-success.sql");
    let mut fixture = start_postgres()?;
    let database = current_database_name(fixture.connection_string())?;
    let admin_url = admin_database_url(fixture.connection_string())?;
    test_runtime()?.block_on(async {
        fixture.initialize_runtime(INITIALIZER).await?;
        let mut runtime = PgConnection::connect(fixture.connection_string()).await?;
        let identity: String = sqlx::query_scalar("SELECT current_user")
            .fetch_one(&mut runtime)
            .await?;
        assert_eq!(identity, format!("{database}_runtime"));
        let sealed: bool = sqlx::query_scalar(
            "SELECT digest = sha256(convert_to($1, 'UTF8')) FROM revaer_system.fixture_seal",
        )
        .bind(INITIALIZER)
        .fetch_one(&mut runtime)
        .await?;
        assert!(sealed);
        let privileges: (bool, bool, bool) = sqlx::query_as(
            "SELECT rolsuper, rolcreaterole, has_schema_privilege(current_user, 'revaer_system', 'CREATE') FROM pg_roles WHERE rolname = current_user",
        )
        .fetch_one(&mut runtime)
        .await?;
        assert_eq!(privileges, (false, false, false));
        let owner_login: bool = sqlx::query_scalar(
            "SELECT rolcanlogin FROM pg_roles WHERE rolname = $1",
        )
        .bind(format!("{database}_owner"))
        .fetch_one(&mut runtime)
        .await?;
        assert!(!owner_login);
        runtime.close().await?;
        assert!(fixture.initialize_runtime(INITIALIZER).await.is_err());
        Ok::<(), anyhow::Error>(())
    })?;
    fixture.close()?;
    assert!(!database_exists(&admin_url, &database)?);
    assert!(!fixture_roles_exist(&admin_url, &database)?);
    Ok(())
}

#[tokio::test]
async fn failed_token_validation_rolls_back_before_reusing_connection() -> anyhow::Result<()> {
    use revaer_config::ConfigError;
    use revaer_test_support::postgres::start_postgres;
    let mut database = start_postgres()?;
    database
        .initialize_runtime(include_str!("../../revaer-data/init.sql"))
        .await?;
    let service = revaer_config::ConfigService::new(database.connection_string()).await?;
    let pool = service.pool().clone();
    // Reserve seven of the service's eight slots so the request must reuse the known backend.
    let mut held = Vec::new();
    for _ in 0..7 {
        held.push(pool.acquire().await?);
    }
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    let error = service
        .validate_setup_token("missing")
        .await
        .expect_err("no token was created");
    assert!(matches!(error, ConfigError::SetupTokenMissing));
    let reused: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    assert_eq!(backend, reused);
    service.snapshot().await?;
    drop(held);
    pool.close().await;
    database.close()?;
    Ok(())
}

#[tokio::test]
async fn failed_rollback_preserves_errors_and_replaces_connection() -> anyhow::Result<()> {
    use revaer_config::ConfigError;
    use std::time::Duration;
    use tokio::time::{sleep, timeout};
    let mut database = start_postgres()?;
    database
        .initialize_runtime(include_str!("../../revaer-data/init.sql"))
        .await?;
    let service = revaer_config::ConfigService::new(database.connection_string()).await?;
    let pool = service.pool().clone();
    let mut held = Vec::new();
    for _ in 0..7 {
        held.push(pool.acquire().await?);
    }
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    let mut observer = database.admin_connection().await?;
    let mut blocker = observer.begin().await?;
    sqlx::query("LOCK TABLE public.setup_tokens IN ACCESS EXCLUSIVE MODE")
        .execute(blocker.as_mut())
        .await?;
    let caller = tokio::spawn(async move { service.validate_setup_token("missing").await });
    let mut probe = database.admin_connection().await?;
    timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid = $1 AND wait_event_type = 'Lock')")
                .bind(backend).fetch_one(&mut probe).await?;
            if waiting { break Ok::<(), sqlx::Error>(()); }
            sleep(Duration::from_millis(10)).await;
        }
    }).await??;
    let terminated: bool = sqlx::query_scalar("SELECT pg_terminate_backend($1)")
        .bind(backend)
        .fetch_one(&mut probe)
        .await?;
    assert!(terminated);
    let error = timeout(Duration::from_secs(5), caller)
        .await??
        .expect_err("operation and rollback must both fail");
    assert!(
        matches!(error, ConfigError::TransactionCleanup { .. }),
        "cleanup must preserve the original operation error: {error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("config.validate_setup_token.cleanup"),
        "original failure must survive cleanup: {message}"
    );
    let replacement: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    assert_ne!(backend, replacement);
    blocker.rollback().await?;
    probe.close().await?;
    observer.close().await?;
    drop(held);
    pool.close().await;
    database.close()?;
    Ok(())
}

#[tokio::test]
async fn cancelled_token_validation_finishes_rollback_and_reuses_connection() -> anyhow::Result<()>
{
    use revaer_test_support::postgres::start_postgres;
    use std::time::Duration;
    use tokio::time::{sleep, timeout};
    let mut database = start_postgres()?;
    database
        .initialize_runtime(include_str!("../../revaer-data/init.sql"))
        .await?;
    let service = revaer_config::ConfigService::new(database.connection_string()).await?;
    let pool = service.pool().clone();
    // Reserve seven of the service's eight slots so the request must reuse the known backend.
    let mut held = Vec::new();
    for _ in 0..7 {
        held.push(pool.acquire().await?);
    }
    let backend: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await?;
    let mut observer = database.admin_connection().await?;
    let mut blocker = observer.begin().await?;
    sqlx::query("LOCK TABLE public.setup_tokens IN ACCESS EXCLUSIVE MODE")
        .execute(blocker.as_mut())
        .await?;
    let caller = tokio::spawn(async move { service.validate_setup_token("missing").await });
    // Observe the actual request waiting on the table lock before cancelling its caller.
    let mut probe = database.admin_connection().await?;
    timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid = $1 AND wait_event_type = 'Lock')")
                .bind(backend).fetch_one(&mut probe).await?;
            if waiting { break Ok::<(), sqlx::Error>(()); }
            sleep(Duration::from_millis(10)).await;
        }
    }).await??;
    caller.abort();
    assert!(caller.await.is_err());
    blocker.rollback().await?;
    let reused: i32 = timeout(
        Duration::from_secs(5),
        sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&pool),
    )
    .await??;
    assert_eq!(
        backend, reused,
        "request cancellation must preserve a healthy reusable connection"
    );
    sqlx::query_scalar::<_, i64>("SELECT revaer_config.fetch_revision()")
        .fetch_one(&pool)
        .await?;
    let state: String = sqlx::query_scalar("SELECT state FROM pg_stat_activity WHERE pid = $1")
        .bind(backend)
        .fetch_one(&mut probe)
        .await?;
    assert_eq!(state, "idle");
    probe.close().await?;
    observer.close().await?;
    drop(held);
    pool.close().await;
    database.close()?;
    Ok(())
}
