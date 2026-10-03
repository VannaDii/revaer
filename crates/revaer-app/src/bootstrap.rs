use std::borrow::Cow;
use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
#[cfg(feature = "libtorrent")]
use std::time::Instant;

use crate::error::{AppError, AppResult};
use crate::import_job_runtime::ImportJobRuntime;
use crate::indexer_runtime::IndexerRuntime;
use crate::indexers::IndexerService;
use crate::media::MediaService;
use crate::media_discovery_runtime::MediaDiscoveryRuntime;
use crate::media_job_runtime::MediaJobRuntime;
use crate::media_retention_runtime::MediaRetentionRuntime;
use crate::media_workspace_retention::MediaWorkspaceRetentionService;
use crate::runtime_shutdown;
use revaer_api::TorrentHandles;
use revaer_api::app::compliance::{
    ComplianceMetadataError, SOURCE_COMPLIANCE_BUNDLE_PATH, SourceComplianceMetadata,
};
use revaer_api::app::media::{MediaCapabilityRefreshParams, MediaFacade};
use revaer_config::{AppMode, ConfigService, ConfigSnapshot, DbSessionConfig};
use revaer_events::EventBus;
use revaer_telemetry::{GlobalContextGuard, LoggingConfig, Metrics, OpenTelemetryConfig};
use tracing::{error, info, warn};

use revaer_media_runtime::capabilities::{
    FfmpegCapabilityDetector, SupervisedCapabilityProbeExecutor,
};
use revaer_media_runtime::process::{
    NativeProcessSupervisor, NeverStopNativeProcess, SystemNativeProcessSupervisor,
};
use revaer_runtime::RuntimeStore;
use revaer_runtime::media::MediaStore;
use uuid::Uuid;

#[cfg(feature = "libtorrent")]
use crate::orchestrator::{
    EngineConfigurator, LibtorrentOrchestratorDeps, spawn_libtorrent_orchestrator,
};
#[cfg(feature = "libtorrent")]
use revaer_torrent_core::{TorrentEngine, TorrentInspector, TorrentWorkflow};

const SYSTEM_USER_PUBLIC_ID: Uuid = Uuid::from_u128(0);

mod root_catalog;

/// Dependencies required to bootstrap the Revaer application.
pub(crate) struct BootstrapDependencies {
    source_compliance: SourceComplianceMetadata,
    logging: LoggingConfig<'static>,
    otel_config: Option<OpenTelemetryConfig<'static>>,
    config: ConfigService,
    snapshot: ConfigSnapshot,
    watcher: revaer_config::ConfigWatcher,
    events: EventBus,
    telemetry: Metrics,
    media_workspace_root: PathBuf,
    media_root_source: root_catalog::CatalogSource,
    #[cfg(feature = "libtorrent")]
    libtorrent: Option<LibtorrentOrchestratorDeps>,
}

impl BootstrapDependencies {
    /// Construct production dependencies from the environment for the binary entrypoint.
    async fn from_env(source_compliance: SourceComplianceMetadata) -> AppResult<Self> {
        let database_url = database_url_from_env()?;
        Self::from_database_url(database_url, source_compliance).await
    }

    async fn from_database_url(
        database_url: String,
        source_compliance: SourceComplianceMetadata,
    ) -> AppResult<Self> {
        let media_workspace_root = media_workspace_root_from_env()?;
        Self::from_database_url_with_workspace_root(
            database_url,
            media_workspace_root,
            source_compliance,
        )
        .await
    }

