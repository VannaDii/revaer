use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, ensure};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use uuid::Uuid;

use super::pool_proof_tests::{Frame, Request, input, outcome, read_request, session};
use super::{SearchResultIngestInput, search_result_ingest};

const PROOF_INPUT: &str = "REVAER_INGESTION_CANCELLATION_PROOF";
const VALID_REQUEST: &str = "56900000-0000-4000-8000-000000000002";

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum CacheState {
    Cold,
    WarmCommitted,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CancellationRequest {
    connection: Request,
    cache_state: CacheState,
}

#[derive(Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
struct StartSignal {
    database: String,
    pid: i32,
    cache_state: CacheState,
}

fn read_cancellation_request(path: Option<&Path>) -> anyhow::Result<Option<CancellationRequest>> {
    path.map(|path| Ok(serde_json::from_reader(fs::File::open(path)?)?))
        .transpose()
}

fn sidecar_path(report_path: &Path, suffix: &str) -> PathBuf {
    let mut path = report_path.as_os_str().to_os_string();
    path.push(suffix);
    path.into()
}

fn checkpoint_path(report_path: &Path) -> PathBuf {
    sidecar_path(report_path, ".cancelled.json")
}

fn require_absent(path: &Path) -> anyhow::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => anyhow::bail!("proof report must not already exist: {}", path.display()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("inspect report {}", path.display())),
    }
}

fn options(request: &Request) -> anyhow::Result<PgConnectOptions> {
    let suffix = request
        .database
        .strip_prefix("ingestion_pool_cancel_reference_")
        .or_else(|| {
            request
                .database
                .strip_prefix("ingestion_pool_cancel_final_")
        })
        .context("cancellation proof requires an owned reference or final database")?;
    ensure!(
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "cancellation database suffix must be nonempty hexadecimal"
    );
    require_absent(&request.report_path)?;
    require_absent(&checkpoint_path(&request.report_path))?;
    require_absent(&sidecar_path(&request.report_path, ".prepared.json"))?;
    require_absent(&sidecar_path(&request.report_path, ".start.json"))?;
    Ok(request
        .options()?
        .application_name("revaer-ingestion-cancellation-proof"))
}

fn publish(path: &Path, frames: &[Frame]) -> anyhow::Result<()> {
    publish_bytes(path, &serde_json::to_vec_pretty(frames)?)
}

fn publish_bytes(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let mut sidecar = path.as_os_str().to_os_string();
    sidecar.push(format!(".{}.tmp", Uuid::new_v4()));
    let sidecar = PathBuf::from(sidecar);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&sidecar)?;
    let result = (|| -> anyhow::Result<()> {
        output.write_all(bytes)?;
        output.sync_all()?;
        // Linking complete bytes publishes atomically without replacing any existing path.
        fs::hard_link(&sidecar, path)?;
        Ok(())
    })();
    drop(output);
    match (result, fs::remove_file(&sidecar)) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("report sidecar cleanup also failed: {cleanup}")))
        }
    }
}

fn read_start_signal(path: &Path) -> anyhow::Result<Option<StartSignal>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            ensure!(metadata.is_file(), "start signal must be a regular file");
            Ok(Some(serde_json::from_reader(fs::File::open(path)?)?))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).context("inspect controller start signal"),
    }
}

