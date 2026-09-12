use std::net::{IpAddr, TcpListener};
use std::process::Command;
use std::sync::LazyLock;

use super::{compliance_tests::fixture_metadata, run_app_with_compliance_loader};
use crate::{AppError, AppResult};
use anyhow::Result;
use revaer_config::{AppMode, ConfigService, SettingsChangeset, SettingsFacade};
use revaer_test_support::postgres::start_postgres;
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::{Duration, timeout};

static BOOTSTRAP_TEST_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
const BOOTSTRAP_BIND_FAILURE_TIMEOUT: Duration = Duration::from_secs(30);
const E2E_SERVING_ENTRY: &str = "bootstrap::runtime_tests::e2e_serving_entry";
const E2E_SERVING_ENV: &str = "REVAER_E2E_SERVING_ENTRY";

#[test]
fn e2e_serving_entry() -> Result<()> {
    let selection = std::env::var_os(E2E_SERVING_ENV);
    if selection.is_none() {
        // Ordinary Rust runs exercise the launch guards, not a detached server.
        for (selection, exact, diagnostic) in [
            ("invalid", true, "invalid E2E serving selection"),
            ("1", false, "E2E serving entry requires exact selection"),
            ("1", true, "E2E serving entry requires DATABASE_URL"),
        ] {
            let mut child = Command::new(std::env::current_exe()?);
            if exact {
                child.arg("--exact");
            }
            let output = child
                .args([E2E_SERVING_ENTRY, "--nocapture"])
                .env(E2E_SERVING_ENV, selection)
                .env_remove("DATABASE_URL")
                .output()?;
            assert!(!output.status.success());
            assert!(String::from_utf8(output.stderr)?.contains(diagnostic));
        }
        return Ok(());
    }
    anyhow::ensure!(
        selection.as_deref() == Some(std::ffi::OsStr::new("1")),
        "invalid E2E serving selection"
    );
    anyhow::ensure!(
        std::env::args()
            .skip(1)
            .eq(["--exact", E2E_SERVING_ENTRY, "--nocapture"]),
        "E2E serving entry requires exact selection"
    );
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("E2E serving entry requires DATABASE_URL"))?;
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run_app_with_database_url(database_url))?;
    Ok(())
}

#[tokio::test]
async fn e2e_serving_preserves_setup_configuration() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_E2E_SETUP").is_none() {
        let postgres = start_postgres()?;
        return run_bootstrap_child(
            "e2e_serving_preserves_setup_configuration",
            &[
                ("REVAER_BOOTSTRAP_CHILD_E2E_SETUP", "1"),
                ("DATABASE_URL", postgres.connection_string()),
            ],
            &[],
        );
    }

    let database_url = std::env::var("DATABASE_URL")?;
    let service = ConfigService::new(database_url.clone()).await?;
    let reserved = TcpListener::bind(("127.0.0.1", 0))?;
    let address = reserved.local_addr()?;
    let mut profile = service.get_app_profile().await?;
    assert_eq!(profile.mode, AppMode::Setup);
    let expected_auth = profile.auth_mode;
    profile.http_port = i32::from(address.port());
    service
        .apply_changeset(
            "tester",
            "e2e-isolated-listener",
            SettingsChangeset {
                app_profile: Some(profile),
                ..SettingsChangeset::default()
            },
        )
        .await?;
    drop(reserved);

    let mut serving = tokio::process::Command::new(std::env::current_exe()?)
        .args(["--exact", E2E_SERVING_ENTRY, "--nocapture"])
        .env(E2E_SERVING_ENV, "1")
        .env("DATABASE_URL", database_url)
        .kill_on_drop(true)
        .spawn()?;
    let requests = async {
        let client = reqwest::Client::new();
        loop {
            match client.get(format!("http://{address}/health")).send().await {
                Ok(response) => {
                    assert_eq!(response.status(), reqwest::StatusCode::OK);
                    let health: revaer_api::models::HealthResponse =
                        serde_json::from_slice(&response.bytes().await?)?;
                    assert_eq!(health.mode, "setup");
                    assert_eq!(health.database.status, "ok");
                    break;
                }
                Err(error) if error.is_connect() => tokio::task::yield_now().await,
                Err(error) => return Err(error.into()),
            }
        }
        let profile = service.get_app_profile().await?;
        assert_eq!(profile.mode, AppMode::Setup);
        assert_eq!(profile.auth_mode, expected_auth);
        assert_eq!(profile.http_port, i32::from(address.port()));
        anyhow::Ok(())
    };
    let result = tokio::select! {
        result = serving.wait() => anyhow::bail!("E2E application stopped before checks completed: {result:?}"),
        result = timeout(BOOTSTRAP_BIND_FAILURE_TIMEOUT, requests) => result,
    };
    // Reap only this focused child; canonical E2E retains process-group teardown.
    serving.kill().await?;
    result??;
    Ok(())
}