    async fn from_database_url_with_workspace_root(
        database_url: String,
        media_workspace_root: PathBuf,
        source_compliance: SourceComplianceMetadata,
    ) -> AppResult<Self> {
        let logging = LoggingConfig::default();
        let otel_config = load_otel_config_from_env();

        let secret_encryption_config = secret_session_from_env()?;
        let config = ConfigService::new_with_session(database_url, secret_encryption_config)
            .await
            .map_err(|err| AppError::config("config_service.new", err))?;

        let (snapshot, watcher) = config
            .watch_settings(Duration::from_secs(5))
            .await
            .map_err(|err| AppError::config("config_service.watch_settings", err))?;

        let events = EventBus::new();
        let telemetry =
            Metrics::new().map_err(|err| AppError::telemetry("telemetry.metrics", err))?;

        #[cfg(feature = "libtorrent")]
        let runtime = Some(RuntimeStore::new(config.pool().clone()));
        #[cfg(not(feature = "libtorrent"))]
        let _runtime: Option<RuntimeStore> = None;

        #[cfg(feature = "libtorrent")]
        let libtorrent = Some(LibtorrentOrchestratorDeps::new(
            &events, &telemetry, runtime,
        )?);

        Ok(Self {
            source_compliance,
            logging,
            otel_config,
            config,
            snapshot,
            watcher,
            events,
            telemetry,
            media_workspace_root,
            media_root_source: root_catalog::source_from_env(),
            #[cfg(feature = "libtorrent")]
            libtorrent,
        })
    }
}

fn database_url_from_env() -> AppResult<String> {
    std::env::var("DATABASE_URL").map_err(|_| AppError::MissingEnv {
        name: "DATABASE_URL",
    })
}

fn media_workspace_root_from_env() -> AppResult<PathBuf> {
    media_workspace_root_from_value(std::env::var_os("REVAER_MEDIA_WORKSPACE_ROOT"))
}

fn media_workspace_root_from_value(value: Option<std::ffi::OsString>) -> AppResult<PathBuf> {
    value
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or(AppError::InvalidConfig {
            field: "REVAER_MEDIA_WORKSPACE_ROOT",
            reason: "absolute_private_workspace_root_required",
            value: None,
        })
}

/// Load the optional database session encryption configuration from the environment.
///
/// This function reads `REVAER_SECRET_KEY_ID` and `REVAER_SECRET_KEY` from the
/// process environment and, when both are present and valid, constructs a
/// [`DbSessionConfig`] used for envelope encryption of session data.
///
/// # When to set these variables
///
/// - Set both `REVAER_SECRET_KEY_ID` and `REVAER_SECRET_KEY` in deployments
///   where database session data must be encrypted at rest using envelope
///   encryption.
/// - Leave both unset if you do not want to enable this encryption mechanism;
///   in that case this function returns `Ok(None)` and no envelope encryption
///   key is configured.
///
/// The `REVAER_SECRET_KEY_ID` value is an opaque identifier that allows the
/// application to distinguish between different encryption keys (for example,
/// when performing key rotation). The `REVAER_SECRET_KEY` value is the actual
/// secret material used to encrypt and decrypt the encrypted payload.
///
/// # Consequences of misconfiguration
///
/// - If only one of `REVAER_SECRET_KEY_ID` or `REVAER_SECRET_KEY` is set, this
///   function returns an [`AppError::MissingEnv`] for the missing variable and
///   the application startup will fail.
/// - If either value is present but empty or only whitespace, this function
///   returns an [`AppError::InvalidConfig`] and the application startup will
///   fail.
/// - If these values are changed in a running deployment without migrating
///   existing data, previously encrypted sessions may become unreadable,
///   causing failures when attempting to decrypt stored data.
///
/// Callers should treat both variables as security-sensitive secrets and
/// manage them via a secure secret management system. Changes to either value
/// should be coordinated with any envelope encryption key-rotation or
/// data-migration process in order to avoid data loss.
fn secret_session_from_env() -> AppResult<Option<DbSessionConfig>> {
    let key_id = optional_env_var("REVAER_SECRET_KEY_ID")?;
    let secret_value = optional_env_var("REVAER_SECRET_KEY")?;

    secret_session_from_values(key_id.as_deref(), secret_value.as_deref())
}

fn optional_env_var(name: &'static str) -> AppResult<Option<String>> {
    optional_env_var_with(name, std::env::var)
}

