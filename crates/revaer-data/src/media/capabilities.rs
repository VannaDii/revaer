//! Stored-procedure access for media capability snapshots.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

const MEDIA_CAPABILITY_SNAPSHOT_RECORD_V1: &str =
    "SELECT media_capability_snapshot_record_v1($1, $2, $3, $4, $5, $6)";
const MEDIA_CAPABILITY_SNAPSHOT_LATEST_V1: &str = "SELECT media_capability_snapshot_id, ffmpeg_version, ffprobe_version, codec_name, encode_supported, decode_supported, observed_at FROM media_capability_snapshot ORDER BY observed_at DESC, media_capability_snapshot_id DESC LIMIT 1";

/// Capability snapshot insert payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCapabilitySnapshotInput<'a> {
    /// Actor public id.
    pub actor_public_id: Uuid,
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

/// Latest capability snapshot row.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct CapabilitySnapshotRow {
    /// Snapshot id.
    pub media_capability_snapshot_id: i64,
    /// ffmpeg version.
    pub ffmpeg_version: String,
    /// ffprobe version.
    pub ffprobe_version: String,
    /// codec name.
    pub codec_name: String,
    /// encode support.
    pub encode_supported: bool,
    /// decode support.
    pub decode_supported: bool,
    /// observation timestamp.
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
    sqlx::query_scalar::<_, i64>(MEDIA_CAPABILITY_SNAPSHOT_RECORD_V1)
        .bind(input.actor_public_id)
        .bind(input.ffmpeg_version)
        .bind(input.ffprobe_version)
        .bind(input.codec_name)
        .bind(input.encode_supported)
        .bind(input.decode_supported)
        .fetch_one(pool)
        .await
        .map_err(try_op("media capability snapshot record"))
}

/// Read the latest capability snapshot row.
///
/// # Errors
///
/// Returns an error when query execution fails.
pub async fn latest_capability_snapshot(pool: &PgPool) -> Result<Option<CapabilitySnapshotRow>> {
    sqlx::query_as::<_, CapabilitySnapshotRow>(MEDIA_CAPABILITY_SNAPSHOT_LATEST_V1)
        .fetch_optional(pool)
        .await
        .map_err(try_op("media capability snapshot latest"))
}

#[cfg(test)]
mod tests {
    use super::{
        RecordCapabilitySnapshotInput, latest_capability_snapshot, record_capability_snapshot,
    };
    use crate::media::schema_tests::setup_media_db;

    #[tokio::test]
    async fn record_capability_snapshot_row() -> anyhow::Result<()> {
        let db = match setup_media_db("record_capability_snapshot_row").await {
            Ok(db) => db,
            Err(err) => {
                eprintln!("skipping record_capability_snapshot_row: {err}");
                return Ok(());
            }
        };
        let snapshot_id = record_capability_snapshot(
            db.pool(),
            &RecordCapabilitySnapshotInput {
                actor_public_id: db.system_user_public_id,
                ffmpeg_version: "7.0",
                ffprobe_version: "7.0",
                codec_name: "hevc",
                encode_supported: true,
                decode_supported: true,
            },
        )
        .await?;

        assert!(snapshot_id > 0);

        let latest_row = latest_capability_snapshot(db.pool()).await?;
        assert!(latest_row.is_some());
        if let Some(row) = latest_row {
            assert_eq!(row.media_capability_snapshot_id, snapshot_id);
            assert_eq!(row.ffmpeg_version, "7.0");
        }
        Ok(())
    }
}