async fn run_app() -> AppResult<()> {
    run_app_with_compliance_loader(None, fixture_metadata, &mut std::io::stderr()).await
}

async fn run_app_with_database_url(database_url: String) -> AppResult<()> {
    run_app_with_compliance_loader(Some(database_url), fixture_metadata, &mut std::io::stderr())
        .await
}

async fn bootstrap_test_guard() -> MutexGuard<'static, ()> {
    BOOTSTRAP_TEST_MUTEX.lock().await
}

#[tokio::test]
async fn injected_compliance_reaches_serving_and_persisted_capabilities() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_COMPLIANCE_SUCCESS").is_none() {
        let postgres = start_postgres()?;
        return run_bootstrap_child(
            "injected_compliance_reaches_serving_and_persisted_capabilities",
            &[
                ("REVAER_BOOTSTRAP_CHILD_COMPLIANCE_SUCCESS", "1"),
                ("DATABASE_URL", postgres.connection_string()),
            ],
            &[
                "REVAER_ENABLE_OTEL",
                "REVAER_SECRET_KEY_ID",
                "REVAER_SECRET_KEY",
            ],
        );
    }

    let database_url = std::env::var("DATABASE_URL")?;
    let service = ConfigService::new(database_url.clone()).await?;
    let reserved = TcpListener::bind(("127.0.0.1", 0))?;
    let address = reserved.local_addr()?;
    let mut profile = service.get_app_profile().await?;
    profile.immutable_keys.clear();
    profile.mode = AppMode::Active;
    profile.auth_mode = revaer_config::AppAuthMode::NoAuth;
    profile.bind_addr = address.ip();
    profile.http_port = i32::from(address.port());
    service
        .apply_changeset(
            "tester",
            "c1-injected-compliance",
            SettingsChangeset {
                app_profile: Some(profile),
                ..SettingsChangeset::default()
            },
        )
        .await?;
    drop(reserved);

    let mut diagnostic = Vec::new();
    let mut loads = 0;
    {
        let app = run_app_with_compliance_loader(
            Some(database_url),
            || {
                loads += 1;
                fixture_metadata()
            },
            &mut diagnostic,
        );
        let requests = async {
            let client = reqwest::Client::new();
            let response = loop {
                match client
                    .get(format!("http://{address}/v1/media/compliance"))
                    .send()
                    .await
                {
                    Ok(response) => break response.error_for_status()?,
                    Err(error) if error.is_connect() => tokio::task::yield_now().await,
                    Err(error) => return Err(error.into()),
                }
            };
            let response: revaer_api::models::MediaComplianceResponse =
                serde_json::from_slice(&response.bytes().await?)?;
            assert_eq!(
                response.source_compliance_bundle_digest,
                fixture_metadata()?.digest()
            );
            assert_eq!(
                response.source_compliance_bundle_path,
                revaer_api::app::compliance::SOURCE_COMPLIANCE_BUNDLE_PATH
            );
            assert_eq!(response.license_mode, "redistributable-gplv3-runtime");
            let capabilities = client
                .get(format!("http://{address}/v1/media/capabilities"))
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?;
            let capabilities: revaer_api::models::MediaCapabilityLatestResponse =
                serde_json::from_slice(&capabilities)?;
            let snapshot = capabilities
                .snapshot
                .ok_or_else(|| anyhow::anyhow!("startup did not persist real capabilities"))?;
            assert!(snapshot.media_capability_snapshot_id > 0);
            assert!(!snapshot.ffmpeg_version.is_empty());
            assert!(!snapshot.ffprobe_version.is_empty());
            assert!(!snapshot.codecs.is_empty());
            anyhow::Ok(())
        };
        tokio::select! {
            result = app => anyhow::bail!("application stopped before requests completed: {result:?}"),
            result = timeout(BOOTSTRAP_BIND_FAILURE_TIMEOUT, requests) => result??,
        }
        // Only this disposable test child cancels the serving future. Its Tokio
        // runtime and process own remaining workers; no production shutdown claim.
    }
    assert_eq!(loads, 1);
    assert!(diagnostic.is_empty());
    Ok(())
}