fn optional_env_var_with(
    name: &'static str,
    getter: impl FnOnce(&'static str) -> Result<String, std::env::VarError>,
) -> AppResult<Option<String>> {
    match getter(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => Err(AppError::InvalidConfig {
            field: name,
            reason: "env_not_unicode",
            value: None,
        }),
    }
}

fn secret_session_from_values(
    key_id: Option<&str>,
    secret_value: Option<&str>,
) -> AppResult<Option<DbSessionConfig>> {
    match (key_id, secret_value) {
        (None, None) => Ok(None),
        (Some(key_id), Some(secret_value)) => {
            // Maximum length: 128 bytes (as defined in `DbSessionConfig::SECRET_KEY_ID_MAX_LEN`).
            let trimmed_key_id = validate_trimmed_field(
                key_id,
                "REVAER_SECRET_KEY_ID",
                DbSessionConfig::SECRET_KEY_ID_MAX_LEN,
            )?;
            let trimmed_secret = validate_trimmed_field(
                secret_value,
                "REVAER_SECRET_KEY",
                DbSessionConfig::SECRET_KEY_MAX_LEN,
            )?;
            Ok(Some(DbSessionConfig::new(trimmed_key_id, trimmed_secret)))
        }
        (Some(_), None) => Err(AppError::MissingEnv {
            name: "REVAER_SECRET_KEY",
        }),
        (None, Some(_)) => Err(AppError::MissingEnv {
            name: "REVAER_SECRET_KEY_ID",
        }),
    }
}

fn validate_trimmed_field<'a>(
    value: &'a str,
    field: &'static str,
    max_len: usize,
) -> AppResult<&'a str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidConfig {
            field,
            reason: "empty",
            value: None,
        });
    }
    if trimmed.len() > max_len {
        return Err(AppError::InvalidConfig {
            field,
            reason: "too_long",
            value: None,
        });
    }
    Ok(trimmed)
}

/// Entry point for the Revaer application boot sequence.
/// Required packaged metadata is checked before constructing any dependencies.
///
/// # Errors
///
/// Returns an error if dependency construction or application startup fails.
pub async fn run_app() -> AppResult<()> {
    run_app_with_compliance_loader(None, load_packaged_compliance, &mut std::io::stderr()).await
}

/// Boot sequence using a provided database URL.
/// Required packaged metadata is checked before constructing any dependencies.
///
/// # Errors
///
/// Returns an error if dependency construction or application startup fails.
pub async fn run_app_with_database_url(database_url: String) -> AppResult<()> {
    run_app_with_compliance_loader(
        Some(database_url),
        load_packaged_compliance,
        &mut std::io::stderr(),
    )
    .await
}

fn load_packaged_compliance() -> Result<SourceComplianceMetadata, ComplianceMetadataError> {
    SourceComplianceMetadata::load(std::path::Path::new(SOURCE_COMPLIANCE_BUNDLE_PATH))
}

async fn run_app_with_compliance_loader(
    database_url: Option<String>,
    loader: impl FnOnce() -> Result<SourceComplianceMetadata, ComplianceMetadataError>,
    diagnostic: &mut impl Write,
) -> AppResult<()> {
    // This synchronous preflight must precede even dependency construction:
    // configuration and native dependency constructors already start work.
    let source_compliance = match loader() {
        Ok(metadata) => metadata,
        Err(compliance) => {
            return Err(
                match writeln!(
                    diagnostic,
                    "compliance_metadata_startup_failed cause={}",
                    compliance.category(),
                ) {
                    Ok(()) => AppError::Compliance { source: compliance },
                    Err(source) => AppError::ComplianceDiagnostic { compliance, source },
                },
            );
        }
    };
    let dependencies = match database_url {
        Some(url) => BootstrapDependencies::from_database_url(url, source_compliance).await?,
        None => BootstrapDependencies::from_env(source_compliance).await?,
    };
    Box::pin(run_app_with(dependencies)).await
}

/// Boot sequence that relies entirely on injected dependencies to simplify testing.
pub(crate) async fn run_app_with(dependencies: BootstrapDependencies) -> AppResult<()> {
    let _otel_guard = init_bootstrap_logging(&dependencies)?;
    let _context = GlobalContextGuard::new("bootstrap");
    info!("Revaer application bootstrap starting");
    Box::pin(run_bootstrap_services(dependencies)).await
}

