//! Same-attempt replay using real filesystem writers and sealed runtime persistence.

use std::os::unix::fs::MetadataExt;

use revaer_media_runtime::workspace::create_managed_workspace;

use super::{ExecutionStep, RuntimeJobTarget, fs, setup_runtime};

pub(super) enum ReplayFault {
    ObsoleteMissing,
    RequiredMissing,
    RequiredCorrupt,
}

pub(super) async fn replay(fault: ReplayFault) -> anyhow::Result<()> {
    let fixture = setup_runtime(false, true, RuntimeJobTarget::SourceGraph).await?;
    let result = async {
        let job = fixture
            .store
            .claim_next_job()
            .await?
            .ok_or_else(|| anyhow::anyhow!("claim missing"))?;
        let workspace =
            create_managed_workspace(&fixture.runtime.workspace_root, "checkpoint-fixture")?;
        let intermediate = workspace.output_path.join("intermediate.mkv");
        let candidate = workspace.output_path.join("candidate.mkv");
        let steps = vec![
            ExecutionStep::CopySidecarSubtitle {
                source_path: job.source_path.clone(),
                output_path: intermediate.to_string_lossy().into_owned(),
            },
            ExecutionStep::CopySidecarSubtitle {
                source_path: intermediate.to_string_lossy().into_owned(),
                output_path: candidate.to_string_lossy().into_owned(),
            },
            ExecutionStep::VerifyOutput {
                output_path: candidate.to_string_lossy().into_owned(),
            },
        ];
        let fingerprint = crate::media_discovery_fingerprint::fingerprint_media_aggregate(
            std::path::Path::new(&job.source_path),
            std::path::Path::new(&job.source_root),
        )?
        .ok_or_else(|| anyhow::anyhow!("source unstable"))?;
        fixture
            .runtime
            .execute_steps(&job, steps.clone(), &workspace, &fingerprint, None)
            .await?;
        let old_candidate = fs::metadata(&candidate)?.ino();
        let first_checkpoint = fixture
            .store
            .get_step_checkpoint(job.media_job_public_id, job.claim_generation, 0)
            .await?
            .ok_or_else(|| anyhow::anyhow!("first checkpoint missing"))?;
        assert_eq!(first_checkpoint.output_sha256.len(), 32);
        assert_eq!(fs::read(&candidate)?, b"source");
        assert!(
            !intermediate.exists(),
            "consumed intermediates are removed after their final writer"
        );
        assert!(
            !fixture
                .store
                .interrupt_job(job.media_job_public_id, job.claim_generation)
                .await?
        );
        let resumed = fixture
            .store
            .claim_next_job()
            .await?
            .ok_or_else(|| anyhow::anyhow!("resume missing"))?;
        assert_eq!(resumed.claim_generation, job.claim_generation);
        match fault {
            ReplayFault::ObsoleteMissing => {}
            ReplayFault::RequiredMissing => {
                fs::remove_file(&candidate)?;
            }
            ReplayFault::RequiredCorrupt => {
                fs::remove_file(&candidate)?;
                fs::write(&intermediate, b"broken")?;
            }
        }
        fixture
            .runtime
            .execute_steps(&resumed, steps, &workspace, &fingerprint, None)
            .await?;
        assert_eq!(fs::read(&candidate)?, b"source");
        if matches!(
            fault,
            ReplayFault::RequiredMissing | ReplayFault::RequiredCorrupt
        ) {
            assert!(
                !intermediate.exists(),
                "rebuilt intermediates are removed after their final writer"
            );
        } else {
            assert!(
                !intermediate.exists(),
                "obsolete predecessor must not rerun"
            );
            assert_eq!(
                fs::metadata(&candidate)?.ino(),
                old_candidate,
                "completed output was replaced"
            );
        }
        assert_eq!(fs::read(&job.source_path)?, b"source");
        Ok(())
    }
    .await;
    fixture.store.pool().close().await;
    fixture.postgres.close()?;
    result
}

