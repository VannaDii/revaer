//! Bounded portable configuration reads in one read-only database snapshot.

use sqlx::{PgPool, Postgres, Transaction};

use super::configuration::{
    MediaCompatibilityTargetRow, MediaDesiredTargetRow, MediaDesiredTargetStreamRow,
    MediaPolicyProfileRow,
};
use crate::error::{Result, try_op};

/// Complete immutable profile body with logical bindings and its exact version.
#[derive(sqlx::FromRow)]
pub struct PortableProfileRow {
    /// Stable logical profile key.
    pub profile_key: String,
    /// Exact immutable body version.
    pub version: i32,
    /// Operator label.
    pub display_name: String,
    /// Operator description.
    pub description: String,
    /// Requested enablement.
    pub enabled: bool,
    /// Profile-only mutation restriction.
    pub dry_run_only: bool,
    /// Logical target key.
    pub desired_target_key: String,
    /// Explicit target version.
    pub desired_target_version: i32,
    /// Logical policy key.
    pub policy_key: String,
    /// Explicit policy version.
    pub policy_version: i32,
    /// Logical output root.
    pub output_root_key: String,
    /// Logical workspace root.
    pub workspace_root_key: String,
    /// Present optional logical backup root.
    pub backup_root_key: Option<String>,
    /// Present optional logical quarantine root.
    pub quarantine_root_key: Option<String>,
}

/// Latest association intent without host identity or readiness evidence.
#[derive(sqlx::FromRow)]
pub struct PortableAssociationRow {
    /// Exact logical association key.
    pub association_key: String,
    /// Logical profile key replacing its host-local public UUID.
    pub profile_key: String,
    /// Exact immutable profile version.
    pub profile_version: i32,
    /// Logical source root.
    pub source_root_key: String,
    /// Explicit relative prefix, including whole-root empty string.
    pub root_relative_path: String,
    /// Requested manual mode.
    pub manual_enabled: bool,
    /// Requested watcher mode.
    pub watcher_enabled: bool,
    /// Requested schedule mode.
    pub schedule_enabled: bool,
}

/// Referenced host-local diagnostic mapping, never import authority.
#[derive(sqlx::FromRow)]
pub struct LocalRootPathRow {
    /// Referenced logical key.
    pub logical_key: String,
    /// Absent for unresolved draft bindings.
    pub canonical_path: Option<String>,
}

/// Bounded resource rows; lookahead rows are retained so callers reject overflow.
pub struct PortableSnapshot {
    /// Immutable profile heads and referenced versions.
    pub profiles: Vec<PortableProfileRow>,
    /// Latest complete association intents.
    pub associations: Vec<PortableAssociationRow>,
    /// Compatibility catalog versions.
    pub compatibility_targets: Vec<MediaCompatibilityTargetRow>,
    /// Complete desired target graphs, unless a resource or child bound overflowed.
    pub targets: Vec<(MediaDesiredTargetRow, Vec<MediaDesiredTargetStreamRow>)>,
    /// Policy versions with explicit output settings.
    pub policies: Vec<MediaPolicyProfileRow>,
}

impl PortableSnapshot {
    /// Count returned top-level resource rows, including overflow lookahead.
    #[must_use]
    pub const fn resource_count(&self) -> usize {
        self.profiles.len()
            + self.associations.len()
            + self.compatibility_targets.len()
            + self.targets.len()
            + self.policies.len()
    }

    /// Count logical binding and ordered stream children.
    #[must_use]
    pub fn child_count(&self) -> usize {
        self.associations.len()
            + self
                .profiles
                .iter()
                .map(|p| {
                    2 + usize::from(p.backup_root_key.is_some())
                        + usize::from(p.quarantine_root_key.is_some())
                })
                .sum::<usize>()
            + self
                .targets
                .iter()
                .map(|(_, streams)| streams.len())
                .sum::<usize>()
    }
}

