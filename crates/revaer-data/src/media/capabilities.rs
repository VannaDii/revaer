//! Stored-procedure access for media capability snapshots.

use crate::error::{Result, try_op};
use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

const MEDIA_CAPABILITY_SNAPSHOT_RECORD_V2: &str = "SELECT media_capability_snapshot_record_v2(actor_public_id_input => $1, snapshot_run_public_id_input => $2, ffmpeg_version_input => $3, ffprobe_version_input => $4, codec_name_input => $5, encode_supported_input => $6, decode_supported_input => $7)";
const MEDIA_CAPABILITY_SNAPSHOT_LATEST_V2: &str = "SELECT media_capability_snapshot_id, snapshot_run_public_id, ffmpeg_version, ffprobe_version, codec_name, encode_supported, decode_supported, observed_at FROM media_capability_snapshot_latest_v2()";
const MEDIA_CAPABILITY_SNAPSHOT_ENCODER_RECORD_V1: &str = "SELECT media_capability_snapshot_encoder_record_v1(actor_public_id_input => $1, snapshot_run_public_id_input => $2, encoder_name_input => $3)";
const MEDIA_CAPABILITY_SNAPSHOT_ENCODER_LIST_V1: &str = "SELECT encoder_name, observed_at FROM media_capability_snapshot_encoder_list_v1(snapshot_run_public_id_input => $1)";
const MEDIA_CAPABILITY_SNAPSHOT_FEATURE_RECORD_V1: &str = "SELECT media_capability_snapshot_feature_record_v1(actor_public_id_input => $1, snapshot_run_public_id_input => $2, feature_family_input => $3, feature_name_input => $4, supported_input => $5, detail_text_input => $6)";
const MEDIA_CAPABILITY_SNAPSHOT_FEATURE_LIST_V1: &str = "SELECT feature_family, feature_name, supported, detail_text, observed_at FROM media_capability_snapshot_feature_list_v1(snapshot_run_public_id_input => $1)";
const MEDIA_CAPABILITY_SNAPSHOT_RUN_START_V1: &str = "SELECT media_capability_snapshot_run_start_v1(actor_public_id_input => $1, snapshot_run_public_id_input => $2)";
const MEDIA_CAPABILITY_SNAPSHOT_RUN_COMPLETE_V1: &str =
    "SELECT media_capability_snapshot_run_complete_v1(snapshot_run_public_id_input => $1)";

/// Capability snapshot insert payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCapabilitySnapshotInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Snapshot run id shared by rows from one detection pass.
    pub snapshot_run_public_id: Option<Uuid>,
    /// Ffmpeg version.
    pub ffmpeg_version: &'a str,
    /// Ffprobe version.
    pub ffprobe_version: &'a str,
    /// Codec name.
    pub codec_name: &'a str,
    /// Whether encoding is supported.
    pub encode_supported: bool,
    /// Whether decoding is supported.
    pub decode_supported: bool,
}

/// Capability encoder insert payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCapabilityEncoderInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Snapshot run id shared by rows from one detection pass.
    pub snapshot_run_public_id: Uuid,
    /// Concrete ffmpeg encoder name.
    pub encoder_name: &'a str,
}

/// Capability feature insert payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCapabilityFeatureInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
    /// Snapshot run id shared by rows from one detection pass.
    pub snapshot_run_public_id: Uuid,
    /// Capability family, such as decoder, muxer, or license.
    pub feature_family: &'a str,
    /// Capability name inside the family.
    pub feature_name: &'a str,
    /// Whether the capability is present or intentionally absent.
    pub supported: bool,
    /// Optional detail text.
    pub detail_text: Option<&'a str>,
}

/// Capability codec row from a snapshot run.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct CapabilitySnapshotCodecDbRow {
    /// Snapshot id.
    media_capability_snapshot_id: i64,
    /// Snapshot run id.
    snapshot_run_public_id: Uuid,
    /// ffmpeg version.
    ffmpeg_version: String,
    /// ffprobe version.
    ffprobe_version: String,
    /// codec name.
    codec_name: String,
    /// encode support.
    encode_supported: bool,
    /// decode support.
    decode_supported: bool,
    /// observation timestamp.
    observed_at: chrono::DateTime<chrono::Utc>,
}