pub(super) async fn source_changed(missing: bool) -> anyhow::Result<()> {
    let fixture = setup_runtime(false, true, RuntimeJobTarget::SourceGraph).await?;
    let result = async {
        let job = fixture
            .store
            .claim_next_job()
            .await?
            .ok_or_else(|| anyhow::anyhow!("claim missing"))?;
        let workspace =
            create_managed_workspace(&fixture.runtime.workspace_root, "checkpoint-source-fixture")?;
        let output = workspace.output_path.join("candidate.mkv");
        let steps = vec![ExecutionStep::CopySidecarSubtitle {
            source_path: job.source_path.clone(),
            output_path: output.to_string_lossy().into_owned(),
        }];
        let fingerprint = crate::media_discovery_fingerprint::fingerprint_media_aggregate(
            std::path::Path::new(&job.source_path),
            std::path::Path::new(&job.source_root),
        )?
        .ok_or_else(|| anyhow::anyhow!("source unstable"))?;
        fixture
            .runtime
            .execute_steps(&job, steps.clone(), &workspace, &fingerprint, None)
            .await?;
        assert!(
            !fixture
                .store
                .interrupt_job(job.media_job_public_id, job.claim_generation)
                .await?
        );
        let resumed = fixture
            .store
            .claim_next_job()
            .await?
            .ok_or_else(|| anyhow::anyhow!("resume missing"))?;
        if missing {
            fs::remove_file(&job.source_path)?;
        } else {
            fs::write(&job.source_path, b"changed")?;
        }
        let error = fixture
            .runtime
            .execute_steps(&resumed, steps, &workspace, &fingerprint, None)
            .await;
        assert!(error.is_err(), "changed source was allowed to replay");
        assert_eq!(fs::read(&output)?, b"source");
        fixture.runtime.process_job(resumed, None).await;
        let recorded = fixture
            .store
            .get_job(job.media_job_public_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("failed source job missing"))?;
        assert_eq!(recorded.status_text, "failed");
        assert!(recorded.last_error.is_some());
        if !missing {
            assert_eq!(fs::read(&job.source_path)?, b"changed");
        }
        Ok(())
    }
    .await;
    fixture.store.pool().close().await;
    fixture.postgres.close()?;
    result
}

pub(super) async fn real_process_replay() -> anyhow::Result<()> {
    let fixture = real_fixture().await?;
    let result = real_process_replay_with_fixture(&fixture).await;
    fixture.store.pool().close().await;
    fixture.postgres.close()?;
    result
}

async fn real_process_replay_with_fixture(fixture: &super::RuntimeFixture) -> anyhow::Result<()> {
    use super::runtime_shutdown;
    let job = fixture
        .store
        .claim_next_job()
        .await?
        .ok_or_else(|| anyhow::anyhow!("claim missing"))?;
    let workspace =
        create_managed_workspace(&fixture.runtime.workspace_root, "checkpoint-real-ffmpeg")?;
    let intermediate = workspace.output_path.join("intermediate.mkv");
    let candidate = workspace.output_path.join("candidate.mkv");
    let original_source = fs::read(&job.source_path)?;
    let fingerprint = crate::media_discovery_fingerprint::fingerprint_media_aggregate(
        std::path::Path::new(&job.source_path),
        std::path::Path::new(&job.source_root),
    )?
    .ok_or_else(|| anyhow::anyhow!("source unstable"))?;

    let steps = vec![
        ffmpeg_step(&job.source_path, &intermediate, false),
        ffmpeg_step(&intermediate.to_string_lossy(), &candidate, true),
        ExecutionStep::VerifyOutput {
            output_path: candidate.to_string_lossy().into_owned(),
        },
    ];
    let (shutdown_tx, shutdown_rx) = runtime_shutdown::channel();
    let interrupt = async {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !candidate.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await?;
        assert!(runtime_shutdown::request(&shutdown_tx));
        Ok::<(), anyhow::Error>(())
    };
    let (execution, interrupted) = tokio::join!(
        fixture.runtime.execute_steps(
            &job,
            steps.clone(),
            &workspace,
            &fingerprint,
            Some(&shutdown_rx)
        ),
        interrupt
    );
    interrupted?;
    assert!(matches!(
        execution,
        Err(super::super::MediaJobRuntimeError::Interrupted)
    ));
    assert!(
        fixture
            .store
            .get_step_checkpoint(job.media_job_public_id, job.claim_generation, 0)
            .await?
            .is_some()
    );
    assert!(
        fixture
            .store
            .get_step_checkpoint(job.media_job_public_id, job.claim_generation, 1)
            .await?
            .is_none()
    );
    assert!(
        !fixture
            .store
            .interrupt_job(job.media_job_public_id, job.claim_generation)
            .await?
    );
    let resumed = fixture
        .store
        .claim_next_job()
        .await?
        .ok_or_else(|| anyhow::anyhow!("resume missing"))?;
    fixture
        .runtime
        .execute_steps(&resumed, steps, &workspace, &fingerprint, None)
        .await?;
    assert!(!intermediate.exists(), "consumed intermediate was retained");
    assert!(
        fixture
            .store
            .get_step_checkpoint(job.media_job_public_id, job.claim_generation, 1)
            .await?
            .is_some()
    );
    assert!(
        !intermediate.exists(),
        "resumed consumer removes its completed input"
    );
    let probe = tokio::process::Command::new("ffprobe")
        .args(["-v", "error", "-show_format"])
        .arg(candidate)
        .output()
        .await?;
    assert!(probe.status.success(), "restarted output is not probeable");
    assert_eq!(fs::read(&job.source_path)?, original_source);
    Ok(())
}