fn run_bootstrap_child(
    test_name: &str,
    envs: &[(&str, &str)],
    removed_envs: &[&str],
) -> Result<()> {
    let media_workspace = tempfile::tempdir()?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .env("REVAER_MEDIA_WORKSPACE_ROOT", media_workspace.path())
        .arg("--exact")
        .arg(format!("bootstrap::runtime_tests::{test_name}"))
        .arg("--nocapture");
    for (key, value) in envs {
        command.env(key, value);
    }
    for key in removed_envs {
        command.env_remove(key);
    }

    assert!(command.status()?.success(), "child bootstrap test failed");
    Ok(())
}

#[tokio::test]
async fn run_app_requires_database_url_env() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_MISSING_DATABASE_URL").is_some() {
        let err = timeout(Duration::from_secs(2), run_app())
            .await
            .expect("run_app should fail fast when DATABASE_URL is missing")
            .expect_err("missing DATABASE_URL should fail bootstrap");
        assert!(matches!(
            err,
            AppError::MissingEnv {
                name: "DATABASE_URL"
            }
        ));
        return Ok(());
    }

    run_bootstrap_child(
        "run_app_requires_database_url_env",
        &[("REVAER_BOOTSTRAP_CHILD_MISSING_DATABASE_URL", "1")],
        &[
            "DATABASE_URL",
            "REVAER_SECRET_KEY_ID",
            "REVAER_SECRET_KEY",
            "REVAER_ENABLE_OTEL",
            "REVAER_OTEL_SERVICE_NAME",
            "REVAER_OTEL_EXPORTER",
        ],
    )
}

#[tokio::test]
async fn run_app_rejects_missing_secret_env_pair() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_MISSING_SECRET_ENV").is_some() {
        let err = timeout(Duration::from_secs(2), run_app())
            .await
            .expect("run_app should fail fast on partial secret session env")
            .expect_err("partial secret session env should fail bootstrap");
        assert!(matches!(
            err,
            AppError::MissingEnv {
                name: "REVAER_SECRET_KEY"
            }
        ));
        return Ok(());
    }

    run_bootstrap_child(
        "run_app_rejects_missing_secret_env_pair",
        &[
            ("REVAER_BOOTSTRAP_CHILD_MISSING_SECRET_ENV", "1"),
            ("DATABASE_URL", "postgres://unused"),
            ("REVAER_SECRET_KEY_ID", "key-id"),
        ],
        &["REVAER_SECRET_KEY"],
    )
}

#[tokio::test]
async fn run_app_rejects_empty_secret_env_value() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_EMPTY_SECRET_ENV").is_some() {
        let err = timeout(Duration::from_secs(2), run_app())
            .await
            .expect("run_app should fail fast on empty secret session env")
            .expect_err("empty secret session env should fail bootstrap");
        assert!(matches!(
            err,
            AppError::InvalidConfig {
                field: "REVAER_SECRET_KEY",
                reason: "empty",
                ..
            }
        ));
        return Ok(());
    }

    run_bootstrap_child(
        "run_app_rejects_empty_secret_env_value",
        &[
            ("REVAER_BOOTSTRAP_CHILD_EMPTY_SECRET_ENV", "1"),
            ("DATABASE_URL", "postgres://unused"),
            ("REVAER_SECRET_KEY_ID", "key-id"),
            ("REVAER_SECRET_KEY", "   "),
        ],
        &[],
    )
}

#[tokio::test]
async fn run_app_rejects_empty_secret_key_id_env_value() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_EMPTY_SECRET_KEY_ID").is_some() {
        let err = timeout(Duration::from_secs(2), run_app())
            .await
            .expect("run_app should fail fast on empty secret key id")
            .expect_err("empty secret key id should fail bootstrap");
        assert!(matches!(
            err,
            AppError::InvalidConfig {
                field: "REVAER_SECRET_KEY_ID",
                reason: "empty",
                ..
            }
        ));
        return Ok(());
    }

    run_bootstrap_child(
        "run_app_rejects_empty_secret_key_id_env_value",
        &[
            ("REVAER_BOOTSTRAP_CHILD_EMPTY_SECRET_KEY_ID", "1"),
            ("DATABASE_URL", "postgres://unused"),
            ("REVAER_SECRET_KEY_ID", "   "),
            ("REVAER_SECRET_KEY", "secret-value"),
        ],
        &[],
    )
}

