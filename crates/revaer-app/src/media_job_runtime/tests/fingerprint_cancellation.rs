//! Cancellation classification and joined reads through the existing job monitor.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::super::SourceFingerprintProbe;
use super::{RuntimeJobTarget, assert_runtime_interrupted, fs, setup_runtime, wait_for_flag};
use crate::media_discovery_fingerprint::{
    FingerprintError, MediaAggregateFingerprint, fingerprint_media_aggregate_cancellable,
};
use crate::runtime_shutdown;

#[derive(Default)]
struct WaitingProbe {
    entered: AtomicBool,
    stopped: AtomicBool,
}

impl SourceFingerprintProbe for WaitingProbe {
    fn fingerprint(
        &self,
        media_path: &Path,
        source_root: &Path,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<MediaAggregateFingerprint>, FingerprintError> {
        let started = Instant::now();
        let result = fingerprint_media_aggregate_cancellable(media_path, source_root, &|| {
            self.entered.store(true, Ordering::Release);
            while !cancelled() && started.elapsed() < Duration::from_secs(5) {
                std::thread::sleep(Duration::from_millis(1));
            }
            cancelled()
        });
        self.stopped.store(true, Ordering::Release);
        result
    }
}

pub(super) async fn stop(explicit: bool) -> anyhow::Result<()> {
    let mut fixture = setup_runtime(false, true, RuntimeJobTarget::SourceGraph).await?;
    let probe = Arc::new(WaitingProbe::default());
    fixture.runtime.source_fingerprint_probe = probe.clone();
    let claimed = fixture
        .store
        .claim_next_job()
        .await?
        .ok_or_else(|| anyhow::anyhow!("job was not claimed"))?;
    let source = PathBuf::from(&claimed.source_path);
    let (shutdown_tx, shutdown_rx) = runtime_shutdown::channel();
    let runtime = fixture.runtime;
    let task = tokio::spawn(async move {
        runtime
            .process_claimed_job_with_shutdown(claimed, shutdown_rx)
            .await
    });
    wait_for_flag(&probe.entered, "source fingerprint read").await?;
    assert!(!probe.stopped.load(Ordering::Acquire));
    if explicit {
        fixture.store.cancel_job(fixture.job_id).await?;
    } else {
        assert!(runtime_shutdown::request(&shutdown_tx));
    }
    tokio::time::timeout(Duration::from_secs(5), task).await???;
    assert!(probe.stopped.load(Ordering::Acquire));
    assert_eq!(fs::read(&source)?, b"source");
    if explicit {
        let job = fixture
            .store
            .get_job(fixture.job_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("cancelled job missing"))?;
        assert_eq!(job.status_text, "cancelled");
        assert_eq!(job.last_error, None);
        assert!(fixture.store.claim_next_job().await?.is_none());
    } else {
        assert_runtime_interrupted(&fixture.store, fixture.job_id, &source).await?;
    }
    fixture.store.pool().close().await;
    fixture.postgres.close()
}

pub(super) async fn stop_recovery() -> anyhow::Result<()> {
    use super::super::{MediaJobRuntimeError, SystemSourceFingerprintProbe, replacement_job_key};
    use revaer_media_runtime::replacement::{
        ReplacementCommitter, ReplacementRequest, SystemReplacementCommitter,
    };

    let mut fixture = setup_runtime(false, true, RuntimeJobTarget::SourceGraph).await?;
    let job = fixture
        .store
        .claim_next_job()
        .await?
        .ok_or_else(|| anyhow::anyhow!("job was not claimed"))?;
    let source = PathBuf::from(&job.source_path);
    let candidate = fixture.temp.path().join("rollback-candidate.mkv");
    fs::write(&candidate, b"replacement")?;
    let key = replacement_job_key(job.media_job_public_id, job.claim_generation);
    let prepared = SystemReplacementCommitter.prepare(ReplacementRequest {
        job_key: &key,
        source_root: Path::new(&job.source_root),
        source_path: &source,
        candidate_path: &candidate,
    })?;
    let backup = prepared.recovery_path().to_path_buf();
    drop(SystemReplacementCommitter.commit(prepared)?);
    assert_eq!(fs::read(&source)?, b"replacement");
    let probe = Arc::new(WaitingProbe::default());
    fixture.runtime.source_fingerprint_probe = probe.clone();
    let (shutdown_tx, shutdown_rx) = runtime_shutdown::channel();
    let runtime = fixture.runtime;
    let task = tokio::spawn(async move {
        let result = runtime
            .recover_interrupted_replacements(Some(&shutdown_rx))
            .await;
        (runtime, result)
    });
    wait_for_flag(&probe.entered, "restored source fingerprint read").await?;
    assert!(runtime_shutdown::request(&shutdown_tx));
    let (mut runtime, result) = tokio::time::timeout(Duration::from_secs(5), task).await??;
    assert!(matches!(result, Err(MediaJobRuntimeError::Interrupted)));
    assert!(probe.stopped.load(Ordering::Acquire));
    assert_eq!(fs::read(&source)?, b"source");
    assert_eq!(fs::read(&backup)?, b"source");
    assert!(
        backup
            .parent()
            .ok_or_else(|| anyhow::anyhow!("backup parent missing"))?
            .join("replacement.json")
            .try_exists()?
    );
    runtime.source_fingerprint_probe = Arc::new(SystemSourceFingerprintProbe);
    runtime.recover_interrupted_replacements(None).await?;
    runtime.resume_interrupted_jobs().await?;
    let resumed = assert_runtime_interrupted(&fixture.store, fixture.job_id, &source).await?;
    assert_eq!(resumed.attempt_number, job.attempt_number);
    assert_eq!(resumed.claim_generation, job.claim_generation);
    assert!(!backup.try_exists()?);
    fixture.store.pool().close().await;
    fixture.postgres.close()
}