async fn await_start(path: &Path, expected: &StartSignal) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(180);
    loop {
        if let Some(signal) = read_start_signal(path)? {
            ensure!(&signal == expected, "controller start identity changed");
            return Ok(());
        }
        ensure!(
            tokio::time::Instant::now() < deadline,
            "controller start timed out"
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn warmup_input(valid: Uuid) -> anyhow::Result<SearchResultIngestInput<'static>> {
    let mut warmup = input(valid, 2)?;
    warmup.source_guid = Some("pool-proof-warmup");
    warmup.infohash_v1 = Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    Ok(warmup)
}

async fn observe(pool: &PgPool, request: &CancellationRequest) -> anyhow::Result<()> {
    let report_path = &request.connection.report_path;
    let valid = Uuid::parse_str(VALID_REQUEST)?;
    let mut frames = Vec::with_capacity(3);
    if request.cache_state == CacheState::WarmCommitted {
        let before = session(pool).await?;
        let outcome = outcome(search_result_ingest(pool, &warmup_input(valid)?).await)?;
        let after = session(pool).await?;
        frames.push(Frame {
            name: "warmup",
            before,
            outcome,
            after,
        });
    }
    let prepared = session(pool).await?;
    let signal = StartSignal {
        database: request.connection.database.clone(),
        pid: prepared.pid,
        cache_state: request.cache_state,
    };
    publish_bytes(
        &sidecar_path(report_path, ".prepared.json"),
        &serde_json::to_vec_pretty(&serde_json::json!({
            "frames": &frames, "session": prepared, "start_signal": &signal,
        }))?,
    )?;
    await_start(&sidecar_path(report_path, ".start.json"), &signal).await?;
    for (name, minute) in [("cancelled", 0), ("recovery", 1)] {
        let before = session(pool).await?;
        let outcome = outcome(search_result_ingest(pool, &input(valid, minute)?).await)?;
        let after = session(pool).await?;
        frames.push(Frame {
            name,
            before,
            outcome,
            after,
        });
        if name == "cancelled" {
            // The controller validates this checkpoint before releasing its lock.
            publish(&checkpoint_path(report_path), &frames)?;
        }
    }
    publish(report_path, &frames)
}

async fn run(request: CancellationRequest) -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect_with(options(&request.connection)?)
        .await?;
    let result = observe(&pool, &request).await;
    pool.close().await;
    result
}

#[tokio::test]
async fn application_cancellation_qualification() -> anyhow::Result<()> {
    let path = std::env::var_os(PROOF_INPUT);
    match read_cancellation_request(path.as_deref().map(Path::new))? {
        Some(request) => run(request).await?,
        None => {
            // Ordinary tests exercise only the explicit-launch guard.
            ensure!(
                read_cancellation_request(None)?.is_none(),
                "absent cancellation proof input must not launch"
            );
        }
    }
    Ok(())
}

fn request_value(database: &str) -> serde_json::Value {
    serde_json::json!({
        "database_url": format!("postgres://postgres@127.0.0.1/{database}"),
        "database": database,
        "role": "postgres",
        "report_path": std::env::temp_dir().join(format!("cancellation-proof-{}.json", Uuid::new_v4())),
    })
}

#[test]
fn cancellation_cache_selection_is_explicit_and_strict() -> anyhow::Result<()> {
    assert!(read_cancellation_request(None)?.is_none());
    let missing = std::env::temp_dir().join(format!("missing-cache-{}.json", Uuid::new_v4()));
    assert!(read_cancellation_request(Some(&missing)).is_err());
    for (name, expected) in [
        ("cold", CacheState::Cold),
        ("warm_committed", CacheState::WarmCommitted),
    ] {
        let value = serde_json::json!({
            "connection": request_value("ingestion_pool_cancel_reference_a1"),
            "cache_state": name,
        });
        let request: CancellationRequest = serde_json::from_value(value.clone())?;
        assert_eq!(request.cache_state, expected);
        for field in ["connection", "cache_state"] {
            let mut missing = value.clone();
            missing
                .as_object_mut()
                .context("request is an object")?
                .remove(field)
                .context("required request field exists")?;
            assert!(serde_json::from_value::<CancellationRequest>(missing).is_err());
            let duplicate = format!(
                "{{{field:?}:{},{}",
                value[field],
                serde_json::to_string(&value)?.trim_start_matches('{')
            );
            assert!(serde_json::from_str::<CancellationRequest>(&duplicate).is_err());
        }
        let mut extra = value.clone();
        extra["extra"] = true.into();
        assert!(serde_json::from_value::<CancellationRequest>(extra).is_err());
        for invalid in [serde_json::Value::Null, false.into(), "warm".into()] {
            let mut changed = value.clone();
            changed["cache_state"] = invalid;
            assert!(serde_json::from_value::<CancellationRequest>(changed).is_err());
        }
    }
    Ok(())
}

