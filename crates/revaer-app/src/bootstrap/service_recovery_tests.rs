//! Explicit native-mount qualification of the serving application's recovery.

#[path = "service_recovery_tests/discovery.rs"]
mod discovery;

#[path = "service_recovery_tests/operator.rs"]
mod operator;

use std::{
    fs,
    io::Write,
    net::TcpListener,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Stdio,
    sync::OnceLock,
    time::Duration,
};

use anyhow::{Context, Result};
use revaer_config::{ConfigService, SettingsChangeset, SettingsFacade};
use revaer_data::media::jobs::{
    get_media_job, list_media_job_attempt_identities, list_media_job_compact_audits,
};
use revaer_data::media::step_checkpoints::{StepCheckpoint, get_step_checkpoint};
use revaer_media_runtime::execute::{CommandRunner, ProcessCommandRunner};
use revaer_media_runtime::replacement::{
    ReplacementCommitter, ReplacementRequest, SystemReplacementCommitter,
};
use revaer_test_support::postgres::{TestDatabase, start_postgres};
use serde_json::{Value, json};
use tokio::{io::AsyncReadExt, process::Child, time::timeout};
use uuid::Uuid;

const BOUND: Duration = Duration::from_mins(5);
const ENTRY: &str = "bootstrap::runtime_tests::e2e_serving_entry";

struct Fixture {
    directory: tempfile::TempDir,
    postgres: TestDatabase,
    config: ConfigService,
    api: reqwest::Client,
    origin: String,
    source: PathBuf,
    original: Vec<u8>,
    api_key: OnceLock<String>,
}

#[tokio::test]
#[ignore = "requires just test-media-service-recovery and an owned persistent Linux mount"]
async fn native_service_shutdown_resumes_active_ffmpeg() -> Result<()> {
    let fixture = Fixture::create_with_duration("0.1").await?;
    let result = operator::qualify(&fixture).await;
    finish_fixture(fixture, result).await?;
    let fixture = Fixture::create_with_duration("0.1").await?;
    let result = operator::scratch(&fixture).await;
    finish_fixture(fixture, result).await?;
    for mode in [discovery::Mode::Watcher, discovery::Mode::Schedule] {
        let fixture = Fixture::create_with_duration("0.1").await?;
        let result = discovery::qualify(&fixture, mode).await;
        finish_fixture(fixture, result).await?;
    }
    let fixture = Fixture::create().await?;
    let result = qualify(&fixture).await;
    finish_fixture(fixture, result).await?;
    for fault in [
        CheckpointFault::Missing,
        CheckpointFault::Corrupt,
        CheckpointFault::PreparedReplacement,
        CheckpointFault::CommittedReplacement,
    ] {
        let mut fixture = Fixture::create().await?;
        let result = async {
            embed_subtitles(&mut fixture)?;
            checkpoint_fault_restart(&fixture, fault).await
        }
        .await;
        finish_fixture(fixture, result).await?;
    }
    Ok(())
}

async fn finish_fixture(fixture: Fixture, result: Result<()>) -> Result<()> {
    fixture.config.pool().close().await;
    let cleanup = settled(
        fixture.postgres.close(),
        fixture.directory.close().map_err(Into::into),
    );
    settled(result, cleanup)
}

