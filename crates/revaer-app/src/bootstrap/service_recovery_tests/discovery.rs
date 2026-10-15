//! Real serving-process discovery interruption and trigger qualification.

use std::collections::BTreeSet;

use revaer_data::media::jobs::list_recent_media_jobs;
use revaer_data::media::rescan::read_rescan_state;

use super::{
    BOUND, Fixture, Result, Uuid, fs, json, settled, signal_and_join, timeout, video_target,
};

const INITIAL_FILES: usize = 131;

#[derive(Clone, Copy, Debug)]
pub(super) enum Mode {
    Watcher,
    Schedule,
}

impl Mode {
    async fn enable(self, fixture: &Fixture, association: Uuid) -> Result<()> {
        let script = match self {
            Self::Watcher => {
                include_str!("../../../../../scripts/tests/media-native-watcher-mode.sql")
            }
            Self::Schedule => {
                fixture.request("POST", &format!("/v1/media/discovery-associations/{association}/schedule"),
                    Some(json!({"association_version":1,"interval_quantity":1,"interval_unit":"minutes"})), None).await?;
                include_str!("../../../../../scripts/tests/media-native-schedule-mode.sql")
            }
        };
        fixture
            .postgres
            .apply_fixture_script(
                script,
                &[("revaer_test.association", &association.to_string())],
            )
            .await?;
        Ok(())
    }

    async fn trigger_later(self, fixture: &Fixture, association: Uuid) -> Result<()> {
        if matches!(self, Self::Schedule) {
            fixture
                .postgres
                .apply_fixture_script(
                    include_str!("../../../../../scripts/tests/media-native-schedule-due.sql"),
                    &[("revaer_test.association", &association.to_string())],
                )
                .await?;
        }
        Ok(())
    }
}

pub(super) async fn qualify(fixture: &Fixture, mode: Mode) -> Result<()> {
    for index in 1..INITIAL_FILES {
        fs::write(
            fixture
                .source
                .with_file_name(format!("video-{index:03}.mkv")),
            &fixture.original,
        )?;
    }
    let server = fixture.start().await?;
    let initial = async {
        fixture.activate().await?;
        fixture.ready().await?;
        let (profile, association) = fixture
            .configure_association(vec![video_target()], true)
            .await?;
        mode.enable(fixture, association).await?;
        let partial = wait_partial(fixture, profile, association).await?;
        Ok((profile, association, partial))
    }
    .await;
    let (profile, association, partial) = settled(initial, signal_and_join(server).await)?;
    let pending = read_rescan_state(fixture.config.pool(), association).await?;
    anyhow::ensure!(
        pending
            .iter()
            .any(|row| row.requested_sequence > row.satisfied_sequence),
        "unfinished discovery was acknowledged on shutdown"
    );
    verify_sources(fixture, false)?;

    let server = fixture.start().await?;
    let resumed = async {
        fixture.ready().await?;
        let identities = wait_clean(fixture, profile, association, INITIAL_FILES).await?;
        anyhow::ensure!(
            partial.is_subset(&identities),
            "restart lost admitted identities"
        );
        fs::write(
            fixture.source.with_file_name("created-after-restart.mkv"),
            &fixture.original,
        )?;
        mode.trigger_later(fixture, association).await?;
        let identities = wait_clean(fixture, profile, association, INITIAL_FILES + 1).await?;
        verify_sources(fixture, true)?;
        Ok(identities)
    }
    .await;
    let identities = settled(resumed, signal_and_join(server).await)?;

    let server = fixture.start().await?;
    let repeated = async {
        fixture.ready().await?;
        let replayed = wait_clean(fixture, profile, association, INITIAL_FILES + 1).await?;
        anyhow::ensure!(replayed == identities, "clean restart duplicated jobs");
        verify_sources(fixture, true)?;
        anyhow::Ok(())
    }
    .await;
    settled(repeated, signal_and_join(server).await)?;
    println!(
        "native service discovery {mode:?}: unfinished scan resumed, later trigger admitted, identities deduplicated and originals unchanged"
    );
    Ok(())
}

async fn wait_partial(
    fixture: &Fixture,
    profile: Uuid,
    association: Uuid,
) -> Result<BTreeSet<Uuid>> {
    timeout(BOUND, async {
        loop {
            let jobs = read_job_ids(fixture, profile).await?;
            anyhow::ensure!(
                jobs.len() < INITIAL_FILES,
                "scan completed before interruption observation"
            );
            let requests = read_rescan_state(fixture.config.pool(), association).await?;
            if !jobs.is_empty()
                && requests
                    .iter()
                    .any(|row| row.requested_sequence > row.satisfied_sequence)
            {
                return Ok(jobs);
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await?
}

async fn wait_clean(
    fixture: &Fixture,
    profile: Uuid,
    association: Uuid,
    expected: usize,
) -> Result<BTreeSet<Uuid>> {
    timeout(BOUND, async {
        loop {
            let jobs = read_job_ids(fixture, profile).await?;
            anyhow::ensure!(
                jobs.len() <= expected,
                "duplicate discovery jobs: {} > {expected}",
                jobs.len()
            );
            let requests = read_rescan_state(fixture.config.pool(), association).await?;
            if jobs.len() == expected
                && !requests.is_empty()
                && requests
                    .iter()
                    .all(|row| row.requested_sequence == row.satisfied_sequence)
            {
                return Ok(jobs);
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await?
}

fn verify_sources(fixture: &Fixture, later: bool) -> Result<()> {
    anyhow::ensure!(
        fs::read(&fixture.source)? == fixture.original,
        "original source changed"
    );
    for index in 1..INITIAL_FILES {
        anyhow::ensure!(
            fs::read(
                fixture
                    .source
                    .with_file_name(format!("video-{index:03}.mkv"))
            )? == fixture.original,
            "discovery source {index} changed"
        );
    }
    if later {
        anyhow::ensure!(
            fs::read(fixture.source.with_file_name("created-after-restart.mkv"))?
                == fixture.original,
            "later discovered source changed"
        );
    }
    Ok(())
}

async fn read_job_ids(fixture: &Fixture, profile: Uuid) -> Result<BTreeSet<Uuid>> {
    let mut cursor = None;
    let mut identities = BTreeSet::new();
    for _ in 0..3 {
        let page =
            list_recent_media_jobs(fixture.config.pool(), 100, cursor, Some(profile)).await?;
        for job in &page {
            anyhow::ensure!(
                job.dry_run && !matches!(job.status_text.as_str(), "failed" | "cancelled"),
                "unexpected discovery outcome: {} {:?}",
                job.status_text,
                job.last_error
            );
            anyhow::ensure!(
                identities.insert(job.media_job_public_id),
                "job page repeated identity"
            );
        }
        anyhow::ensure!(
            identities.len() <= INITIAL_FILES + 1,
            "discovery duplicated job identities"
        );
        if page.len() < 100 {
            return Ok(identities);
        }
        let last = page
            .last()
            .ok_or_else(|| anyhow::anyhow!("full job page is empty"))?;
        let next = (last.queued_at, last.media_job_public_id);
        anyhow::ensure!(cursor != Some(next), "job page made no progress");
        cursor = Some(next);
    }
    Err(anyhow::anyhow!(
        "discovery exceeded fixture's bounded job pages"
    ))
}