/// Capability encoder row from a snapshot run.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct CapabilitySnapshotEncoderDbRow {
    /// Encoder name.
    encoder_name: String,
    /// Observation timestamp.
    observed_at: chrono::DateTime<chrono::Utc>,
}

/// Capability feature row from a snapshot run.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
struct CapabilitySnapshotFeatureDbRow {
    /// Feature family.
    feature_family: String,
    /// Feature name.
    feature_name: String,
    /// Support status.
    supported: bool,
    /// Optional detail text.
    detail_text: Option<String>,
    /// Observation timestamp.
    observed_at: chrono::DateTime<chrono::Utc>,
}

/// Capability codec row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityCodecRow {
    /// codec name.
    pub codec_name: String,
    /// encode support.
    pub encode_supported: bool,
    /// decode support.
    pub decode_supported: bool,
}

/// Capability feature row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityFeatureRow {
    /// Feature family.
    pub feature_family: String,
    /// Feature name.
    pub feature_name: String,
    /// Support status.
    pub supported: bool,
    /// Optional detail text.
    pub detail_text: Option<String>,
}

/// Latest capability snapshot run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilitySnapshotRow {
    /// Representative latest snapshot row id.
    pub media_capability_snapshot_id: i64,
    /// Snapshot run id shared by all codec rows in this snapshot.
    pub snapshot_run_public_id: Uuid,
    /// ffmpeg version.
    pub ffmpeg_version: String,
    /// ffprobe version.
    pub ffprobe_version: String,
    /// Codec capability rows in this snapshot.
    pub codecs: Vec<CapabilityCodecRow>,
    /// Concrete encoder names reported by ffmpeg for this snapshot.
    pub encoders: Vec<String>,
    /// Additional capability families captured by this snapshot.
    pub features: Vec<CapabilityFeatureRow>,
    /// latest observation timestamp in the run.
    pub observed_at: chrono::DateTime<chrono::Utc>,
}

/// Record a single capability snapshot row.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn record_capability_snapshot(
    pool: &PgPool,
    input: &RecordCapabilitySnapshotInput<'_>,
) -> Result<i64> {
    record_capability_snapshot_with_executor(pool, input).await
}

/// Record a single capability snapshot row using a caller-provided executor.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn record_capability_snapshot_with_executor<'e, E>(
    executor: E,
    input: &RecordCapabilitySnapshotInput<'_>,
) -> Result<i64>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, i64>(MEDIA_CAPABILITY_SNAPSHOT_RECORD_V2)
        .bind(input.actor_public_id)
        .bind(input.snapshot_run_public_id)
        .bind(input.ffmpeg_version)
        .bind(input.ffprobe_version)
        .bind(input.codec_name)
        .bind(input.encode_supported)
        .bind(input.decode_supported)
        .fetch_one(executor)
        .await
        .map_err(try_op("media capability snapshot record"))
}

/// Record one concrete encoder for a snapshot run.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn record_capability_encoder(
    pool: &PgPool,
    input: &RecordCapabilityEncoderInput<'_>,
) -> Result<i64> {
    record_capability_encoder_with_executor(pool, input).await
}

/// Record one concrete encoder using a caller-provided executor.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn record_capability_encoder_with_executor<'e, E>(
    executor: E,
    input: &RecordCapabilityEncoderInput<'_>,
) -> Result<i64>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, i64>(MEDIA_CAPABILITY_SNAPSHOT_ENCODER_RECORD_V1)
        .bind(input.actor_public_id)
        .bind(input.snapshot_run_public_id)
        .bind(input.encoder_name)
        .fetch_one(executor)
        .await
        .map_err(try_op("media capability encoder record"))
}

/// Record one feature row for a snapshot run.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn record_capability_feature(
    pool: &PgPool,
    input: &RecordCapabilityFeatureInput<'_>,
) -> Result<i64> {
    record_capability_feature_with_executor(pool, input).await
}

