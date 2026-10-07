//! Authenticated configuration, dry-run and scratch deferral on the real service.

use super::{
    BOUND, Fixture, Result, Uuid, fs, get_media_job, json, settled, signal_and_join, timeout,
    video_target, wait_completed,
};
use anyhow::Context;
use std::time::Duration;

pub(super) async fn qualify(fixture: &Fixture) -> Result<()> {
    let server = fixture.start().await?;
    let initial = async {
        fixture.activate().await?;
        fixture.ready().await?;
        let (profile, association) = fixture
            .configure_association(vec![video_target()], true)
            .await?;
        let path = format!("/v1/media/profiles/{profile}");
        let saved = fixture.request("GET", &path, None, None).await?;
        let denied = fixture
            .api
            .get(format!("{}{path}", fixture.origin))
            .header("x-revaer-api-key", "invalid:invalid")
            .send()
            .await?;
        anyhow::ensure!(denied.status() == reqwest::StatusCode::UNAUTHORIZED);
        let job = discover(fixture, association).await?;
        wait_completed(fixture, job).await?;
        let row = get_media_job(fixture.config.pool(), job)
            .await?
            .context("dry-run job")?;
        anyhow::ensure!(row.dry_run, "saved dry-run profile was overridden");
        anyhow::ensure!(fs::read(&fixture.source)? == fixture.original);
        anyhow::ensure!(
            fs::read_dir(fixture.directory.path().join("source"))?
                .collect::<std::io::Result<Vec<_>>>()?
                .len()
                == 1
        );
        anyhow::ensure!(
            fs::read_dir(fixture.directory.path().join("workspace"))?
                .collect::<std::io::Result<Vec<_>>>()?
                .is_empty()
        );
        Ok((path, saved))
    }
    .await;
    let (path, saved) = settled(initial, signal_and_join(server).await)?;
    let server = fixture.start().await?;
    let reopened = async {
        fixture.ready().await?;
        anyhow::ensure!(fixture.request("GET", &path, None, None).await? == saved);
        anyhow::ensure!(fs::read(&fixture.source)? == fixture.original);
        anyhow::Ok(())
    }
    .await;
    settled(reopened, signal_and_join(server).await)?;
    println!(
        "native operator: authenticated configuration persisted across restart; dry-run completed without media or adjacent artifacts"
    );
    Ok(())
}

async fn discover(fixture: &Fixture, association: Uuid) -> Result<Uuid> {
    let admitted = fixture
        .request(
            "POST",
            "/v1/media/discovery/runs",
            Some(json!({
                "media_discovery_association_public_id":association, "source_paths":["video.mkv"]
            })),
            None,
        )
        .await?;
    let jobs = admitted["queued_jobs"].as_array().context("queued jobs")?;
    anyhow::ensure!(jobs.len() == 1, "expected one admitted job");
    jobs[0]["media_job_public_id"]
        .as_str()
        .context("job id")?
        .parse()
        .map_err(Into::into)
}

pub(super) async fn scratch(fixture: &Fixture) -> Result<()> {
    // Sparse fixture consumes the service's logical-byte budget without allocating
    // 20 GiB of test media. It is not a disk-free-space or throughput measurement.
    let blocker = fixture
        .directory
        .path()
        .join("workspace/operator-owned-blocker");
    fs::File::create(&blocker)?.set_len(20 * 1024 * 1024 * 1024 + 1)?;
    let server = fixture.start().await?;
    let initial = async {
        fixture.activate().await?;
        fixture.ready().await?;
        let (job, _) = fixture.admit(vec![video_target()]).await?;
        timeout(BOUND, async {
            loop {
                let response = fixture
                    .api
                    .get(format!("{}/metrics", fixture.origin))
                    .header(
                        "x-revaer-api-key",
                        fixture.api_key.get().context("API key")?,
                    )
                    .send()
                    .await?;
                anyhow::ensure!(response.status().is_success());
                let metrics = response.text().await?;
                if metrics.lines().any(|line| {
                    line.starts_with("media_job_outcomes_total{")
                        && line.contains("outcome=\"deferred\"")
                        && !line.ends_with(" 0")
                }) {
                    break;
                }
                let row = get_media_job(fixture.config.pool(), job)
                    .await?
                    .context("scratch job")?;
                anyhow::ensure!(
                    !matches!(
                        row.status_text.as_str(),
                        "completed" | "failed" | "cancelled"
                    ),
                    "{row:?}"
                );
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            anyhow::Ok(())
        })
        .await??;
        anyhow::ensure!(fs::read(&fixture.source)? == fixture.original);
        anyhow::ensure!(
            blocker.try_exists()?,
            "service removed caller-owned scratch"
        );
        fixture.attempt(job).await.map(|attempt| (job, attempt))
    }
    .await;
    let (job, attempt) = settled(initial, signal_and_join(server).await)?;
    anyhow::ensure!(attempt.0 == 1, "deferral consumed failure retry");
    let row = get_media_job(fixture.config.pool(), job)
        .await?
        .context("deferred job")?;
    anyhow::ensure!(
        row.status_text == "queued" && row.last_error.is_none(),
        "{row:?}"
    );
    fs::remove_file(blocker)?;
    let server = fixture.start().await?;
    let replay = async {
        fixture.ready().await?;
        wait_completed(fixture, job).await?;
        anyhow::ensure!(fixture.attempt(job).await? == attempt);
        anyhow::ensure!(
            fs::read(&fixture.source)? != fixture.original,
            "verified replacement absent"
        );
        anyhow::Ok(())
    }
    .await;
    settled(replay, signal_and_join(server).await)?;
    anyhow::ensure!(
        fs::read_dir(fixture.directory.path().join("workspace"))?
            .collect::<std::io::Result<Vec<_>>>()?
            .is_empty(),
        "completed scratch retained"
    );
    println!(
        "native operator: scratch exhaustion deferred without replacing source; capacity recovery completed the same attempt and reclaimed temporary media"
    );
    Ok(())
}
