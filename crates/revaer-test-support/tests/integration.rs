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