fn ffmpeg_step(input: &str, output: &std::path::Path, slow: bool) -> ExecutionStep {
    let mut argv = ["-nostdin", "-y", "-loglevel", "error"]
        .map(str::to_owned)
        .to_vec();
    if slow {
        argv.extend(["-re", "-stream_loop", "-1"].map(str::to_owned));
    }
    argv.extend(["-i".to_owned(), input.to_owned()]);
    if slow {
        argv.extend(["-t", "2"].map(str::to_owned));
    }
    argv.extend(["-c", "copy"].map(str::to_owned));
    argv.push(output.to_string_lossy().into_owned());
    ExecutionStep::Command {
        bin: "ffmpeg".to_owned(),
        argv,
    }
}

async fn real_fixture() -> anyhow::Result<super::RuntimeFixture> {
    use super::RuntimeCommandRunner;
    use revaer_media_runtime::execute::{CommandRunner, ProcessCommandRunner};
    let source_dir = tempfile::tempdir()?;
    let source = source_dir.path().join("source.mkv");
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
            "testsrc2=size=64x64:rate=10",
            "-t",
            "0.5",
            "-c:v",
            "ffv1",
            source
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("source UTF-8"))?,
        ]
        .map(str::to_owned),
    )?;
    let mut fixture = super::setup_runtime_with_source(
        false,
        true,
        RuntimeJobTarget::SourceGraph,
        true,
        &fs::read(source)?,
    )
    .await?;
    fixture.runtime.command_runner =
        std::sync::Arc::new(ProcessCommandRunner) as std::sync::Arc<RuntimeCommandRunner>;
    Ok(fixture)
}

pub(super) async fn real_process_cancel() -> anyhow::Result<()> {
    let fixture = real_fixture().await?;
    let result = real_process_cancel_with_fixture(&fixture).await;
    fixture.store.pool().close().await;
    fixture.postgres.close()?;
    result
}

async fn real_process_cancel_with_fixture(fixture: &super::RuntimeFixture) -> anyhow::Result<()> {
    use revaer_media_runtime::workspace::{
        TerminalWorkspaceCleanupPolicy, TerminalWorkspaceState, cleanup_terminal_workspace,
    };
    let job = fixture
        .store
        .claim_next_job()
        .await?
        .ok_or_else(|| anyhow::anyhow!("claim missing"))?;
    let key = crate::media_workspace_identity::workspace_key(
        job.media_job_public_id,
        job.attempt_number,
        job.claim_generation,
    );
    let workspace = create_managed_workspace(&fixture.runtime.workspace_root, &key)?;
    let candidate = workspace.output_path.join("cancelled.mkv");
    let original = fs::read(&job.source_path)?;
    let fingerprint = crate::media_discovery_fingerprint::fingerprint_media_aggregate(
        std::path::Path::new(&job.source_path),
        std::path::Path::new(&job.source_root),
    )?
    .ok_or_else(|| anyhow::anyhow!("source unstable"))?;
    let steps = vec![ffmpeg_step(&job.source_path, &candidate, true)];
    let cancel = async {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !candidate.exists() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await?;
        fixture.store.cancel_job(job.media_job_public_id).await?;
        Ok::<(), anyhow::Error>(())
    };
    let (execution, cancellation) = tokio::join!(
        fixture
            .runtime
            .execute_steps(&job, steps, &workspace, &fingerprint, None),
        cancel
    );
    cancellation?;
    assert!(matches!(
        execution,
        Err(super::super::MediaJobRuntimeError::Cancelled)
    ));
    // The returned executor result means its actual child has stopped and joined.
    fixture.runtime.persist_cancellation(&job).await?;
    cleanup_terminal_workspace(
        &workspace,
        TerminalWorkspaceState::Cancelled,
        TerminalWorkspaceCleanupPolicy {
            retain_diagnostics: false,
        },
    )?;
    let terminal = fixture
        .store
        .get_job(job.media_job_public_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("cancelled job missing"))?;
    assert_eq!(terminal.status_text, "cancelled");
    assert_eq!(terminal.last_error, None);
    assert!(
        fixture.store.claim_next_job().await?.is_none(),
        "explicit cancellation replayed"
    );
    assert!(!candidate.exists());
    assert_eq!(fs::read(&job.source_path)?, original);
    Ok(())
}