#[tokio::test]
async fn cancellation_start_requires_exact_published_controller_identity() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("cancellation-start-{}", Uuid::new_v4()));
    fs::create_dir(&directory)?;
    let result = async {
        let path = directory.join("start.json");
        assert!(read_start_signal(&path)?.is_none());
        assert!(read_start_signal(&directory).is_err());
        let signal = StartSignal {
            database: "ingestion_pool_cancel_reference_a1".into(),
            pid: 123,
            cache_state: CacheState::WarmCommitted,
        };
        let value = serde_json::to_value(&signal)?;
        for field in ["database", "pid", "cache_state"] {
            let mut changed = value.clone();
            changed[field] = match field {
                "pid" => 456.into(),
                "cache_state" => "cold".into(),
                _ => "ingestion_pool_cancel_reference_b2".into(),
            };
            publish_bytes(&path, &serde_json::to_vec(&changed)?)?;
            assert!(await_start(&path, &signal).await.is_err());
            fs::remove_file(&path)?;
        }
        publish_bytes(&path, &serde_json::to_vec(&value)?)?;
        await_start(&path, &signal).await?;
        fs::remove_file(&path)?;
        let mut extra = value.clone();
        extra["extra"] = true.into();
        publish_bytes(&path, &serde_json::to_vec(&extra)?)?;
        assert!(read_start_signal(&path).is_err());
        fs::remove_file(&path)?;
        publish_bytes(&path, b"{")?;
        assert!(read_start_signal(&path).is_err());
        Ok::<(), anyhow::Error>(())
    }
    .await;
    fs::remove_dir_all(&directory)?;
    result
}

#[test]
fn cancellation_requires_explicit_strict_input() -> anyhow::Result<()> {
    assert!(read_request(None)?.is_none());
    let missing =
        std::env::temp_dir().join(format!("missing-cancellation-{}.json", Uuid::new_v4()));
    assert!(read_request(Some(&missing)).is_err());
    let value = request_value("ingestion_pool_cancel_reference_a1");
    let mut unknown = value.clone();
    unknown["unexpected"] = true.into();
    assert!(serde_json::from_value::<Request>(unknown).is_err());
    for field in ["database_url", "database", "role", "report_path"] {
        let mut missing = value.clone();
        missing
            .as_object_mut()
            .context("request must be an object")?
            .remove(field)
            .context("request field must exist")?;
        assert!(serde_json::from_value::<Request>(missing).is_err());
        let duplicate = format!(
            "{{{field:?}:{},{}",
            value[field],
            serde_json::to_string(&value)?.trim_start_matches('{')
        );
        assert!(serde_json::from_str::<Request>(&duplicate).is_err());
    }
    Ok(())
}

#[test]
fn cancellation_requires_local_owned_database_identity() -> anyhow::Result<()> {
    for (kind, role) in [("reference", "postgres"), ("final", "proof_runtime_unit")] {
        let database = format!("ingestion_pool_cancel_{kind}_a01F");
        let mut value = request_value(&database);
        value["role"] = role.into();
        value["database_url"] = format!("postgres://{role}@127.0.0.1/{database}").into();
        let options = options(&serde_json::from_value(value)?)?;
        assert_eq!(options.get_database(), Some(database.as_str()));
        assert_eq!(options.get_username(), role);
        assert_eq!(
            options.get_application_name(),
            Some("revaer-ingestion-cancellation-proof")
        );
    }
    for database in [
        "ingestion_pool_unit",
        "ingestion_pool_cancel_other_a1",
        "ingestion_pool_cancel_reference_",
        "ingestion_pool_cancel_final_g1",
        "ingestion_pool_cancel_final_a1_extra",
    ] {
        assert!(options(&serde_json::from_value(request_value(database))?).is_err());
    }
    let value = request_value("ingestion_pool_cancel_reference_a1");
    for (field, invalid) in [
        (
            "database_url",
            "postgres://postgres@localhost/ingestion_pool_cancel_reference_a1",
        ),
        (
            "database_url",
            "postgres://postgres@example.com/ingestion_pool_cancel_reference_a1",
        ),
        ("database", "ingestion_pool_cancel_reference_b2"),
        ("role", "proof_runtime_other"),
    ] {
        let mut request = value.clone();
        request[field] = invalid.into();
        assert!(options(&serde_json::from_value(request)?).is_err());
    }
    Ok(())
}