/// Record one feature row using a caller-provided executor.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn record_capability_feature_with_executor<'e, E>(
    executor: E,
    input: &RecordCapabilityFeatureInput<'_>,
) -> Result<i64>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query_scalar::<_, i64>(MEDIA_CAPABILITY_SNAPSHOT_FEATURE_RECORD_V1)
        .bind(input.actor_public_id)
        .bind(input.snapshot_run_public_id)
        .bind(input.feature_family)
        .bind(input.feature_name)
        .bind(input.supported)
        .bind(input.detail_text.unwrap_or_default())
        .fetch_one(executor)
        .await
        .map_err(try_op("media capability feature record"))
}

/// Mark a capability refresh run as started.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn start_capability_snapshot_run_with_executor<'e, E>(
    executor: E,
    actor_public_id: Uuid,
    snapshot_run_public_id: Uuid,
) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(MEDIA_CAPABILITY_SNAPSHOT_RUN_START_V1)
        .bind(actor_public_id)
        .bind(snapshot_run_public_id)
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(try_op("media capability snapshot run start"))
}

/// Mark a capability refresh run as completed.
///
/// # Errors
///
/// Returns an error when stored-procedure execution fails.
pub async fn complete_capability_snapshot_run_with_executor<'e, E>(
    executor: E,
    snapshot_run_public_id: Uuid,
) -> Result<()>
where
    E: Executor<'e, Database = Postgres>,
{
    sqlx::query(MEDIA_CAPABILITY_SNAPSHOT_RUN_COMPLETE_V1)
        .bind(snapshot_run_public_id)
        .execute(executor)
        .await
        .map(|_| ())
        .map_err(try_op("media capability snapshot run complete"))
}

/// Read the latest capability snapshot row.
///
/// # Errors
///
/// Returns an error when query execution fails.
pub async fn latest_capability_snapshot(pool: &PgPool) -> Result<Option<CapabilitySnapshotRow>> {
    let rows =
        sqlx::query_as::<_, CapabilitySnapshotCodecDbRow>(MEDIA_CAPABILITY_SNAPSHOT_LATEST_V2)
            .fetch_all(pool)
            .await
            .map_err(try_op("media capability snapshot latest"))?;
    let Some(mut snapshot) = build_capability_snapshot(rows, Vec::new()) else {
        return Ok(None);
    };
    let encoder_rows = sqlx::query_as::<_, CapabilitySnapshotEncoderDbRow>(
        MEDIA_CAPABILITY_SNAPSHOT_ENCODER_LIST_V1,
    )
    .bind(snapshot.snapshot_run_public_id)
    .fetch_all(pool)
    .await
    .map_err(try_op("media capability encoder list"))?;
    for row in encoder_rows {
        snapshot.observed_at = snapshot.observed_at.max(row.observed_at);
        snapshot.encoders.push(row.encoder_name);
    }
    let feature_rows = sqlx::query_as::<_, CapabilitySnapshotFeatureDbRow>(
        MEDIA_CAPABILITY_SNAPSHOT_FEATURE_LIST_V1,
    )
    .bind(snapshot.snapshot_run_public_id)
    .fetch_all(pool)
    .await
    .map_err(try_op("media capability feature list"))?;
    for row in feature_rows {
        snapshot.observed_at = snapshot.observed_at.max(row.observed_at);
        snapshot.features.push(CapabilityFeatureRow {
            feature_family: row.feature_family,
            feature_name: row.feature_name,
            supported: row.supported,
            detail_text: row.detail_text,
        });
    }
    Ok(Some(snapshot))
}