fn init_bootstrap_logging(
    dependencies: &BootstrapDependencies,
) -> AppResult<Option<revaer_telemetry::OpenTelemetryGuard>> {
    let otel_ref = dependencies
        .otel_config
        .as_ref()
        .map(|cfg| cfg as &OpenTelemetryConfig);
    revaer_telemetry::init_logging_with_otel(&dependencies.logging, otel_ref)
        .map_err(|err| AppError::telemetry("telemetry.init", err))
}

async fn run_bootstrap_services(dependencies: BootstrapDependencies) -> AppResult<()> {
    let BootstrapDependencies {
        source_compliance,
        logging: _,
        otel_config: _,
        config,
        snapshot,
        watcher,
        events,
        telemetry,
        media_workspace_root,
        media_root_source,
        #[cfg(feature = "libtorrent")]
        libtorrent,
    } = dependencies;

    let addr = bootstrap_listener_addr(&snapshot.app_profile, &telemetry, &events)?;
    // Retain catalog descriptors and root locks until the service has stopped.
    let media_roots = root_catalog::start(&config, media_root_source).await?;
    let (shutdown, receiver) = runtime_shutdown::channel();

    #[cfg(feature = "libtorrent")]
    let (fsops_worker, mut config_task, torrent_handles) = {
        let libtorrent = libtorrent.ok_or(AppError::MissingDependency { name: "libtorrent" })?;
        let (_engine, orchestrator, worker) = spawn_libtorrent_orchestrator(
            &events,
            snapshot.fs_policy.clone(),
            snapshot.engine_profile.clone(),
            libtorrent,
            Some(Arc::new(config.clone())),
        )
        .await?;
        info!("Filesystem post-processing orchestrator ready");
        let workflow: Arc<dyn TorrentWorkflow> = orchestrator.clone();
        let inspector: Arc<dyn TorrentInspector> = orchestrator.clone();
        let handles = TorrentHandles::new(workflow, inspector);
        let config_task = spawn_config_watch_task(
            watcher,
            Arc::clone(&orchestrator),
            events.clone(),
            telemetry.clone(),
            receiver.clone(),
        );
        (worker, config_task, Some(handles))
    };

    #[cfg(not(feature = "libtorrent"))]
    let torrent_handles: Option<TorrentHandles> = {
        let _ = watcher;
        let _ = &snapshot.fs_policy;
        let _ = &snapshot.engine_profile;
        None
    };

    let native_process_supervisor = system_native_process_supervisor();
    let media = Arc::new(
        build_media_service(
            &config,
            telemetry.clone(),
            Arc::clone(&native_process_supervisor),
        )
        .with_association_source(media_roots.as_ref().map(Arc::clone)),
    );
    refresh_startup_media_capabilities(&media, &events, &telemetry).await;
    let api = build_api_server(
        &config,
        &events,
        torrent_handles,
        telemetry.clone(),
        media,
        source_compliance,
    )?;
    let indexer_runtime_task =
        IndexerRuntime::new(Arc::new(config.clone()), telemetry.clone()).spawn();
    let import_job_runtime_task =
        ImportJobRuntime::new(Arc::new(config.clone()), telemetry.clone()).spawn();
    let media_runtime_tasks = spawn_media_runtime_tasks(
        &config,
        &events,
        &telemetry,
        media_workspace_root,
        native_process_supervisor,
        receiver,
    );
    info!(addr = %addr, "Launching API listener");

    let serve_result = api.serve(addr).await;

    request_runtime_shutdown(&shutdown);
    #[cfg(feature = "libtorrent")]
    stop_config_watch_task(&mut config_task, &shutdown).await;
    stop_runtime_task(indexer_runtime_task, "indexer").await;
    stop_runtime_task(import_job_runtime_task, "import_job").await;
    stop_media_runtime_tasks(media_runtime_tasks, &shutdown).await;

    #[cfg(feature = "libtorrent")]
    stop_runtime_task(fsops_worker, "fsops").await;

    serve_result.map_err(|err| AppError::api_server("api_server.serve", err))?;
    info!("API server shutdown complete");
    Ok(())
}