/// Read every exported body from one restricted, read-only repeatable-read snapshot.
///
/// # Errors
/// Propagates procedure, privilege, decoding, commit and rollback failures.
pub async fn read_snapshot(pool: &PgPool) -> Result<PortableSnapshot> {
    read_export_snapshot(pool, false)
        .await
        .map(|(snapshot, _)| snapshot)
}

/// Read configuration and referenced diagnostic paths in the same snapshot.
///
/// # Errors
/// Propagates procedure, decoding, transaction and privilege failures.
pub async fn read_local_snapshot(
    pool: &PgPool,
) -> Result<(PortableSnapshot, Vec<LocalRootPathRow>)> {
    read_export_snapshot(pool, true).await
}

async fn read_export_snapshot(
    pool: &PgPool,
    include_local_paths: bool,
) -> Result<(PortableSnapshot, Vec<LocalRootPathRow>)> {
    let mut transaction = pool
        .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .await
        .map_err(try_op("begin portable media snapshot"))?;
    let result = async {
        let snapshot = read_rows(&mut transaction).await?;
        let paths = if include_local_paths {
            sqlx::query_as("SELECT * FROM media_local_root_paths_v1()")
                .fetch_all(&mut *transaction)
                .await
                .map_err(try_op("read local diagnostic root paths"))?
        } else {
            Vec::new()
        };
        Ok((snapshot, paths))
    }
    .await;
    match result {
        Ok(snapshot) => {
            transaction
                .commit()
                .await
                .map_err(try_op("commit portable media snapshot"))?;
            Ok(snapshot)
        }
        Err(error) => {
            transaction
                .rollback()
                .await
                .map_err(try_op("rollback portable media snapshot"))?;
            Err(error)
        }
    }
}

async fn read_rows(transaction: &mut Transaction<'_, Postgres>) -> Result<PortableSnapshot> {
    let profiles = sqlx::query_as("SELECT * FROM media_portable_profile_versions_v1()")
        .fetch_all(&mut **transaction)
        .await
        .map_err(try_op("read portable profiles"))?;
    let associations = sqlx::query_as("SELECT * FROM media_portable_associations_v1()")
        .fetch_all(&mut **transaction)
        .await
        .map_err(try_op("read portable associations"))?;
    let compatibility_targets =
        sqlx::query_as("SELECT * FROM media_compatibility_target_list_v1() LIMIT 129")
            .fetch_all(&mut **transaction)
            .await
            .map_err(try_op("read portable compatibility targets"))?;
    let policies = sqlx::query_as("SELECT * FROM media_policy_profile_list_v1() LIMIT 129")
        .fetch_all(&mut **transaction)
        .await
        .map_err(try_op("read portable policies"))?;
    let targets: Vec<MediaDesiredTargetRow> =
        sqlx::query_as("SELECT * FROM media_desired_target_list_v1() LIMIT 129")
            .fetch_all(&mut **transaction)
            .await
            .map_err(try_op("read portable target headers"))?;
    let mut snapshot = PortableSnapshot {
        profiles,
        associations,
        compatibility_targets,
        policies,
        targets: targets
            .into_iter()
            .map(|target| (target, Vec::new()))
            .collect(),
    };
    if snapshot.resource_count() > 128 || snapshot.child_count() > 4_096 {
        return Ok(snapshot);
    }
    let mut remaining = 4_097 - snapshot.child_count();
    for (target, streams) in &mut snapshot.targets {
        let limit = i64::try_from(remaining).map_err(|_| {
            try_op("portable stream read bound")(sqlx::Error::Protocol(
                "invalid portable bound".into(),
            ))
        })?;
        *streams = sqlx::query_as("SELECT * FROM media_desired_target_stream_list_v5(media_desired_target_profile_public_id_input => $1) LIMIT $2")
            .bind(target.media_desired_target_profile_public_id).bind(limit)
            .fetch_all(&mut **transaction).await.map_err(try_op("read portable target streams"))?;
        remaining -= streams.len();
        if remaining == 0 {
            break;
        }
    }
    Ok(snapshot)
}