#[tokio::test]
async fn run_app_reads_env_database_url_and_surfaces_bind_failures() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_ENV_BIND_CONFLICT").is_some() {
        let database_url = std::env::var("DATABASE_URL")
            .expect("child env-bind-conflict test requires DATABASE_URL");
        let reserved_listener = TcpListener::bind(("127.0.0.1", 0))?;
        let reserved_port = i32::from(reserved_listener.local_addr()?.port());

        let service = ConfigService::new(database_url.clone()).await?;
        let mut app_profile = service.get_app_profile().await?;
        app_profile.immutable_keys.clear();
        app_profile.mode = AppMode::Setup;
        app_profile.bind_addr = IpAddr::from([127, 0, 0, 1]);
        app_profile.http_port = reserved_port;
        service
            .apply_changeset(
                "tester",
                "bootstrap-env-bind-conflict",
                SettingsChangeset {
                    app_profile: Some(app_profile),
                    ..SettingsChangeset::default()
                },
            )
            .await?;

        let err = timeout(BOOTSTRAP_BIND_FAILURE_TIMEOUT, run_app())
            .await
            .expect("run_app should surface bind failure without serving")
            .expect_err("occupied socket should fail api server startup");
        assert!(matches!(err, AppError::ApiServer { .. }), "{err:?}");
        return Ok(());
    }

    let postgres = match start_postgres() {
        Ok(database) => database,
        Err(err) => {
            eprintln!("skipping run_app_reads_env_database_url_and_surfaces_bind_failures: {err}");
            return Ok(());
        }
    };

    run_bootstrap_child(
        "run_app_reads_env_database_url_and_surfaces_bind_failures",
        &[
            ("REVAER_BOOTSTRAP_CHILD_ENV_BIND_CONFLICT", "1"),
            ("DATABASE_URL", postgres.connection_string()),
        ],
        &[
            "REVAER_ENABLE_OTEL",
            "REVAER_OTEL_SERVICE_NAME",
            "REVAER_OTEL_EXPORTER",
        ],
    )
}

#[tokio::test]
async fn run_app_reads_secret_session_env_and_surfaces_bind_failures() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_SECRET_ENV_BIND_CONFLICT").is_some() {
        let database_url = std::env::var("DATABASE_URL")
            .expect("child secret-env bind-conflict test requires DATABASE_URL");
        let reserved_listener = TcpListener::bind(("127.0.0.1", 0))?;
        let reserved_port = i32::from(reserved_listener.local_addr()?.port());

        let service = ConfigService::new(database_url.clone()).await?;
        let mut app_profile = service.get_app_profile().await?;
        app_profile.immutable_keys.clear();
        app_profile.mode = AppMode::Setup;
        app_profile.bind_addr = IpAddr::from([127, 0, 0, 1]);
        app_profile.http_port = reserved_port;
        service
            .apply_changeset(
                "tester",
                "bootstrap-secret-env-bind-conflict",
                SettingsChangeset {
                    app_profile: Some(app_profile),
                    ..SettingsChangeset::default()
                },
            )
            .await?;

        let err = timeout(BOOTSTRAP_BIND_FAILURE_TIMEOUT, run_app())
            .await
            .expect("run_app should surface bind failure with secret session env")
            .expect_err("occupied socket should fail api server startup");
        assert!(matches!(err, AppError::ApiServer { .. }), "{err:?}");
        return Ok(());
    }

    let postgres = match start_postgres() {
        Ok(database) => database,
        Err(err) => {
            eprintln!(
                "skipping run_app_reads_secret_session_env_and_surfaces_bind_failures: {err}"
            );
            return Ok(());
        }
    };

    run_bootstrap_child(
        "run_app_reads_secret_session_env_and_surfaces_bind_failures",
        &[
            ("REVAER_BOOTSTRAP_CHILD_SECRET_ENV_BIND_CONFLICT", "1"),
            ("DATABASE_URL", postgres.connection_string()),
            ("REVAER_SECRET_KEY_ID", "  test-key  "),
            ("REVAER_SECRET_KEY", "  test-secret  "),
        ],
        &[],
    )
}