struct MediaRuntimeTasks {
    discovery: tokio::task::JoinHandle<()>,
    job: tokio::task::JoinHandle<()>,
    retention: tokio::task::JoinHandle<()>,
}

fn spawn_media_runtime_tasks(
    config: &ConfigService,
    events: &EventBus,
    telemetry: &Metrics,
    media_workspace_root: PathBuf,
    native_process_supervisor: Arc<dyn NativeProcessSupervisor>,
    receiver: runtime_shutdown::RuntimeShutdownReceiver,
) -> MediaRuntimeTasks {
    let media_store = MediaStore::new(config.pool().clone());
    let discovery =
        MediaDiscoveryRuntime::new(media_store.clone(), telemetry.clone()).spawn(receiver.clone());
    let job = MediaJobRuntime::new(
        media_store.clone(),
        events.clone(),
        telemetry.clone(),
        media_workspace_root.clone(),
        native_process_supervisor,
    )
    .spawn(receiver.clone());
    let media_workspace_retention = Arc::new(MediaWorkspaceRetentionService::new(
        media_store.clone(),
        telemetry.clone(),
        media_workspace_root,
    ));
    let retention =
        MediaRetentionRuntime::new(media_store, media_workspace_retention, telemetry.clone())
            .spawn(receiver);
    MediaRuntimeTasks {
        discovery,
        job,
        retention,
    }
}

fn request_runtime_shutdown(shutdown: &runtime_shutdown::RuntimeShutdownSender) {
    if runtime_shutdown::request(shutdown) {
        info!("media runtime shutdown requested");
    } else {
        warn!("media runtime shutdown requested after receivers closed");
    }
}

async fn stop_media_runtime_tasks(
    tasks: MediaRuntimeTasks,
    shutdown: &runtime_shutdown::RuntimeShutdownSender,
) {
    stop_runtime_task_gracefully(tasks.discovery, "media_discovery", shutdown).await;
    stop_runtime_task_gracefully(tasks.job, "media_job", shutdown).await;
    stop_runtime_task_gracefully(tasks.retention, "media_retention", shutdown).await;
}

#[cfg(any(feature = "libtorrent", test))]
async fn stop_config_watch_task(
    task: &mut tokio::task::JoinHandle<()>,
    shutdown: &runtime_shutdown::RuntimeShutdownSender,
) {
    tokio::select! {
        biased;
        result = &mut *task => {
            if let Err(error) = result {
                log_runtime_task_join_error(&error, "config_watcher", false);
            }
        }
        elapsed = runtime_shutdown::deadline_elapsed(shutdown) => {
            task.abort();
            match elapsed {
                Ok(()) => warn!(
                    reason = "shared_shutdown_deadline",
                    "config watcher task aborted during bootstrap shutdown"
                ),
                Err(error) => warn!(
                    error = %error,
                    task = "config_watcher",
                    "runtime shutdown authority closed before deadline observation"
                ),
            }
            // No time remains for another grace or an unbounded post-abort join.
            // Keep the handle with bootstrap and report only an observed outcome.
            let joined = std::future::poll_fn(|context| {
                std::task::Poll::Ready(std::future::Future::poll(
                    std::pin::Pin::new(&mut *task), context,
                ))
            }).await;
            match joined {
                std::task::Poll::Ready(Ok(())) => {}
                std::task::Poll::Ready(Err(error)) => {
                    log_runtime_task_join_error(&error, "config_watcher", true);
                }
                std::task::Poll::Pending => {
                    warn!("config watcher task settlement unconfirmed after shutdown abort request");
                }
            }
        }
    }
}

async fn stop_runtime_task<T>(task: tokio::task::JoinHandle<T>, task_name: &'static str) {
    let abort_requested = !task.is_finished();
    if abort_requested {
        task.abort();
    }
    if let Err(err) = task.await {
        log_runtime_task_join_error(&err, task_name, abort_requested);
    }
}