#[test]
fn cancellation_input_reuses_valid_request_at_successive_minutes() -> anyhow::Result<()> {
    let valid = Uuid::parse_str(VALID_REQUEST)?;
    let cancelled = input(valid, 0)?;
    let recovery = input(valid, 1)?;
    let warmup = warmup_input(valid)?;
    assert_ne!(warmup.source_guid, cancelled.source_guid);
    assert_ne!(warmup.infohash_v1, cancelled.infohash_v1);
    assert_eq!(
        warmup.search_request_public_id,
        cancelled.search_request_public_id
    );
    assert_eq!(
        (warmup.observed_at - cancelled.observed_at).num_seconds(),
        120
    );
    assert_eq!(cancelled.search_request_public_id, valid);
    assert_eq!(recovery.search_request_public_id, valid);
    assert_eq!(
        cancelled.indexer_instance_public_id,
        recovery.indexer_instance_public_id
    );
    assert_eq!(cancelled.source_guid, recovery.source_guid);
    assert_eq!(cancelled.infohash_v1, recovery.infohash_v1);
    assert_eq!(
        (recovery.observed_at - cancelled.observed_at).num_seconds(),
        60
    );
    assert!(input(valid, 60).is_err());
    Ok(())
}

#[test]
fn cancellation_reports_publish_exclusively_and_reject_existing_paths() -> anyhow::Result<()> {
    let directory = std::env::temp_dir().join(format!("cancellation-reports-{}", Uuid::new_v4()));
    fs::create_dir(&directory)?;
    let result = (|| -> anyhow::Result<()> {
        let mut request: Request =
            serde_json::from_value(request_value("ingestion_pool_cancel_reference_a1"))?;
        request.report_path = directory.join("report.json");
        let checkpoint = checkpoint_path(&request.report_path);
        ensure!(checkpoint == directory.join("report.json.cancelled.json"));
        ensure!(options(&request)?.get_database() == Some(request.database.as_str()));
        publish(&checkpoint, &[])?;
        ensure!(
            options(&request).is_err(),
            "existing checkpoint must reject launch"
        );
        ensure!(!request.report_path.try_exists()?);
        ensure!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&checkpoint)?)?
                == serde_json::json!([])
        );
        fs::remove_file(&checkpoint)?;
        publish(&request.report_path, &[])?;
        ensure!(
            options(&request).is_err(),
            "existing report must reject launch"
        );
        let original = fs::read(&request.report_path)?;
        ensure!(
            publish(&request.report_path, &[]).is_err(),
            "publication must not overwrite"
        );
        ensure!(fs::read(&request.report_path)? == original);
        let entries = fs::read_dir(&directory)?.collect::<std::io::Result<Vec<_>>>()?;
        ensure!(entries.len() == 1, "report sidecars must be removed");
        ensure!(publish(&directory.join("missing/report.json"), &[]).is_err());
        Ok(())
    })();
    match (result, fs::remove_dir_all(&directory)) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Err(error)) => Err(error.into()),
        (Err(error), Err(cleanup)) => {
            Err(error.context(format!("test directory cleanup also failed: {cleanup}")))
        }
    }
}