#[tokio::test]
async fn run_app_with_database_url_rejects_public_setup_bind_from_persisted_config() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_PUBLIC_SETUP_BIND").is_some() {
        let database_url = std::env::var("DATABASE_URL")
            .expect("child public-setup-bind test requires DATABASE_URL");
        let service = ConfigService::new(database_url.clone()).await?;
        let mut app_profile = service.get_app_profile().await?;
        app_profile.immutable_keys.clear();
        app_profile.mode = AppMode::Setup;
        app_profile.bind_addr = IpAddr::from([192, 168, 10, 20]);
        service
            .apply_changeset(
                "tester",
                "bootstrap-public-bind",
                SettingsChangeset {
                    app_profile: Some(app_profile),
                    ..SettingsChangeset::default()
                },
            )
            .await?;
        let stored = service.get_app_profile().await?;
        assert_eq!(stored.mode, AppMode::Setup);
        assert_eq!(stored.bind_addr, IpAddr::from([192, 168, 10, 20]));

        let err = timeout(
            BOOTSTRAP_BIND_FAILURE_TIMEOUT,
            run_app_with_database_url(database_url),
        )
        .await
        .expect("bootstrap should fail fast instead of serving")
        .expect_err("public setup bind should fail validation");
        assert!(
            matches!(
                err,
                AppError::InvalidConfig {
                    field: "bind_addr",
                    reason: "non_loopback_in_setup",
                    ..
                }
            ),
            "unexpected bootstrap error: {err:?}"
        );
        return Ok(());
    }

    let postgres = match start_postgres() {
        Ok(database) => database,
        Err(err) => {
            eprintln!(
                "skipping run_app_with_database_url_rejects_public_setup_bind_from_persisted_config: {err}"
            );
            return Ok(());
        }
    };

    run_bootstrap_child(
        "run_app_with_database_url_rejects_public_setup_bind_from_persisted_config",
        &[
            ("REVAER_BOOTSTRAP_CHILD_PUBLIC_SETUP_BIND", "1"),
            ("DATABASE_URL", postgres.connection_string()),
        ],
        &[],
    )
}

#[tokio::test]
async fn run_app_with_database_url_surfaces_bind_failures_without_child_process() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_DIRECT_BIND_CONFLICT").is_some() {
        let database_url = std::env::var("DATABASE_URL")
            .expect("child direct bind-conflict test requires DATABASE_URL");
        let reserved_listener = TcpListener::bind(("127.0.0.1", 0))?;
        let reserved_port = i32::from(reserved_listener.local_addr()?.port());

        let service = ConfigService::new(database_url.clone()).await?;
        let mut app_profile = service.get_app_profile().await?;
        app_profile.immutable_keys.clear();
        app_profile.mode = AppMode::Setup;
        app_profile.bind_addr = IpAddr::from([127, 0, 0, 1]);
        app_profile.http_port = reserved_port;
        service
            .apply_changeset(
                "tester",
                "bootstrap-bind-conflict-direct",
                SettingsChangeset {
                    app_profile: Some(app_profile),
                    ..SettingsChangeset::default()
                },
            )
            .await?;

        let err = timeout(
            BOOTSTRAP_BIND_FAILURE_TIMEOUT,
            run_app_with_database_url(database_url),
        )
        .await
        .expect("bootstrap should surface bind failure without serving")
        .expect_err("occupied socket should fail api server startup");
        assert!(matches!(err, AppError::ApiServer { .. }), "{err:?}");
        return Ok(());
    }

    let postgres = match start_postgres() {
        Ok(database) => database,
        Err(err) => {
            eprintln!(
                "skipping run_app_with_database_url_surfaces_bind_failures_without_child_process: {err}"
            );
            return Ok(());
        }
    };

    run_bootstrap_child(
        "run_app_with_database_url_surfaces_bind_failures_without_child_process",
        &[
            ("REVAER_BOOTSTRAP_CHILD_DIRECT_BIND_CONFLICT", "1"),
            ("DATABASE_URL", postgres.connection_string()),
        ],
        &[],
    )
}