async fn stop_runtime_task_gracefully<T>(
    mut task: tokio::task::JoinHandle<T>,
    task_name: &'static str,
    shutdown: &runtime_shutdown::RuntimeShutdownSender,
) {
    if task.is_finished() {
        if let Err(err) = task.await {
            log_runtime_task_join_error(&err, task_name, false);
        }
        return;
    }

    tokio::select! {
        result = &mut task => {
            if let Err(err) = result {
                log_runtime_task_join_error(&err, task_name, false);
            }
        }
        elapsed = runtime_shutdown::deadline_elapsed(shutdown) => {
            task.abort();
            match elapsed {
                Ok(()) => warn!(
                    task = task_name,
                    "runtime task aborted after graceful shutdown timeout"
                ),
                Err(error) => warn!(
                    error = %error,
                    task = task_name,
                    "runtime shutdown authority closed before deadline observation"
                ),
            }
            // S2's independent owner and native settlement remain separate work.
            // This existing join does not grant a new cooperative grace period.
            if let Err(err) = task.await {
                log_runtime_task_join_error(&err, task_name, true);
            }
        }
    }
}

fn log_runtime_task_join_error(
    error: &tokio::task::JoinError,
    task_name: &'static str,
    abort_requested: bool,
) {
    if abort_requested && error.is_cancelled() {
        info!(error = %error, task = task_name, "runtime task cancelled after shutdown abort request");
    } else {
        warn!(error = %error, task = task_name, "runtime task join failed");
    }
}

fn bootstrap_listener_addr(
    app_profile: &revaer_config::AppProfile,
    telemetry: &Metrics,
    events: &EventBus,
) -> AppResult<SocketAddr> {
    enforce_loopback_guard(&app_profile.mode, app_profile.bind_addr, telemetry, events)?;
    let port = bootstrap_http_port(app_profile.http_port)?;
    Ok(SocketAddr::new(app_profile.bind_addr, port))
}

fn bootstrap_http_port(http_port: i32) -> AppResult<u16> {
    let port = u16::try_from(http_port).map_err(|_| AppError::InvalidConfig {
        field: "http_port",
        reason: "out_of_range",
        value: Some(http_port.to_string()),
    })?;
    if port == 0 {
        return Err(AppError::InvalidConfig {
            field: "http_port",
            reason: "zero",
            value: Some(http_port.to_string()),
        });
    }
    Ok(port)
}

fn build_api_server(
    config: &ConfigService,
    events: &EventBus,
    torrent_handles: Option<TorrentHandles>,
    telemetry: Metrics,
    media: Arc<MediaService>,
    source_compliance: SourceComplianceMetadata,
) -> AppResult<revaer_api::ApiServer> {
    let indexers = Arc::new(IndexerService::new(
        Arc::new(config.clone()),
        telemetry.clone(),
    ));
    revaer_api::ApiServer::new_with_media(
        config.clone(),
        indexers,
        media,
        events.clone(),
        torrent_handles,
        telemetry,
        source_compliance,
    )
    .map_err(|err| AppError::api_server("api_server.new", err))
}

fn system_native_process_supervisor() -> Arc<dyn NativeProcessSupervisor> {
    Arc::new(SystemNativeProcessSupervisor)
}

fn build_media_service(
    config: &ConfigService,
    telemetry: Metrics,
    native_process_supervisor: Arc<dyn NativeProcessSupervisor>,
) -> MediaService {
    MediaService::new(
        MediaStore::new(config.pool().clone()),
        Arc::new(FfmpegCapabilityDetector::new(
            Arc::new(SupervisedCapabilityProbeExecutor::new(
                native_process_supervisor,
                Arc::new(NeverStopNativeProcess),
            )),
            "ffmpeg",
            "ffprobe",
            "ffplay",
        )),
        telemetry,
    )
}

