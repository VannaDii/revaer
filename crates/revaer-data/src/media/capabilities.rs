//! Stored-procedure access for media capability snapshots.

use crate::error::{Result, try_op};
use sqlx::PgPool;
use uuid::Uuid;

const MEDIA_CAPABILITY_SNAPSHOT_RECORD_V1: &str =
    "SELECT media_capability_snapshot_record_v1($1, $2, $3, $4, $5, $6)";

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

#[cfg(test)]
mod tests {
    use super::{RecordCapabilitySnapshotInput, record_capability_snapshot};
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
        Ok(())
    }
}