async fn qualify(fixture: &Fixture) -> Result<()> {
    let first = fixture.start().await?;
    let initial = async {
        fixture.activate().await?;
        fixture.ready().await?;
        let (job, association) = fixture.admit(vec![video_target()]).await?;
        let child = wait_for_ffmpeg(fixture, job, first.id().context("serving PID")?).await?;
        let attempt = fixture.attempt(job).await?;
        anyhow::ensure!(attempt == (1, 1), "unexpected attempt: {attempt:?}");
        anyhow::Ok((job, child, attempt, association))
    }
    .await;
    let (job, child, original_attempt, association) =
        settled(initial, signal_and_join(first).await)?;
    assert!(!PathBuf::from(format!("/proc/{child}")).try_exists()?);
    let interrupted = get_media_job(fixture.config.pool(), job)
        .await?
        .context("job")?;
    assert_eq!(interrupted.status_text, "queued", "{interrupted:?}");
    assert!(interrupted.last_error.is_none());
    assert!(interrupted.completed_at.is_none());
    assert_eq!(fixture.attempt(job).await?, original_attempt);
    assert_eq!(fs::read(&fixture.source)?, fixture.original);

    let prior_audits = list_media_job_compact_audits(fixture.config.pool(), job).await?;
    let second = fixture.start().await?;
    let replay = async {
        fixture.ready().await?;
        wait_for_ffmpeg(fixture, job, second.id().context("replay PID")?).await?;
        anyhow::ensure!(fixture.attempt(job).await? == original_attempt);
        timeout(BOUND, async {
            loop {
                let row = get_media_job(fixture.config.pool(), job)
                    .await?
                    .context("replay job")?;
                if row.status_text == "completed" {
                    anyhow::ensure!(row.last_error.is_none(), "{row:?}");
                    break;
                }
                anyhow::ensure!(
                    row.status_text != "failed" && row.status_text != "cancelled",
                    "{row:?}"
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            anyhow::Ok(())
        })
        .await??;
        anyhow::ensure!(
            fs::read(&fixture.source)? != fixture.original,
            "replacement did not change source"
        );
        cancel_active(
            fixture,
            association,
            second.id().context("cancellation PID")?,
        )
        .await
    }
    .await;
    let cancelled = settled(replay, signal_and_join(second).await)?;
    let replay_audits = list_media_job_compact_audits(fixture.config.pool(), job).await?;
    for prior in prior_audits {
        assert!(
            replay_audits
                .iter()
                .any(|row| row.audit_index == prior.audit_index
                    && row.fact_kind == prior.fact_kind
                    && row.fact_text == prior.fact_text)
        );
    }
    assert_eq!(
        replay_audits
            .iter()
            .filter(|row| row.fact_kind == "workspace_capacity")
            .count(),
        2
    );
    verify_cancelled_restart(fixture, cancelled).await?;
    for fault in [SourceFault::Changed, SourceFault::Missing] {
        source_fault_restart(fixture, association, fault).await?;
    }
    println!(
        "native service: active FFmpeg joined; original preserved on interruption; attempt 1/generation 1 replay completed without failure retry"
    );
    Ok(())
}

fn video_target() -> Value {
    json!({"stream_key":"video-main", "stream_kind":"video", "optional":false,
        "sort_order":0, "codec":"hevc", "default_disposition":true, "forced_disposition":false})
}

fn embed_subtitles(fixture: &mut Fixture) -> Result<()> {
    let subtitles = fixture.directory.path().join("cues.srt");
    let mut cues = std::io::BufWriter::new(fs::File::create(&subtitles)?);
    // A substantial real subtitle track keeps extraction observable without
    // changing the production command runner or adding a synthetic execution step.
    for index in 1..=100_000 {
        writeln!(
            cues,
            "{index}\n00:00:00,000 --> 00:02:59,000\nCheckpoint qualification cue {index}\n"
        )?;
    }
    cues.flush()?;
    drop(cues);
    let output = fixture.directory.path().join("subtitled.mkv");
    ProcessCommandRunner.run(
        "ffmpeg",
        &[
            "-nostdin",
            "-y",
            "-loglevel",
            "error",
            "-i",
            fixture.source.to_str().context("subtitle source")?,
            "-i",
            subtitles.to_str().context("subtitle input")?,
            "-map",
            "0:v",
            "-map",
            "1:0",
            "-c",
            "copy",
            "-metadata:s:s:0",
            "language=eng",
            output.to_str().context("subtitle output")?,
        ]
        .map(str::to_owned),
    )?;
    fs::rename(output, &fixture.source)?;
    fs::remove_file(subtitles)?;
    fixture.original = fs::read(&fixture.source)?;
    Ok(())
}

#[derive(Clone, Copy)]
enum CheckpointFault {
    Missing,
    Corrupt,
    PreparedReplacement,
    CommittedReplacement,
}

impl CheckpointFault {
    fn apply(self, fixture: &Fixture, job: Uuid, generation: i64, output: &Path) -> Result<()> {
        match self {
            Self::Missing => fs::remove_file(output)?,
            Self::Corrupt => fs::write(output, b"corrupt completed checkpoint")?,
            Self::PreparedReplacement | Self::CommittedReplacement => {
                let key = format!("{job}-g{generation}");
                let prepared = SystemReplacementCommitter.prepare(ReplacementRequest {
                    job_key: &key,
                    source_root: &fixture.directory.path().join("source"),
                    source_path: &fixture.source,
                    candidate_path: output,
                })?;
                anyhow::ensure!(fs::read(prepared.recovery_path())? == fixture.original);
                anyhow::ensure!(fs::read(&fixture.source)? == fixture.original);
                // Leave the actual durable transaction for startup to reconcile.
                if matches!(self, Self::CommittedReplacement) {
                    let committed = SystemReplacementCommitter.commit(prepared)?;
                    anyhow::ensure!(fs::read(&fixture.source)? != fixture.original);
                    drop(committed);
                } else {
                    drop(prepared);
                }
            }
        }
        Ok(())
    }
}

async fn checkpoint_fault_restart(fixture: &Fixture, fault: CheckpointFault) -> Result<()> {
    let first = fixture.start().await?;
    let initial = async {
        fixture.activate().await?;
        fixture.ready().await?;
        let (job, _) = fixture.admit(vec![video_target(), json!({
            "stream_key":"subtitle-main", "stream_kind":"subtitle", "optional":false,
            "sort_order":1, "codec":"subrip", "language_code":"eng", "subtitle_placement":"sidecar",
            "default_disposition":false, "forced_disposition":false
        })]).await?;
        wait_for_ffmpeg(fixture, job, first.id().context("initial checkpoint PID")?).await?;
        let attempt = fixture.attempt(job).await?;
        let checkpoint = wait_checkpoint(fixture, job, attempt.1).await?;
        let child = wait_for_ffmpeg(fixture, job, first.id().context("checkpoint PID")?).await?;
        let command = fs::read(format!("/proc/{child}/cmdline"))?;
        anyhow::ensure!(
            command
                .split(|byte| *byte == 0)
                .any(|arg| arg.ends_with(b".srt")),
            "observed FFmpeg is not the subtitle writer"
        );
        let pid =
            rustix::process::Pid::from_raw(i32::try_from(child)?).context("subtitle writer PID")?;
        rustix::process::kill_process(pid, rustix::process::Signal::STOP)?;
        anyhow::ensure!(
            get_step_checkpoint(fixture.config.pool(), job, attempt.1, 2)
                .await?
                .is_none(),
            "subtitle writer already checkpointed"
        );
        anyhow::Ok((job, attempt, checkpoint, child))
    }
    .await;
    let (job, attempt, checkpoint, child) = settled(initial, signal_and_join(first).await)?;
    anyhow::ensure!(
        !PathBuf::from(format!("/proc/{child}")).try_exists()?,
        "stopped writer still active"
    );
    anyhow::ensure!(
        fs::read(&fixture.source)? == fixture.original,
        "interruption changed source"
    );
    let output = PathBuf::from(&checkpoint.output_path);
    anyhow::ensure!(
        output.starts_with(fixture.directory.path().join("workspace")),
        "checkpoint escaped owned workspace"
    );
    fault.apply(fixture, job, attempt.1, &output)?;
    let second = fixture.start().await?;
    let replay = async {
        fixture.ready().await?;
        let child =
            wait_for_ffmpeg(fixture, job, second.id().context("checkpoint replay PID")?).await?;
        let command = fs::read(format!("/proc/{child}/cmdline"))?;
        anyhow::ensure!(
            command.split(|byte| *byte == 0).any(|arg| match fault {
                CheckpointFault::PreparedReplacement | CheckpointFault::CommittedReplacement =>
                    arg.ends_with(b".srt"),
                CheckpointFault::Missing | CheckpointFault::Corrupt => arg == b"libx265",
            }),
            "replay did not restart the required writer"
        );
        wait_completed(fixture, job).await?;
        anyhow::ensure!(
            fixture.attempt(job).await? == attempt,
            "checkpoint replay consumed a retry"
        );
        anyhow::ensure!(
            fs::read(&fixture.source)? != fixture.original,
            "checkpoint replay did not replace verified media"
        );
        Ok(())
    }
    .await;
    settled(replay, signal_and_join(second).await)?;
    // Completion is committed before backup finalization; join its owner first.
    anyhow::ensure!(
        !fixture
            .directory
            .path()
            .join("source/.revaer-media-runtime/replacements")
            .join(format!("{job}-g{}", attempt.1))
            .try_exists()?,
        "completed job retained an unfinished replacement transaction"
    );
    println!(
        "native service: checkpoint/replacement fault reconciled; required writer completed on the same attempt"
    );
    Ok(())
}

async fn wait_checkpoint(fixture: &Fixture, job: Uuid, generation: i64) -> Result<StepCheckpoint> {
    timeout(BOUND, async {
        loop {
            if let Some(checkpoint) =
                get_step_checkpoint(fixture.config.pool(), job, generation, 0).await?
            {
                return Ok(checkpoint);
            }
            let row = get_media_job(fixture.config.pool(), job)
                .await?
                .context("checkpoint job")?;
            anyhow::ensure!(
                row.status_text != "failed" && row.status_text != "completed",
                "{row:?}"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?
}

async fn wait_completed(fixture: &Fixture, job: Uuid) -> Result<()> {
    timeout(BOUND, async {
        loop {
            let row = get_media_job(fixture.config.pool(), job)
                .await?
                .context("completed checkpoint job")?;
            if row.status_text == "completed" {
                anyhow::ensure!(row.last_error.is_none(), "{row:?}");
                return Ok(());
            }
            anyhow::ensure!(
                row.status_text != "failed" && row.status_text != "cancelled",
                "{row:?}"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?
}

#[derive(Clone, Copy)]
enum SourceFault {
    Changed,
    Missing,
}

impl SourceFault {
    const fn name(self) -> &'static str {
        match self {
            Self::Changed => "changed.mkv",
            Self::Missing => "missing.mkv",
        }
    }

    const fn code(self) -> &'static str {
        match self {
            Self::Changed => "media_job_source_fingerprint_mismatch",
            Self::Missing => "media_job_source_fingerprint_unavailable",
        }
    }

    fn verify(self, source: &std::path::Path) -> Result<()> {
        match self {
            Self::Changed => anyhow::ensure!(
                fs::read(source)? == b"operator changed source",
                "changed source was overwritten"
            ),
            Self::Missing => anyhow::ensure!(!source.try_exists()?, "missing source was recreated"),
        }
        Ok(())
    }
}

async fn source_fault_restart(
    fixture: &Fixture,
    association: Uuid,
    fault: SourceFault,
) -> Result<()> {
    let source = fixture.directory.path().join("source").join(fault.name());
    fs::write(&source, &fixture.original)?;
    let first = fixture.start().await?;
    let initial = async {
        fixture.ready().await?;
        let admitted = fixture.request("POST", "/v1/media/discovery/runs", Some(json!({
            "media_discovery_association_public_id": association, "source_paths": [fault.name()]
        })), None).await?;
        anyhow::ensure!(
            admitted["queued_jobs"]
                .as_array()
                .context("fault jobs")?
                .len()
                == 1,
            "{admitted}"
        );
        let job: Uuid = admitted["queued_jobs"][0]["media_job_public_id"]
            .as_str()
            .context("source fault job")?
            .parse()?;
        let child = wait_for_ffmpeg(fixture, job, first.id().context("source fault PID")?).await?;
        let attempt = fixture.attempt(job).await?;
        anyhow::ensure!(
            attempt.0 == 1,
            "unexpected source fault attempt: {attempt:?}"
        );
        anyhow::Ok((job, child, attempt))
    }
    .await;
    let (job, child, attempt) = settled(initial, signal_and_join(first).await)?;
    anyhow::ensure!(
        !PathBuf::from(format!("/proc/{child}")).try_exists()?,
        "old FFmpeg still active"
    );
    let interrupted = get_media_job(fixture.config.pool(), job)
        .await?
        .context("source fault job")?;
    anyhow::ensure!(
        interrupted.status_text == "queued" && interrupted.last_error.is_none(),
        "{interrupted:?}"
    );
    anyhow::ensure!(
        fixture.attempt(job).await? == attempt,
        "interruption consumed an attempt"
    );
    anyhow::ensure!(
        fs::read(&source)? == fixture.original,
        "interruption changed source"
    );
    match fault {
        SourceFault::Changed => fs::write(&source, b"operator changed source")?,
        SourceFault::Missing => fs::remove_file(&source)?,
    }
    let second = fixture.start().await?;
    let replay = async {
        fixture.ready().await?;
        timeout(BOUND, async {
            loop {
                let row = get_media_job(fixture.config.pool(), job)
                    .await?
                    .context("source fault replay")?;
                if row.status_text == "failed" {
                    anyhow::ensure!(row.last_error.as_deref() == Some(fault.code()), "{row:?}");
                    anyhow::ensure!(row.completed_at.is_some(), "{row:?}");
                    break;
                }
                anyhow::ensure!(
                    row.status_text != "completed" && row.status_text != "cancelled",
                    "{row:?}"
                );
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            anyhow::Ok(())
        })
        .await??;
        anyhow::ensure!(
            fixture.attempt(job).await? == attempt,
            "source fault changed attempt identity"
        );
        fault.verify(&source)
    }
    .await;
    settled(replay, signal_and_join(second).await)?;
    fault.verify(&source)?;
    println!(
        "native service: {} rejected after joined interruption; source fault preserved",
        fault.name()
    );
    Ok(())
}

async fn cancel_active(
    fixture: &Fixture,
    association: Uuid,
    server: u32,
) -> Result<(Uuid, (i32, i64))> {
    let source = fixture.directory.path().join("source/cancel.mkv");
    fs::write(&source, &fixture.original)?;
    let admitted = fixture
        .request(
            "POST",
            "/v1/media/discovery/runs",
            Some(json!({
                "media_discovery_association_public_id": association,
                "source_paths": ["cancel.mkv"]
            })),
            None,
        )
        .await?;
    anyhow::ensure!(
        admitted["queued_jobs"]
            .as_array()
            .context("cancel jobs")?
            .len()
            == 1,
        "{admitted}"
    );
    let job: Uuid = admitted["queued_jobs"][0]["media_job_public_id"]
        .as_str()
        .context("cancel job")?
        .parse()?;
    let child = wait_for_ffmpeg(fixture, job, server).await?;
    let attempt = fixture.attempt(job).await?;
    anyhow::ensure!(
        attempt.0 == 1,
        "unexpected cancellation attempt: {attempt:?}"
    );
    let response = fixture
        .api
        .post(format!("{}/v1/media/jobs/{job}/cancel", fixture.origin))
        .header(
            "x-revaer-api-key",
            fixture.api_key.get().context("API key")?,
        )
        .send()
        .await?;
    anyhow::ensure!(
        response.status() == reqwest::StatusCode::NO_CONTENT,
        "cancel: {}",
        response.status()
    );
    timeout(BOUND, async {
        loop {
            let row = get_media_job(fixture.config.pool(), job)
                .await?
                .context("cancelled job")?;
            if row.status_text == "cancelled"
                && !PathBuf::from(format!("/proc/{child}")).try_exists()?
            {
                anyhow::ensure!(row.last_error.is_none(), "{row:?}");
                anyhow::ensure!(row.completed_at.is_some(), "{row:?}");
                break;
            }
            anyhow::ensure!(
                row.status_text != "failed" && row.status_text != "completed",
                "{row:?}"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        anyhow::Ok(())
    })
    .await??;
    anyhow::ensure!(
        fixture.attempt(job).await? == attempt,
        "cancellation consumed an attempt"
    );
    anyhow::ensure!(
        fs::read(source)? == fixture.original,
        "cancelled source changed"
    );
    println!("native service: explicit cancellation joined active FFmpeg and preserved source");
    Ok((job, attempt))
}

async fn verify_cancelled_restart(fixture: &Fixture, cancelled: (Uuid, (i32, i64))) -> Result<()> {
    let (job, attempt) = cancelled;
    let server = fixture.start().await?;
    let verification = async {
        fixture.ready().await?;
        let row = get_media_job(fixture.config.pool(), job)
            .await?
            .context("cancelled restart")?;
        anyhow::ensure!(row.status_text == "cancelled", "{row:?}");
        anyhow::ensure!(
            fixture.attempt(job).await? == attempt,
            "cancellation consumed an attempt"
        );
        anyhow::ensure!(
            fs::read(fixture.directory.path().join("source/cancel.mkv"))? == fixture.original,
            "cancelled source changed after restart"
        );
        anyhow::Ok(())
    }
    .await;
    settled(verification, signal_and_join(server).await)?;
    println!("native service: explicit cancellation remains terminal after restart");
    Ok(())
}

impl Fixture {
    async fn create() -> Result<Self> {
        Self::create_with_duration("180").await
    }

    async fn create_with_duration(duration: &str) -> Result<Self> {
        let root = std::env::var_os("REVAER_NATIVE_RECOVERY_ROOT")
            .context("REVAER_NATIVE_RECOVERY_ROOT must select an owned persistent Linux mount")?;
        let directory = tempfile::Builder::new()
            .prefix(".revaer-service-recovery-")
            .tempdir_in(root)?;
        for name in ["source", "workspace"] {
            let path = directory.path().join(name);
            fs::create_dir(&path)?;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
        write_catalog(&directory)?;
        let source = directory.path().join("source/video.mkv");
        ProcessCommandRunner.run(
            "ffmpeg",
            &[
                "-nostdin",
                "-y",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=640x360:rate=30",
                "-t",
                duration,
                "-c:v",
                "ffv1",
                source.to_str().context("source encoding")?,
            ]
            .map(str::to_owned),
        )?;
        let original = fs::read(&source)?;
        let mut postgres = start_postgres()?;
        postgres
            .initialize_runtime(include_str!("../../../revaer-data/init.sql"))
            .await?;
        let config = ConfigService::new(postgres.connection_string()).await?;
        let reserved = TcpListener::bind(("127.0.0.1", 0))?;
        let address = reserved.local_addr()?;
        let mut profile = config.get_app_profile().await?;
        profile.http_port = i32::from(address.port());
        config
            .apply_changeset(
                "tester",
                "native-service-recovery",
                SettingsChangeset {
                    app_profile: Some(profile),
                    ..SettingsChangeset::default()
                },
            )
            .await?;
        drop(reserved);
        Ok(Self {
            directory,
            postgres,
            config,
            api: reqwest::Client::new(),
            origin: format!("http://{address}"),
            source,
            original,
            api_key: OnceLock::new(),
        })
    }

    async fn start(&self) -> Result<Child> {
        let mut child = tokio::process::Command::new(std::env::current_exe()?)
            .args(["--exact", ENTRY, "--nocapture"])
            .env("REVAER_E2E_SERVING_ENTRY", "1")
            .env("DATABASE_URL", self.postgres.connection_string())
            .env(
                "REVAER_MEDIA_ROOT_CATALOG_FILE",
                self.directory.path().join("catalog.json"),
            )
            .env(
                "REVAER_MEDIA_WORKSPACE_ROOT",
                self.directory.path().join("workspace"),
            )
            .env("RUST_LOG", "info")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;
        timeout(BOUND, async {
            loop {
                if let Some(status) = child.try_wait()? {
                    let mut diagnostics = String::new();
                    child
                        .stdout
                        .take()
                        .context("service stdout")?
                        .read_to_string(&mut diagnostics)
                        .await?;
                    child
                        .stderr
                        .take()
                        .context("service stderr")?
                        .read_to_string(&mut diagnostics)
                        .await?;
                    return Err(anyhow::anyhow!(
                        "service exited during startup: {status}: {diagnostics}"
                    ));
                }
                match self.api.get(format!("{}/health", self.origin)).send().await {
                    Ok(response) => {
                        anyhow::ensure!(response.status().is_success());
                        break;
                    }
                    Err(error) if error.is_connect() => {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            anyhow::Ok(())
        })
        .await??;
        Ok(child)
    }

    async fn ready(&self) -> Result<()> {
        let readiness = self
            .request("GET", "/v1/media/root-catalog/readiness", None, None)
            .await?;
        anyhow::ensure!(readiness["attestation_state"] == "ready", "{readiness}");
        Ok(())
    }

    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
        token: Option<&str>,
    ) -> Result<Value> {
        let mut request = self
            .api
            .request(method.parse()?, format!("{}{path}", self.origin));
        if let Some(body) = body {
            request = request
                .header("Content-Type", "application/json")
                .body(serde_json::to_vec(&body)?);
        }
        if let Some(api_key) = self.api_key.get() {
            request = request.header("x-revaer-api-key", api_key);
        }
        if let Some(token) = token {
            request = request.header("x-revaer-setup-token", token);
        }
        if path == "/v1/media/profiles"
            || path == "/v1/media/discovery-associations"
            || (method == "POST" && path.ends_with("/schedule"))
        {
            request = request.header("If-None-Match", "*");
        }
        let response = request.send().await?;
        let status = response.status();
        let body: Value = serde_json::from_slice(&response.bytes().await?)?;
        anyhow::ensure!(status.is_success(), "{method} {path}: {status}: {body}");
        Ok(body)
    }

    async fn activate(&self) -> Result<()> {
        let started = self
            .request("POST", "/admin/setup/start", Some(json!({})), None)
            .await?;
        let token = started["token"].as_str().context("setup token")?;
        let snapshot = self
            .request("GET", "/.well-known/revaer.json", None, None)
            .await?;
        let mut profile = snapshot["app_profile"].clone();
        profile["auth_mode"] = json!("api_key");
        let mut policy = snapshot["fs_policy"].clone();
        policy["allow_paths"] = json!([self.directory.path()]);
        let completed = self
            .request(
                "POST",
                "/admin/setup/complete",
                Some(json!({
                    "app_profile": profile, "fs_policy": policy
                })),
                Some(token),
            )
            .await?;
        let key = completed["api_key"].as_str().context("bootstrap API key")?;
        self.api_key
            .set(key.to_owned())
            .map_err(|_| anyhow::anyhow!("fixture already activated"))?;
        let denied = self
            .api
            .get(format!("{}/v1/media/profiles", self.origin))
            .send()
            .await?;
        anyhow::ensure!(
            denied.status() == reqwest::StatusCode::UNAUTHORIZED,
            "anonymous configuration was accepted"
        );
        Ok(())
    }

    async fn configure_association(
        &self,
        streams: Vec<Value>,
        dry_run: bool,
    ) -> Result<(Uuid, Uuid)> {
        self.request(
            "POST",
            "/v1/media/targets",
            Some(json!({
                "target_key":"service-target", "version":1, "display_name":"Service recovery",
                "container_format":"matroska", "streams": streams
            })),
            None,
        )
        .await?;
        self.request("POST", "/v1/media/policies", Some(json!({
            "policy_key":"service-policy", "version":1, "display_name":"Service recovery",
            "video_intent":"archival", "verification_strictness":"strict",
            "verification_duration_tolerance_millis":100,
            "verification_mux_validation":true, "verification_decode_all_streams":true,
            "verification_keyframe_seek":true, "verification_playback_probe":true,
            "output":{"dry_run":false, "replacement_mode":"atomic_replace",
                "quarantine_enabled":false, "preserve_permissions":true, "preserve_ownership":true}
        })), None).await?;
        let profile = self.request("POST", "/v1/media/profiles", Some(json!({
            "profile_key":"service-profile", "display_name":"Service recovery", "description":"",
            "enabled":true, "dry_run_only":dry_run, "desired_target_key":"service-target",
            "desired_target_version":1, "policy_key":"service-policy", "policy_version":1,
            "output_root_key":"service-source", "workspace_root_key":"service-workspace"
        })), None).await?;
        let association = self.request("POST", "/v1/media/discovery-associations", Some(json!({
            "association_key":"service-discovery", "media_profile_public_id":profile["media_profile_public_id"],
            "profile_version":1, "source_root_key":"service-source", "root_relative_path":"",
            "manual_enabled":true, "watcher_enabled":false, "schedule_enabled":false
        })), None).await?;
        Ok((
            profile["media_profile_public_id"]
                .as_str()
                .context("profile id")?
                .parse()?,
            association["media_discovery_association_public_id"]
                .as_str()
                .context("association id")?
                .parse()?,
        ))
    }

    async fn admit(&self, streams: Vec<Value>) -> Result<(Uuid, Uuid)> {
        let (_, association) = self.configure_association(streams, false).await?;
        let admitted = self
            .request(
                "POST",
                "/v1/media/discovery/runs",
                Some(json!({
                    "media_discovery_association_public_id":association,
                    "source_paths":["video.mkv"]
                })),
                None,
            )
            .await?;
        anyhow::ensure!(
            admitted["queued_jobs"]
                .as_array()
                .context("queued jobs")?
                .len()
                == 1,
            "{admitted}"
        );
        anyhow::ensure!(admitted["queued_jobs"][0]["dry_run"] == false, "{admitted}");
        Ok((
            admitted["queued_jobs"][0]["media_job_public_id"]
                .as_str()
                .context("job id")?
                .parse()?,
            association,
        ))
    }

    async fn attempt(&self, job: Uuid) -> Result<(i32, i64)> {
        let rows = list_media_job_attempt_identities(self.config.pool(), job).await?;
        anyhow::ensure!(rows.len() == 1, "unexpected attempt history: {rows:?}");
        let row = rows.into_iter().next().context("job attempt")?;
        anyhow::ensure!(row.is_current, "attempt is not current");
        Ok((row.attempt_number, row.claim_generation))
    }
}

fn write_catalog(directory: &tempfile::TempDir) -> Result<()> {
    let slots: Vec<_> = [("source", vec!["source", "output"]),
        ("workspace", vec!["workspace"])]
        .into_iter().map(|(name, kinds)| json!({
            "key":format!("service-{name}"), "path":directory.path().join(name), "allowed_kinds":kinds,
            "durability_class":"restart_persistent", "durability_evidence":"linux_dedicated_mount",
            "sole_writer_class":"revaer_exclusive", "sole_writer_evidence":"linux_dedicated_service"
        })).collect();
    fs::write(
        directory.path().join("catalog.json"),
        serde_json::to_vec(&json!({"format_version":1,"slots":slots}))?,
    )?;
    Ok(())
}

async fn wait_for_ffmpeg(fixture: &Fixture, job: Uuid, server: u32) -> Result<u32> {
    timeout(BOUND, async {
        loop {
            let row = get_media_job(fixture.config.pool(), job)
                .await?
                .context("waiting job")?;
            anyhow::ensure!(
                row.status_text != "failed" && row.status_text != "cancelled",
                "{row:?}"
            );
            for task in fs::read_dir(format!("/proc/{server}/task"))? {
                let task = task?;
                let children = match fs::read_to_string(task.path().join("children")) {
                    Ok(children) => children,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(error.into()),
                };
                for child in children.split_whitespace() {
                    let command = match fs::read(format!("/proc/{child}/cmdline")) {
                        Ok(command) => command,
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                        Err(error) => return Err(error.into()),
                    };
                    if command.split(|byte| *byte == 0).next() == Some(b"ffmpeg") {
                        return Ok(child.parse()?);
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await?
}

fn settled<T>(result: Result<T>, shutdown: Result<()>) -> Result<T> {
    match (result, shutdown) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), Ok(())) | (Ok(_), Err(error)) => Err(error),
        (Err(error), Err(shutdown)) => {
            Err(error.context(format!("service shutdown also failed: {shutdown}")))
        }
    }
}

async fn signal_and_join(child: Child) -> Result<()> {
    let pid = child.id().context("owned service PID")?;
    let pid = rustix::process::Pid::from_raw(i32::try_from(pid)?).context("service PID")?;
    rustix::process::kill_process(pid, rustix::process::Signal::TERM)?;
    let output = timeout(BOUND, child.wait_with_output()).await??;
    let diagnostics = format!(
        "{}{}",
        String::from_utf8(output.stdout)?,
        String::from_utf8(output.stderr)?
    );
    println!("{diagnostics}");
    anyhow::ensure!(output.status.success(), "{diagnostics}");
    assert!(
        diagnostics.contains("API server shutdown complete"),
        "{diagnostics}"
    );
    Ok(())
}