async fn refresh_startup_media_capabilities(
    media: &MediaService,
    events: &EventBus,
    telemetry: &Metrics,
) {
    match media
        .media_capability_refresh(MediaCapabilityRefreshParams {
            actor_user_public_id: SYSTEM_USER_PUBLIC_ID,
        })
        .await
    {
        Ok(snapshot_id) => {
            info!(
                media_capability_snapshot_id = snapshot_id,
                "startup media capability refresh completed"
            );
            publish_event(
                events,
                revaer_events::Event::MediaCapabilitiesRefreshed {
                    media_capability_snapshot_id: snapshot_id,
                },
            );
        }
        Err(error) => {
            let code = error
                .code()
                .unwrap_or("media_capability_refresh_failed")
                .to_string();
            warn!(
                error = %error,
                code = %code,
                "startup media capability refresh failed; media execution remains not ready"
            );
            telemetry.inc_event("media_capability_refresh_failed");
            publish_event(
                events,
                revaer_events::Event::MediaCapabilitiesRefreshFailed { reason: code },
            );
            publish_event(
                events,
                revaer_events::Event::HealthChanged {
                    degraded: vec!["media_capability".to_string()],
                },
            );
        }
    }
}

fn load_otel_config_from_env() -> Option<OpenTelemetryConfig<'static>> {
    let enabled = env_flag("REVAER_ENABLE_OTEL");
    let service_name =
        std::env::var("REVAER_OTEL_SERVICE_NAME").unwrap_or_else(|_| "revaer-app".to_string());
    let endpoint = std::env::var("REVAER_OTEL_EXPORTER")
        .ok()
        .or_else(|| std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok());
    otel_config_from_values(enabled, service_name, endpoint)
}

fn env_flag(name: &str) -> bool {
    env_flag_value(std::env::var(name).ok().as_deref())
}