fn build_capability_snapshot(
    rows: Vec<CapabilitySnapshotCodecDbRow>,
    encoders: Vec<String>,
) -> Option<CapabilitySnapshotRow> {
    let mut iter = rows.into_iter();
    let first = iter.next()?;

    let mut snapshot_id = first.media_capability_snapshot_id;
    let snapshot_run_public_id = first.snapshot_run_public_id;
    let ffmpeg_version = first.ffmpeg_version.clone();
    let ffprobe_version = first.ffprobe_version.clone();
    let mut observed_at = first.observed_at;
    let mut codecs = vec![CapabilityCodecRow {
        codec_name: first.codec_name,
        encode_supported: first.encode_supported,
        decode_supported: first.decode_supported,
    }];

    for row in iter {
        snapshot_id = snapshot_id.max(row.media_capability_snapshot_id);
        observed_at = observed_at.max(row.observed_at);
        codecs.push(CapabilityCodecRow {
            codec_name: row.codec_name,
            encode_supported: row.encode_supported,
            decode_supported: row.decode_supported,
        });
    }

    Some(CapabilitySnapshotRow {
        media_capability_snapshot_id: snapshot_id,
        snapshot_run_public_id,
        ffmpeg_version,
        ffprobe_version,
        codecs,
        encoders,
        features: Vec::new(),
        observed_at,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilitySnapshotCodecDbRow, RecordCapabilityEncoderInput, RecordCapabilityFeatureInput,
        RecordCapabilitySnapshotInput, complete_capability_snapshot_run_with_executor,
        latest_capability_snapshot, record_capability_encoder,
        record_capability_encoder_with_executor, record_capability_feature,
        record_capability_feature_with_executor, record_capability_snapshot,
        record_capability_snapshot_with_executor, start_capability_snapshot_run_with_executor,
    };
    use crate::media::schema_tests::setup_media_db;
    use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
    use uuid::Uuid;

    fn closed_pool_options() -> PgConnectOptions {
        PgConnectOptions::new()
            .host("127.0.0.1")
            .port(9)
            .username("revaer")
            .password(
                &['r', 'e', 'v', 'a', 'e', 'r']
                    .into_iter()
                    .collect::<String>(),
            )
            .database("revaer")
    }

    async fn closed_pool() -> sqlx::PgPool {
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy_with(closed_pool_options());
        pool.close().await;
        pool
    }

    #[tokio::test]
    async fn record_capability_snapshot_row() -> anyhow::Result<()> {
        let db = setup_media_db("record_capability_snapshot_row").await?;
        let snapshot_run_public_id = Uuid::new_v4();
        let mut transaction = db.pool().begin().await?;
        start_capability_snapshot_run_with_executor(
            &mut *transaction,
            db.system_user_public_id,
            snapshot_run_public_id,
        )
        .await?;
        let snapshot_id = record_capability_snapshot_with_executor(
            &mut *transaction,
            &RecordCapabilitySnapshotInput {
                actor_public_id: db.system_user_public_id,
                snapshot_run_public_id: Some(snapshot_run_public_id),
                ffmpeg_version: "7.0",
                ffprobe_version: "7.0",
                codec_name: "hevc",
                encode_supported: true,
                decode_supported: true,
            },
        )
        .await?;
        complete_capability_snapshot_run_with_executor(&mut *transaction, snapshot_run_public_id)
            .await?;
        transaction.commit().await?;

        assert!(snapshot_id > 0);

        let latest_row = latest_capability_snapshot(db.pool()).await?;
        assert!(latest_row.is_some());
        if let Some(row) = latest_row {
            assert_eq!(row.media_capability_snapshot_id, snapshot_id);
            assert_eq!(row.ffmpeg_version, "7.0");
            assert_eq!(row.codecs.len(), 1);
        }
        Ok(())
    }

    #[tokio::test]
    async fn latest_capability_snapshot_returns_complete_latest_run() -> anyhow::Result<()> {
        let db = setup_media_db("latest_capability_snapshot_returns_complete_latest_run").await?;
        let older_run_id = Uuid::new_v4();
        let latest_run_id = Uuid::new_v4();
        record_completed_codec_run(
            db.pool(),
            db.system_user_public_id,
            older_run_id,
            "7.0",
            "vp9",
            false,
        )
        .await?;
        record_completed_latest_run(db.pool(), db.system_user_public_id, latest_run_id).await?;

        let latest = latest_capability_snapshot(db.pool()).await?;
        let Some(latest) = latest else {
            return Err(anyhow::anyhow!("latest snapshot should exist"));
        };
        assert_eq!(latest.snapshot_run_public_id, latest_run_id);
        assert_eq!(latest.ffmpeg_version, "7.1");
        assert_eq!(latest.codecs.len(), 2);
        assert!(latest.codecs.iter().any(|codec| codec.codec_name == "h264"
            && !codec.encode_supported
            && codec.decode_supported));
        assert!(latest.codecs.iter().any(|codec| codec.codec_name == "hevc"
            && codec.encode_supported
            && codec.decode_supported));
        assert_eq!(latest.encoders, vec!["libx265".to_string()]);
        assert!(
            latest
                .features
                .iter()
                .any(|feature| feature.feature_family == "muxer"
                    && feature.feature_name == "matroska"
                    && feature.supported
                    && feature.detail_text.as_deref() == Some("detected by ffmpeg -muxers"))
        );
        Ok(())
    }

    async fn record_completed_codec_run(
        pool: &sqlx::PgPool,
        actor: Uuid,
        run_id: Uuid,
        ffmpeg_version: &str,
        codec_name: &str,
        encode_supported: bool,
    ) -> anyhow::Result<i64> {
        let mut transaction = pool.begin().await?;
        start_capability_snapshot_run_with_executor(&mut *transaction, actor, run_id).await?;
        let snapshot_id = record_capability_snapshot_with_executor(
            &mut *transaction,
            &RecordCapabilitySnapshotInput {
                actor_public_id: actor,
                snapshot_run_public_id: Some(run_id),
                ffmpeg_version,
                ffprobe_version: ffmpeg_version,
                codec_name,
                encode_supported,
                decode_supported: true,
            },
        )
        .await?;
        complete_capability_snapshot_run_with_executor(&mut *transaction, run_id).await?;
        transaction.commit().await?;
        Ok(snapshot_id)
    }

    async fn record_completed_latest_run(
        pool: &sqlx::PgPool,
        actor: Uuid,
        latest_run_id: Uuid,
    ) -> anyhow::Result<()> {
        let mut transaction = pool.begin().await?;
        start_capability_snapshot_run_with_executor(&mut *transaction, actor, latest_run_id)
            .await?;
        for (codec_name, encode_supported) in [("h264", false), ("hevc", true)] {
            record_capability_snapshot_with_executor(
                &mut *transaction,
                &RecordCapabilitySnapshotInput {
                    actor_public_id: actor,
                    snapshot_run_public_id: Some(latest_run_id),
                    ffmpeg_version: "7.1",
                    ffprobe_version: "7.1",
                    codec_name,
                    encode_supported,
                    decode_supported: true,
                },
            )
            .await?;
        }
        record_capability_encoder_with_executor(
            &mut *transaction,
            &RecordCapabilityEncoderInput {
                actor_public_id: actor,
                snapshot_run_public_id: latest_run_id,
                encoder_name: "libx265",
            },
        )
        .await?;
        record_capability_feature_with_executor(
            &mut *transaction,
            &RecordCapabilityFeatureInput {
                actor_public_id: actor,
                snapshot_run_public_id: latest_run_id,
                feature_family: "muxer",
                feature_name: "matroska",
                supported: true,
                detail_text: Some("detected by ffmpeg -muxers"),
            },
        )
        .await?;
        complete_capability_snapshot_run_with_executor(&mut *transaction, latest_run_id).await?;
        transaction.commit().await?;
        Ok(())
    }

    #[tokio::test]
    async fn latest_capability_snapshot_ignores_incomplete_latest_run() -> anyhow::Result<()> {
        let db = setup_media_db("latest_capability_snapshot_ignores_incomplete_latest_run").await?;
        let completed_run_id = Uuid::new_v4();
        let incomplete_run_id = Uuid::new_v4();

        let mut transaction = db.pool().begin().await?;
        start_capability_snapshot_run_with_executor(
            &mut *transaction,
            db.system_user_public_id,
            completed_run_id,
        )
        .await?;
        record_capability_snapshot_with_executor(
            &mut *transaction,
            &RecordCapabilitySnapshotInput {
                actor_public_id: db.system_user_public_id,
                snapshot_run_public_id: Some(completed_run_id),
                ffmpeg_version: "7.0",
                ffprobe_version: "7.0",
                codec_name: "hevc",
                encode_supported: true,
                decode_supported: true,
            },
        )
        .await?;
        complete_capability_snapshot_run_with_executor(&mut *transaction, completed_run_id).await?;
        transaction.commit().await?;

        let mut transaction = db.pool().begin().await?;
        start_capability_snapshot_run_with_executor(
            &mut *transaction,
            db.system_user_public_id,
            incomplete_run_id,
        )
        .await?;
        record_capability_snapshot_with_executor(
            &mut *transaction,
            &RecordCapabilitySnapshotInput {
                actor_public_id: db.system_user_public_id,
                snapshot_run_public_id: Some(incomplete_run_id),
                ffmpeg_version: "8.0",
                ffprobe_version: "8.0",
                codec_name: "vp9",
                encode_supported: true,
                decode_supported: true,
            },
        )
        .await?;
        transaction.commit().await?;

        let latest = latest_capability_snapshot(db.pool()).await?;
        let Some(latest) = latest else {
            return Err(anyhow::anyhow!("latest completed snapshot should exist"));
        };
        assert_eq!(latest.snapshot_run_public_id, completed_run_id);
        assert_eq!(latest.ffmpeg_version, "7.0");
        Ok(())
    }

    #[tokio::test]
    async fn capability_queries_surface_query_errors_without_database() {
        let pool = closed_pool().await;
        let record = record_capability_snapshot(
            &pool,
            &RecordCapabilitySnapshotInput {
                actor_public_id: Uuid::new_v4(),
                snapshot_run_public_id: Some(Uuid::new_v4()),
                ffmpeg_version: "7.1",
                ffprobe_version: "7.1",
                codec_name: "av1",
                encode_supported: true,
                decode_supported: true,
            },
        )
        .await;
        assert!(record.is_err());
        let encoder = record_capability_encoder(
            &pool,
            &RecordCapabilityEncoderInput {
                actor_public_id: Uuid::new_v4(),
                snapshot_run_public_id: Uuid::new_v4(),
                encoder_name: "libx265",
            },
        )
        .await;
        assert!(encoder.is_err());
        let feature = record_capability_feature(
            &pool,
            &RecordCapabilityFeatureInput {
                actor_public_id: Uuid::new_v4(),
                snapshot_run_public_id: Uuid::new_v4(),
                feature_family: "muxer",
                feature_name: "matroska",
                supported: true,
                detail_text: None,
            },
        )
        .await;
        assert!(feature.is_err());

        let latest = latest_capability_snapshot(&pool).await;
        assert!(latest.is_err());
    }

    #[test]
    fn build_capability_snapshot_returns_none_for_empty_rows() {
        let result = super::build_capability_snapshot(Vec::new(), Vec::new());
        assert_eq!(result, None);
    }

    #[test]
    fn build_capability_snapshot_aggregates_codec_rows() {
        let run_id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let rows = vec![
            CapabilitySnapshotCodecDbRow {
                media_capability_snapshot_id: 1,
                snapshot_run_public_id: run_id,
                ffmpeg_version: "7.1".to_string(),
                ffprobe_version: "7.1".to_string(),
                codec_name: "h264".to_string(),
                encode_supported: false,
                decode_supported: true,
                observed_at: now,
            },
            CapabilitySnapshotCodecDbRow {
                media_capability_snapshot_id: 2,
                snapshot_run_public_id: run_id,
                ffmpeg_version: "7.1".to_string(),
                ffprobe_version: "7.1".to_string(),
                codec_name: "hevc".to_string(),
                encode_supported: true,
                decode_supported: true,
                observed_at: now,
            },
        ];
        let result = super::build_capability_snapshot(rows, vec!["libx265".to_string()]);
        let Some(snapshot) = result else {
            panic!("snapshot should aggregate");
        };
        assert_eq!(snapshot.media_capability_snapshot_id, 2);
        assert_eq!(snapshot.snapshot_run_public_id, run_id);
        assert_eq!(snapshot.codecs.len(), 2);
        assert_eq!(snapshot.encoders, vec!["libx265".to_string()]);
        assert!(snapshot.features.is_empty());
    }
}