#[tokio::test]
async fn run_app_with_database_url_rejects_zero_http_port_from_persisted_config() -> Result<()> {
    let _guard = bootstrap_test_guard().await;
    let postgres = match start_postgres() {
        Ok(database) => database,
        Err(err) => {
            eprintln!(
                "skipping run_app_with_database_url_rejects_zero_http_port_from_persisted_config: {err}"
            );
            return Ok(());
        }
    };

    let service = ConfigService::new(postgres.connection_string()).await?;
    let mut app_profile = service.get_app_profile().await?;
    app_profile.immutable_keys.clear();
    app_profile.mode = AppMode::Setup;
    app_profile.bind_addr = IpAddr::from([127, 0, 0, 1]);
    app_profile.http_port = 0;
    let err = service
        .apply_changeset(
            "tester",
            "bootstrap-zero-port",
            SettingsChangeset {
                app_profile: Some(app_profile),
                ..SettingsChangeset::default()
            },
        )
        .await
        .expect_err("zero http_port should be rejected before bootstrap starts");
    assert!(matches!(
        err,
        revaer_config::ConfigError::InvalidField {
            field,
            value,
            reason,
            ..
        } if field == "http_port" && value.as_deref() == Some("0") && reason == "must be between 1 and 65535"
    ));
    Ok(())
}

#[tokio::test]
async fn run_app_with_database_url_surfaces_bind_failures_for_valid_persisted_config() -> Result<()>
{
    let _guard = bootstrap_test_guard().await;
    if std::env::var_os("REVAER_BOOTSTRAP_CHILD_BIND_CONFLICT").is_some() {
        let database_url =
            std::env::var("DATABASE_URL").expect("child bind-conflict test requires DATABASE_URL");
        let reserved_listener = TcpListener::bind(("127.0.0.1", 0))?;
        let reserved_port = i32::from(reserved_listener.local_addr()?.port());

        let service = ConfigService::new(database_url.clone()).await?;
        let mut app_profile = service.get_app_profile().await?;
        app_profile.immutable_keys.clear();
        app_profile.mode = AppMode::Setup;
        app_profile.bind_addr = IpAddr::from([127, 0, 0, 1]);
        app_profile.http_port = reserved_port;
        service
            .apply_changeset(
                "tester",
                "bootstrap-bind-conflict",
                SettingsChangeset {
                    app_profile: Some(app_profile),
                    ..SettingsChangeset::default()
                },
            )
            .await?;

        let err = timeout(
            BOOTSTRAP_BIND_FAILURE_TIMEOUT,
            run_app_with_database_url(database_url),
        )
        .await
        .expect("bootstrap should surface bind failure without serving")
        .expect_err("occupied socket should fail api server startup");
        assert!(matches!(err, AppError::ApiServer { .. }), "{err:?}");
        return Ok(());
    }

    let postgres = match start_postgres() {
        Ok(database) => database,
        Err(err) => {
            eprintln!(
                "skipping run_app_with_database_url_surfaces_bind_failures_for_valid_persisted_config: {err}"
            );
            return Ok(());
        }
    };

    run_bootstrap_child(
        "run_app_with_database_url_surfaces_bind_failures_for_valid_persisted_config",
        &[
            ("DATABASE_URL", postgres.connection_string()),
            ("REVAER_TEST_DATABASE_URL", postgres.connection_string()),
            ("REVAER_BOOTSTRAP_CHILD_BIND_CONFLICT", "1"),
        ],
        &[],
    )
}

#[tokio::test]
async fn run_app_with_database_url_rejects_out_of_range_http_port_changes_before_bootstrap()
-> Result<()> {
    let _guard = bootstrap_test_guard().await;
    let postgres = match start_postgres() {
        Ok(database) => database,
        Err(err) => {
            eprintln!(
                "skipping run_app_with_database_url_rejects_out_of_range_http_port_changes_before_bootstrap: {err}"
            );
            return Ok(());
        }
    };

    let service = ConfigService::new(postgres.connection_string()).await?;
    let mut app_profile = service.get_app_profile().await?;
    app_profile.immutable_keys.clear();
    app_profile.mode = AppMode::Setup;
    app_profile.bind_addr = IpAddr::from([127, 0, 0, 1]);
    app_profile.http_port = 70_000;
    let err = service
        .apply_changeset(
            "tester",
            "bootstrap-out-of-range-port",
            SettingsChangeset {
                app_profile: Some(app_profile),
                ..SettingsChangeset::default()
            },
        )
        .await
        .expect_err("out of range http_port should be rejected before bootstrap starts");
    assert!(matches!(
        err,
        revaer_config::ConfigError::InvalidField {
            field,
            value,
            reason,
            ..
        } if field == "http_port" && value.as_deref() == Some("70000") && reason == "must be between 1 and 65535"
    ));
    Ok(())
}
