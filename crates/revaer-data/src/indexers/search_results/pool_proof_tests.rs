use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgConnectOptions, PgDatabaseError, PgPoolOptions};

use super::*;

const PROOF_INPUT: &str = "REVAER_INGESTION_POOL_PROOF";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    database_url: String,
    database: String,
    role: String,
    report_path: std::path::PathBuf,
}

impl Request {
    fn options(&self) -> anyhow::Result<PgConnectOptions> {
        let options = PgConnectOptions::from_str(&self.database_url)?;
        ensure!(
            options.get_host() == "127.0.0.1",
            "proof requires literal loopback"
        );
        ensure!(
            self.database.starts_with("ingestion_pool_")
                && self
                    .database
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'),
            "proof requires an owned database name"
        );
        ensure!(
            options.get_database() == Some(self.database.as_str()),
            "database identity differs"
        );
        ensure!(options.get_username() == self.role, "role identity differs");
        ensure!(
            self.role == "postgres" || self.role.starts_with("proof_runtime_"),
            "proof requires a direct reference or runtime role"
        );
        ensure!(
            !self.report_path.exists(),
            "proof report must not already exist"
        );
        Ok(options.application_name("revaer-ingestion-pool-proof"))
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct Backend {
    pid: i32,
    database: String,
    session_role: String,
    current_role: String,
    superuser: bool,
    create_role: bool,
    bypass_rls: bool,
    server_version: String,
    conflict_setting: Option<String>,
}

async fn session(pool: &PgPool) -> anyhow::Result<Backend> {
    Ok(sqlx::query_as(
        "SELECT pg_backend_pid() AS pid, current_database() AS database,
         session_user::text AS session_role, current_user::text AS current_role,
         r.rolsuper AS superuser, r.rolcreaterole AS create_role,
         r.rolbypassrls AS bypass_rls,
         current_setting('server_version_num') AS server_version,
         current_setting('plpgsql.variable_conflict', true) AS conflict_setting
         FROM pg_roles r WHERE r.rolname = current_user",
    )
    .fetch_one(pool)
    .await?)
}

#[derive(Serialize)]
struct Row {
    canonical: Uuid,
    source: Uuid,
    observation_created: bool,
    durable_source_created: bool,
    canonical_changed: bool,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Outcome {
    Success {
        row: Row,
    },
    DatabaseError {
        operation: String,
        state: String,
        message: String,
        detail: Option<String>,
    },
}

fn outcome(result: crate::DataResult<SearchResultIngestRow>) -> anyhow::Result<Outcome> {
    match result {
        Ok(row) => Ok(Outcome::Success {
            row: Row {
                canonical: row.canonical_torrent_public_id,
                source: row.canonical_torrent_source_public_id,
                observation_created: row.observation_created,
                durable_source_created: row.durable_source_created,
                canonical_changed: row.canonical_changed,
            },
        }),
        Err(crate::DataError::QueryFailed {
            operation,
            source: sqlx::Error::Database(error),
        }) => Ok(Outcome::DatabaseError {
            operation: operation.to_owned(),
            state: error
                .code()
                .context("database error has no SQLSTATE")?
                .into_owned(),
            message: error.message().to_owned(),
            detail: error
                .try_downcast_ref::<PgDatabaseError>()
                .and_then(PgDatabaseError::detail)
                .map(str::to_owned),
        }),
        Err(error) => Err(error.into()),
    }
}

fn input(request: Uuid, minute: u32) -> anyhow::Result<SearchResultIngestInput<'static>> {
    Ok(SearchResultIngestInput {
        search_request_public_id: request,
        indexer_instance_public_id: Uuid::parse_str("56900000-0000-4000-8000-000000000001")?,
        source_guid: Some("pool-proof-source"),
        details_url: None,
        download_url: None,
        magnet_uri: None,
        title_raw: "Pool proof title",
        size_bytes: Some(1024),
        infohash_v1: Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        infohash_v2: None,
        magnet_hash: None,
        seeders: Some(5),
        leechers: Some(2),
        published_at: None,
        uploader: None,
        observed_at: DateTime::parse_from_rfc3339(&format!("2026-09-10T00:{minute:02}:00Z"))?
            .with_timezone(&Utc),
        attr_keys: None,
        attr_types: None,
        attr_value_text: None,
        attr_value_int: None,
        attr_value_bigint: None,
        attr_value_numeric: None,
        attr_value_bool: None,
        attr_value_uuid: None,
    })
}

#[derive(Serialize)]
struct Frame {
    name: &'static str,
    before: Backend,
    outcome: Outcome,
    after: Backend,
}

async fn observe(pool: &PgPool) -> anyhow::Result<Vec<Frame>> {
    let valid = Uuid::parse_str("56900000-0000-4000-8000-000000000002")?;
    let missing = Uuid::parse_str("56900000-0000-4000-8000-000000000099")?;
    let mut frames = Vec::new();
    for (name, request, minute) in [
        ("cold", valid, 0),
        ("warm-committed", valid, 1),
        ("invalid-request", missing, 2),
        ("after-error", valid, 3),
    ] {
        let before = session(pool).await?;
        let outcome = outcome(search_result_ingest(pool, &input(request, minute)?).await)?;
        let after = session(pool).await?;
        frames.push(Frame {
            name,
            before,
            outcome,
            after,
        });
    }
    Ok(frames)
}

async fn run(request: Request) -> anyhow::Result<()> {
    let options = request.options()?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await?;
    let result = observe(&pool).await;
    pool.close().await;
    let frames = result?;
    let output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(request.report_path)?;
    serde_json::to_writer_pretty(output, &frames)?;
    Ok(())
}

fn read_request(path: Option<&Path>) -> anyhow::Result<Option<Request>> {
    path.map(|path| {
        let file = std::fs::File::open(path)?;
        Ok(serde_json::from_reader(file)?)
    })
    .transpose()
}

#[tokio::test]
async fn application_pool_qualification() -> anyhow::Result<()> {
    let path = std::env::var_os(PROOF_INPUT);
    match read_request(path.as_deref().map(Path::new))? {
        Some(request) => run(request).await?,
        None => {
            // Ordinary tests exercise the launch guard, never switch bootstrap authority.
            ensure!(
                read_request(None)?.is_none(),
                "absent proof input must not launch"
            );
        }
    }
    Ok(())
}

#[test]
fn proof_requires_explicit_local_owned_inputs() -> anyhow::Result<()> {
    let request = |database_url: &str, database: &str, role: &str| Request {
        database_url: database_url.to_owned(),
        database: database.to_owned(),
        role: role.to_owned(),
        report_path: std::env::temp_dir().join(format!("pool-proof-{}.json", Uuid::new_v4())),
    };
    assert!(
        request(
            "postgres://postgres@127.0.0.1/ingestion_pool_unit",
            "ingestion_pool_unit",
            "postgres"
        )
        .options()
        .is_ok()
    );
    for value in [
        request(
            "postgres://postgres@example.com/ingestion_pool_unit",
            "ingestion_pool_unit",
            "postgres",
        ),
        request("postgres://postgres@127.0.0.1/revaer", "revaer", "postgres"),
        request(
            "postgres://postgres@127.0.0.1/ingestion_pool_unit",
            "ingestion_pool_other",
            "postgres",
        ),
        request(
            "postgres://postgres@127.0.0.1/ingestion_pool_unit",
            "ingestion_pool_unit",
            "other",
        ),
        request(
            "postgres://operator@127.0.0.1/ingestion_pool_unit",
            "ingestion_pool_unit",
            "operator",
        ),
    ] {
        assert!(value.options().is_err());
    }
    assert!(read_request(None)?.is_none());
    assert!(read_request(Some(Path::new("missing-pool-proof-input.json"))).is_err());
    assert!(
        outcome(Err(crate::DataError::QueryFailed {
            operation: "search result ingest",
            source: sqlx::Error::RowNotFound,
        }))
        .is_err()
    );
    Ok(())
}