fn env_flag_value(value: Option<&str>) -> bool {
    value.is_some_and(|v| {
        matches!(
            v.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn otel_config_from_values(
    enabled: bool,
    service_name: String,
    endpoint: Option<String>,
) -> Option<OpenTelemetryConfig<'static>> {
    if !enabled {
        return None;
    }
    Some(OpenTelemetryConfig {
        enabled: true,
        service_name: Cow::Owned(service_name),
        endpoint: endpoint.map(Cow::Owned),
    })
}

#[cfg(feature = "libtorrent")]
fn spawn_config_watch_task<E>(
    mut watcher: revaer_config::ConfigWatcher,
    orchestrator: Arc<crate::orchestrator::TorrentOrchestrator<E>>,
    events: EventBus,
    telemetry: Metrics,
    mut shutdown: runtime_shutdown::RuntimeShutdownReceiver,
) -> tokio::task::JoinHandle<()>
where
    E: TorrentEngine + EngineConfigurator + 'static,
{
    tokio::spawn(async move {
        const APPLY_SLA: Duration = Duration::from_secs(2);
        let mut config_degraded = false;
        loop {
            let wait_started = Instant::now();
            let result = Box::pin(config_watch_step(
                watcher.next(),
                |snapshot| async {
                    telemetry.observe_config_watch_latency(wait_started.elapsed());
                    apply_config_snapshot(
                        snapshot,
                        &orchestrator,
                        &events,
                        &telemetry,
                        &mut config_degraded,
                        APPLY_SLA,
                    )
                    .await;
                },
                &mut shutdown,
            ))
            .await;
            match result {
                Ok(std::ops::ControlFlow::Continue(())) => {}
                Ok(std::ops::ControlFlow::Break(())) => break,
                Err(err) => {
                    telemetry.inc_config_update_failure();
                    warn!(error = %err, "configuration watcher terminated");
                    set_config_degraded(&events, &mut config_degraded, true);
                    break;
                }
            }
        }
    })
}

#[cfg(any(feature = "libtorrent", test))]
async fn config_watch_step<T, E, A: std::future::Future<Output = ()>>(
    next: impl std::future::Future<Output = Result<T, E>>,
    apply: impl FnOnce(T) -> A,
    shutdown: &mut runtime_shutdown::RuntimeShutdownReceiver,
) -> Result<std::ops::ControlFlow<()>, E> {
    let update = tokio::select! {
        biased;
        () = runtime_shutdown::changed(shutdown) => return Ok(std::ops::ControlFlow::Break(())),
        update = next => update?,
    };
    // Drain can latch while next is polled. This check admits the snapshot;
    // once admitted, its serial apply (including limits ACK) must finish.
    if runtime_shutdown::requested(shutdown) {
        return Ok(std::ops::ControlFlow::Break(()));
    }
    apply(update).await;
    Ok(std::ops::ControlFlow::Continue(()))
}

#[cfg(feature = "libtorrent")]
async fn apply_config_snapshot<E>(
    snapshot: revaer_config::ConfigSnapshot,
    orchestrator: &crate::orchestrator::TorrentOrchestrator<E>,
    events: &EventBus,
    telemetry: &Metrics,
    config_degraded: &mut bool,
    apply_sla: Duration,
) where
    E: TorrentEngine + EngineConfigurator + 'static,
{
    orchestrator
        .update_fs_policy(snapshot.fs_policy.clone())
        .await;
    let apply_started = Instant::now();
    match orchestrator
        .update_engine_profile(snapshot.engine_profile.clone())
        .await
    {
        Ok(()) => {
            let apply_elapsed = apply_started.elapsed();
            telemetry.observe_config_apply_latency(apply_elapsed);
            let mut description = format!(
                "watcher revision {} applied in {}ms",
                snapshot.revision,
                apply_elapsed.as_millis()
            );
            if apply_elapsed > apply_sla {
                telemetry.inc_config_watch_slow();
                warn!(
                    revision = snapshot.revision,
                    elapsed_ms = apply_elapsed.as_millis(),
                    "configuration update exceeded latency guard rail"
                );
                description = format!(
                    "watcher revision {} applied after {}ms (exceeded guard rail)",
                    snapshot.revision,
                    apply_elapsed.as_millis()
                );
                set_config_degraded(events, config_degraded, true);
            } else {
                set_config_degraded(events, config_degraded, false);
            }
            publish_event(
                events,
                revaer_events::Event::SettingsChanged { description },
            );
            info!(
                revision = snapshot.revision,
                elapsed_ms = apply_elapsed.as_millis(),
                "applied configuration update from watcher"
            );
        }
        Err(err) => {
            telemetry.inc_config_update_failure();
            warn!(
                error = %err,
                revision = snapshot.revision,
                "failed to apply engine profile update from watcher"
            );
            let description = format!(
                "failed to apply watcher revision {}: {}",
                snapshot.revision, err
            );
            publish_event(
                events,
                revaer_events::Event::SettingsChanged { description },
            );
            set_config_degraded(events, config_degraded, true);
        }
    }
}

#[cfg(feature = "libtorrent")]
fn set_config_degraded(events: &EventBus, config_degraded: &mut bool, degraded: bool) {
    if *config_degraded == degraded {
        return;
    }
    let degraded_list = if degraded {
        vec!["config_watcher".to_string()]
    } else {
        Vec::new()
    };
    publish_event(
        events,
        revaer_events::Event::HealthChanged {
            degraded: degraded_list,
        },
    );
    *config_degraded = degraded;
}

fn enforce_loopback_guard(
    mode: &AppMode,
    bind_addr: IpAddr,
    telemetry: &Metrics,
    events: &EventBus,
) -> AppResult<()> {
    if matches!(mode, AppMode::Setup) && !bind_addr.is_loopback() {
        error!(
            bind_addr = %bind_addr,
            "refusing to bind setup mode API listener to non-loopback address"
        );
        telemetry.inc_guardrail_violation();
        publish_event(
            events,
            revaer_events::Event::HealthChanged {
                degraded: vec!["loopback_guard".to_string()],
            },
        );
        return Err(AppError::InvalidConfig {
            field: "bind_addr",
            reason: "non_loopback_in_setup",
            value: Some(bind_addr.to_string()),
        });
    }
    Ok(())
}

fn publish_event(events: &EventBus, event: revaer_events::Event) {
    if let Err(error) = events.publish(event) {
        tracing::warn!(
            event_id = error.event_id(),
            event_kind = error.event_kind(),
            error = %error,
            "failed to publish event"
        );
    }
}

#[cfg(test)]
#[path = "bootstrap/tests.rs"]
mod tests;

#[cfg(test)]
mod compliance_tests;
#[cfg(test)]
mod runtime_tests;

#[cfg(test)]
#[path = "bootstrap/shutdown_tests.rs"]
mod shutdown_tests;
